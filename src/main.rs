use clap::{Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod ansi;
mod banner;
mod config;
mod input;
mod obs;
mod service;

use config::config_path;
use input::{find_keyboards, spawn_keyboard_reader};

/// Main event loop poll interval (ms).
const EVENT_LOOP_POLL_MS: u64 = 10;
/// Minimum time between duplicate key presses (debounce window).
const DEBOUNCE_MS: u64 = 50;

#[derive(Parser, Debug)]
#[command(
    name = "obs-hotkey",
    version = env!("CARGO_PKG_VERSION"),
    about = "Lightweight daemon for controlling OBS Studio with global hotkeys on Wayland and X11",
    infer_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long = "config", global = true, help = "Path to config file")]
    config: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[command(about = "Run the obs-hotkey daemon")]
    Daemon {
        #[arg(long = "config", help = "Path to config file")]
        config: Option<PathBuf>,
    },
    #[command(about = "Set up systemd user service for auto-start on login")]
    Setup,
    #[command(about = "Remove systemd user service and stop obs-hotkey")]
    Teardown {
        #[arg(long = "purge", help = "Also remove config directory")]
        purge: bool,
    },
    #[command(about = "Show service status, config state, and OBS connectivity")]
    Status,
}

struct ActionContext {
    client: obs::OBSClient,
    screenshot_source: String,
    screenshot_dir: String,
    mic_name: String,
    mic_volume: f64,
}

#[derive(Clone)]
struct ActionBinding {
    id: String,
    key_name: String,
    chord: input::KeyChord,
    label: String,
    actions: Vec<Arc<dyn Fn() + Send + Sync>>,
}

const ACTION_DEFINITIONS: &[(&str, &str)] = &[
    ("toggle_recording", "Toggle Recording"),
    ("toggle_pause", "Toggle Pause/Resume"),
    ("toggle_streaming", "Toggle Streaming"),
    ("screenshot", "Screenshot"),
    ("toggle_mute_mic", "Toggle Mic Mute"),
    ("set_mic_volume", "Set Mic Volume"),
    ("toggle_studio_mode", "Toggle Studio Mode"),
    ("toggle_replay_buffer", "Toggle Replay Buffer"),
    ("save_replay", "Save Replay"),
];

fn action_label(action: &str) -> &str {
    ACTION_DEFINITIONS
        .iter()
        .find_map(|(name, label)| (*name == action).then_some(*label))
        .unwrap_or(action)
}

fn is_known_action(action: &str) -> bool {
    ACTION_DEFINITIONS.iter().any(|(name, _)| *name == action)
}

fn action_labels(actions: &[String]) -> String {
    actions
        .iter()
        .map(|action| action_label(action))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn build_action_map(ctx: &ActionContext) -> HashMap<&'static str, Arc<dyn Fn() + Send + Sync>> {
    HashMap::from([
        (
            "toggle_recording",
            Arc::new({
                let c = ctx.client.clone();
                move || c.toggle_recording()
            }) as _,
        ),
        (
            "toggle_pause",
            Arc::new({
                let c = ctx.client.clone();
                move || c.toggle_pause()
            }) as _,
        ),
        (
            "toggle_streaming",
            Arc::new({
                let c = ctx.client.clone();
                move || c.toggle_streaming()
            }) as _,
        ),
        (
            "screenshot",
            Arc::new({
                let c = ctx.client.clone();
                let src = ctx.screenshot_source.clone();
                let dir = ctx.screenshot_dir.clone();
                move || c.screenshot(&src, &dir)
            }) as _,
        ),
        (
            "toggle_mute_mic",
            Arc::new({
                let c = ctx.client.clone();
                let mic = ctx.mic_name.clone();
                move || c.toggle_mute_mic(&mic)
            }) as _,
        ),
        (
            "set_mic_volume",
            Arc::new({
                let c = ctx.client.clone();
                let mic = ctx.mic_name.clone();
                let volume = ctx.mic_volume;
                move || c.set_mic_volume(&mic, volume)
            }) as _,
        ),
        (
            "toggle_studio_mode",
            Arc::new({
                let c = ctx.client.clone();
                move || c.toggle_studio_mode()
            }) as _,
        ),
        (
            "toggle_replay_buffer",
            Arc::new({
                let c = ctx.client.clone();
                move || c.toggle_replay_buffer()
            }) as _,
        ),
        (
            "save_replay",
            Arc::new({
                let c = ctx.client.clone();
                move || c.save_replay()
            }) as _,
        ),
    ])
}

