use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::config::{expand_home, real_home};

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
    let c_user = std::ffi::CString::new(user.clone()).unwrap();

    // Start with a reasonable buffer, grow if needed
    let mut ngroups: libc::c_int = 64;
    let mut groups = vec![0u32; ngroups as usize];

    loop {
        let result =
            unsafe { libc::getgrouplist(c_user.as_ptr(), uid, groups.as_mut_ptr(), &mut ngroups) };

        if result >= 0 {
            break;
        }

        // Buffer too small — ngroups was updated with the required size
        if ngroups as usize > groups.len() {
            groups.resize(ngroups as usize, 0);
            continue;
        }

        // getgrouplist failed for a reason other than buffer size
        log::warn!("getgrouplist failed for user '{}'", user);
        return false;
    }

    for &gid in groups.iter().take(ngroups as usize) {
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

pub fn write_service_file(exe_path: &str, config_path: &str) -> anyhow::Result<()> {
    let content = format!(
        r#"[Unit]
Description=OBS Hotkey Controller
After=graphical-session.target

[Service]
Type=simple
ExecStart={} daemon --config {}
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=graphical-session.target
"#,
        exe_path, config_path
    );
    let path = service_unit_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn run_setup(config_path: &str) {
    use crate::ansi::*;

    // Expand tilde in config path before any operations
    let config_path = expand_home(config_path);

    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "obs-hotkey".to_string());

    // Show version info before doing anything
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  obs-hotkey setup{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
    println!("  {:<14} {}", "Installing:", key(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    println!("  {:<14} {}", "Binary:", muted(&exe_path));

    // Show currently installed/running version if any
    let unit_path = service_unit_path();
    if unit_path.exists() {
        if let Ok(existing) = std::fs::read_to_string(&unit_path) {
            if let Some(line) = existing.lines().find(|l| l.starts_with("ExecStart=")) {
                println!("  {:<14} {}", "Replacing:", muted(line.trim_start_matches("ExecStart=")));
            }
        }
    }
    if is_autostart_enabled() {
        println!("  {:<14} {}", "Service:", ok("currently running"));
    }
    println!();

    if !in_input_group() {
        println!();
        println!(
            "  {} {}",
            warn(""),
            heading("Warning:") + " not in 'input' group"
        );
        println!();
        println!("  Add yourself with:");
        println!("    {} sudo usermod -aG input $(whoami)", key(""));
        println!();
        println!("  {} On NixOS:", muted(""));
        println!(
            "    {}users.users.\"$USER\".extraGroups = [ \"input\" ];{}",
            CYAN, RESET
        );
        println!(
            "  {} Then log out and back in for changes to take effect.",
            muted("")
        );
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

    if let Err(e) = write_service_file(&exe_path, &config_path) {
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
    println!(
        "     {}Open OBS → Tools → WebSocket Server Settings{}",
        CYAN, RESET
    );
    println!("     Check {}Enable{} (port 4455, no auth)", GREEN, RESET);
    println!();
    println!("  {}  Default hotkeys configured:", heading("2."));
    println!("     {}Scroll Lock{} → Toggle recording", CYAN, RESET);
    println!("     {}Pause{}       → Toggle pause", CYAN, RESET);
    println!();
    println!("  {}  Test it:", heading("3."));
    println!(
        "     Press {}Scroll Lock{} — recording should toggle",
        CYAN, RESET
    );
    println!();
    println!("  {}  View logs:", heading("4."));
    println!(
        "     {}",
        muted("journalctl --user -u obs-hotkey.service -f")
    );
    println!();
    println!("  {}  Service commands:", heading("5."));
    println!(
        "     {}systemctl --user restart obs-hotkey.service{}",
        CYAN, RESET
    );
    println!();
    println!("  {}  Customize:", heading("6."));
    println!("     {}", muted("~/.config/obs-hotkey/hotkeys.json"));
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
}

pub fn run_teardown(purge: bool) {
    use crate::ansi::*;

    // Show version info before doing anything
    let exe_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  obs-hotkey teardown{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();
    println!("  {:<14} {}", "Removing:", key(&exe_version));

    // Show currently installed/running version
    let unit_path = service_unit_path();
    if unit_path.exists() {
        if let Ok(existing) = std::fs::read_to_string(&unit_path) {
            if let Some(line) = existing.lines().find(|l| l.starts_with("ExecStart=")) {
                println!("  {:<14} {}", "Service:", muted(line.trim_start_matches("ExecStart=")));
            }
        }
    }
    if is_autostart_enabled() {
        println!("  {:<14} {}", "Status:", ok("currently running"));
    }
    if purge {
        println!("  {:<14} {}", "Purge:", err("config will be deleted"));
    }
    println!();

    // Kill all running obs-hotkey processes before removing binaries.
    // This prevents "text file busy" errors when removing the binary.
    let stale_local_bin = std::env::home_dir()
        .map(|h| h.join(".local/bin/obs-hotkey"))
        .unwrap_or_default();

    // Kill running processes first
    if let Ok(output) = Command::new("pgrep").arg("-x").arg("obs-hotkey").output() {
        let pids: Vec<u32> = output
            .stdout
            .split(|&b| b == b'\n')
            .filter_map(|s| {
                let s = s.trim_ascii();
                if s.is_empty() {
                    return None;
                }
                std::str::from_utf8(s).ok()?.parse().ok()
            })
            .collect();
        if !pids.is_empty() {
            println!();
            println!("  {} Stopping {} running process(es)...", heading("▶"), pids.len());
            for pid in pids {
                // Ignore errors (processes may already be gone)
                let _ = Command::new("kill").arg(format!("{}", pid)).output();
                let _ = Command::new("kill").arg("-0").arg(format!("{}", pid)).output(); // check alive
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            // Force kill any that are still alive
            for pid in pids {
                let _ = Command::new("kill").arg("-9").arg(format!("{}", pid)).output();
            }
        }
        if stale_local_bin.exists() {
            println!("  {} Removing stale binary: {}", heading("▶"), stale_local_bin.display());
            std::fs::remove_file(&stale_local_bin).ok();
        }
    }

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

    if let Some(home) = std::env::home_dir() {
        let new_binary = home.join(".cargo/bin/obs-hotkey");
        let old_binary = home.join(".cargo/bin/obs-wayland-hotkey");
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
            let result = msg.to_text().map(|t| t.starts_with("{")).unwrap_or(false);
            let _ = ws.close(None);  // Send proper WebSocket close frame
            result
        }
        Err(_) => false,
    }
}

pub fn run_status(config_path: &str) {
    use crate::ansi::*;

    // Expand tilde for consistent display
    let config_path = expand_home(config_path);

    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  OBS Hotkey Status{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();

    let autostart = is_autostart_enabled();
    let input_grp = in_input_group();
    let cfg_exists = std::path::Path::new(&config_path).exists();
    let dir_exists = std::path::Path::new(&config_path)
        .parent()
        .map(|p| p.exists())
        .unwrap_or(false);

    // Try to probe the port configured in the config file, fall back to 4455
    let obs_port = if cfg_exists {
        if let Ok(cfg) = crate::config::load_config(std::path::Path::new(&config_path)) {
            cfg.obs_host
                .rsplit(':')
                .next()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(4455)
        } else {
            4455
        }
    } else {
        4455
    };
    let obs_ok = probe_obs_websocket(obs_port);

    // Auto-start row
    if autostart {
        println!("  {:<14}  {}", "Auto-start:", ok("enabled (systemd)"));
    } else {
        println!(
            "  {:<14}  {}  (run {} to enable)",
            "Auto-start:",
            warn("disabled"),
            key("obs-hotkey setup")
        );
    }

    // Input group row
    println!(
        "  {:<14}  {}",
        "Input group:",
        if input_grp {
            ok("member")
        } else {
            err("not a member")
        }
    );

    // Config row
    if cfg_exists {
        println!("  {:<14}  {}  {}", "Config:", ok(""), muted(&config_path));
    } else {
        println!("  {:<14}  {}  ({})", "Config:", err(""), muted(&config_path));
    }

    // Config dir row
    if dir_exists {
        let dir = std::path::Path::new(&config_path)
            .parent()
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
        println!(
            "  {:<14}  {}  (run {} to enable)",
            "Daemon:",
            warn("not running"),
            key("obs-hotkey daemon")
        );
    }

    println!();
    if !autostart {
        println!(
            "  Run {}obs-hotkey setup{} to enable auto-start.",
            key(""),
            key("setup")
        );
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
        let result = write_service_file(
            "/usr/bin/obs-hotkey",
            temp.path().join(".config/obs-hotkey").to_str().unwrap(),
        );
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
