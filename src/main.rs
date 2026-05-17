use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod banner;
mod config;
mod input;
mod obs;
mod service;

use config::config_path;
use input::{find_keyboards, get_key_code, spawn_keyboard_reader};

const MAX_RETRIES: usize = 10;
const RETRY_DELAY_SECS: u64 = 30;
const RECONNECT_INTERVAL_SECS: u64 = 60;

#[derive(Parser, Debug)]
#[command(
    name = "obs-hotkey",
    version = "1.0.0",
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
    #[command(about = "Run the obs-hotkey daemon (default)")]
    Daemon,
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

#[derive(Clone)]
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
    ];

    let client = obs::OBSClient::new(ws_url.clone());

    for b in &bindings {
        if !b.key_name.is_empty() {
            if get_key_code(&b.key_name).is_some() {
                log::info!("  {} → {}", b.key_name, b.label);
            } else if !b.key_name.is_empty() {
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

    if bindings.iter().all(|b| b.key_name.is_empty() || get_key_code(&b.key_name).is_none()) {
        anyhow::bail!("No valid hotkeys configured");
    }

    let mut hotkey_actions: std::collections::HashMap<
        u16,
        std::sync::Arc<std::sync::Mutex<obs::OBSClient>>,
    > = std::collections::HashMap::new();

    for b in &bindings {
        if b.key_name.is_empty() {
            continue;
        }
        if let Some(code) = get_key_code(&b.key_name) {
            hotkey_actions.insert(code, ctx.client.clone());
        }
    }

    let keyboards = find_keyboards()?;
    if keyboards.is_empty() {
        anyhow::bail!("No keyboard devices found! Make sure you're in the input group.");
    }
    log::info!("Found {} keyboard device(s)", keyboards.len());
    for k in &keyboards {
        log::info!("  - {}", k.name());
    }

    log::info!("Connecting to OBS WebSocket at {}...", ws_url);
    let mut retries = 0;
    while retries < MAX_RETRIES {
        if ctx.client.connect().is_ok() {
            break;
        }
        retries += 1;
        log::info!(
            "Connection attempt {}/{} failed, waiting {}s...",
            retries,
            MAX_RETRIES,
            RETRY_DELAY_SECS
        );
        std::thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
    }

    if !ctx.client.is_connected() {
        log::info!(
            "Failed to connect to OBS after {} attempts.",
            MAX_RETRIES
        );
        log::info!("Hotkeys are ready but will only work when OBS is running.");
    }

    let (device_handles, rx_channels): (Vec<_>, Vec<_>) = keyboards
        .into_iter()
        .enumerate()
        .map(|(i, dev)| {
            let (handle, rx) = spawn_keyboard_reader(dev, i);
            (handle, rx)
        })
        .unzip();

    let client_for_reconnect = ctx.client.clone();
    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_stop_clone = should_stop.clone();
    let _reconnect_handle = std::thread::spawn(move || {
        let interval = std::time::Duration::from_secs(RECONNECT_INTERVAL_SECS);
        loop {
            std::thread::sleep(interval);
            if should_stop_clone.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            if !client_for_reconnect.is_connected() {
                log::info!("Attempting to reconnect to OBS...");
                let _ = client_for_reconnect.connect();
            }
        }
    });

    let action_client = ctx.client.clone();
    let action_ctx = ctx.clone();

    let action_map: std::collections::HashMap<&str, Box<dyn Fn() + Send + Sync>> =
        std::collections::HashMap::from([
            ("toggle_recording", Box::new(move || action_client.toggle_recording()) as _),
            (
                "toggle_pause",
                Box::new(move || action_client.toggle_pause()) as _,
            ),
            (
                "toggle_streaming",
                Box::new(move || action_client.toggle_streaming()) as _,
            ),
            (
                "screenshot",
                Box::new({
                    let c = action_ctx.client.clone();
                    let src = action_ctx.screenshot_source.clone();
                    let dir = action_ctx.screenshot_dir.clone();
                    move || c.screenshot(&src, &dir)
                }) as _,
            ),
            (
                "toggle_mute_mic",
                Box::new({
                    let c = action_ctx.client.clone();
                    let mic = action_ctx.mic_name.clone();
                    move || c.toggle_mute_mic(&mic)
                }) as _,
            ),
            (
                "toggle_studio_mode",
                Box::new(move || action_client.toggle_studio_mode()) as _,
            ),
            (
                "toggle_replay_buffer",
                Box::new(move || action_client.toggle_replay_buffer()) as _,
            ),
            ("save_replay", Box::new(move || action_client.save_replay()) as _),
        ]);

    let mut binding_actions: std::collections::HashMap<u16, std::sync::Arc<dyn Fn() + Send + Sync>> =
        std::collections::HashMap::new();
    for b in &bindings {
        if b.key_name.is_empty() {
            continue;
        }
        if let Some(code) = get_key_code(&b.key_name) {
            if let Some(action) = action_map.get(b.action) {
                binding_actions.insert(code, std::sync::Arc::new((action.clone()) as Box<dyn Fn() + Send + Sync>));
            }
        }
    }

    loop {
        let mut all_closed = true;
        for rx in &rx_channels {
            while let Ok(event) = rx.try_recv() {
                if event.value == 1 {
                    if let Some(action) = binding_actions.get(&event.code) {
                        action();
                    }
                }
            }
            if rx.is_empty() {
            } else {
                all_closed = false;
            }
        }

        if all_closed && rx_channels.iter().all(|rx| rx.is_empty()) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    log::info!("All keyboard devices closed, shutting down...");
    should_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    ctx.client.close();

    Ok(())
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let cli = Cli::parse();

    let cfg_path = cli
        .config
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| config_path().to_string_lossy().to_string());

    let config_path_for_status = cfg_path.clone();
    let config_path_for_setup = cfg_path.clone();

    match cli.command.as_ref().unwrap_or(&Commands::Daemon) {
        Commands::Daemon => {
            if let Err(e) = run_daemon(&cfg_path) {
                log::error!("Fatal error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Setup => {
            service::run_setup(&config_path_for_setup);
        }
        Commands::Teardown { purge } => {
            service::run_teardown(*purge);
        }
        Commands::Status => {
            service::run_status(&config_path_for_status);
        }
    }
}