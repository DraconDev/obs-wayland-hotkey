use crate::ansi::*;
use crate::config::AppConfig;

pub struct HotkeyBinding {
    pub key_name: String,
    pub action: &'static str,
    pub label: String,
}

pub fn print_banner(_cfg: &AppConfig, bindings: &[HotkeyBinding], autostart: bool) {
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!(
        "  {}  OBS Hotkey Controller  v{}{}",
        BOLD,
        env!("CARGO_PKG_VERSION"),
        RESET
    );
    println!(
        "  {}  Wayland-compatible  |  {} hotkeys configured{}",
        DIM,
        bindings.iter().filter(|b| !b.key_name.is_empty()).count(),
        RESET
    );
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();

    for b in bindings {
        if b.key_name.is_empty() {
            continue;
        }
        if crate::input::get_key_code(&b.key_name).is_none() {
            continue;
        }
        println!("  {} → {}", key(&b.key_name), b.label);
    }
    println!();

    if autostart {
        println!("  {} Auto-start enabled", ok(""));
    } else {
        println!("  {} Auto-start not configured", warn(""));
        println!("     {} to enable", key("obs-hotkey setup"));
    }
    println!();

    if !running_under_systemd() {
        println!(
            "  {} Listening for hotkeys... (Ctrl+C to exit){}",
            DIM, RESET
        );
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
                label: "Toggle Recording".to_string(),
            },
            HotkeyBinding {
                key_name: "".to_string(),
                action: "toggle_pause",
                label: "Toggle Pause/Resume".to_string(),
            },
        ];
        print_banner(&cfg, &bindings, false);
    }

    #[test]
    fn test_running_under_systemd() {
        std::env::remove_var("INVOCATION_ID");
        std::env::remove_var("JOURNAL_STREAM");
        let result = running_under_systemd();
        assert!(
            !result,
            "should return false when not running under systemd"
        );
    }
}
