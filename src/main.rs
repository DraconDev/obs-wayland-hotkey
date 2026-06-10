use clap::{Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod ansi;
mod banner;
mod config;
mod http_api;
mod input;
mod notify;
mod obs;
mod service;

use config::{config_path, ActionItem};
use input::{find_keyboards_with_filter, spawn_keyboard_reader};

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
    #[command(about = "Run a startup diagnostic checklist for a live show")]
    Doctor,
    #[command(
        about = "Trigger a single OBS action or named macro once, without running the daemon. Useful for scripts and systemd timers."
    )]
    Action {
        #[arg(
            help = "Action or macro name, e.g. toggle_recording, switch_scene, countdown_record"
        )]
        name: String,
        #[arg(long = "scene", help = "Scene name, required for switch_scene")]
        scene: Option<String>,
        #[arg(long = "config", help = "Path to config file")]
        config: Option<PathBuf>,
    },
}

#[derive(Clone)]
struct ActionContext {
    client: obs::OBSClient,
    screenshot_source: String,
    screenshot_dir: String,
    mic_name: String,
    mic_volume: f64,
}

#[derive(Clone)]
struct ComboStep {
    action: Arc<dyn Fn() + Send + Sync>,
    delay: Duration,
}

#[derive(Clone)]
struct ActionBinding {
    id: String,
    key_name: String,
    chord: input::KeyChord,
    label: String,
    steps: Vec<ComboStep>,
    /// Optional steps to run when the chord transitions from matched to
    /// unmatched (i.e. the operator releases a key in the chord).
    release_steps: Vec<ComboStep>,
}

const ACTION_DEFINITIONS: &[(&str, &str)] = &[
    ("toggle_recording", "Toggle Recording"),
    ("start_recording", "Start Recording"),
    ("stop_recording", "Stop Recording"),
    ("toggle_pause", "Toggle Pause/Resume"),
    ("toggle_streaming", "Toggle Streaming"),
    ("start_streaming", "Start Streaming"),
    ("stop_streaming", "Stop Streaming"),
    ("screenshot", "Screenshot"),
    ("toggle_mute_mic", "Toggle Mic Mute"),
    ("set_mic_volume", "Set Mic Volume"),
    ("toggle_studio_mode", "Toggle Studio Mode"),
    ("toggle_replay_buffer", "Toggle Replay Buffer"),
    ("start_replay_buffer", "Start Replay Buffer"),
    ("stop_replay_buffer", "Stop Replay Buffer"),
    ("save_replay", "Save Replay"),
    ("switch_scene", "Switch Scene"),
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

fn action_item_label(item: &ActionItem) -> String {
    let base = action_label(item.name()).to_string();
    if let Some(scene) = item.scene() {
        if !scene.is_empty() {
            return format!("{} ({})", base, scene);
        }
    }
    base
}

fn macro_label(cfg: &config::AppConfig, macro_name: &str) -> String {
    cfg.macros
        .iter()
        .find(|m| m.name == macro_name)
        .map(|m| format!("macro:{}", action_labels_with_cfg(cfg, &m.actions)))
        .unwrap_or_else(|| format!("macro:{}", macro_name))
}

fn action_item_label_with_cfg(cfg: &config::AppConfig, item: &ActionItem) -> String {
    if cfg.macros.iter().any(|m| m.name == item.name()) {
        macro_label(cfg, item.name())
    } else {
        action_item_label(item)
    }
}

#[allow(dead_code)]
fn action_labels(actions: &[ActionItem]) -> String {
    actions
        .iter()
        .map(action_item_label)
        .collect::<Vec<_>>()
        .join(" + ")
}

fn action_labels_with_cfg(cfg: &config::AppConfig, actions: &[ActionItem]) -> String {
    actions
        .iter()
        .map(|item| action_item_label_with_cfg(cfg, item))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Build a closure that runs the given action, capturing the parameter it
/// needs. Returns None if the action is unknown.
pub(crate) fn build_action_runner(
    action: &str,
    scene: Option<&str>,
    ctx: &ActionContext,
    cfg: &config::AppConfig,
) -> Option<Arc<dyn Fn() + Send + Sync>> {
    match action {
        "toggle_recording" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.toggle_recording()
        })),
        "start_recording" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.start_recording()
        })),
        "stop_recording" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.stop_recording()
        })),
        "toggle_pause" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.toggle_pause()
        })),
        "toggle_streaming" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.toggle_streaming()
        })),
        "start_streaming" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.start_streaming()
        })),
        "stop_streaming" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.stop_streaming()
        })),
        "screenshot" => Some(Arc::new({
            let c = ctx.client.clone();
            let src = ctx.screenshot_source.clone();
            let dir = ctx.screenshot_dir.clone();
            move || c.screenshot(&src, &dir)
        })),
        "toggle_mute_mic" => Some(Arc::new({
            let c = ctx.client.clone();
            let mic = ctx.mic_name.clone();
            move || c.toggle_mute_mic(&mic)
        })),
        "set_mic_volume" => Some(Arc::new({
            let c = ctx.client.clone();
            let mic = ctx.mic_name.clone();
            let volume = ctx.mic_volume;
            move || c.set_mic_volume(&mic, volume)
        })),
        "toggle_studio_mode" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.toggle_studio_mode()
        })),
        "toggle_replay_buffer" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.toggle_replay_buffer()
        })),
        "start_replay_buffer" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.start_replay_buffer()
        })),
        "stop_replay_buffer" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.stop_replay_buffer()
        })),
        "save_replay" => Some(Arc::new({
            let c = ctx.client.clone();
            move || c.save_replay()
        })),
        "switch_scene" => {
            let scene_name = scene.unwrap_or("").to_string();
            Some(Arc::new({
                let c = ctx.client.clone();
                move || c.set_current_scene(&scene_name)
            }))
        }
        _ if cfg.macros.iter().any(|m| m.name == action) => {
            let macro_name = action.to_string();
            let ctx = ctx.clone();
            let cfg = cfg.clone();
            Some(Arc::new(move || {
                if let Err(e) = run_macro_by_name(&macro_name, &ctx, &cfg) {
                    log::warn!("Macro '{}' failed: {}", macro_name, e);
                }
            }))
        }
        _ => None,
    }
}

