use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HotkeyCombo {
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    pub actions: Vec<String>,
    /// Optional per-action delays in milliseconds. When non-empty, the length
    /// must match `actions`. Each delay is how long to wait *before* running
    /// that action. A value of `0` (or an empty list) means "run immediately".
    #[serde(default, rename = "action_delays_ms")]
    pub action_delays_ms: Vec<u64>,
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
    #[serde(default, rename = "hotkey_combos")]
    pub hotkey_combos: Vec<HotkeyCombo>,
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
        hotkey_combos: Vec::new(),
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
        if combo.actions.is_empty() {
            anyhow::bail!(
                "hotkey_combo '{}' must include at least one action",
                combo.name
            );
        }
        if !combo.action_delays_ms.is_empty() {
            if combo.action_delays_ms.len() != combo.actions.len() {
                anyhow::bail!(
                    "hotkey_combo '{}' action_delays_ms length ({}) must match actions length ({})",
                    combo.name,
                    combo.action_delays_ms.len(),
                    combo.actions.len()
                );
            }
            for &delay in &combo.action_delays_ms {
                if delay > MAX_ACTION_DELAY_MS {
                    anyhow::bail!(
                        "hotkey_combo '{}' action delay {} ms exceeds maximum {} ms",
                        combo.name,
                        delay,
                        MAX_ACTION_DELAY_MS
                    );
                }
            }
        }
    }

    Ok(())
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
            vec!["toggle_recording", "set_mic_volume"]
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
    fn test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBUYkFPMnVmSmNIRVJWd2JFNW5McXZ4ZjhCRmpNKzBkOTNvcWZvL2ovNUFzClV2MVI3Vkk4ZndiWHhXTFgyZVJTd3h0NDBQZWtZdjZiWDV1YkhEcUEvRGMKLT4gWDI1NTE5IFAxV3h1R1IzR3ZvL2F3UDVpbzlsVjFCUnd0TnpaZHk0OUdBVzBIdkUxQ1UKbG5vUG9HNjdkSlNidUNObUU1VEdNa2pBd0Q3UUs4Ympvc3lZWDVSTFlMOAotPiBYMjU1MTkgNmgwdnV1YWRjZ0hvSVdjS3B5VDRJUWRMaUFOQWFLeUhpVmtncHBzQ0RIdwpvMS9JM3dDYmhpdFA3QmJLenJwL1l4ZXJQM01TdUZSUFl5OE1RRTdaam5RCi0+IFgyNTUxOSBCa2VLOGJtRXBiUDJBUGJEcU5GQzBKZUJsdzB3VS9vZFJyQW1NV1ZiY2tJCkY1RThobkNIN0UwZlAyN1dwMjM1cUk4amVETUkvQ2hhMFd0NVVlQnorK1EKLT4gWDI1NTE5IEYrMmRveVpZNXlKMHdla0U5bkt0dXRYcHUySzU4RjZ6VHNTejRnVEJZa2cKVnJxWWFxWkhlRXZBRUE5WTFxdzYrc25hclRSVHRvcTVGNHlqalBpZE5kbwotPiAnI2dLQC1ncmVhc2UgSXEhIH4qcCAhIGFTe2VmXwpwNjZITUwzMjBwS3RNUGtVaVdDVUhsbVNicXZjMk8vRjNIZ2JXaWRqeXBFUzZCcHcrWHA5MEZYWnNvZS9TRVVaCi81dzBsR2RLajQ1Y0dQb29NYWhQNW9KWUxCam01dDA1b0ZTaTh4UzFmbFdaREdpNGNsN2FmdmF5ZWpYN2ZucwotLS0gQTZmRWZJS2taV093a1lEUkdjRmxGcE5UNFpqUG9EVUkyTFAvUjBlZlY3YwoJrd0JO+bYBbVXR5QoGnPhBN0nVSRLz8svCcwx6pF8lzPsinFZdP6DSGWjDDy19onomw8n/mbHZ58=]() {
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
    fn test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBuUlZobWplRjNRc3lMaHczMG5qTmJlelg4Y1ZOVDUybTFlblBaRXFITkNzCmsvalJhT2JDcFdadVZOTFlMaTJXRHd2Q0tCYUVRa1BrdU00d0lmbUVneXcKLT4gWDI1NTE5IHB3NExyeng3YTZkZnNCdjJsbTBLVS9tZ3REUE56Nmh2ZUloRWNjQ2tJaEkKYnVKMGpBZmwwY0xld2lZTVVpTDhRVGR3UGw3UTZnMEZKNGFGeXV3WjFadwotPiBYMjU1MTkgUzZ6T1hyN2VvYmxuMlFrcS9rSy9Rd0hSSWVVRzFPMWdoaXBYUS83NUxqYwpiTzhsa0d5bDB4THZnQ0w4WkdlRzErL3JVcVcvNVJLK1k4ZXlQcGFCUXhNCi0+IFgyNTUxOSAzZDJHT2pPYlBQZ2tTdEl5dWlIMTMrMGVjMEgwUXRWNWtwdDdmODg3V0IwCm5jNnBXbzN2WG5vdnpydGxVSUhXczhtVkdjRXhrbjhxeFdIOHdsdmRlRkUKLT4gWDI1NTE5IHp1c2d1WjV2bzRxV1ZOcHFZdFJTVkVSTXVPMmxMYnZZRDVtSTNzaWNwQmsKSjlUZ3BZQzJJYW9kWkF4WDM2WTRNOW90UlpYcjN2SGxlOWdPYXBaTWpZYwotPiBJWHUtZ3JlYXNlIHZdTV9KOCBzdXIweSA6dGdNNgp3VFkyN00zVFkrQ2paNWxXUmIvaWhsZ0lTenk3YWhrOE9Fd0k0OU9wYnNXY0l4S1BRRkVpMG1Ma210dFkzRkpqCmYxREkvemtiSGZ4QWUvdmo3UGdqWWJGTFpOcTZwL0Y1WGJHaHFYdksyQ1FvTWxiYy9ONUpodkNCbGcKLS0tIDMwQ0daM2tta1c4VUJ0UmRsQVk1U1M1cjh0d2pGNlYvQUZGQ0NISWwwUVkKPEOz2E5gb+fmIfYDjZCQuQxZ7KoTG8/ebCFcjiTGeoOP+M3rVqhvZI6HBo5Lhpm1UJDktqd/zSQ3Zzfx]() {
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
}