fn run_actions(actions: Vec<Arc<dyn Fn() + Send + Sync>>) {
    for action in actions {
        action();
    }
}

fn build_action_bindings(cfg: &config::AppConfig, ctx: &ActionContext) -> Vec<ActionBinding> {
    let action_map = build_action_map(ctx);
    let mut bindings = Vec::new();

    let single_action_bindings = [
        ("toggle_recording", cfg.hotkeys.toggle_recording.as_str()),
        ("toggle_pause", cfg.hotkeys.toggle_pause.as_str()),
        ("toggle_streaming", cfg.hotkeys.toggle_streaming.as_str()),
        ("screenshot", cfg.hotkeys.screenshot.as_str()),
        ("toggle_mute_mic", cfg.hotkeys.toggle_mute_mic.as_str()),
        (
            "toggle_studio_mode",
            cfg.hotkeys.toggle_studio_mode.as_str(),
        ),
        (
            "toggle_replay_buffer",
            cfg.hotkeys.toggle_replay_buffer.as_str(),
        ),
        ("save_replay", cfg.hotkeys.save_replay.as_str()),
    ];

    for (action, key_name) in single_action_bindings {
        if key_name.trim().is_empty() {
            continue;
        }

        let chord = match input::KeyChord::parse(key_name) {
            Ok(chord) => chord,
            Err(e) => {
                log::warn!("Invalid hotkey for {}: {}", action_label(action), e);
                continue;
            }
        };

        let action_fn = match action_map.get(action) {
            Some(action_fn) => action_fn.clone(),
            None => {
                log::warn!("Unknown action '{}' for {}", action, action_label(action));
                continue;
            }
        };

        bindings.push(ActionBinding {
            id: action.to_string(),
            key_name: key_name.to_string(),
            chord,
            label: action_label(action).to_string(),
            actions: vec![action_fn],
        });
    }

    for combo in &cfg.hotkey_combos {
        let key_spec = combo.key_spec();
        if key_spec.trim().is_empty() {
            log::warn!("Ignoring hotkey_combo '{}' with an empty key", combo.name);
            continue;
        }

        let chord = match input::KeyChord::parse(&key_spec) {
            Ok(chord) => chord,
            Err(e) => {
                log::warn!("Invalid hotkey for combo '{}': {}", combo.name, e);
                continue;
            }
        };

        let mut actions = Vec::with_capacity(combo.actions.len());
        let mut valid = true;
        for action in &combo.actions {
            if !is_known_action(action) {
                log::warn!(
                    "Unknown action '{}' in hotkey_combo '{}'",
                    action,
                    combo.name
                );
                valid = false;
                break;
            }
            actions.push(
                action_map
                    .get(action.as_str())
                    .expect("known action must have a runner")
                    .clone(),
            );
        }

        if !valid || actions.is_empty() {
            continue;
        }

        bindings.push(ActionBinding {
            id: format!("combo:{}", combo.name),
            key_name: key_spec,
            chord,
            label: action_labels(&combo.actions),
            actions,
        });
    }

    bindings
}

fn build_banner_bindings(bindings: &[ActionBinding]) -> Vec<banner::HotkeyBinding> {
    bindings
        .iter()
        .map(|binding| banner::HotkeyBinding {
            key_name: binding.key_name.clone(),
            label: binding.label.clone(),
        })
        .collect()
}

