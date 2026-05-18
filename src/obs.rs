use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Timeout for TCP connect attempts to OBS WebSocket.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Timeout for individual WebSocket reads (responses from OBS).
const READ_TIMEOUT: Duration = Duration::from_secs(10);

pub struct OBSClient {
    conn: Arc<Mutex<Option<Conn>>>,
    connected: Arc<AtomicBool>,
    studio_mode_enabled: Arc<AtomicBool>,
    studio_mode_queried: Arc<AtomicBool>,
    ws_url: String,
}

impl Clone for OBSClient {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            connected: Arc::clone(&self.connected),
            studio_mode_enabled: Arc::clone(&self.studio_mode_enabled),
            studio_mode_queried: Arc::clone(&self.studio_mode_queried),
            ws_url: self.ws_url.clone(),
        }
    }
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OBSClient>();
};

struct Conn {
    ws: tungstenite::WebSocket<TcpStream>,
}

impl OBSClient {
    pub fn new(ws_url: String) -> Self {
        Self {
            conn: Arc::new(Mutex::new(None)),
            connected: Arc::new(AtomicBool::new(false)),
            studio_mode_enabled: Arc::new(AtomicBool::new(false)),
            studio_mode_queried: Arc::new(AtomicBool::new(false)),
            ws_url,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn connect(&self) -> anyhow::Result<()> {
        // Set connected=false BEFORE acquiring the lock.
        // This is safe because:
        //   - If the reconnect thread sees connected=false, it will try to
        //     call connect(), which will block on the lock (not deadlock,
        //     since we don't hold it yet).
        //   - If the main event loop sees connected=false, it will skip
        //     sending and wait for the reconnect thread.
        self.connected.store(false, Ordering::SeqCst);

        let mut guard = self.conn.lock().unwrap();
        if let Some(ref mut c) = *guard {
            let _ = c.ws.close(None);
        }
        *guard = None;

        // Strip ws:// prefix to get host:port for TCP
        let tcp_addr = self.ws_url.strip_prefix("ws://").unwrap_or(&self.ws_url);
        // Reject wss:// (TLS) -- not supported yet
        if self.ws_url.starts_with("wss://") {
            anyhow::bail!(
                "wss:// (TLS) is not yet supported. Use ws:// or a plain host:port in your config."
            );
        }
        // Resolve host:port with DNS, apply connect timeout
        let socket_addrs: Vec<SocketAddr> = tcp_addr
            .to_socket_addrs()?
            .collect();
        let stream = TcpStream::connect_timeout(
            socket_addrs.first().ok_or_else(|| anyhow::anyhow!("could not resolve '{}'", tcp_addr))?,
            CONNECT_TIMEOUT,
        )?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;

        let (mut ws, resp) = tungstenite::client(&self.ws_url, stream)?;

        log::info!("Connected to OBS WebSocket");
        log::debug!("Handshake response: {:?}", resp);

        let mut msg = ws.read()?;
        let hello: HelloMessage =
            serde_json::from_str(msg.to_text().map_err(|e| {
                anyhow::anyhow!("unexpected binary frame during handshake: {}", e)
            })?)?;
        log::info!("OBS WebSocket version: {}", hello.d.obs_web_socket_version);
        // Reject if OBS requires authentication (not yet supported)
        if hello.d.authentication.is_some() {
            anyhow::bail!(
                "OBS WebSocket has authentication enabled, which is not yet supported. \
                Please disable authentication in OBS → Tools → WebSocket Server Settings."
            );
        }
        // obs-websocket 5.x supports rpcVersion 1 but needs eventSubscriptions
        let event_subscriptions: u32 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x40 | 0x80;

        let ident = IdentifyMessage {
            op: 1,
            d: IdentifyData {
                rpc_version: 1,
                event_subscriptions,
                authentication: None,
            },
        };
        let ident_json = serde_json::to_string(&ident).unwrap();
        log::info!("Sending identify: {}", ident_json);
        ws.send(tungstenite::Message::Text(ident_json.into()))?;

        msg = ws.read()?;
        let text = msg
            .to_text()
            .map_err(|e| anyhow::anyhow!("unexpected binary frame after identify: {}", e))?;
        log::info!("Identify response: {}", text);
        // Verify we got Identified (op 2) response
        let resp: serde_json::Value = serde_json::from_str(text)?;
        let op = resp.get("op").and_then(|o| o.as_u64()).unwrap_or(0);
        if op != 2 {
            anyhow::bail!("OBS rejected identification (op={}): {}", op, text);
        }

        log::info!("Successfully identified to OBS WebSocket");
        *guard = Some(Conn { ws });
        self.connected.store(true, Ordering::SeqCst);
        drop(guard);
        self.query_studio_mode();
        Ok(())
    }

    pub fn send_request(&self, request_type: &str) -> anyhow::Result<()> {
        self.send_request_with_data(request_type, None)
    }

    pub fn send_request_with_data(
        &self,
        request_type: &str,
        request_data: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let request_id = format!(
            "{}_{}",
            request_type,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let req = RequestMessage {
            op: 6,
            d: RequestData {
                request_type: request_type.to_string(),
                request_id,
                request_data,
            },
        };
        let json = serde_json::to_string(&req)?;

        let mut guard = self.conn.lock().unwrap();
        let conn = match guard.as_mut() {
            Some(c) => c,
            None => {
                drop(guard);
                log::info!("Not connected to OBS. Reconnecting...");
                self.connect()?;
                guard = self.conn.lock().unwrap();
                guard.as_mut().unwrap()
            }
        };
        conn.ws.send(tungstenite::Message::Text(json.into()))?;

        // Keep lock while reading response to prevent connection close race.
        // Without this, OBS could close the connection between send and read,
        // causing "Broken pipe" instead of a clean reconnection trigger.
        let resp = self.read_response_guarded(&mut guard)?;
        if let Some(status) = resp.get("d").and_then(|d| d.get("requestStatus")) {
            if !status
                .get("result")
                .and_then(|r| r.as_bool())
                .unwrap_or(false)
            {
                anyhow::bail!("request {} failed: {:?}", request_type, status);
            }
        }
        Ok(())
    }

    fn read_response_guarded(&self, guard: &mut std::sync::MutexGuard<'_, Option<Conn>>) -> anyhow::Result<serde_json::Value> {
        let msg = match guard.as_mut() {
            Some(c) => match c.ws.read() {
                Ok(m) => m,
                Err(tungstenite::Error::Io(ref e))
                    if e.kind() == std::io::ErrorKind::BrokenPipe
                        || e.kind() == std::io::ErrorKind::ConnectionReset
                        || e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Connection was closed by OBS — clear state so next request reconnects
                    *guard = None;
                    self.connected.store(false, Ordering::SeqCst);
                    anyhow::bail!("OBS WebSocket closed: {:?}", e);
                }
                Err(e) => anyhow::bail!("WebSocket read error: {:?}", e),
            },
            None => anyhow::bail!("not connected to OBS"),
        };
        let text = msg
            .into_text()
            .map_err(|e| anyhow::anyhow!("unexpected non-text WebSocket frame: {}", e))?;
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("invalid JSON: {}", e))
    }

    /// Reads a response from the WebSocket connection.
    ///
    /// # Safety
    ///
    /// This method assumes single-threaded access to the WebSocket. In a
    /// multi-threaded context, responses could be consumed by a different
    /// thread than the one that issued the request. The current design uses
    /// a `Mutex<Conn>` which serializes access, but does not pair requests
    /// with responses. This is safe only when calls are made from a single
    /// thread (e.g., the main event loop), which is the case for this
    /// daemon.
    fn read_response(&self) -> anyhow::Result<serde_json::Value> {
        // Re-check connected before reading — the connection may have been
        // closed by OBS between send_request_with_data releasing the lock and
        // this read call acquiring it again. Without this check, a closed
        // connection produces "Broken pipe" instead of a clear error.
        if !self.connected.load(Ordering::SeqCst) {
            anyhow::bail!("not connected to OBS");
        }
        let mut guard = self.conn.lock().unwrap();
        self.read_response_guarded(&mut guard)
    }

    pub fn toggle_recording(&self) {
        log::info!("Toggling recording...");
        if let Err(e) = self.send_request("ToggleRecord") {
            log::warn!("Error toggling recording: {}", e);
        }
    }

    pub fn toggle_pause(&self) {
        log::info!("Toggling record pause...");
        if let Err(e) = self.send_request("ToggleRecordPause") {
            log::warn!("Error toggling pause: {}", e);
        }
    }

    pub fn toggle_streaming(&self) {
        log::info!("Toggling stream...");
        if let Err(e) = self.send_request("ToggleStream") {
            log::warn!("Error toggling stream: {}", e);
        }
    }

    pub fn screenshot(&self, source_name: &str, save_dir: &str) {
        log::info!("Taking screenshot...");

        // Ensure the screenshot directory exists
        if let Err(e) = std::fs::create_dir_all(save_dir) {
            log::warn!(
                "Could not create screenshot directory '{}': {}",
                save_dir,
                e
            );
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let mut data = serde_json::json!({
            "imageFormat": "png",
            "imageFilePath": format!("{}/obs-screenshot-{}.png", save_dir, now_ms),
        });
        if !source_name.is_empty() {
            data["sourceName"] = serde_json::json!(source_name);
        }
        if let Err(e) = self.send_request_with_data("SaveSourceScreenshot", Some(data)) {
            log::warn!("Error taking screenshot: {}", e);
        } else {
            log::info!("Screenshot saved");
        }
    }

    pub fn toggle_mute_mic(&self, input_name: &str) {
        if input_name.is_empty() {
            log::info!("Mic input name not configured, skipping mute toggle");
            return;
        }
        log::info!("Toggling mic mute...");
        let data = serde_json::json!({ "inputName": input_name });
        if let Err(e) = self.send_request_with_data("ToggleInputMute", Some(data)) {
            log::warn!("Error toggling mic mute: {}", e);
        }
    }

    pub fn toggle_studio_mode(&self) {
        log::info!("Toggling studio mode...");
        // If state is unknown, query first to avoid sending wrong value
        if !self.studio_mode_queried.load(Ordering::SeqCst) {
            log::info!("Studio mode state unknown, querying...");
            self.query_studio_mode();
            if !self.studio_mode_queried.load(Ordering::SeqCst) {
                log::warn!("Cannot toggle studio mode: failed to query current state");
                return;
            }
        }
        let new_state = !self.studio_mode_enabled.load(Ordering::SeqCst);
        let data = serde_json::json!({ "studioModeEnabled": new_state });
        if let Err(e) = self.send_request_with_data("SetStudioModeEnabled", Some(data)) {
            log::warn!("Error toggling studio mode: {}", e);
            // Re-query to resync state — our local state may be wrong
            self.studio_mode_queried.store(false, Ordering::SeqCst);
        } else {
            self.studio_mode_enabled.store(new_state, Ordering::SeqCst);
            log::info!("Studio mode set to: {}", new_state);
        }
    }

    pub fn toggle_replay_buffer(&self) {
        log::info!("Toggling replay buffer...");
        if let Err(e) = self.send_request("ToggleReplayBuffer") {
            log::warn!("Error toggling replay buffer: {}", e);
        }
    }

    pub fn save_replay(&self) {
        log::info!("Saving replay buffer...");
        if let Err(e) = self.send_request("SaveReplayBuffer") {
            log::warn!("Error saving replay: {}", e);
        }
    }

    pub fn close(&self) {
        let mut guard = self.conn.lock().unwrap();
        if let Some(ref mut c) = *guard {
            let _ = c.ws.close(None);
        }
        *guard = None;
        self.connected.store(false, Ordering::SeqCst);
    }

    fn query_studio_mode(&self) {
        let request_id = format!(
            "GetStudioModeEnabled_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let req = RequestMessage {
            op: 6,
            d: RequestData {
                request_type: "GetStudioModeEnabled".to_string(),
                request_id: request_id.clone(),
                request_data: None,
            },
        };
        let json = serde_json::to_string(&req).unwrap();

        {
            let mut guard = self.conn.lock().unwrap();
            if !self.connected.load(Ordering::SeqCst) {
                log::info!("Cannot query studio mode: not connected");
                return;
            }
            let conn = guard.as_mut().unwrap();
            if conn
                .ws
                .send(tungstenite::Message::Text(json.into()))
                .is_err()
            {
                log::warn!("Failed to send studio mode query");
                return;
            }
        }

        let resp = self.read_response();
        if let Ok(data) = resp {
            // Verify the response matches our request_id
            if let Some(resp_id) = data.get("d").and_then(|d| d.get("requestId")) {
                if resp_id.as_str() == Some(&request_id) {
                    if let Some(resp_data) = data.get("d").and_then(|d| d.get("responseData")) {
                        let enabled = resp_data
                            .get("studioModeEnabled")
                            .and_then(|e| e.as_bool())
                            .unwrap_or(false);
                        self.studio_mode_enabled.store(enabled, Ordering::SeqCst);
                    }
                } else {
                    log::warn!(
                        "Studio mode response id mismatch: expected {}, got {:?}",
                        request_id,
                        resp_id
                    );
                }
            }
        }
        self.studio_mode_queried.store(true, Ordering::SeqCst);
        log::info!(
            "Studio mode is currently: {}",
            self.studio_mode_enabled.load(Ordering::SeqCst)
        );
    }
}

#[derive(Debug, Deserialize)]
struct HelloMessage {
    #[allow(dead_code)]
    op: u8,
    d: HelloData,
}

#[derive(Debug, Deserialize)]
struct HelloData {
    #[serde(rename = "obsWebSocketVersion")]
    obs_web_socket_version: String,
    #[serde(rename = "authentication", default)]
    #[allow(dead_code)]
    authentication: Option<AuthenticationChallenge>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AuthenticationChallenge {
    #[serde(rename = "challenge")]
    challenge: String,
    #[serde(rename = "salt")]
    salt: String,
}

#[derive(Debug, Serialize)]
struct IdentifyMessage {
    op: u8,
    d: IdentifyData,
}

#[derive(Debug, Serialize)]
struct IdentifyData {
    #[serde(rename = "rpcVersion")]
    rpc_version: u32,
    #[serde(rename = "eventSubscriptions")]
    event_subscriptions: u32,
    #[serde(rename = "authentication", skip_serializing_if = "Option::is_none")]
    authentication: Option<String>,
}

#[derive(Debug, Serialize)]
struct RequestMessage {
    op: u8,
    d: RequestData,
}

#[derive(Debug, Serialize)]
struct RequestData {
    #[serde(rename = "requestType")]
    request_type: String,
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "requestData")]
    request_data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obs_client_creation() {
        let client = OBSClient::new("ws://localhost:4455".to_string());
        assert!(!client.connected.load(Ordering::SeqCst));
    }

    #[test]
    fn test_obs_client_clone_send_sync() {
        fn assert_send<T: Send + Sync>(_: &T) {}
        let client = OBSClient::new("ws://localhost:4455".to_string());
        assert_send(&client);
    }

    #[test]
    fn test_identify_data_skips_none_auth() {
        let ident = IdentifyData {
            rpc_version: 1,
            event_subscriptions: 255,
            authentication: None,
        };
        let json = serde_json::to_string(&ident).unwrap();
        assert!(!json.contains("authentication"));
    }

    #[test]
    fn test_identify_data_includes_some_auth() {
        let ident = IdentifyData {
            rpc_version: 1,
            event_subscriptions: 255,
            authentication: Some("token123".to_string()),
        };
        let json = serde_json::to_string(&ident).unwrap();
        assert!(json.contains("authentication"));
        assert!(json.contains("token123"));
    }
}
