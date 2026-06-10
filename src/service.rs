use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::config::{expand_home, real_home, load_config};
use crate::input::find_keyboards_with_filter;
use crate::obs::OBSClient;

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
    let c_user = match std::ffi::CString::new(user.clone()) {
        Ok(c) => c,
        Err(_) => {
            log::warn!(
                "user name '{}' contains a null byte — cannot check groups",
                user
            );
            return false;
        }
    };

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

    // getgrouplist succeeded but 'input' group was not in the list
    log::warn!(
        "user '{}' has {} group(s) but 'input' is not among them — hint: sudo usermod -aG input $(whoami)",
        user,
        ngroups
    );

    false
}

fn print_check(name: &str, ok: bool, detail: &str) {
    let status = if ok { "ok" } else { "fail" };
    println!("  {:<24} {}  {}", name, status, detail);
}

fn ws_url_from_config_path(config_path: &str) -> String {
    let expanded = expand_home(config_path);
    if let Ok(cfg) = load_config(std::path::Path::new(&expanded)) {
        if cfg.obs_host.is_empty() {
            "ws://localhost:4455".to_string()
        } else {
            cfg.obs_host.clone()
        }
    } else {
        "ws://localhost:4455".to_string()
    }
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
    println!(
        "  {:<14} {}",
        "Installing:",
        key(&format!("v{}", env!("CARGO_PKG_VERSION")))
    );
    println!("  {:<14} {}", "Binary:", muted(&exe_path));

    // Show currently installed/running version if any
    let unit_path = service_unit_path();
    if unit_path.exists() {
        if let Ok(existing) = std::fs::read_to_string(&unit_path) {
            if let Some(line) = existing.lines().find(|l| l.starts_with("ExecStart=")) {
                println!(
                    "  {:<14} {}",
                    "Replacing:",
                    muted(line.trim_start_matches("ExecStart="))
                );
            }
        }
    }
    if is_autostart_enabled() {
        println!("  {:<14} {}", "Service:", ok("currently running"));
    }
    println!();

    // Ensure config directory and default config exist before starting the
    // service, so the daemon doesn't fail on first run due to missing paths.
    let cfg_dir = std::path::Path::new(&config_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    if let Err(e) = crate::config::ensure_config(cfg_dir, std::path::Path::new(&config_path)) {
        log::warn!("Could not ensure config exists: {}", e);
    }

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

    match Command::new("systemctl")
        .args(["--user", "start", "obs-hotkey.service"])
        .output()
    {
        Ok(output) if output.status.success() => {
            println!("  {} Service started", ok(""));
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("  {} Service failed to start: {}", err(""), stderr.trim());
        }
        Err(e) => {
            println!("  {} Could not start service: {}", err(""), e);
        }
    }

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
                println!(
                    "  {:<14} {}",
                    "Service:",
                    muted(line.trim_start_matches("ExecStart="))
                );
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
            println!(
                "  {} Stopping {} running process(es)...",
                heading("▶"),
                pids.len()
            );
            for pid in &pids {
                let _ = Command::new("kill").arg(format!("{}", pid)).output();
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            // Force kill any that are still alive
            for pid in &pids {
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(format!("{}", pid))
                    .output();
            }
        }
        if stale_local_bin.exists() {
            println!(
                "  {} Removing stale binary: {}",
                heading("▶"),
                stale_local_bin.display()
            );
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
    let old_service = service_unit_path()
        .parent()
        .unwrap()
        .join("obs-wayland-hotkey.service");

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
            let _ = ws.close(None); // Send proper WebSocket close frame
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
    let cfg = cfg_exists.then(|| load_config(std::path::Path::new(&config_path))).transpose();
    let ws_url = ws_url_from_config_path(&config_path);
    let obs_ok = probe_obs_websocket_url(&ws_url);

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
        println!(
            "  {:<14}  {}  ({})",
            "Config:",
            err(""),
            muted(&config_path)
        );
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

    if let Ok(cfg) = &cfg {
        match OBSClient::new(ws_url.clone()).get_status(&cfg.mic_name) {
            Ok(status) => {
                println!(
                    "  {:<14}  {}  {}",
                    "Recording:",
                    if status.record_active { ok("") } else { warn("") },
                    if status.record_active {
                        format!("active{}", status.record_timecode.as_deref().map(|s| format!(" {}", s)).unwrap_or_default())
                    } else {
                        "inactive".to_string()
                    }
                );
                println!(
                    "  {:<14}  {}  {}",
                    "Streaming:",
                    if status.stream_active { ok("") } else { warn("") },
                    if status.stream_active {
                        format!("active{}", status.stream_timecode.as_deref().map(|s| format!(" {}", s)).unwrap_or_default())
                    } else {
                        "inactive".to_string()
                    }
                );
                println!(
                    "  {:<14}  {}  {}",
                    "Replay:",
                    if status.replay_active { ok("") } else { warn("") },
                    if status.replay_active { "active" } else { "inactive" }
                );
                println!(
                    "  {:<14}  {}  {}",
                    "Scene:",
                    ok(""),
                    status.current_scene.as_deref().unwrap_or("unknown")
                );
                if !cfg.mic_name.trim().is_empty() {
                    println!(
                        "  {:<14}  {}  {}{}",
                        "Mic:",
                        ok(""),
                        cfg.mic_name,
                        status.input_muted.map(|m| if m { " muted" } else { " unmuted" }).unwrap_or_default()
                    );
                    if let Some(volume) = status.input_volume_mul {
                        println!("  {:<14}  {}  {:.2}x", "Mic volume:", ok(""), volume);
                    }
                }
            }
            Err(e) => {
                println!("  {:<14}  {}  {}", "OBS status:", warn(""), e);
            }
        }
    } else if let Err(e) = &cfg {
        println!("  {:<14}  {}  {}", "Config parse:", err(""), e);
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

fn probe_obs_websocket_url(url: &str) -> bool {
    let socket_addrs: Vec<std::net::SocketAddr> = match url
        .strip_prefix("ws://")
        .unwrap_or(url)
        .to_socket_addrs()
    {
        Ok(addrs) => addrs.collect(),
        Err(_) => return false,
    };
    let Some(addr) = socket_addrs.first() else {
        return false;
    };
    let stream = match TcpStream::connect_timeout(addr, Duration::from_secs(1)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

    let mut ws = match tungstenite::client(url, stream) {
        Ok((ws, _)) => ws,
        Err(_) => return false,
    };

    match ws.read() {
        Ok(msg) => {
            let result = msg.to_text().map(|t| t.starts_with("{")).unwrap_or(false);
            let _ = ws.close(None);
            result
        }
        Err(_) => false,
    }
}

pub fn run_doctor(config_path: &str) -> anyhow::Result<()> {
    use crate::ansi::*;

    println!();
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!("  {}  OBS Hotkey Doctor{}", BOLD, RESET);
    println!("  {}", heading("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    println!();

    let mut failed = false;
    let config_path = expand_home(config_path);
    let path = std::path::Path::new(&config_path);
    let config_exists = path.exists();
    print_check("Config exists", config_exists, &config_path);
    failed |= !config_exists;
    if !config_exists {
        println!("  hint: run `obs-hotkey setup` or create {}", config_path);
    }

    let cfg = match config_exists.then(|| load_config(path)).transpose() {
        Ok(Some(cfg)) => {
            print_check("Config parses", true, "ok");
            cfg
        }
        Ok(None) => return Ok(()),
        Err(e) => {
            print_check("Config parses", false, &e.to_string());
            failed |= true;
            println!("  hint: fix the JSON/schema error before starting the daemon");
            return Ok(());
        }
    };

    let combo_validation = crate::validate_combo_actions(&cfg);
    print_check("Combo actions", combo_validation.is_ok(), &combo_validation.as_ref().map(|_| "ok").unwrap_or_else(|e| e.to_string()));
    failed |= combo_validation.is_err();

    let chord_validation = crate::validate_configured_chords(&cfg);
    print_check("Hotkey chords", chord_validation.is_ok(), &chord_validation.as_ref().map(|_| "ok").unwrap_or_else(|e| e.to_string()));
    failed |= chord_validation.is_err();

    let input_grp = in_input_group();
    print_check("Input group", input_grp, if input_grp { "member" } else { "not a member" });
    failed |= !input_grp;

    let keyboards = find_keyboards_with_filter(&cfg.allowed_devices);
    match keyboards {
        Ok(paths) => {
            print_check("Keyboard devices", !paths.is_empty(), &format!("{} found", paths.len()));
            failed |= paths.is_empty();
        }
        Err(e) => {
            print_check("Keyboard devices", false, &e.to_string());
            failed |= true;
        }
    }

    let ws_url = if cfg.obs_host.is_empty() {
        "ws://localhost:4455".to_string()
    } else {
        cfg.obs_host.clone()
    };
    let obs_ok = probe_obs_websocket_url(&ws_url);
    print_check("OBS WebSocket", obs_ok, if obs_ok { "reachable" } else { "unreachable" });
    failed |= !obs_ok;

    if obs_ok {
        match OBSClient::new(ws_url).get_status(&cfg.mic_name) {
            Ok(status) => {
                print_check("OBS status", true, "queried");
                println!(
                    "  detail: recording={} streaming={} replay={} scene={}",
                    status.record_active,
                    status.stream_active,
                    status.replay_active,
                    status.current_scene.as_deref().unwrap_or("unknown")
                );
            }
            Err(e) => {
                print_check("OBS status", false, &e.to_string());
                failed |= true;
            }
        }
    }

    print_check("Notify config", !cfg.notify.command.is_empty(), "ok");
    failed |= cfg.notify.command.is_empty();
    print_check("HTTP config", !cfg.http.enabled || crate::config::http_config_is_safe(&cfg.http), "ok");
    failed |= cfg.http.enabled && !crate::config::http_config_is_safe(&cfg.http);

    println!();
    if failed {
        anyhow::bail!("doctor found one or more problems")
    }
    println!("  {} All checks passed", ok(""));
    Ok(())
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