fn run_steps(steps: Vec<ComboStep>) {
    for step in steps {
        if !step.delay.is_zero() {
            std::thread::sleep(step.delay);
        }
        (step.action)();
    }
}

pub(crate) fn run_macro_by_name(
    macro_name: &str,
    ctx: &ActionContext,
    cfg: &config::AppConfig,
) -> anyhow::Result<()> {
    run_macro_by_name_inner(macro_name, ctx, cfg, &mut Vec::new())
}

fn run_macro_by_name_inner(
    macro_name: &str,
    ctx: &ActionContext,
    cfg: &config::AppConfig,
    stack: &mut Vec<String>,
) -> anyhow::Result<()> {
    if stack.iter().any(|name| name == macro_name) {
        let mut cycle = stack.clone();
        cycle.push(macro_name.to_string());
        anyhow::bail!("macro cycle detected: {}", cycle.join(" -> "));
    }
    let macro_config = cfg
        .macros
        .iter()
        .find(|m| m.name == macro_name)
        .ok_or_else(|| anyhow::anyhow!("macro '{}' not found", macro_name))?;

    stack.push(macro_name.to_string());
    for (index, item) in macro_config.actions.iter().enumerate() {
        let delay_ms = macro_config
            .action_delays_ms
            .get(index)
            .copied()
            .unwrap_or(0);
        if delay_ms > config::MAX_ACTION_DELAY_MS {
            anyhow::bail!(
                "macro '{}' action delay {} ms exceeds maximum {} ms",
                macro_name,
                delay_ms,
                config::MAX_ACTION_DELAY_MS
            );
        }
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
        let action = item.name();
        if cfg.macros.iter().any(|m| m.name == action) {
            run_macro_by_name_inner(action, ctx, cfg, stack)?;
        } else {
            let runner = build_action_runner(action, item.scene(), ctx, cfg).ok_or_else(|| {
                anyhow::anyhow!("unknown action '{}' in macro '{}'", action, macro_name)
            })?;
            runner();
        }
    }
    stack.pop();
    Ok(())
}

