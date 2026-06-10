use crate::config::{AppConfig, HttpConfig, NotifyConfig};
use crate::notify;
use crate::obs::ObsStatus;
use crate::{run_action_by_name, ActionContext};
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

pub fn spawn(cfg: HttpConfig, app_cfg: AppConfig, ctx: ActionContext, notify_cfg: NotifyConfig) {
    if !cfg.enabled {
        return;
    }

    let bind = cfg.bind.clone();
    let listener = match TcpListener::bind(&bind) {
        Ok(listener) => listener,
        Err(e) => {
            log::error!("failed to bind HTTP listener on {}: {}", bind, e);
            return;
        }
    };

    thread::spawn(move || {
        log::info!("HTTP listener started on {}", bind);
        for stream in listener.incoming().flatten() {
            if let Err(e) = handle_connection(stream, &cfg, &app_cfg, &ctx, &notify_cfg) {
                log::warn!("HTTP request failed: {}", e);
            }
        }
    });
}

fn handle_connection(
    mut stream: std::net::TcpStream,
    cfg: &HttpConfig,
    app_cfg: &AppConfig,
    ctx: &ActionContext,
    notify_cfg: &NotifyConfig,
) -> anyhow::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;

    let mut buffer = [0u8; 8192];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let parsed = parse_request(&request);

    if !authorized(cfg, &parsed.headers) {
        write_response(
            &mut stream,
            401,
            json!({"ok": false, "error": "unauthorized"}),
        )?;
        return Ok(());
    }

    let response = route_request(parsed, cfg, app_cfg, ctx, notify_cfg);
    write_response(&mut stream, response.status, response.body)
}

#[derive(Debug, PartialEq, Eq)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Debug, PartialEq, Eq)]
struct HttpResponse {
    status: u16,
    body: serde_json::Value,
}

fn parse_request(raw: &str) -> HttpRequest {
    let mut lines = raw.split("\r\n");
    let mut parts = lines.next().unwrap_or_default().split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or_default().to_string();
    HttpRequest {
        method,
        path,
        headers,
        body,
    }
}

fn authorized(cfg: &HttpConfig, headers: &HashMap<String, String>) -> bool {
    let Some(token) = cfg
        .token
        .as_ref()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
    else {
        return true;
    };
    let auth = headers
        .get("authorization")
        .map(|v| v.as_str())
        .unwrap_or_default();
    let header_token = headers
        .get("x-obs-hotkey-token")
        .map(|v| v.as_str())
        .unwrap_or_default();
    auth == format!("Bearer {}", token) || header_token == token
}

fn route_request(
    request: HttpRequest,
    _cfg: &HttpConfig,
    app_cfg: &AppConfig,
    ctx: &ActionContext,
    notify_cfg: &NotifyConfig,
) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => HttpResponse {
            status: 200,
            body: json!({"ok": true, "service": "obs-hotkey"}),
        },
        ("GET", "/status") => match ctx.client.get_status(&ctx.mic_name) {
            Ok(status) => obs_status_response(status, &ctx.mic_name),
            Err(e) => obs_status_unavailable_response(&e.to_string(), &ctx.mic_name),
        },
        ("POST", "/actions") => match parse_action_request(&request.body) {
            Ok((action, scene)) => run_http_action(action, scene, app_cfg, ctx, notify_cfg),
            Err(e) => HttpResponse {
                status: 400,
                body: json!({"ok": false, "error": e.to_string()}),
            },
        },
        ("POST", "/macros") => match parse_macro_request(&request.body) {
            Ok(macro_name) => run_http_macro(macro_name, app_cfg, ctx, notify_cfg),
            Err(e) => HttpResponse {
                status: 400,
                body: json!({"ok": false, "error": e.to_string()}),
            },
        },
        ("POST", path) if path.starts_with("/actions/") => {
            let action = path.trim_start_matches("/actions/").to_string();
            let scene = query_param(&request.path, "scene")
                .or_else(|| parse_scene_from_body(&request.body).ok().flatten());
            run_http_action(action, scene, app_cfg, ctx, notify_cfg)
        }
        ("POST", path) if path.starts_with("/macros/") => {
            let macro_name = path.trim_start_matches("/macros/").to_string();
            run_http_macro(macro_name, app_cfg, ctx, notify_cfg)
        }
        _ => HttpResponse {
            status: 404,
            body: json!({"ok": false, "error": "not found"}),
        },
    }
}

