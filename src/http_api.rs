use crate::config::{HttpConfig, NotifyConfig};
use crate::notify;
use crate::{run_action_by_name, ActionContext};
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

pub fn spawn(cfg: HttpConfig, ctx: ActionContext, notify_cfg: NotifyConfig) {
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
            if let Err(e) = handle_connection(stream, &cfg, &ctx, &notify_cfg) {
                log::warn!("HTTP request failed: {}", e);
            }
        }
    });
}

fn handle_connection(
    mut stream: std::net::TcpStream,
    cfg: &HttpConfig,
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

    let response = route_request(parsed, cfg, ctx, notify_cfg);
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
    ctx: &ActionContext,
    notify_cfg: &NotifyConfig,
) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => HttpResponse {
            status: 200,
            body: json!({"ok": true, "service": "obs-hotkey"}),
        },
        ("GET", "/status") => match ctx.client.get_status(&ctx.mic_name) {
            Ok(status) => HttpResponse {
                status: 200,
                body: json!({"ok": true, "status": status}),
            },
            Err(e) => HttpResponse {
                status: 200,
                body: json!({"ok": true, "status": {"unavailable": true}, "error": e.to_string()}),
            },
        },
        ("POST", "/actions") => match parse_action_request(&request.body) {
            Ok((action, scene)) => run_http_action(action, scene, ctx, notify_cfg),
            Err(e) => HttpResponse {
                status: 400,
                body: json!({"ok": false, "error": e.to_string()}),
            },
        },
        ("POST", path) if path.starts_with("/actions/") => {
            let action = path.trim_start_matches("/actions/").to_string();
            let scene = query_param(&request.path, "scene")
                .or_else(|| parse_scene_from_body(&request.body).ok().flatten());
            run_http_action(action, scene, ctx, notify_cfg)
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

fn run_http_action(
    action: String,
    scene: Option<String>,
    ctx: &ActionContext,
    notify_cfg: &NotifyConfig,
) -> HttpResponse {
    match run_action_by_name(&action, scene.as_deref(), ctx) {
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
}