fn build_steps(
    actions: &[ActionItem],
    delays_ms: &[u64],
    ctx: &ActionContext,
    cfg: &config::AppConfig,
    combo_name: &str,
    field: &str,
) -> Option<Vec<ComboStep>> {
    let mut steps = Vec::with_capacity(actions.len());
    for (index, item) in actions.iter().enumerate() {
        let action = item.name();
        let runner = match build_action_runner(action, item.scene(), ctx, cfg) {
            Some(r) => r,
            None => {
                log::warn!(
                    "Unknown action '{}' in hotkey_combo '{}' {} (index {})",
                    action,
                    combo_name,
                    field,
                    index
                );
                return None;
            }
        };
        let delay_ms = delays_ms.get(index).copied().unwrap_or(0);
        steps.push(ComboStep {
            action: runner,
            delay: Duration::from_millis(delay_ms),
        });
    }
    if steps.is_empty() {
        None
    } else {
        Some(steps)
    }
}

pub(crate) fn validate_combo_actions(cfg: &config::AppConfig) -> anyhow::Result<()> {
    let mut combo_names = HashSet::new();

    for combo in &cfg.hotkey_combos {
        if !combo_names.insert(combo.name.as_str()) {
            anyhow::bail!("duplicate hotkey_combo name '{}'", combo.name);
        }

        for item in combo.actions.iter().chain(combo.release_actions.iter()) {
            if !is_known_action(item.name()) && !cfg.macros.iter().any(|m| m.name == item.name()) {
                anyhow::bail!(
                    "unknown action or macro '{}' in hotkey_combo '{}'",
                    item.name(),
                    combo.name
                );
            }
            if item.name() == "switch_scene"
                && is_known_action(item.name())
                && item.scene().map(str::trim).unwrap_or("").is_empty()
            {
                anyhow::bail!(
                    "hotkey_combo '{}' uses switch_scene without a scene name",
                    combo.name
                );
            }
        }

        let needs_mic = combo
            .actions
            .iter()
            .chain(combo.release_actions.iter())
            .any(|item| matches!(item.name(), "set_mic_volume" | "toggle_mute_mic"));
        if needs_mic && cfg.mic_name.trim().is_empty() {
            anyhow::bail!(
                "hotkey_combo '{}' uses set_mic_volume or toggle_mute_mic but mic_name is empty",
                combo.name
            );
        }
    }

    Ok(())
}

pub(crate) fn validate_configured_chords(cfg: &config::AppConfig) -> anyhow::Result<()> {
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
        if !key_name.trim().is_empty() {
            input::KeyChord::parse(key_name)
                .map_err(|e| anyhow::anyhow!("invalid hotkey for {}: {}", action, e))?;
        }
    }

    for combo in &cfg.hotkey_combos {
        let key_spec = combo.key_spec();
        if !key_spec.trim().is_empty() {
            input::KeyChord::parse(&key_spec)
                .map_err(|e| anyhow::anyhow!("invalid hotkey for {}: {}", combo.name, e))?;
        }
    }

    Ok(())
}

pub(crate) fn run_action_by_name(
    action_name: &str,
    scene: Option<&str>,
    ctx: &ActionContext,
    cfg: &config::AppConfig,
) -> anyhow::Result<()> {
    if cfg.macros.iter().any(|m| m.name == action_name) {
        if scene.is_some() {
            anyhow::bail!("macros do not accept --scene");
        }
        return run_macro_by_name(action_name, ctx, cfg);
    }
    if !is_known_action(action_name) {
        anyhow::bail!(
            "unknown action '{}'. Run `obs-hotkey --help` or see the README for the list of supported actions.",
            action_name
        );
    }
    if action_name == "switch_scene" && scene.map(str::trim).unwrap_or("").is_empty() {
        anyhow::bail!("switch_scene requires a scene name");
    }
    if matches!(action_name, "set_mic_volume" | "toggle_mute_mic") && ctx.mic_name.trim().is_empty()
    {
        anyhow::bail!(
            "{} requires 'mic_name' to be set in the config",
            action_name
        );
    }

    let runner = build_action_runner(action_name, scene, ctx, cfg)
        .ok_or_else(|| anyhow::anyhow!("action '{}' has no runner", action_name))?;
    runner();
    Ok(())
}