fn parse_action_request(body: &str) -> anyhow::Result<(String, Option<String>)> {
    if body.trim().is_empty() {
        anyhow::bail!("empty request body");
    }
    let value: serde_json::Value = serde_json::from_str(body)?;
    let action = value
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing action"))?
        .to_string();
    let scene = value
        .get("scene")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    Ok((action, scene))
}

fn parse_macro_request(body: &str) -> anyhow::Result<String> {
    if body.trim().is_empty() {
        anyhow::bail!("empty request body");
    }
    let value: serde_json::Value = serde_json::from_str(body)?;
    value
        .get("macro")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("missing macro"))
}

fn parse_scene_from_body(body: &str) -> anyhow::Result<Option<String>> {
    if body.trim().is_empty() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(body)?;
    Ok(value
        .get("scene")
        .and_then(|v| v.as_str())
        .map(ToString::to_string))
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let (_, query) = path.split_once('?')?;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value);
                    i += 3;
                    continue;
                }
            }
        }
        output.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&output).to_string()
}

fn obs_status_response(status: ObsStatus, mic_name: &str) -> HttpResponse {
    HttpResponse {
        status: 200,
        body: json!({
            "ok": true,
            "service": "obs-hotkey",
            "obs": {
                "reachable": true,
                "recording": {
                    "active": status.record_active,
                    "paused": status.record_paused,
                    "timecode": status.record_timecode
                },
                "streaming": {
                    "active": status.stream_active,
                    "timecode": status.stream_timecode
                },
                "replay_buffer": {
                    "active": status.replay_active
                },
                "current_scene": status.current_scene,
                "input": input_status_json(mic_name, status.input_muted, status.input_volume_mul)
            },
            "status": status
        }),
    }
}

fn obs_status_unavailable_response(error: &str, mic_name: &str) -> HttpResponse {
    HttpResponse {
        status: 200,
        body: json!({
            "ok": true,
            "service": "obs-hotkey",
            "obs": {
                "reachable": false,
                "error": error,
                "recording": {
                    "active": false,
                    "paused": false,
                    "timecode": null
                },
                "streaming": {
                    "active": false,
                    "timecode": null
                },
                "replay_buffer": {
                    "active": false
                },
                "current_scene": null,
                "input": input_status_json(mic_name, None, None)
            },
            "status": {
                "unavailable": true,
                "error": error
            }
        }),
    }
}

fn input_status_json(
    mic_name: &str,
    muted: Option<bool>,
    volume_mul: Option<f64>,
) -> serde_json::Value {
    if mic_name.trim().is_empty() {
        serde_json::Value::Null
    } else {
        json!({
            "name": mic_name,
            "muted": muted,
            "volume_mul": volume_mul
        })
    }
}

fn run_http_action(
    action: String,
    scene: Option<String>,
    app_cfg: &AppConfig,
    ctx: &ActionContext,
    notify_cfg: &NotifyConfig,
) -> HttpResponse {
    match run_action_by_name(&action, scene.as_deref(), ctx, app_cfg) {
        Ok(()) => {
            notify::send_notification(notify_cfg, &format!("HTTP action {} triggered", action));
            HttpResponse {
                status: 200,
                body: json!({"ok": true, "message": format!("action {} triggered", action)}),
            }
        }
        Err(e) => HttpResponse {
            status: 400,
            body: json!({"ok": false, "error": e.to_string()}),
        },
    }
}

