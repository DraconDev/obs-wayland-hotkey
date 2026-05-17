use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod ansi;
mod banner;
mod config;
mod input;
mod obs;
mod service;

use config::config_path;
use input::{find_keyboards, get_key_code, spawn_keyboard_reader};

const RETRY_DELAY_SECS: u64 = 30;
const RECONNECT_INTERVAL_SECS: u64 = 60;

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
}

fn run_daemon(config_path_str: &str) -> anyhow::Result<()> {
    let config_path = PathBuf::from(config_path_str);
    let dir_path = config_path.parent().unwrap_or(&config_path);

    config::ensure_config(dir_path, &config_path)?;

    let cfg = config::load_config(&config_path)?;
    log::info!("Loaded config from: {}", config_path.display());

    let ws_url = if cfg.obs_host.is_empty() {
        "ws://localhost:4455".to_string()
    } else {
        cfg.obs_host.clone()
    };

    let bindings = vec![
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_recording.clone(),
            action: "toggle_recording",
            label: "Toggle Recording",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_pause.clone(),
            action: "toggle_pause",
            label: "Toggle Pause/Resume",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_streaming.clone(),
            action: "toggle_streaming",
            label: "Toggle Streaming",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.screenshot.clone(),
            action: "screenshot",
            label: "Screenshot",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_mute_mic.clone(),
            action: "toggle_mute_mic",
            label: "Toggle Mic Mute",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_studio_mode.clone(),
            action: "toggle_studio_mode",
            label: "Toggle Studio Mode",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_replay_buffer.clone(),
            action: "toggle_replay_buffer",
            label: "Toggle Replay Buffer",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.save_replay.clone(),
            action: "save_replay",
            label: "Save Replay",
        },
        banner::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_virtual_cam.clone(),
            action: "toggle_virtual_cam",
            label: "Toggle Virtual Camera",
        },
    ];

    let client = obs::OBSClient::new(ws_url.clone());

    for b in &bindings {
        if !b.key_name.is_empty() {
            if get_key_code(&b.key_name).is_some() {
                log::info!("  {} → {}", b.key_name, b.label);
            } else {
                log::warn!("Warning: unknown key '{}' for {}", b.key_name, b.label);
            }
        }
    }

    let autostart = service::is_autostart_enabled();

    let ctx = ActionContext {
        client: client.clone(),
        screenshot_source: cfg.screenshot_source.clone(),
        screenshot_dir: config::expand_home(&cfg.screenshot_dir),
        mic_name: cfg.mic_name.clone(),
    };

    banner::print_banner(&cfg, &bindings, autostart);

    if bindings
        .iter()
        .all(|b| b.key_name.is_empty() || get_key_code(&b.key_name).is_none())
    {
        anyhow::bail!("No valid hotkeys configured");
    }

    let action_map: std::collections::HashMap<&str, std::sync::Arc<dyn Fn() + Send + Sync>> =
        std::collections::HashMap::from([
            (
                "toggle_recording",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.toggle_recording()
                }) as _,
            ),
            (
                "toggle_pause",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.toggle_pause()
                }) as _,
            ),
            (
                "toggle_streaming",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.toggle_streaming()
                }) as _,
            ),
            (
                "screenshot",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    let src = ctx.screenshot_source.clone();
                    let dir = ctx.screenshot_dir.clone();
                    move || c.screenshot(&src, &dir)
                }) as _,
            ),
            (
                "toggle_mute_mic",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    let mic = ctx.mic_name.clone();
                    move || c.toggle_mute_mic(&mic)
                }) as _,
            ),
            (
                "toggle_studio_mode",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.toggle_studio_mode()
                }) as _,
            ),
            (
                "toggle_replay_buffer",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.toggle_replay_buffer()
                }) as _,
            ),
            (
                "save_replay",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.save_replay()
                }) as _,
            ),
            (
                "toggle_virtual_cam",
                std::sync::Arc::new({
                    let c = ctx.client.clone();
                    move || c.toggle_virtual_cam()
                }) as _,
            ),
        ]);

    let mut binding_actions: std::collections::HashMap<u16, std::sync::Arc<dyn Fn() + Send + Sync>> =
        std::collections::HashMap::new();
    for b in &bindings {
        if b.key_name.is_empty() {
            continue;
        }
        if let Some(code) = get_key_code(&b.key_name) {
            if let Some(action) = action_map.get(b.action) {
                binding_actions.insert(code, action.clone());
            }
        }
    }

    let keyboard_paths = find_keyboards()?;
    if keyboard_paths.is_empty() {
        anyhow::bail!("No keyboard devices found! Make sure you're in the input group.");
    }
    log::info!("Found {} keyboard device(s)", keyboard_paths.len());
    for p in &keyboard_paths {
        log::info!("  - {}", p.display());
    }

    use ansi::*;

    // Background reconnection thread — retries forever with visible output
    let client_for_reconnect = ctx.client.clone();
    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_stop_clone = should_stop.clone();

    let reconnect_handle = std::thread::spawn(move || {
        loop {
            if should_stop_clone.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            if client_for_reconnect.is_connected() {
                // Already connected — check again in 60s
                std::thread::sleep(std::time::Duration::from_secs(RECONNECT_INTERVAL_SECS));
                continue;
            }
            match client_for_reconnect.connect() {
                Ok(()) => {
                    println!("  {} Connected to OBS!{}", ok(""), RESET);
                }
                Err(e) => {
                    println!(
                        "  {} Could not reach OBS: {} — retrying in {}s...",
                        muted("~"),
                        e,
                        RETRY_DELAY_SECS
                    );
                    std::thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
                }
            }
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

    println!("  {} Hotkeys ready — connecting to OBS in background{}", muted("~"), RESET);

    let close_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    ctrlc::set_handler({
        let should_stop_clone = should_stop.clone();
        let close_flag_clone = close_flag.clone();
        move || {
            close_flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            should_stop_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }).expect("error setting Ctrl-C handler");

    loop {
        if close_flag.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("Shutting down...");
            break;
        }

        for rx in &rx_channels {
            while let Ok(event) = rx.try_recv() {
                if event.value == 1 {
                    if let Some(action) = binding_actions.get(&event.code) {
                        action();
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    should_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    ctx.client.close();
    let _ = reconnect_handle.join();

    Ok(())
}

fn print_quickstart() {
    use ansi::*;
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  obs-hotkey {}  {}", BOLD, env!("CARGO_PKG_VERSION"), RESET);
    println!("  {}  Wayland-compatible OBS hotkey daemon{}", DIM, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();

    println!("  {}", heading("▶ Quick Start"));
    println!();
    println!("  {}" , heading("1.") + " " + "Enable OBS WebSocket Server");
    println!("     Open OBS → Tools → WebSocket Server Settings");
    println!("     {}Enable{}", GREEN, RESET);
    println!("     (port 4455, no auth needed)");
    println!();
    println!("  {}" , heading("2.") + " " + "Add yourself to the input group");
    println!("     {}", muted("sudo usermod -aG input $(whoami)"));
    println!("     {}", muted("(then log out and back in)"));
    println!();
    println!("  {}" , heading("3.") + " " + "Set up auto-start on login");
    println!("     {}", key("obs-hotkey setup"));
    println!();
    println!("  {}" , heading("4.") + " " + "Run the daemon");
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
    println!("  {}  Config:  {}", muted("~"), muted("~/.config/obs-hotkey/hotkeys.json"));
    println!("  {}  Logs:    {}", muted("~"), muted("journalctl --user -u obs-hotkey.service -f"));
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

    #[test]
    fn test_cli_no_args_shows_no_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey"]).unwrap();
        assert!(cli.command.is_none());
        assert!(cli.config.is_none());
    }

    #[test]
    fn test_cli_daemon_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "daemon"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Daemon { config: None })));
    }

    #[test]
    fn test_cli_daemon_with_config() {
        let cli = Cli::try_parse_from(["obs-hotkey", "daemon", "--config", "/path/to/config.json"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Daemon { config: Some(_) })));
    }

    #[test]
    fn test_cli_setup_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "setup"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Setup)));
    }

    #[test]
    fn test_cli_teardown_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "teardown"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Teardown { purge: false })));
    }

    #[test]
    fn test_cli_teardown_with_purge() {
        let cli = Cli::try_parse_from(["obs-hotkey", "teardown", "--purge"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Teardown { purge: true })));
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
        let cli = Cli::try_parse_from(["obs-hotkey", "--config", "/path/config.json", "status"]).unwrap();
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
}