fn build_action_bindings(cfg: &config::AppConfig, ctx: &ActionContext) -> Vec<ActionBinding> {
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

        let Some(runner) = build_action_runner(action, None, ctx, cfg) else {
            log::warn!("Unknown action '{}' for {}", action, action_label(action));
            continue;
        };

        bindings.push(ActionBinding {
            id: action.to_string(),
            key_name: key_name.to_string(),
            chord,
            label: action_label(action).to_string(),
            steps: vec![ComboStep {
                action: runner,
                delay: Duration::ZERO,
            }],
            release_steps: Vec::new(),
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

        let Some(steps) = build_steps(
            &combo.actions,
            &combo.action_delays_ms,
            ctx,
            cfg,
            &combo.name,
            "actions",
        ) else {
            continue;
        };
        let release_steps = build_steps(
            &combo.release_actions,
            &combo.release_action_delays_ms,
            ctx,
            cfg,
            &combo.name,
            "release_actions",
        )
        .unwrap_or_default();

        bindings.push(ActionBinding {
            id: format!("combo:{}", combo.name),
            key_name: key_spec,
            chord,
            label: action_labels_with_cfg(cfg, &combo.actions),
            steps,
            release_steps,
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
    validate_combo_actions(&cfg)?;
    config::validate_macros(&cfg)?;
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

    http_api::spawn(
        cfg.http.clone(),
        cfg.clone(),
        ctx.clone(),
        cfg.notify.clone(),
    );

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

    let keyboard_paths = find_keyboards_with_filter(&cfg.allowed_devices)?;
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
                                let steps = binding.steps.clone();
                                let label = binding.label.clone();
                                let log_label = label.clone();
                                let notify_cfg = cfg.notify.clone();
                                std::thread::spawn(move || {
                                    notify::send_notification(
                                        &notify_cfg,
                                        &format!("Triggered {}", label),
                                    );
                                    run_steps(steps);
                                });
                                log::info!("Triggered hotkey: {}", log_label);
                            }
                        }
                    }
                    0 => {
                        pressed_keys.remove(&event.code);
                        // Find bindings that were active and whose chord no
                        // longer matches. Run their release_steps (push-to-
                        // record / push-to-talk) and drop them from the
                        // active set.
                        let mut to_release: Vec<(String, Vec<ComboStep>, String)> = Vec::new();
                        active_bindings.retain(|binding_id| {
                            let binding = match action_bindings.iter().find(|b| b.id == *binding_id)
                            {
                                Some(b) => b,
                                None => return false,
                            };
                            if binding.chord.matches(&pressed_keys) {
                                return true;
                            }
                            if !binding.release_steps.is_empty() {
                                to_release.push((
                                    binding.id.clone(),
                                    binding.release_steps.clone(),
                                    binding.label.clone(),
                                ));
                            }
                            false
                        });
                        for (id, steps, label) in to_release {
                            let notify_cfg = cfg.notify.clone();
                            let log_label = label.clone();
                            std::thread::spawn(move || {
                                notify::send_notification(
                                    &notify_cfg,
                                    &format!("Released {}", label),
                                );
                                run_steps(steps);
                            });
                            log::info!("Released hotkey ({}): {}", id, log_label);
                        }
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
        Some(Commands::Doctor) => {
            if let Err(e) = service::run_doctor(&config_path_for_status) {
                log::error!("Doctor found problems: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Action {
            name,
            scene,
            config,
        }) => {
            let action_cfg = config
                .clone()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| cfg_path.clone());
            if let Err(e) = run_one_shot_action(&action_cfg, name, scene.as_deref()) {
                log::error!("Action failed: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            print_quickstart();
        }
    }
}

/// Run a single OBS action or named macro and exit. Does not start the event loop or watch
/// any keyboards. Useful for systemd timers, scripts, and one-off triggers.
fn run_one_shot_action(
    config_path_str: &str,
    action_name: &str,
    scene: Option<&str>,
) -> anyhow::Result<()> {
    let config_path = PathBuf::from(config::expand_home(config_path_str));
    let dir_path = config_path.parent().unwrap_or(&config_path);
    config::ensure_config(dir_path, &config_path)?;
    let cfg = config::load_config(&config_path)?;

    if !is_known_action(action_name) && !cfg.macros.iter().any(|m| m.name == action_name) {
        anyhow::bail!(
            "unknown action or macro '{}'. Run `obs-hotkey --help` or see the README for the list of supported actions and macros.",
            action_name
        );
    }

    let ws_url = if cfg.obs_host.is_empty() {
        "ws://localhost:4455".to_string()
    } else {
        cfg.obs_host.clone()
    };

    let client = obs::OBSClient::new(ws_url);
    client.connect()?;

    let ctx = ActionContext {
        client: client.clone(),
        screenshot_source: cfg.screenshot_source.clone(),
        screenshot_dir: config::expand_home(&cfg.screenshot_dir),
        mic_name: cfg.mic_name.clone(),
        mic_volume: cfg.mic_volume,
    };

    if is_known_action(action_name) {
        if action_name == "switch_scene" && scene.map(str::trim).unwrap_or("").is_empty() {
            anyhow::bail!("switch_scene requires a scene name");
        }
        if matches!(action_name, "set_mic_volume" | "toggle_mute_mic")
            && ctx.mic_name.trim().is_empty()
        {
            anyhow::bail!(
                "{} requires 'mic_name' to be set in the config",
                action_name
            );
        }
    }

    run_action_by_name(action_name, scene, &ctx, &cfg)?;
    notify::send_notification(&cfg.notify, &format!("Action {} triggered", action_name));

    // Allow the background WebSocket read to complete so any failure log
    // makes it to stderr before we exit.
    std::thread::sleep(Duration::from_millis(50));
    Ok(())
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
        let actions = vec![
            ActionItem::Bare("toggle_recording".to_string()),
            ActionItem::Bare("set_mic_volume".to_string()),
        ];
        assert_eq!(action_labels(&actions), "Toggle Recording + Set Mic Volume");
    }

    #[test]
    fn test_action_labels_include_scene_param() {
        let actions = vec![
            ActionItem::Bare("toggle_recording".to_string()),
            ActionItem::Detailed {
                action: "switch_scene".to_string(),
                scene: Some("Gaming".to_string()),
            },
        ];
        assert_eq!(
            action_labels(&actions),
            "Toggle Recording + Switch Scene (Gaming)"
        );
    }

    #[test]
    fn test_build_banner_bindings_preserves_combo_label() {
        let chord = input::KeyChord::parse("ctrl + f1").unwrap();
        let bindings = vec![ActionBinding {
            id: "combo:record_and_mic".to_string(),
            key_name: "ctrl + f1".to_string(),
            chord,
            label: "Toggle Recording + Set Mic Volume".to_string(),
            steps: Vec::new(),
            release_steps: Vec::new(),
        }];

        let banner_bindings = build_banner_bindings(&bindings);
        assert_eq!(banner_bindings.len(), 1);
        assert_eq!(
            banner_bindings[0].label,
            "Toggle Recording + Set Mic Volume"
        );
    }

    #[test]
    fn test_action_labels_include_macro() {
        let mut cfg = config::default_config();
        cfg.macros.push(config::MacroConfig {
            name: "countdown_record".to_string(),
            actions: vec![
                ActionItem::Detailed {
                    action: "switch_scene".to_string(),
                    scene: Some("Intro".to_string()),
                },
                ActionItem::Bare("start_recording".to_string()),
            ],
            action_delays_ms: vec![10000, 0],
        });
        let actions = vec![ActionItem::Bare("countdown_record".to_string())];
        assert_eq!(
            action_labels_with_cfg(&cfg, &actions),
            "macro:Switch Scene (Intro) + Start Recording"
        );
    }

    #[test]
    fn test_validate_combo_actions_accepts_macro_reference() {
        let mut cfg = config::default_config();
        cfg.macros.push(config::MacroConfig {
            name: "countdown_record".to_string(),
            actions: vec![ActionItem::Bare("start_recording".to_string())],
            action_delays_ms: Vec::new(),
        });
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "record_countdown".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("countdown_record".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_combo_actions_rejects_unknown_macro_reference() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "bad_macro".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("missing_macro".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_steps_executes_combo_actions() {
        let count = Arc::new(AtomicUsize::new(0));
        let first = Arc::new({
            let count = count.clone();
            move || {
                count.fetch_add(1, Ordering::SeqCst);
            }
        }) as Arc<dyn Fn() + Send + Sync>;
        let second = Arc::new({
            let count = count.clone();
            move || {
                count.fetch_add(1, Ordering::SeqCst);
            }
        }) as Arc<dyn Fn() + Send + Sync>;

        run_steps(vec![
            ComboStep {
                action: first,
                delay: Duration::ZERO,
            },
            ComboStep {
                action: second,
                delay: Duration::ZERO,
            },
        ]);

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_run_macro_by_name_rejects_unknown_macro() {
        let cfg = config::default_config();
        let ctx = ActionContext {
            client: obs::OBSClient::new("ws://localhost:4455".to_string()),
            screenshot_source: String::new(),
            screenshot_dir: String::new(),
            mic_name: String::new(),
            mic_volume: 1.0,
        };
        let result = run_macro_by_name("missing", &ctx, &cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_steps_respects_per_action_delay() {
        let order = Arc::new(std::sync::Mutex::new(Vec::<(usize, Instant)>::new()));
        let make = |i: usize, order: Arc<std::sync::Mutex<Vec<(usize, Instant)>>>| {
            Arc::new(move || {
                order.lock().unwrap().push((i, Instant::now()));
            }) as Arc<dyn Fn() + Send + Sync>
        };

        let steps = vec![
            ComboStep {
                action: make(0, order.clone()),
                delay: Duration::from_millis(0),
            },
            ComboStep {
                action: make(1, order.clone()),
                delay: Duration::from_millis(80),
            },
            ComboStep {
                action: make(2, order.clone()),
                delay: Duration::from_millis(0),
            },
        ];

        let start = Instant::now();
        run_steps(steps);
        let elapsed = start.elapsed();

        let recorded = order.lock().unwrap().clone();
        assert_eq!(recorded.len(), 3);
        assert_eq!(recorded[0].0, 0);
        assert_eq!(recorded[1].0, 1);
        assert_eq!(recorded[2].0, 2);
        // Action 1 must run at least 80ms after action 0.
        let gap = recorded[1].1.duration_since(recorded[0].1);
        assert!(
            gap >= Duration::from_millis(70),
            "expected >=70ms gap, got {:?}",
            gap
        );
        // Total elapsed should be at least 80ms.
        assert!(
            elapsed >= Duration::from_millis(70),
            "expected >=70ms total elapsed, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_validate_combo_actions_accepts_default_config() {
        let cfg = config::default_config();
        assert!(validate_combo_actions(&cfg).is_ok());
    }

    #[test]
    fn test_validate_combo_actions_rejects_unknown_action() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "bad".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("not_real".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_combo_actions_rejects_duplicate_names() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "dup".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("toggle_recording".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "dup".to_string(),
            key: Some("f2".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("toggle_streaming".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_combo_actions_rejects_mic_volume_without_mic_name() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "record_and_mic".to_string(),
            key: Some("ctrl + f1".to_string()),
            keys: Vec::new(),
            actions: vec![
                ActionItem::Bare("toggle_recording".to_string()),
                ActionItem::Bare("set_mic_volume".to_string()),
            ],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_combo_actions_rejects_release_mic_volume_without_mic_name() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "push_to_mute".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("toggle_recording".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: vec![ActionItem::Bare("set_mic_volume".to_string())],
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_combo_actions_rejects_switch_scene_without_scene_name() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "to_scene".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Detailed {
                action: "switch_scene".to_string(),
                scene: None,
            }],
            action_delays_ms: Vec::new(),
            release_actions: Vec::new(),
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_combo_actions_accepts_release_actions() {
        let mut cfg = config::default_config();
        cfg.hotkey_combos.push(config::HotkeyCombo {
            name: "push_to_talk".to_string(),
            key: Some("f1".to_string()),
            keys: Vec::new(),
            actions: vec![ActionItem::Bare("toggle_recording".to_string())],
            action_delays_ms: Vec::new(),
            release_actions: vec![ActionItem::Bare("toggle_recording".to_string())],
            release_action_delays_ms: Vec::new(),
        });

        let result = validate_combo_actions(&cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_action_subcommand() {
        let cli = Cli::try_parse_from(["obs-hotkey", "action", "toggle_recording"]).unwrap();
        match cli.command {
            Some(Commands::Action { name, scene, .. }) => {
                assert_eq!(name, "toggle_recording");
                assert!(scene.is_none());
            }
            _ => panic!("expected Action subcommand"),
        }
    }

    #[test]
    fn test_cli_action_subcommand_with_scene() {
        let cli =
            Cli::try_parse_from(["obs-hotkey", "action", "switch_scene", "--scene", "Gaming"])
                .unwrap();
        match cli.command {
            Some(Commands::Action { name, scene, .. }) => {
                assert_eq!(name, "switch_scene");
                assert_eq!(scene.as_deref(), Some("Gaming"));
            }
            _ => panic!("expected Action subcommand"),
        }
    }
}
