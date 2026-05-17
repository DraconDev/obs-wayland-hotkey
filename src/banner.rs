use crate::config::AppConfig;

pub struct HotkeyBinding {
    pub key_name: String,
    pub action: &'static str,
    pub label: &'static str,
}

pub fn print_banner(_cfg: &AppConfig, bindings: &[HotkeyBinding], autostart: bool) {
    println!();
    println!("OBS Hotkey Controller - Wayland compatible");
    println!();
    for b in bindings {
        if b.key_name.is_empty() {
            continue;
        }
        if crate::input::get_key_code(&b.key_name).is_none() {
            continue;
        }
        println!("  {:-12} → {}", b.key_name, b.label);
    }
    println!();
    if autostart {
        println!("  Auto-start: enabled (systemd user service)");
    } else {
        println!("  Auto-start: not configured (run 'obs-hotkey setup' to enable)");
    }
    println!();
    if !running_under_systemd() {
        println!("Listening for hotkeys... (Ctrl+C to exit)");
    }
}

fn running_under_systemd() -> bool {
    std::env::var("INVOCATION_ID").is_ok() || std::env::var("JOURNAL_STREAM").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;

    #[test]
    fn test_print_banner_runs_without_panic() {
        let cfg = default_config();
        let bindings = vec![
            HotkeyBinding {
                key_name: "scroll lock".to_string(),
                action: "toggle_recording",
                label: "Toggle Recording",
            },
            HotkeyBinding {
                key_name: "".to_string(),
                action: "toggle_pause",
                label: "Toggle Pause/Resume",
            },
        ];
        print_banner(&cfg, &bindings, false);
    }

    #[test]
    fn test_running_under_systemd() {
        let result = running_under_systemd();
        assert!(!result);
    }
}