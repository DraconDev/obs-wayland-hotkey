use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
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

struct Conn {
    ws: tungstenite::WebSocket<TcpStream>,
}

#[derive(Debug, Deserialize)]
struct HelloMessage {
    op: u8,
    d: HelloData,
}

#[derive(Debug, Deserialize)]
struct HelloData {
    #[serde(rename = "obsWebSocketVersion")]
    obs_web_socket_version: String,
    #[serde(rename = "rpcVersion")]
    rpc_version: u32,
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

    pub fn connect(&self) -> anyhow::Result<()> {
        let mut guard = self.conn.lock().unwrap();
        if let Some(ref mut c) = *guard {
            let _ = c.ws.write_close(None);
        }
        *guard = None;
        self.connected.store(false, Ordering::SeqCst);

        let stream = TcpStream::connect(&self.ws_url)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let ws = tungstenite::client(
            tungstenite::handshake::client::ClientHandshake::start(
                &stream,
                &self.ws_url,
                tungstenite::handshake::client::NoProxy,
            )?,
            &self.ws_url,
        )?
        .0;
        ws.set_read_timeout(Some(Duration::from_secs(10)))?;

        let mut msg = ws.read_message()?;
        let hello: HelloMessage = serde_json::from_str(msg.to_text().unwrap())?;
        log::info!("Connected to OBS WebSocket v{}", hello.d.obs_web_socket_version);

        let ident = IdentifyMessage {
            op: 1,
            d: IdentifyData { rpc_version: 1 },
        };
        ws.write_message(&tungstenite::Message::Text(serde_json::to_string(&ident).unwrap()))?;

        msg = ws.read_message()?;
        let text = msg.to_text().unwrap();
        if !text.starts_with(r#"{"op":2,"d":{"#"#) {
            anyhow::bail!("failed to identify to OBS");
        }
        ws.set_read_timeout(None)?;
        ws.write_message(&tungstenite::Message::Ping(vec![].into()))?;

        log::info!("Successfully identified to OBS WebSocket");
        *guard = Some(Conn { ws });
        self.connected.store(true, Ordering::SeqCst);
        drop(guard);
        self.query_studio_mode();
        Ok(())
    }

    fn read_response(&self, request_id: &str) -> anyhow::Result<serde_json::Value> {
        let mut guard = self.conn.lock().unwrap();
        let conn = guard.as_mut().unwrap();
        conn.ws.set_read_timeout(Some(Duration::from_secs(5)))?;
        let msg = conn.ws.read_message()?;
        let data: serde_json::Value = serde_json::from_str(msg.to_text().unwrap())?;
        conn.ws.set_read_timeout(None)?;
        Ok(data)
    }

    pub fn send_request(&self, request_type: &str) -> anyhow::Result<()> {
        self.send_request_with_data(request_type, None)
    }

    pub fn send_request_with_data(
        &self,
        request_type: &str,
        request_data: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let request_id = format!("{}_{}", request_type, std::time::SystemTime::now().elapsed().unwrap().as_secs());
        let req = RequestMessage {
            op: 6,
            d: RequestData {
                request_type: request_type.to_string(),
                request_id: request_id.clone(),
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
            conn.ws.write_message(&tungstenite::Message::Text(json.into()))?;
        }

        let resp = self.read_response(&request_id)?;
        if let Some(status) = resp.get("d").and_then(|d| d.get("requestStatus")) {
            if !status.get("result").and_then(|r| r.as_bool()).unwrap_or(false) {
                anyhow::bail!("request {} failed: {:?}", request_type, status);
            }
        }
        Ok(())
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
        let mut data = serde_json::json!({
            "imageFormat": "png",
            "imageFilePath": format!("{}/obs-screenshot-{}.png", save_dir, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
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
            self.query_studio_mode();
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
            let _ = c.ws.write_close(None);
        }
        *guard = None;
    }

    fn query_studio_mode(&self) {
        #[derive(Debug, Deserialize)]
        struct StudioModeResponse {
            op: u8,
            d: StudioModeData,
        }
        #[derive(Debug, Deserialize)]
        struct StudioModeData {
            #[serde(rename = "requestId")]
            request_id: String,
            #[serde(rename = "requestStatus")]
            request_status: RequestStatus,
            #[serde(rename = "responseData")]
            response_data: Option<ResponseData>,
        }
        #[derive(Debug, Deserialize)]
        struct RequestStatus {
            result: bool,
        }
        #[derive(Debug, Deserialize)]
        struct ResponseData {
            #[serde(rename = "studioModeEnabled")]
            studio_mode_enabled: bool,
        }

        let request_id = format!("GetStudioModeEnabled_{}", std::time::SystemTime::now().elapsed().unwrap().as_secs());
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
            conn.ws.write_message(&tungstenite::Message::Text(json.into()))?;
        }

        let resp = self.read_response(&request_id);
        if let Ok(data) = resp {
            if let Some(resp_data) = data.get("d").and_then(|d| d.get("responseData")) {
                let enabled = resp_data.get("studioModeEnabled").and_then(|e| e.as_bool()).unwrap_or(false);
                self.studio_mode_enabled.store(enabled, Ordering::SeqCst);
            }
        }
        self.studio_mode_queried.store(true, Ordering::SeqCst);
        log::info!("Studio mode is currently: {}", self.studio_mode_enabled.load(Ordering::SeqCst));
    }
}

impl Drop for OBSClient {
    fn drop(&mut self) {
        self.close();
    }
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
    fn test_sanitize_host() {
        // can't test full connect, but verify URL parsing works
        let client = OBSClient::new("localhost:4455".to_string());
        assert_eq!(client.ws_url, "localhost:4455");
    }
}