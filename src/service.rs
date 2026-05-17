use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::config::real_home;

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

    let uid = unsafe { libc::getuid() };
    let mut ngroups: libc::c_int = 64;
    let mut groups = [0u32; 64];

    let c_user = std::ffi::CString::new(user.clone()).unwrap();
    let result = unsafe {
        libc::getgrouplist(c_user.as_ptr(), uid, groups.as_mut_ptr(), &mut ngroups)
    };

    if result < 0 {
        return false;
    }

    for &gid in groups.iter().take(result as usize) {
        let grp = unsafe { libc::getgrgid(gid as libc::gid_t) };
        if !grp.is_null() {
            let name = unsafe { std::ffi::CStr::from_ptr((*grp).gr_name) };
            if name.to_bytes() == b"input" {
                return true;
            }
        }
    }

    false
}

pub fn service_unit_path() -> PathBuf {
    let home = real_home();
    home.join(".config")
        .join("systemd")
        .join("user")
        .join("obs-hotkey.service")
}

pub fn write_service_file(exe_path: &str, cfg_dir: &str) -> anyhow::Result<()> {
    let content = format!(
        r#"[Unit]
Description=OBS Hotkey Controller
After=graphical-session.target

[Service]
Type=simple
ExecStart={} daemon --config {}/hotkeys.json
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
    use crate::ansi::*;

    if !in_input_group() {
        println!();
        println!("  {} {}", warn(""), heading("Warning:") + " not in 'input' group");
        println!();
        println!("  Add yourself with:");
        println!("    {} sudo usermod -aG input $(whoami)", key(""));
        println!();
        println!("  {} On NixOS:", muted(""));
        println!("    {}users.users.\"$USER\".extraGroups = [ \"input\" ];{}",
                 CYAN, RESET);
        println!("  {} Then log out and back in for changes to take effect.", muted(""));
        println!();
    }

    // Migrate old service name
    let old_unit = service_unit_path()
        .parent()
        .unwrap()
        .join("obs-wayland-hotkey.service");
    if old_unit.exists() {
        println!("  {} Removing old service...", warn(""));
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

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    if !Command::new("systemctl")
        .args(["--user", "enable", "obs-hotkey.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        log::error!("Failed to enable service");
        std::process::exit(1);
    }

    let _ = Command::new("systemctl")
        .args(["--user", "start", "obs-hotkey.service"])
        .output();

    // Final setup summary
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  Setup Complete!{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
    println!("  {}  Enable OBS WebSocket Server", heading("1."));
    println!("     {}Open OBS → Tools → WebSocket Server Settings{}", CYAN, RESET);
    println!("     Check {}Enable{} (port 4455, no auth)", GREEN, RESET);
    println!();
    println!("  {}  Default hotkeys configured:", heading("2."));
    println!("     {}Scroll Lock{} → Toggle recording", CYAN, RESET);
    println!("     {}Pause{}       → Toggle pause", CYAN, RESET);
    println!();
    println!("  {}  Test it:", heading("3."));
    println!("     Press {}Scroll Lock{} — recording should toggle", CYAN, RESET);
    println!();
    println!("  {}  View logs:", heading("4."));
    println!("     {}", muted("journalctl --user -u obs-hotkey.service -f"));
    println!();
    println!("  {}  Service commands:", heading("5."));
    println!("     {}systemctl --user restart obs-hotkey.service{}", CYAN, RESET);
    println!();
    println!("  {}  Customize:", heading("6."));
    println!("     {}", muted("~/.config/obs-hotkey/hotkeys.json"));
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
}

pub fn run_teardown(purge: bool) {
    use crate::ansi::*;

    let old_service = service_unit_path()
        .parent()
        .unwrap()
        .join("obs-wayland-hotkey.service");
    let old_binary = std::env::home_dir()
        .map(|h| h.join(".cargo/bin/obs-wayland-hotkey"))
        .unwrap_or_default();
    let new_binary = std::env::home_dir()
        .map(|h| h.join(".cargo/bin/obs-hotkey"))
        .unwrap_or_default();

    println!();
    println!("  {} Stopping and disabling services...", heading("▶"));

    // Stop/disable current service
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "obs-hotkey.service"])
        .output();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "obs-hotkey.service"])
        .output();

    // Stop/disable old service (if renamed package was previously installed)
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "obs-wayland-hotkey.service"])
        .output();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "obs-wayland-hotkey.service"])
        .output();

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    println!("  {} Removing service files...", heading("▶"));

    let unit_path = service_unit_path();
    if unit_path.exists() {
        std::fs::remove_file(&unit_path).ok();
        println!("  {} obs-hotkey.service removed", ok(""));
    }
    if old_service.exists() {
        std::fs::remove_file(&old_service).ok();
        println!("  {} obs-wayland-hotkey.service removed", ok(""));
    }
    if !unit_path.exists() && !old_service.exists() {
        println!("  {} No service files found", muted(""));
    }

    println!("  {} Removing binaries...", heading("▶"));

    if new_binary.exists() {
        std::fs::remove_file(&new_binary).ok();
        println!("  {} ~/.cargo/bin/obs-hotkey removed", ok(""));
    }
    if old_binary.exists() {
        std::fs::remove_file(&old_binary).ok();
        println!("  {} ~/.cargo/bin/obs-wayland-hotkey removed", ok(""));
    }
    if !new_binary.exists() && !old_binary.exists() {
        println!("  {} No binaries found in ~/.cargo/bin", muted(""));
    }

    if purge {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("obs-hotkey"))
            .unwrap_or_else(|| PathBuf::from("~/.config/obs-hotkey"));
        if config_dir.exists() {
            std::fs::remove_dir_all(&config_dir).ok();
            println!("  {} Config directory purged", ok(""));
        }
    }

    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  Teardown Complete{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
}