fn run_daemon(config_path_str: &str) -> anyhow::Result<()> {
    let config_path_str = config::expand_home(config_path_str);
    let config_path = PathBuf::from(&config_path_str);
    let dir_path = config_path.parent().unwrap_or(&config_path);

    config::ensure_config(dir_path, &config_path)?;

    let cfg = config::load_config(&config_path)?;
    log::info!("Loaded config from: {}", config_path.display());

    let ws_url = if cfg.obs_host.is_empty() {
        "ws://localhost:4455".to_string()
    } else {
        // Validate host — refuse to start if it looks malformed.
        // This prevents confusing errors from the WebSocket layer.
        if cfg.obs_host.contains('\0') {
            anyhow::bail!("obs_host contains a null byte — check your config");
        }
        if cfg.obs_host.len() > 4096 {
            anyhow::bail!("obs_host is suspiciously long (>4096 chars)");
        }
        cfg.obs_host.clone()
    };

    let client = obs::OBSClient::new(ws_url.clone());

    let ctx = ActionContext {
        client: client.clone(),
        screenshot_source: cfg.screenshot_source.clone(),
        screenshot_dir: config::expand_home(&cfg.screenshot_dir),
        mic_name: cfg.mic_name.clone(),
        mic_volume: cfg.mic_volume,
    };

    let action_bindings = build_action_bindings(&cfg, &ctx);
    let banner_bindings = build_banner_bindings(&action_bindings);

    for binding in &action_bindings {
        log::info!("  {} → {}", binding.chord.display(), binding.label);
    }

    let autostart = service::is_autostart_enabled();
    banner::print_banner(&cfg, &banner_bindings, autostart);

    if action_bindings.is_empty() {
        anyhow::bail!("No valid hotkeys configured");
    }

    let keyboard_paths = find_keyboards()?;
    if keyboard_paths.is_empty() {
        anyhow::bail!("No keyboard devices found! Make sure you're in the input group.");
    }
    log::info!("Found {} keyboard device(s)", keyboard_paths.len());
    for p in &keyboard_paths {
        log::info!("  - {}", p.display());
    }

    // Background connection thread — tries once to connect at startup.
    // On failure, subsequent hotkey actions will trigger reconnection automatically.
    std::thread::spawn(move || {
        if let Err(e) = ctx.client.connect() {
            log::warn!("Initial OBS connection failed: {}", e);
        } else {
            log::info!("Connected to OBS!");
        }
    });

    let (_device_handles, rx_channels): (Vec<_>, Vec<_>) = keyboard_paths
        .into_iter()
        .enumerate()
        .map(|(i, path)| {
            let (handle, rx) = spawn_keyboard_reader(path, i);
            (handle, rx)
        })
        .unzip();

    println!(
        "  {} Hotkeys ready — connecting to OBS in background{}",
        ansi::muted("~"),
        ansi::RESET
    );

    // Ctrl-C cleanly exits the event loop
    ctrlc::set_handler(|| {}).expect("error setting Ctrl-C handler");

    let mut pressed_keys: HashSet<u16> = HashSet::new();
    let mut active_bindings: HashSet<String> = HashSet::new();
    let mut last_press: HashMap<u16, Instant> = HashMap::new();
    let debounce = Duration::from_millis(DEBOUNCE_MS);

    loop {
        for rx in &rx_channels {
            while let Ok(event) = rx.try_recv() {
                match event.value {
                    1 => {
                        if !pressed_keys.insert(event.code) {
                            continue;
                        }

                        let now = Instant::now();
                        if let Some(last) = last_press.get(&event.code) {
                            if now.duration_since(*last) < debounce {
                                continue;
                            }
                        }
                        last_press.insert(event.code, now);

                        for binding in &action_bindings {
                            if binding.chord.matches(&pressed_keys)
                                && active_bindings.insert(binding.id.clone())
                            {
                                let actions = binding.actions.clone();
                                let label = binding.label.clone();
                                std::thread::spawn(move || run_actions(actions));
                                log::info!("Triggered hotkey: {}", label);
                            }
                        }
                    }
                    0 => {
                        pressed_keys.remove(&event.code);
                        active_bindings.retain(|binding_id| {
                            action_bindings
                                .iter()
                                .find(|binding| binding.id == *binding_id)
                                .map(|binding| !binding.chord.matches(&pressed_keys))
                                .unwrap_or(true)
                        });
                    }
                    _ => {}
                }
            }
        }

        std::thread::sleep(Duration::from_millis(EVENT_LOOP_POLL_MS));
    }
}

