use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

const DEFAULT_WS_URL: &str = "ws://localhost:4455";
const CONFIG_DIR_NAME: &str = "obs-hotkey";
const CONFIG_FILE_NAME: &str = "hotkeys.json";

/// Maximum per-action delay allowed in a hotkey combo, in milliseconds.
/// 10 minutes is long enough for a real "start recording after a countdown"
/// workflow while preventing absurd values that look like typos.
pub const MAX_ACTION_DELAY_MS: u64 = 600_000;

fn default_mic_volume() -> f64 {
    1.0
}

fn default_notify_command() -> Vec<String> {
    vec![
        "notify-send".to_string(),
        "obs-hotkey".to_string(),
        "{message}".to_string(),
    ]
}

fn default_http_bind() -> String {
    "127.0.0.1:7999".to_string()
}

/// A single OBS action inside a combo. A bare string (e.g. `"toggle_recording"`)
/// is shorthand for a parameterized action with no extra arguments. The
/// object form (e.g. `{"action": "switch_scene", "scene": "Gaming"}`) lets
/// the action carry parameters.
///
/// Currently supported parameter keys:
/// - `scene`: the OBS scene name used by `switch_scene`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ActionItem {
    /// Bare action name, e.g. `"toggle_recording"`. Backward-compatible form.
    Bare(String),
    /// Parameterized action, e.g. `{"action": "switch_scene", "scene": "Gaming"}`.
    Detailed {
        action: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scene: Option<String>,
    },
}

impl ActionItem {
    /// Returns the action name regardless of representation.
    pub fn name(&self) -> &str {
        match self {
            ActionItem::Bare(name) => name,
            ActionItem::Detailed { action, .. } => action,
        }
    }

    /// Returns the optional scene name for `switch_scene` actions.
    pub fn scene(&self) -> Option<&str> {
        match self {
            ActionItem::Bare(_) => None,
            ActionItem::Detailed { scene, .. } => scene.as_deref(),
        }
    }
}

/// Notification command used by the daemon when an action is triggered.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NotifyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_notify_command")]
    pub command: Vec<String>,
}

/// Localhost HTTP API configuration for Companion/Touch Portal integrations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HttpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_http_bind")]
    pub bind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HotkeyCombo {
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    pub actions: Vec<ActionItem>,
    /// Optional per-action delays in milliseconds. When non-empty, the length
    /// must match `actions`. Each delay is how long to wait *before* running
    /// that action. A value of `0` (or an empty list) means "run immediately".
    #[serde(default, rename = "action_delays_ms")]
    pub action_delays_ms: Vec<u64>,
    /// Optional actions to run when the chord is released. Same shape as
    /// `actions`. Enables push-to-record / push-to-talk style workflows:
    /// `actions: ["toggle_recording"]`, `release_actions: ["toggle_recording"]`.
    #[serde(default, rename = "release_actions")]
    pub release_actions: Vec<ActionItem>,
    /// Optional per-action delays in milliseconds for `release_actions`.
    #[serde(default, rename = "release_action_delays_ms")]
    pub release_action_delays_ms: Vec<u64>,
}

