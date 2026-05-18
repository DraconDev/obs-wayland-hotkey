use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct OBSClient {
    conn: Arc<Mutex<Option<Conn>>>,
    connected: AtomicBool,
    studio_mode_enabled: AtomicBool,
    studio_mode_queried: AtomicBool,
    ws_url: String,
}

impl Clone for OBSClient {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
            studio_mode_enabled: AtomicBool::new(self.studio_mode_enabled.load(Ordering::SeqCst)),
            studio_mode_queried: AtomicBool::new(self.studio_mode_queried.load(Ordering::SeqCst)),
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
            connected: AtomicBool::new(false),
            studio_mode_enabled: AtomicBool::new(false),
            studio_mode_queried: AtomicBool::new(false),
            ws_url,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn connect(&self) -> anyhow::Result<()> {
        let mut guard = self.conn.lock().unwrap();
        if let Some(ref mut c) = *guard {
            let _ = c.ws.close(None);
        }
        *guard = None;
        self.connected.store(false, Ordering::SeqCst);

        // Strip ws:// prefix to get host:port for TCP
        let tcp_addr = self
            .ws_url
            .strip_prefix("ws://")
            .or_else(|| self.ws_url.strip_prefix("wss://"))
            .unwrap_or(&self.ws_url);
        let stream = TcpStream::connect(tcp_addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;

        let (mut ws, resp) = tungstenite::client(&self.ws_url, stream)?;

        log::info!("Connected to OBS WebSocket");
        log::debug!("Handshake response: {:?}", resp);

        let mut msg = ws.read()?;
        let hello: HelloMessage =
            serde_json::from_str(msg.to_text().map_err(|e| {
                anyhow::anyhow!("unexpected binary frame during handshake: {}", e)
            })?)?;
        log::info!("OBS WebSocket version: {}", hello.d.obs_web_socket_version);

        // obs-websocket 5.x supports rpcVersion 1 but needs eventSubscriptions
        let event_subscriptions: u32 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x40 | 0x80;

        let ident = IdentifyMessage {
            op: 1,
            d: IdentifyData { 
                rpc_version: 1,
                event_subscriptions,
                authentication: None,  // TODO: implement password auth
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
        if !text.starts_with("{") {
            anyhow::bail!("failed to identify to OBS: unexpected message: {}", text);
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

        {
            let mut guard = self.conn.lock().unwrap();
            if !self.connected.load(Ordering::SeqCst) {
                drop(guard);
                log::info!("Not connected to OBS. Reconnecting...");
                self.connect()?;
                guard = self.conn.lock().unwrap();
            }
            let conn = guard.as_mut().unwrap();
            conn.ws.send(tungstenite::Message::Text(json.into()))?;
        }

        let resp = self.read_response()?;
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
        let mut guard = self.conn.lock().unwrap();
        let conn = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not connected to OBS"))?;
        let msg = conn.ws.read()?;
        let text = msg
            .into_text()
            .map_err(|e| anyhow::anyhow!("unexpected non-text WebSocket frame: {}", e))?;
        let data: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("invalid JSON: {}", e))?;
        Ok(data)
    }

    pub fn toggle_recording(&self) {
        log::info!("Toggling recording...");
        if let Err(e) = self.send_request("ToggleRecord") {
            log::info!("Error toggling recording: {}", e);
        }
    }

    pub fn toggle_pause(&self) {
        log::info!("Toggling record pause...");
        if let Err(e) = self.send_request("ToggleRecordPause") {
            log::info!("Error toggling pause: {}", e);
        }
    }

    pub fn toggle_streaming(&self) {
        log::info!("Toggling stream...");
        if let Err(e) = self.send_request("ToggleStream") {
            log::info!("Error toggling stream: {}", e);
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
            log::info!("Error taking screenshot: {}", e);
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
            log::info!("Error toggling mic mute: {}", e);
        }
    }

    pub fn toggle_studio_mode(&self) {
        log::info!("Toggling studio mode...");
        if !self.studio_mode_queried.load(Ordering::SeqCst) {
            log::info!("Studio mode state unknown, querying...");
        }
        let new_state = !self.studio_mode_enabled.load(Ordering::SeqCst);
        let data = serde_json::json!({ "studioModeEnabled": new_state });
        if let Err(e) = self.send_request_with_data("SetStudioModeEnabled", Some(data)) {
            log::info!("Error toggling studio mode: {}", e);
        } else {
            self.studio_mode_enabled.store(new_state, Ordering::SeqCst);
            log::info!("Studio mode set to: {}", new_state);
        }
    }

    pub fn toggle_replay_buffer(&self) {
        log::info!("Toggling replay buffer...");
        if let Err(e) = self.send_request("ToggleReplayBuffer") {
            log::info!("Error toggling replay buffer: {}", e);
        }
    }

    pub fn save_replay(&self) {
        log::info!("Saving replay buffer...");
        if let Err(e) = self.send_request("SaveReplayBuffer") {
            log::info!("Error saving replay: {}", e);
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
    authentication: Option<AuthenticationChallenge>,
}

#[derive(Debug, Deserialize)]
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
    #[serde(rename = "authentication", default, skip_serializing_if = "Option::is_none")]
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
}
