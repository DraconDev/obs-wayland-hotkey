use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_WS_URL: &str = "ws://localhost:4455";
const CONFIG_DIR_NAME: &str = "obs-hotkey";
const CONFIG_FILE_NAME: &str = "hotkeys.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HotkeyCombo {
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    pub actions: Vec<String>,
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
    #[serde(default, rename = "mic_volume")]
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
    fn test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAvSkdTN3ZtMlZWeWRpK3ppOWtJRHRJRG9rQXhTWFVxTXBMU2x0NGFpQndRCjhKLzZhTm5ZNDlWUDZub1FCR2s5OXdoMDJFRUdMTGNKMGlwa2xvVFljZEUKLT4gWDI1NTE5IGR5V1F0MndjRmtqdGVNTW50T1AvU0pvTkdXS0VQYkpQU3Vka3IvMVlNVGMKc0ZUb2VJUHVnZDBJVDQ5TSs0bmRQWjdnYXlHM2RKRlZla1BnbTFBQjBUWQotPiBYMjU1MTkgTmltMGlaT3kwZ0lEZWUvR04vR2VQbG1mS2pzbXlHOVJFc3BGb3p5NVBVcwpXZHlJQkd2M0FPSmFadlViUDhxSk9SUlB0TVV2b1hLcXVWRSszMHZwVVNBCi0+IFgyNTUxOSBRYy94NXZBeXpIZFJiSXpBQ3NDMGc0Vnh2U2RJZWd3Q1BUU09DY1IyS25BCkhSMVV5elY0UkYyR0lVTzlJYmpLRzBpTmdrVlhnYm5tNTZwaHF2LzhXTXMKLT4gWDI1NTE5IGN0UTZ1UlFpSnNvemM5eW5Sb1pOQXF5YlJMK3hzRnEwemk4MTFEajdwMVEKT0xFbGhZWW1iblhZK3RWMmVxbnUvNEN5U3hwTFNNaTlMSlBBa0JaYnc0OAotPiBnIjVuQkd7LWdyZWFzZSBKOyVkISM3IEJlVgo4d1drbmtFUFVKR2drcUwzVGNHaDNUMkRDWnlyM0lEOVJUTXp5dFNhSnhRCi0tLSBrMUdhbm14U2pXMHZDNGpiLzhrd3UrM2Q1cXY0NTRoYjdsaHp1c05KcTBrCkzpsjcMpQRz5UMbTTvCslVXjN2IqWNbnFg2kNbxMHg/FAb9NC8Dco4tQ5ap/5Ih5uRem+4cKaTzWw==]() {
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
    fn test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBsSHZCdWt5ai9yV3RJMVd5S1h0djJ1SUlKQ2tqSGo5dmcrNVZUOHczbmlFCmFxeFVpZEQ2VFBmMjdoR3NrUmlJS2VGM2p2aEZFbW94NkJYYWVxaDJhWGMKLT4gWDI1NTE5IHRyN2ZMMFVjZWNWY0ZLZ3dZNzZycklud29uQjFOczJrMmtoak82TUJsakkKSHpWemRZdmE1WVQwc3laQnlIVTBIRGNpcUpWdHZIN3hUZkxIdjQxejBlOAotPiBYMjU1MTkgdFJDWjI0UzRGWk40cTdjNitxemJWc1ZWYkZEUGt6ejF0MFF1L1hBNCtTNAprWnRvRUhTWGVpUkErMTk0anprQlRwQ1VLcStHSklvRUt6YmFZQTFzc1RjCi0+IFgyNTUxOSBKa1RpV1dpeXRQbzN5Ym5BcWRWWkttOXRBdHEyMG9uNkQzK29BV1NEakNJCjNrOE8zM1pTWEQyc1Zoa0NPblVtZlB3WnFDUUkzcVdDUlRiU05XdmNIMXMKLT4gWDI1NTE5IFNRZ1NjOXpudXBmcWIyV1FsWUdtM0JZMXFqOE5MVFJlcXVxVEtyR3lpVU0KM3M2VmljeFhydUNqV0hORWErbmdtTmgwV1h4Qno4clgzTlNobDZUeWdndwotPiBmdS1fJC1ncmVhc2UgZGk/PiByTHdlKFZoIGcgLGVlfjcsJAo1bDhsVU0zOXE3WmRjNnlzMGZZaFpjWU1jRVIrb3lKRXlNZUJRU3ZRYSt6ZTR0SjlTaTRBd1VHRWEvODdSQXdUCkFvSW5GZ29wK0VMQks1Tko4UTZPZzZvdwotLS0gMVBHaW5oV21pWDNxODFnT1dIT01ZYXdIK0QwNmVqYVlsOHY4NHZRcnNyMAqZH4vqPovQC7fem0LxyjrpIs7lC5Wm/ZPoTvTwu2msx2UdmcZOWcM7hPdls/xICny3L5tI8ISmriB2KfU=]() {
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