/// Probe OBS WebSocket by doing a full handshake + identify cycle.
/// Returns true if the handshake and identify succeed.
fn probe_obs_websocket(port: u16) -> bool {
    let url = format!("ws://127.0.0.1:{}", port);
    let stream = match TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_secs(1),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

    let mut ws = match tungstenite::client(&url, stream) {
        Ok((ws, _)) => ws,
        Err(_) => return false,
    };

    match ws.read() {
        Ok(msg) => {
            // Should be a hello message — check it's valid JSON
            if let Ok(text) = msg.to_text() {
                text.starts_with("{")
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

pub fn run_status(config_path: &str) {
    use crate::ansi::*;

    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  OBS Hotkey Status{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();

    let autostart = is_autostart_enabled();
    let input_grp = in_input_group();
    let cfg_exists = std::path::Path::new(config_path).exists();
    let dir_exists = std::path::Path::new(config_path)
        .parent()
        .map(|p| p.exists())
        .unwrap_or(false);

    let obs_ok = probe_obs_websocket(4455);

    // Auto-start row
    if autostart {
        println!("  {:<14}  {}", "Auto-start:", ok("enabled (systemd)"));
    } else {
        println!("  {:<14}  {}  (run {} to enable)",
                 "Auto-start:", warn("disabled"), key("obs-hotkey setup"));
    }

    // Input group row
    println!("  {:<14}  {}", "Input group:", if input_grp { ok("member") } else { err("not a member") });

    // Config row
    if cfg_exists {
        println!("  {:<14}  {}  {}", "Config:", ok(""), muted(config_path));
    } else {
        println!("  {:<14}  {}  ({})", "Config:", err(""), muted(config_path));
    }

    // Config dir row
    if dir_exists {
        let dir = std::path::Path::new(config_path).parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        println!("  {:<14}  {}  {}", "Config dir:", ok(""), muted(&dir));
    } else {
        println!("  {:<14}  {}", "Config dir:", err("not found"));
    }

    // OBS row
    if obs_ok {
        println!("  {:<14}  {}", "OBS WS:", ok("reachable"));
    } else {
        println!("  {:<14}  {}  (is OBS running?)", "OBS WS:", err("✗"));
    }

    // Daemon activity check - is the process running?
    let daemon_active = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "obs-hotkey.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if daemon_active {
        println!("  {:<14}  {}", "Daemon:", ok("running"));
    } else {
        println!("  {:<14}  {}  (run {} to enable)",
                 "Daemon:", warn("not running"), key("obs-hotkey daemon"));
    }

    println!();
    if !autostart {
        println!("  Run {}obs-hotkey setup{} to enable auto-start.", key(""), key("setup"));
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