fn print_quickstart() {
    use ansi::*;
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!(
        "  {}  obs-hotkey {}  {}",
        BOLD,
        env!("CARGO_PKG_VERSION"),
        RESET
    );
    println!("  {}  Wayland-compatible OBS hotkey daemon{}", DIM, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();

    println!("  {}", heading("▶ Quick Start"));
    println!();
    println!("  {}", heading("1.") + " " + "Enable OBS WebSocket Server");
    println!("     Open OBS → Tools → WebSocket Server Settings");
    println!("     {}Enable{}", GREEN, RESET);
    println!("     (port 4455, no auth needed)");
    println!();
    println!(
        "  {}",
        heading("2.") + " " + "Add yourself to the input group"
    );
    println!("     {}", muted("sudo usermod -aG input $(whoami)"));
    println!("     {}", muted("(then log out and back in)"));
    println!();
    println!("  {}", heading("3.") + " " + "Set up auto-start on login");
    println!("     {}", key("obs-hotkey setup"));
    println!();
    println!("  {}", heading("4.") + " " + "Run the daemon");
    println!("     {}", key("obs-hotkey daemon"));
    println!();

    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
    println!("  {}", heading("Commands:"));
    println!("  {:>12}  Run the hotkey daemon", key("daemon"));
    println!("  {:>12}  Install systemd service", key("setup"));
    println!("  {:>12}  Remove systemd service", key("teardown"));
    println!("  {:>12}  Check service & OBS", key("status"));
    println!();
    println!("  {}", heading("Flags:"));
    println!("  {:>12}  Use a custom config file", key("--config"));
    println!("  {:>12}  Show version", key("--version"));
    println!("  {:>12}  Show full help", key("--help"));
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
    println!(
        "  {}  Config:  {}",
        muted("~"),
        muted("~/.config/obs-hotkey/hotkeys.json")
    );
    println!(
        "  {}  Logs:    {}",
        muted("~"),
        muted("journalctl --user -u obs-hotkey.service -f")
    );
    println!();
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    let cfg_path = cli
        .config
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| config_path().to_string_lossy().to_string());

    let config_path_for_status = cfg_path.clone();
    let config_path_for_setup = cfg_path.clone();

    match cli.command.as_ref() {
        Some(Commands::Daemon { config }) => {
            let daemon_cfg = config
                .clone()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| cfg_path.clone());
            if let Err(e) = run_daemon(&daemon_cfg) {
                log::error!("Fatal error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Setup) => {
            service::run_setup(&config_path_for_setup);
        }
        Some(Commands::Teardown { purge }) => {
            service::run_teardown(*purge);
        }
        Some(Commands::Status) => {
            service::run_status(&config_path_for_status);
        }
        None => {
            print_quickstart();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_cli_no_args_shows_no_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey"]).unwrap();
        assert!(cli.command.is_none());
        assert!(cli.config.is_none());
    }

    #[test]
    fn test_cli_daemon_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "daemon"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Daemon { config: None })
        ));
    }

    #[test]
    fn test_cli_daemon_with_config() {
        let cli = Cli::try_parse_from(["obs-hotkey", "daemon", "--config", "/path/to/config.json"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Daemon { config: Some(_) })
        ));
    }

    #[test]
    fn test_cli_setup_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "setup"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Setup)));
    }

    #[test]
    fn test_cli_teardown_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "teardown"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Teardown { purge: false })
        ));
    }

    #[test]
    fn test_cli_teardown_with_purge() {
        let cli = Cli::try_parse_from(["obs-hotkey", "teardown", "--purge"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Teardown { purge: true })
        ));
    }

    #[test]
    fn test_cli_status_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Status)));
    }

    #[test]
    fn test_cli_global_config_flag() {
        let cli = Cli::try_parse_from(["obs-hotkey", "--config", "/path/to/config.json"]).unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.json")));
    }

    #[test]
    fn test_cli_config_flag_with_subcommand() {
        let cli =
            Cli::try_parse_from(["obs-hotkey", "--config", "/path/config.json", "status"]).unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("/path/config.json")));
        assert!(matches!(cli.command, Some(Commands::Status)));
    }

    #[test]
    fn test_cli_help_flag() {
        let result = Cli::try_parse_from(["obs-hotkey", "--help"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_version_flag() {
        let result = Cli::try_parse_from(["obs-hotkey", "--version"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_unknown_subcommand() {
        let result = Cli::try_parse_from(["obs-hotkey", "invalid-subcommand"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_action_label_lookup() {
        assert_eq!(action_label("toggle_recording"), "Toggle Recording");
        assert_eq!(action_label("set_mic_volume"), "Set Mic Volume");
        assert_eq!(action_label("unknown"), "unknown");
    }

    #[test]
    fn test_action_labels_join_combo() {
        let actions = vec!["toggle_recording".to_string(), "set_mic_volume".to_string()];
        assert_eq!(action_labels(&actions), "Toggle Recording + Set Mic Volume");
    }

    #[test]
    fn test_build_banner_bindings_preserves_combo_label() {
        let chord = input::KeyChord::parse("ctrl + f1").unwrap();
        let bindings = vec![ActionBinding {
            id: "combo:record_and_mic".to_string(),
            key_name: "ctrl + f1".to_string(),
            chord,
            label: "Toggle Recording + Set Mic Volume".to_string(),
            actions: Vec::new(),
        }];

        let banner_bindings = build_banner_bindings(&bindings);
        assert_eq!(banner_bindings.len(), 1);
        assert_eq!(
            banner_bindings[0].label,
            "Toggle Recording + Set Mic Volume"
        );
    }
}
