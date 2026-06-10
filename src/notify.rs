use crate::config::NotifyConfig;

pub fn send_notification(cfg: &NotifyConfig, message: &str) {
    if !cfg.enabled {
        return;
    }
    if cfg.command.is_empty() {
        log::warn!("notifications enabled but notify.command is empty");
        return;
    }

    let mut command = cfg.command.iter().map(|arg| arg.replace("{message}", message));
    let program = match command.next() {
        Some(program) => program,
        None => return,
    };
    let args: Vec<String> = command.collect();
    match std::process::Command::new(&program).args(&args).output() {
        Ok(output) if output.status.success() => {
            log::info!("desktop notification sent: {}", message);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!(
                "desktop notification command failed with status {:?}: {} {}",
                output.status.code(),
                stderr.trim(),
                message
            );
        }
        Err(e) => {
            log::warn!(
                "failed to run desktop notification command '{}': {}",
                program,
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_disabled_does_not_run() {
        let cfg = NotifyConfig {
            enabled: false,
            command: vec!["/bin/this/does/not/exist".to_string()],
        };
        send_notification(&cfg, "ignored");
    }

    #[test]
    fn test_notification_empty_command_logs_without_panic() {
        let cfg = NotifyConfig {
            enabled: true,
            command: Vec::new(),
        };
        send_notification(&cfg, "ignored");
    }
}