impl HotkeyCombo {
    pub fn key_spec(&self) -> String {
        match &self.key {
            Some(key) => key.clone(),
            None => self.keys.join(" + "),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MacroConfig {
    pub name: String,
    pub actions: Vec<ActionItem>,
    /// Optional per-action delays in milliseconds. When non-empty, the length
    /// must match `actions`. Each delay is how long to wait *before* running
    /// that action. This is useful for countdown workflows such as
    /// "switch to intro scene, wait 10 seconds, start recording".
    #[serde(default, rename = "action_delays_ms")]
    pub action_delays_ms: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HotkeyConfig {
    pub toggle_recording: String,
    pub toggle_pause: String,
    pub toggle_streaming: String,
    pub screenshot: String,
    pub toggle_mute_mic: String,
    pub toggle_studio_mode: String,
    pub toggle_replay_buffer: String,
    pub save_replay: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(rename = "obs_host")]
    pub obs_host: String,
    pub hotkeys: HotkeyConfig,
    #[serde(rename = "screenshot_source")]
    pub screenshot_source: String,
    #[serde(rename = "screenshot_dir")]
    pub screenshot_dir: String,
    #[serde(rename = "mic_name")]
    pub mic_name: String,
    #[serde(default = "default_mic_volume", rename = "mic_volume")]
    pub mic_volume: f64,
    /// Optional allowlist of evdev device names to monitor. When empty, all
    /// detected keyboards are used. Useful in setups with multiple keyboards
    /// (laptop, dock, stream deck, guest USB, drawing tablet, macro pad)
    /// where you only want a specific keyboard to trigger broadcast hotkeys.
    /// Device names are the kernel-assigned strings reported by evdev, e.g.
    /// `"AT Translated Set 2 keyboard"` or `"Stream Deck XL"`.
    #[serde(default, rename = "allowed_devices")]
    pub allowed_devices: Vec<String>,
    #[serde(default, rename = "hotkey_combos")]
    pub hotkey_combos: Vec<HotkeyCombo>,
    #[serde(default, rename = "macros")]
    pub macros: Vec<MacroConfig>,
    #[serde(default = "default_notify_config", rename = "notify")]
    pub notify: NotifyConfig,
    #[serde(default = "default_http_config", rename = "http")]
    pub http: HttpConfig,
}

fn default_notify_config() -> NotifyConfig {
    NotifyConfig {
        enabled: false,
        command: default_notify_command(),
    }
}

fn default_http_config() -> HttpConfig {
    HttpConfig {
        enabled: false,
        bind: default_http_bind(),
        token: None,
    }
}

pub fn default_config() -> AppConfig {
    AppConfig {
        obs_host: DEFAULT_WS_URL.to_string(),
        hotkeys: HotkeyConfig {
            toggle_recording: "scroll lock".to_string(),
            toggle_pause: "pause".to_string(),
            toggle_streaming: String::new(),
            screenshot: String::new(),
            toggle_mute_mic: String::new(),
            toggle_studio_mode: String::new(),
            toggle_replay_buffer: String::new(),
            save_replay: String::new(),
        },
        screenshot_source: String::new(),
        screenshot_dir: "~/Pictures".to_string(),
        mic_name: String::new(),
        mic_volume: 1.0,
        allowed_devices: Vec::new(),
        hotkey_combos: Vec::new(),
        macros: Vec::new(),
        notify: NotifyConfig {
            enabled: false,
            command: default_notify_command(),
        },
        http: HttpConfig {
            enabled: false,
            bind: default_http_bind(),
            token: None,
        },
    }
}

pub fn real_home() -> PathBuf {
    if let Some(sudo_user) = std::env::var_os("SUDO_USER") {
        let passwd = fs::read_to_string("/etc/passwd").ok();
        if let Some(pw) = passwd {
            for line in pw.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 6 && parts[0] == sudo_user.to_str().unwrap_or("") {
                    return PathBuf::from(parts[5]);
                }
            }
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg)
            .join(CONFIG_DIR_NAME)
            .join(CONFIG_FILE_NAME);
    }
    let home = real_home();
    home.join(".config")
        .join(CONFIG_DIR_NAME)
        .join(CONFIG_FILE_NAME)
}

pub fn expand_home(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix('~') {
        format!("{}{}", real_home().display(), stripped)
    } else {
        path.to_string()
    }
}

pub fn sanitize_obs_host(host: &str) -> String {
    if host.is_empty() {
        return host.to_string();
    }
    if host.starts_with("ws://") || host.starts_with("wss://") {
        host.to_string()
    } else {
        format!("ws://{}", host)
    }
}

pub fn load_config(path: &Path) -> anyhow::Result<AppConfig> {
    let data =
        fs::read_to_string(path).map_err(|e| anyhow::anyhow!("failed to read config: {}", e))?;
    let mut cfg: AppConfig = serde_json::from_str(&data)
        .map_err(|e| anyhow::anyhow!("failed to parse config: {}", e))?;
    validate_config(&cfg)?;
    cfg.obs_host = sanitize_obs_host(&cfg.obs_host);
    cfg.screenshot_dir = expand_home(&cfg.screenshot_dir);
    Ok(cfg)
}

fn validate_config(cfg: &AppConfig) -> anyhow::Result<()> {
    if !(cfg.mic_volume.is_finite() && cfg.mic_volume >= 0.0) {
        anyhow::bail!("mic_volume must be a finite non-negative number");
    }

    validate_notify_config(&cfg.notify)?;
    validate_http_config(&cfg.http)?;
    validate_macros(cfg)?;

    for combo in &cfg.hotkey_combos {
        if combo.name.trim().is_empty() {
            anyhow::bail!("hotkey_combos entries require a non-empty name");
        }
        if combo.key.is_some() && !combo.keys.is_empty() {
            anyhow::bail!("hotkey_combo '{}' cannot set both key and keys", combo.name);
        }
        if combo.key.is_none() && combo.keys.is_empty() {
            anyhow::bail!("hotkey_combo '{}' must set key or keys", combo.name);
        }
        if combo.actions.is_empty() && combo.release_actions.is_empty() {
            anyhow::bail!(
                "hotkey_combo '{}' must include at least one action or release_action",
                combo.name
            );
        }
        validate_action_list(
            &combo.name,
            "actions",
            &combo.actions,
            &combo.action_delays_ms,
        )?;
        validate_action_list(
            &combo.name,
            "release_actions",
            &combo.release_actions,
            &combo.release_action_delays_ms,
        )?;
        if combo.actions.iter().any(requires_mic_name) && cfg.mic_name.trim().is_empty() {
            anyhow::bail!(
                "hotkey_combo '{}' uses set_mic_volume or toggle_mute_mic but mic_name is empty",
                combo.name
            );
        }
        if combo.release_actions.iter().any(requires_mic_name) && cfg.mic_name.trim().is_empty() {
            anyhow::bail!(
                "hotkey_combo '{}' uses set_mic_volume or toggle_mute_mic in release_actions but mic_name is empty",
                combo.name
            );
        }
    }

    Ok(())
}

fn validate_notify_config(notify: &NotifyConfig) -> anyhow::Result<()> {
    if notify.command.is_empty() {
        anyhow::bail!("notify.command must contain at least one command element");
    }
    Ok(())
}

fn validate_http_config(http: &HttpConfig) -> anyhow::Result<()> {
    if http.bind.parse::<SocketAddr>().is_err() {
        anyhow::bail!(
            "http.bind '{}' is not a valid address:port value",
            http.bind
        );
    }
    if http.enabled && !is_loopback_bind(&http.bind) && http.token.is_none() {
        anyhow::bail!(
            "http.token is required when http.bind is not loopback: {}",
            http.bind
        );
    }
    Ok(())
}

pub fn http_config_is_safe(http: &HttpConfig) -> bool {
    http.bind.parse::<SocketAddr>().is_ok()
        && (!http.enabled || is_loopback_bind(&http.bind) || http.token.is_some())
}

fn is_loopback_bind(bind: &str) -> bool {
    bind.parse::<SocketAddr>()
        .map(|addr| addr.ip().is_loopback())
        .unwrap_or(false)
}

pub(crate) fn validate_macros(cfg: &AppConfig) -> anyhow::Result<()> {
    let mut macro_names = HashSet::new();
    for macro_config in &cfg.macros {
        if macro_config.name.trim().is_empty() {
            anyhow::bail!("macros entries require a non-empty name");
        }
        if !macro_names.insert(macro_config.name.as_str()) {
            anyhow::bail!("duplicate macro name '{}'", macro_config.name);
        }
        if macro_config.actions.is_empty() {
            anyhow::bail!(
                "macro '{}' must include at least one action",
                macro_config.name
            );
        }
        validate_action_list(
            &macro_config.name,
            "actions",
            &macro_config.actions,
            &macro_config.action_delays_ms,
        )?;
        if macro_config
            .actions
            .iter()
            .any(|item| matches!(item.name(), "set_mic_volume" | "toggle_mute_mic"))
            && cfg.mic_name.trim().is_empty()
        {
            anyhow::bail!(
                "macro '{}' uses set_mic_volume or toggle_mute_mic but mic_name is empty",
                macro_config.name
            );
        }
    }

    for macro_config in &cfg.macros {
        validate_macro_references(cfg, &macro_config.name, &mut Vec::new())?;
    }
    Ok(())
}

fn validate_macro_references(
    cfg: &AppConfig,
    macro_name: &str,
    stack: &mut Vec<String>,
) -> anyhow::Result<()> {
    if stack.iter().any(|name| name == macro_name) {
        let mut cycle = stack.clone();
        cycle.push(macro_name.to_string());
        anyhow::bail!("macro cycle detected: {}", cycle.join(" -> "));
    }
    stack.push(macro_name.to_string());
    let macro_config = cfg
        .macros
        .iter()
        .find(|m| m.name == macro_name)
        .ok_or_else(|| anyhow::anyhow!("macro '{}' not found", macro_name))?;

    for item in &macro_config.actions {
        let action = item.name();
        if is_known_action_name(action) {
            validate_action_item(macro_name, item)?;
        } else if cfg.macros.iter().any(|m| m.name == action) {
            validate_macro_references(cfg, action, stack)?;
        } else {
            anyhow::bail!(
                "unknown action or macro '{}' in macro '{}'",
                action,
                macro_name
            );
        }
    }

    stack.pop();
    Ok(())
}

fn validate_action_item(owner: &str, item: &ActionItem) -> anyhow::Result<()> {
    if item.name() == "switch_scene" && item.scene().map(str::trim).unwrap_or("").is_empty() {
        anyhow::bail!("{} uses switch_scene without a scene name", owner);
    }
    Ok(())
}

pub(crate) fn is_known_action_name(action: &str) -> bool {
    matches!(
        action,
        "toggle_recording"
            | "toggle_pause"
            | "toggle_streaming"
            | "screenshot"
            | "toggle_mute_mic"
            | "set_mic_volume"
            | "toggle_studio_mode"
            | "toggle_replay_buffer"
            | "save_replay"
            | "switch_scene"
            | "start_recording"
            | "stop_recording"
            | "start_streaming"
            | "stop_streaming"
            | "start_replay_buffer"
            | "stop_replay_buffer"
    )
}

fn validate_action_list(
    owner: &str,
    field: &str,
    items: &[ActionItem],
    delays: &[u64],
) -> anyhow::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    if !delays.is_empty() && delays.len() != items.len() {
        anyhow::bail!(
            "{} '{}' {}_delays_ms length ({}) must match {} length ({})",
            owner,
            owner,
            field,
            delays.len(),
            field,
            items.len()
        );
    }
    for (index, _item) in items.iter().enumerate() {
        if let Some(delay) = delays.get(index) {
            if *delay > MAX_ACTION_DELAY_MS {
                anyhow::bail!(
                    "{} '{}' {} delay {} ms exceeds maximum {} ms",
                    owner,
                    owner,
                    field,
                    delay,
                    MAX_ACTION_DELAY_MS
                );
            }
        }
    }
    Ok(())
}

/// Returns true if the action requires a non-empty `mic_name` in the config.
fn requires_mic_name(item: &ActionItem) -> bool {
    matches!(item.name(), "set_mic_volume" | "toggle_mute_mic")
}

pub fn ensure_config(dir_path: &Path, file_path: &Path) -> anyhow::Result<()> {
    if file_path.exists() {
        return Ok(());
    }
    fs::create_dir_all(dir_path)
        .map_err(|e| anyhow::anyhow!("failed to create config directory: {}", e))?;
    let cfg = default_config();
    let data = serde_json::to_string_pretty(&cfg)
        .map_err(|e| anyhow::anyhow!("failed to marshal default config: {}", e))?;
    let mut file = fs::File::create(file_path)
        .map_err(|e| anyhow::anyhow!("failed to create config file: {}", e))?;
    file.write_all(data.as_bytes())
        .map_err(|e| anyhow::anyhow!("failed to write config file: {}", e))?;
    log::info!("Created default config at: {}", file_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = default_config();
        assert_eq!(cfg.obs_host, "ws://localhost:4455");
        assert_eq!(cfg.hotkeys.toggle_recording, "scroll lock");
        assert_eq!(cfg.hotkeys.toggle_pause, "pause");
        assert_eq!(cfg.mic_volume, 1.0);
        assert!(cfg.hotkey_combos.is_empty());
        assert!(cfg.macros.is_empty());
        assert!(!cfg.notify.enabled);
        assert!(!cfg.http.enabled);
        assert_eq!(cfg.http.bind, "127.0.0.1:7999");
        assert!(cfg.http.token.is_none());
    }

    #[test]
    fn test_sanitize_obs_host() {
        assert_eq!(sanitize_obs_host("localhost:4455"), "ws://localhost:4455");
        assert_eq!(
            sanitize_obs_host("ws://localhost:4455"),
            "ws://localhost:4455"
        );
        assert_eq!(
            sanitize_obs_host("wss://localhost:4455"),
            "wss://localhost:4455"
        );
        assert_eq!(sanitize_obs_host(""), "");
    }

    #[test]
    fn test_expand_home() {
        let home = real_home();
        assert_eq!(
            expand_home("~/Pictures"),
            format!("{}/Pictures", home.display())
        );
        assert_eq!(expand_home("/tmp/abs"), "/tmp/abs");
    }

    #[test]
    fn test_load_config_missing() {
        let result = load_config(Path::new("/nonexistent/path/config.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_with_macro() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys-macro.json");
        fs::write(&path, r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","macros":[{"name":"countdown_record","actions":[{"action":"switch_scene","scene":"Intro"},{"action":"start_recording"}],"action_delays_ms":[10000,0]}]}"#).unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.macros.len(), 1);
        assert_eq!(cfg.macros[0].name, "countdown_record");
        assert_eq!(cfg.macros[0].action_delays_ms, vec![10000, 0]);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_macros_rejects_duplicate_names() {
        let mut cfg = default_config();
        cfg.macros.push(MacroConfig {
            name: "dup".to_string(),
            actions: vec![ActionItem::Bare("start_recording".to_string())],
            action_delays_ms: Vec::new(),
        });
        cfg.macros.push(MacroConfig {
            name: "dup".to_string(),
            actions: vec![ActionItem::Bare("stop_recording".to_string())],
            action_delays_ms: Vec::new(),
        });

        let result = validate_macros(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macros_rejects_unknown_action() {
        let mut cfg = default_config();
        cfg.macros.push(MacroConfig {
            name: "bad".to_string(),
            actions: vec![ActionItem::Bare("not_real".to_string())],
            action_delays_ms: Vec::new(),
        });

        let result = validate_macros(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macros_rejects_cycle() {
        let mut cfg = default_config();
        cfg.macros.push(MacroConfig {
            name: "a".to_string(),
            actions: vec![ActionItem::Bare("b".to_string())],
            action_delays_ms: Vec::new(),
        });
        cfg.macros.push(MacroConfig {
            name: "b".to_string(),
            actions: vec![ActionItem::Bare("a".to_string())],
            action_delays_ms: Vec::new(),
        });

        let result = validate_macros(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macros_rejects_delay_length_mismatch() {
        let mut cfg = default_config();
        cfg.macros.push(MacroConfig {
            name: "bad_delay".to_string(),
            actions: vec![
                ActionItem::Bare("start_recording".to_string()),
                ActionItem::Bare("stop_recording".to_string()),
            ],
            action_delays_ms: vec![1000],
        });

        let result = validate_macros(&cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macros_accepts_macro_reference() {
        let mut cfg = default_config();
        cfg.macros.push(MacroConfig {
            name: "base".to_string(),
            actions: vec![ActionItem::Bare("start_recording".to_string())],
            action_delays_ms: Vec::new(),
        });
        cfg.macros.push(MacroConfig {
            name: "wrapper".to_string(),
            actions: vec![ActionItem::Bare("base".to_string())],
            action_delays_ms: Vec::new(),
        });

        let result = validate_macros(&cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_config_valid() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys.json");
        fs::write(&path, r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"f2","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"~/Pictures","mic_name":""}"#).unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.obs_host, "ws://localhost:4455");
        assert_eq!(cfg.hotkeys.toggle_recording, "f1");
        assert_eq!(cfg.hotkeys.toggle_pause, "f2");
        assert_eq!(cfg.mic_volume, 1.0);
        assert!(cfg.hotkey_combos.is_empty());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_with_combo_hotkeys() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_combo.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"scroll lock","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"~/Pictures","mic_name":"Mic","mic_volume":0.75,"hotkey_combos":[{"name":"record_and_mic","key":"ctrl + f1","actions":["toggle_recording","set_mic_volume"]}]}"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkey_combos.len(), 1);
        assert_eq!(cfg.hotkey_combos[0].name, "record_and_mic");
        assert_eq!(cfg.hotkey_combos[0].key_spec(), "ctrl + f1");
        assert_eq!(
            cfg.hotkey_combos[0].actions,
            vec![
                ActionItem::Bare("toggle_recording".to_string()),
                ActionItem::Bare("set_mic_volume".to_string()),
            ]
        );
        assert_eq!(cfg.mic_volume, 0.75);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_with_combo_action_delays() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_delays.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"scroll lock","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"~/Pictures","mic_name":"Mic","mic_volume":0.75,"hotkey_combos":[{"name":"start_recording_after_3s","key":"ctrl + f1","actions":["toggle_recording","set_mic_volume"],"action_delays_ms":[0,3000]}]}"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkey_combos.len(), 1);
        assert_eq!(cfg.hotkey_combos[0].name, "start_recording_after_3s");
        assert_eq!(cfg.hotkey_combos[0].action_delays_ms, vec![0, 3000]);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_combo_action_delays_length_mismatch() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_delays_bad.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"scroll lock","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","hotkey_combos":[{"name":"bad","key":"f1","actions":["toggle_recording","set_mic_volume"],"action_delays_ms":[1000]}]}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_combo_action_delay_too_large() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_delays_huge.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"scroll lock","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","hotkey_combos":[{"name":"huge","key":"f1","actions":["toggle_recording"],"action_delays_ms":[999999999]}]}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_negative_mic_volume() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_negative_volume.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","mic_volume":-1,"hotkey_combos":[]}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_combo_without_keys() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_bad_combo.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","hotkey_combos":[{"name":"bad","actions":["toggle_recording"]}]}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_bare_host() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys2.json");
        fs::write(&path, r#"{"obs_host":"localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":""}"#).unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.obs_host, "ws://localhost:4455");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBPcW4wYlVlLzdSMUl6OUNFZXQvYklwZHlidUxObVFmL2luVm8weEt6TW1jCnRpcVRtWkJ6b09RTTBmSTlPM0JNYkR3WFY4Z21HZVBIak9kcGNDenFRekkKLT4gWDI1NTE5IDIwTXpCZmtlMUFGUDNadXhRMDN1S1FaQkFHclZ5Q0I5TUVuYzZuWTdCaXcKdG1DWGNvdDk2eXB4R2NZL2s4Y2FnWXJ5R05KeXhiSURrYmhsd05SK0cvSQotPiBYMjU1MTkgVWZJdjZxZ2FhcmI2QzRDamlBVnF6OUFLZ0lwN1ZCejloOVgwTExvaXBuWQo5ZXpFUk5zT2ZPazN6MFlGOTc4OGpvYk4wM0NiWEJmTll6TlpDMmZUK2FzCi0+IFgyNTUxOSA4R1dwT1hacjNBZDgxYlpUTHFnQnJtSXFTNlRSaUw2RWJjQ2FSaFFLVkEwCkhGSmsvN0Q0VWFndUw0Rm9xZS80VzZxckRwRHkxR3NOeStBYm9ZWTFMMVkKLT4gWDI1NTE5IHQ4R0ZwYWU5aDNXalAvWDJkL1NqbWI0WFQrTlJrSExhOFE2NzUzS2pUbUUKV2VUYU1Jb1NYZ3JWNjcwU1o1cm1vM1M2eHlLcko2b0g2VlljdkxtZk9hVQotPiAmcStyUi4tZ3JlYXNlCkp3SlA4ZkR5TUhZCi0tLSBVSEVVaXpEdVAveTJjejJzVXE2dTJHaFNvYzVZN2toY21WMGU5VXRKdk1VCp98Hnf62kF8rQXpgL2RG/T1LLYrYJKERU70EDJ7Yi8kf6JL4XeeBDLsiT32okuWbtWeZEGAw+rd8g==]() {
        let temp = std::env::temp_dir().join("obs-hotkey-test");
        let dir = temp.join(".config").join("obs-hotkey");
        let path = dir.join("hotkeys.json");
        ensure_config(&dir, &path).unwrap();
        assert!(path.exists());
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkeys.toggle_recording, "scroll lock");
        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBqbXgrSDkyb1FEb0JpaVM1T1NsdUdpZ0lxYlQyRllRRWdwR1dDZWExRVF3CmN1RzNGUnkyZDVaNlhVWGVMaDE2QSsrYmdJZEJ4bUhrS1BKRnhueG9yTm8KLT4gWDI1NTE5IDc3Vk00V1R4VnRRN1huUmpra29Kd010UGhHcTNMNVZlcTMrYVlPbDNNU2MKVWR2OWVwUlI2ZDlaaEtycHJndGhBUW5QUTBHTHVyU2FWUXdZaTVVeG9lTQotPiBYMjU1MTkgK1N1Y0hGWjE3RUpXNWdSRWQ2TDNiK1RxcUgvaGdiQ2tkQnZkTm92T1ZoRQpRMDhqV0Nib2lkUnpsYzgvZHZHOUlaOERqd3dNTmliTmQ5dW9iQWk1NktRCi0+IFgyNTUxOSBXaE5RTFozRXlXVk1YYkExa0hNZjZmd3VDVFBXRjdvWmJka3llOWo0MER3ClExUkN3TzFndFYycGJvK0Nadnk1b3hIRHZnWE9ua1ZzWTMwU2FxZUw5TEkKLT4gWDI1NTE5IGxBQkFUcUhBL0VQOGR3ekxSNHEwWlFrZDRkbENRNEgxR3R3SDNLeGZ2bG8KYWd2V2RPTytkSTFKTmkxMEFJZ0kvbDFxVytwOXNWNGF2ODgyQ0svUHQ5RQotPiBxLWdyZWFzZSAxSD9sd20xXwo2ajladXV3SHlZRGxBSGI0TGZZcG9JYks0NUhoNmt1MWozUHdiNWN3V0JXN2d4QUE2ZGlwd1FYNlZnbWpxZGZ2CjNOb3p1azRqcnFEeVZvMFY2bW54cmd4eAotLS0gZnc2QVRRaUhxdmIxQWpSYURDZEdLVi9SSHR0V3UxbXZjbFg5dDBoVStSNArOiRVw7hQdSgfLf1gkawBUfXptOdJr+cG80TwGuAWOzPWWgoIGyfyOzdTNkazlXFC5HaqrLVpdV0r7HfM=]() {
        let temp = std::env::temp_dir().join("obs-hotkey-test2");
        let dir = temp.join(".config").join("obs-hotkey");
        let path = dir.join("hotkeys.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, r#"{"obs_host":"ws://custom:1234","hotkeys":{"toggle_recording":"scroll lock","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":""}"#).unwrap();
        ensure_config(&dir, &path).unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.obs_host, "ws://custom:1234");
        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_load_config_unknown_field_rejected() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_unknown.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","unknown_field":true}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err(), "expected error for unknown field");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_with_scene_combo() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_scene.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","hotkey_combos":[{"name":"to_gaming","key":"f13","actions":[{"action":"switch_scene","scene":"Gaming"}]}]}"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkey_combos.len(), 1);
        assert_eq!(cfg.hotkey_combos[0].actions.len(), 1);
        assert_eq!(cfg.hotkey_combos[0].actions[0].name(), "switch_scene");
        assert_eq!(cfg.hotkey_combos[0].actions[0].scene(), Some("Gaming"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_with_release_actions() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_release.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"Mic","hotkey_combos":[{"name":"push_to_talk","key":"ctrl + space","actions":["toggle_mute_mic"],"release_actions":["toggle_mute_mic"]}]}"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkey_combos.len(), 1);
        assert_eq!(cfg.hotkey_combos[0].actions.len(), 1);
        assert_eq!(cfg.hotkey_combos[0].release_actions.len(), 1);
        assert_eq!(
            cfg.hotkey_combos[0].release_actions[0].name(),
            "toggle_mute_mic"
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_combo_with_only_release_actions_and_no_mic() {
        // release_actions: set_mic_volume requires mic_name; even if actions is non-empty,
        // release_actions is validated too.
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_release_mic.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","hotkey_combos":[{"name":"push","key":"f1","actions":["toggle_recording"],"release_actions":["set_mic_volume"]}]}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_with_allowed_devices() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_devices.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","allowed_devices":["AT Translated Set 2 keyboard","Stream Deck XL"]}"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(
            cfg.allowed_devices,
            vec!["AT Translated Set 2 keyboard", "Stream Deck XL"]
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_with_notify_and_http() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_tier1.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","notify":{"enabled":true,"command":["notify-send","obs-hotkey","{message}"]},"http":{"enabled":true,"bind":"127.0.0.1:7999","token":"secret"}}"#,
        )
        .unwrap();
        let cfg = load_config(&path).unwrap();
        assert!(cfg.notify.enabled);
        assert_eq!(cfg.notify.command[0], "notify-send");
        assert!(cfg.http.enabled);
        assert_eq!(cfg.http.bind, "127.0.0.1:7999");
        assert_eq!(cfg.http.token.as_deref(), Some("secret"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_non_loopback_http_without_token() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_http_bad.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","http":{"enabled":true,"bind":"0.0.0.0:7999"}}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_config_rejects_empty_notify_command() {
        let temp = std::env::temp_dir();
        let path = temp.join("hotkeys_notify_bad.json");
        fs::write(
            &path,
            r#"{"obs_host":"ws://localhost:4455","hotkeys":{"toggle_recording":"f1","toggle_pause":"","toggle_streaming":"","screenshot":"","toggle_mute_mic":"","toggle_studio_mode":"","toggle_replay_buffer":"","save_replay":""},"screenshot_source":"","screenshot_dir":"","mic_name":"","notify":{"enabled":true,"command":[]}}"#,
        )
        .unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_action_item_bare() {
        let item: ActionItem = serde_json::from_str("\"toggle_recording\"").unwrap();
        assert_eq!(item.name(), "toggle_recording");
        assert_eq!(item.scene(), None);
    }

    #[test]
    fn test_action_item_detailed() {
        let item: ActionItem =
            serde_json::from_str("{\"action\":\"switch_scene\",\"scene\":\"Gaming\"}").unwrap();
        assert_eq!(item.name(), "switch_scene");
        assert_eq!(item.scene(), Some("Gaming"));
    }
}