fn run_http_macro(
    macro_name: String,
    app_cfg: &AppConfig,
    ctx: &ActionContext,
    notify_cfg: &NotifyConfig,
) -> HttpResponse {
    match crate::run_macro_by_name(&macro_name, ctx, app_cfg) {
        Ok(()) => {
            notify::send_notification(notify_cfg, &format!("HTTP macro {} triggered", macro_name));
            HttpResponse {
                status: 200,
                body: json!({"ok": true, "message": format!("macro {} triggered", macro_name)}),
            }
        }
        Err(e) => HttpResponse {
            status: 400,
            body: json!({"ok": false, "error": e.to_string()}),
        },
    }
}

fn write_response(
    stream: &mut std::net::TcpStream,
    status: u16,
    body: serde_json::Value,
) -> anyhow::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "OK",
    };
    let body = serde_json::to_string(&body)?;
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, reason, body.len(), body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let raw = "POST /actions/switch_scene?scene=Gaming HTTP/1.1\r\nHost: localhost\r\n\r\n{}";
        let request = parse_request(raw);
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/actions/switch_scene?scene=Gaming");
        assert_eq!(request.headers.get("host"), Some(&"localhost".to_string()));
        assert_eq!(request.body, "{}");
    }

    #[test]
    fn test_parse_macro_request() {
        let request = parse_macro_request(r#"{"macro":"countdown_record"}"#).unwrap();
        assert_eq!(request, "countdown_record");
    }

    #[test]
    fn test_parse_macro_request_missing_macro() {
        let result = parse_macro_request(r#"{"action":"countdown_record"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_authorized_without_token() {
        let cfg = HttpConfig {
            enabled: true,
            bind: "127.0.0.1:7999".to_string(),
            token: None,
        };
        assert!(authorized(&cfg, &HashMap::new()));
    }

    #[test]
    fn test_authorized_with_bearer_token() {
        let cfg = HttpConfig {
            enabled: true,
            bind: "127.0.0.1:7999".to_string(),
            token: Some("secret".to_string()),
        };
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Bearer secret".to_string());
        assert!(authorized(&cfg, &headers));
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("Gaming%20Scene"), "Gaming Scene");
    }

    #[test]
    fn test_obs_status_response_feedback_shape() {
        let response = obs_status_response(
            ObsStatus {
                stream_active: false,
                stream_timecode: None,
                record_active: true,
                record_paused: false,
                record_timecode: Some("00:01:02".to_string()),
                replay_active: true,
                current_scene: Some("Live".to_string()),
                input_muted: Some(true),
                input_volume_mul: Some(0.75),
            },
            "Mic",
        );

        assert_eq!(response.status, 200);
        assert_eq!(response.body["ok"], true);
        assert_eq!(response.body["obs"]["reachable"], true);
        assert_eq!(response.body["obs"]["recording"]["active"], true);
        assert_eq!(response.body["obs"]["recording"]["timecode"], "00:01:02");
        assert_eq!(response.body["obs"]["current_scene"], "Live");
        assert_eq!(response.body["obs"]["input"]["name"], "Mic");
        assert_eq!(response.body["obs"]["input"]["muted"], true);
        assert_eq!(response.body["obs"]["input"]["volume_mul"], 0.75);
    }

    #[test]
    fn test_obs_status_unavailable_response_feedback_shape() {
        let response = obs_status_unavailable_response("not reachable", "Mic");

        assert_eq!(response.status, 200);
        assert_eq!(response.body["ok"], true);
        assert_eq!(response.body["obs"]["reachable"], false);
        assert_eq!(response.body["obs"]["error"], "not reachable");
        assert_eq!(response.body["obs"]["recording"]["active"], false);
        assert_eq!(response.body["status"]["unavailable"], true);
        assert_eq!(response.body["status"]["error"], "not reachable");
        assert_eq!(response.body["obs"]["input"]["name"], "Mic");
    }

    #[test]
    fn test_input_status_json_null_when_mic_not_configured() {
        assert_eq!(
            input_status_json("", Some(true), Some(0.5)),
            serde_json::Value::Null
        );
    }
}
