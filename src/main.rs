use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod banner;
mod config;
mod input;
mod obs;
mod service;

use config::{config_path, expand_home, load_config};
use input::{find_keyboards, get_key_code, key_name, spawn_keyboard_reader, KeyEvent};
use obs::OBSClient;

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

#[derive(Subcommand, Debug)]
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

fn run_daemon(config_path_str: &str) -> anyhow::Result<()> {
    let config_path = PathBuf::from(config_path_str);
    let dir_path = config_path.parent().unwrap_or(&config_path);

    config::ensure_config(dir_path, &config_path)?;

    let cfg = load_config(&config_path)?;
    log::info!("Loaded config from: {}", config_path.display());

    let ws_url = if cfg.obs_host.is_empty() {
        "ws://localhost:4455".to_string()
    } else {
        cfg.obs_host.clone()
    };

    let bindings = vec![
        input::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_recording.clone(),
            action: "toggle_recording",
            label: "Toggle Recording",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_pause.clone(),
            action: "toggle_pause",
            label: "Toggle Pause/Resume",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_streaming.clone(),
            action: "toggle_streaming",
            label: "Toggle Streaming",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.screenshot.clone(),
            action: "screenshot",
            label: "Screenshot",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_mute_mic.clone(),
            action: "toggle_mute_mic",
            label: "Toggle Mic Mute",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_studio_mode.clone(),
            action: "toggle_studio_mode",
            label: "Toggle Studio Mode",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.toggle_replay_buffer.clone(),
            action: "toggle_replay_buffer",
            label: "Toggle Replay Buffer",
        },
        input::HotkeyBinding {
            key_name: cfg.hotkeys.save_replay.clone(),
            action: "save_replay",
            label: "Save Replay",
        },
    ];

    let client = OBSClient::new(ws_url.clone());

    for b in &bindings {
        if !b.key_name.is_empty() {
            if let Some(code) = get_key_code(&b.key_name) {
                log::info!("  {} → {}", b.key_name, b.label);
            } else if !b.key_name.is_empty() {
                log::warn!("Warning: unknown key '{}' for {}", b.key_name, b.label);
            }
        }
    }

    let autostart = service::is_autostart_enabled();
    banner::print_banner(&cfg, &bindings, autostart);

    if bindings.iter().all(|b| b.key_name.is_empty() || get_key_code(&b.key_name).is_none()) {
        anyhow::bail!("No valid hotkeys configured");
    }

    let mut hotkey_actions: std::collections::HashMap<u16, Box<dyn Fn() + Send + Sync>> =
        std::collections::HashMap::new();

    for b in &bindings {
        if b.key_name.is_empty() {
            continue;
        }
        if let Some(code) = get_key_code(&b.key_name) {
            let action: Box<dyn Fn() + Send + Sync> = match b.action {
                "toggle_recording" => Box::new({
                    let c = (&client as &dyn obs::ObsActions).clone_void();
                    move || c.toggle_recording()
                }),
                "toggle_pause" => Box::new({
                    let c = (&client as &dyn obs::ObsActions).clone_void();
                    move || c.toggle_pause()
                }),
                "toggle_streaming" => Box::new({
                    let c = (&client as &dyn obs::ObsActions).clone_void();
                    move || c.toggle_streaming()
                }),
                "screenshot" => Box::new({
                    let c = client.clone();
                    let src = cfg.screenshot_source.clone();
                    let dir = expand_home(&cfg.screenshot_dir);
                    move || c.screenshot(&src, &dir)
                }),
                "toggle_mute_mic" => Box::new({
                    let c = client.clone();
                    let mic = cfg.mic_name.clone();
                    move || c.toggle_mute_mic(&mic)
                }),
                "toggle_studio_mode" => Box::new({
                    let c = (&client as &dyn obs::ObsActions).clone_void();
                    move || c.toggle_studio_mode()
                }),
                "toggle_replay_buffer" => Box::new({
                    let c = (&client as &dyn obs::ObsActions).clone_void();
                    move || c.toggle_replay_buffer()
                }),
                "save_replay" => Box::new({
                    let c = (&client as &dyn obs::ObsActions).clone_void();
                    move || c.save_replay()
                }),
                _ => continue,
            };
            hotkey_actions.insert(code, action);
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
        if client.connect().is_ok() {
            break;
        }
        retries += 1;
        log::info!("Connection attempt {}/{} failed, waiting {}s...", retries, MAX_RETRIES, RETRY_DELAY_SECS);
        std::thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
    }

    if !client.is_connected() {
        log::info!("Failed to connect to OBS after {} attempts.", MAX_RETRIES);
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

    let (close_tx, close_rx) = std::sync::mpsc::channel::<()>();
    let client_clone = client.clone();
    let reconnect_handle = std::thread::spawn(move || {
        let interval = std::time::Duration::from_secs(RECONNECT_INTERVAL_SECS);
        loop {
            std::thread::sleep(interval);
            if close_rx.try_recv().is_ok() {
                break;
            }
            if !client_clone.is_connected() {
                log::info!("Attempting to reconnect to OBS...");
                let _ = client_clone.connect();
            }
        }
    });

    let (sig_tx, sig_rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 1];
        std::io::stdin().read_exact(&mut buf).ok();
        let _ = sig_tx.send(());
    });

    loop {
        for (i, rx) in rx_channels.iter().enumerate() {
            while let Ok(event) = rx.try_recv() {
                if event.value == 1 {
                    if let Some(action) = hotkey_actions.get(&event.code) {
                        action();
                    }
                }
            }
        }

        if sig_rx.try_recv().is_ok() {
            log::info!("Shutting down...");
            let _ = close_tx.send(());
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    reconnect_handle.join().ok();
    for h in device_handles {
        h.device File.close().ok();
    }

    Ok(())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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