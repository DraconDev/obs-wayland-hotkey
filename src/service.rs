use std::path::PathBuf;
use std::process::Command;

pub fn is_autostart_enabled() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-enabled", "obs-hotkey.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn in_input_group() -> bool {
    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty() {
        return false;
    }
    Command::new("groups")
        .arg(&user)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("input"))
        .unwrap_or(false)
}

pub fn service_unit_path() -> PathBuf {
    let home = real_home();
    home.join(".config")
        .join("systemd")
        .join("user")
        .join("obs-hotkey.service")
}

fn real_home() -> PathBuf {
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        if let Ok(pw) = std::fs::read_to_string("/etc/passwd") {
            for line in pw.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 6 && parts[0] == sudo_user {
                    return PathBuf::from(parts[5]);
                }
            }
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn write_service_file(exe_path: &str, cfg_dir: &str) -> anyhow::Result<()> {
    let content = format!(
        r#"[Unit]
Description=OBS Hotkey Controller
After=graphical-session.target

[Service]
Type=simple
ExecStart={} --config {}/hotkeys.json
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=graphical-session.target
"#,
        exe_path, cfg_dir
    );
    let path = service_unit_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn run_setup(config_path: &str) {
    if !in_input_group() {
        println!("Warning: you are not in the 'input' group.");
        println!("  On NixOS: add 'users.users.\"$USER\".extraGroups = [ \"input\" ];' to your configuration.nix");
        println!("  On others: run: sudo usermod -aG input $USER");
        println!("  Then log out and back in for changes to take effect.");
        println!();
    }

    let old_unit = service_unit_path()
        .parent()
        .unwrap()
        .join("obs-wayland-hotkey.service");
    if old_unit.exists() {
        println!("Found old obs-wayland-hotkey.service, removing...");
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "obs-wayland-hotkey.service"])
            .output();
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "obs-wayland-hotkey.service"])
            .output();
        let _ = std::fs::remove_file(&old_unit);
    }

    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "obs-hotkey".to_string());
    let cfg_dir = PathBuf::from(config_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            dirs::config_dir()
                .map(|p| p.join("obs-hotkey").to_string_lossy().to_string())
                .unwrap_or_default()
        });

    if let Err(e) = write_service_file(&exe_path, &cfg_dir) {
        log::error!("Failed to write service file: {}", e);
        std::process::exit(1);
    }
    log::info!("Service file written to {}", service_unit_path().display());

    if !Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        log::warn!("failed to reload systemd");
    }
    if !Command::new("systemctl")
        .args(["--user", "enable", "obs-hotkey.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        log::error!("Failed to enable service");
        std::process::exit(1);
    }

    println!("Service enabled. Starting now...");
    let _ = Command::new("systemctl")
        .args(["--user", "start", "obs-hotkey.service"])
        .output();

    println!();
    println!("=== Setup Complete! ===");
    println!();
    println!("1. ENABLE OBS WEBSOCKET SERVER:");
    println!("   - Open OBS Studio → Tools → WebSocket Server Settings");
    println!("   - Check 'Enable WebSocket server', port 4455, no auth");
    println!();
    println!("2. DEFAULT HOTKEYS (already configured):");
    println!("   - Scroll Lock → Toggle recording");
    println!("   - Pause       → Toggle recording pause");
    println!();
    println!("3. VERIFY IT'S WORKING:");
    println!("   - Press Scroll Lock — recording should stop/resume");
    println!();
    println!("4. VIEW LOGS:  journalctl --user -u obs-hotkey.service -f");
    println!("5. SERVICE:     systemctl --user restart obs-hotkey.service");
    println!("6. CUSTOMIZE:   ~/.config/obs-hotkey/hotkeys.json");
}

pub fn run_teardown(purge: bool) {
    println!("Stopping service...");
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "obs-hotkey.service"])
        .output();
    println!("Disabling service...");
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "obs-hotkey.service"])
        .output();

    let unit_path = service_unit_path();
    if unit_path.exists() {
        std::fs::remove_file(&unit_path).ok();
        println!("Service file removed.");
    } else {
        println!("No service file found (already removed?).");
    }

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    if purge {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("obs-hotkey"))
            .unwrap_or_else(|| PathBuf::from("~/.config/obs-hotkey"));
        std::fs::remove_dir_all(&config_dir).ok();
        println!("Config directory purged.");
    }

    println!("Teardown complete.");
}

pub fn run_status(config_path: &str) {
    println!("=== OBS Hotkey Status ===");
    println!();

    if is_autostart_enabled() {
        println!("  Auto-start: enabled (systemd user service)");
    } else {
        println!("  Auto-start: not configured");
        println!("               Run 'obs-hotkey setup' to enable");
    }

    if in_input_group() {
        println!("  Input group: ✓ member");
    } else {
        println!("  Input group: ✗ not a member");
    }

    if std::path::Path::new(config_path).exists() {
        println!("  Config:      ✓ {}", config_path);
    } else {
        println!("  Config:      ✗ not found ({})", config_path);
    }

    let dir_path = std::path::Path::new(config_path).parent().unwrap_or(std::path::Path::new(""));
    if dir_path.exists() {
        println!("  Config dir: ✓ {}", dir_path.display());
    } else {
        println!("  Config dir: ✗ not found");
    }

    print!("  OBS WS:     ");
    if let Ok(conn) = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], 4455)),
        std::time::Duration::from_secs(1),
    ) {
        drop(conn);
        println!("✓ reachable (port 4455)");
    } else {
        println!("✗ not reachable (is OBS running?)");
    }

    println!();
    if !is_autostart_enabled() {
        println!("Run 'obs-hotkey setup' to enable auto-start on login.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_unit_path() {
        let path = service_unit_path();
        assert!(path.to_string_lossy().ends_with("obs-hotkey.service"));
    }

    #[test]
    fn test_write_service_file() {
        let temp = tempfile::tempdir().unwrap();
        let old_home = std::env::var("HOME");
        std::env::set_var("HOME", temp.path());
        let result = write_service_file("/usr/bin/obs-hotkey", temp.path().join(".config/obs-hotkey").to_str().unwrap());
        std::env::set_var("HOME", old_home.unwrap_or_default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_in_input_group() {
        let result = in_input_group();
        println!("in_input_group = {}", result);
    }

    #[test]
    fn test_is_autostart_enabled() {
        let result = is_autostart_enabled();
        println!("is_autostart_enabled = {}", result);
    }
}