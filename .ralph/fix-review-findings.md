Fix the code review findings in /home/dracon/Dev/obs-wayland-hotkey

## Priority: Critical first, then High, then Medium

### C1 - CRITICAL: OBSClient::clone() copies atomics by value -- reconnection broken
**File:** src/obs.rs, lines 29-36
**Fix:** Wrap AtomicBool fields in Arc so all clones share them

### C2 - CRITICAL: wss:// TLS URLs silently broken  
**File:** src/obs.rs, lines 56-62
**Fix:** Add explicit check rejecting wss:// with clear error message

### C3 - CRITICAL: No OBS WebSocket auth -- silently fails with password
**File:** src/obs.rs, line 88 + config.rs
**Fix:** Implement auth or reject with clear error. For now, reject when auth is required.

### H2 - HIGH: Errors logged at info level -- invisible with default warn
**File:** src/obs.rs, all toggle methods
**Fix:** Change log::info! error paths to log::warn!

### H5 - HIGH: connect() doesn't verify Identified (op 5) response
**File:** src/obs.rs, lines 92-98
**Fix:** Parse response op code and verify it's op 5

### M4 - MEDIUM: Unused sha2/base64 deps
**File:** Cargo.toml
**Fix:** Remove sha2 and base64 until auth is implemented

### M7 - MEDIUM: probe_obs_websocket doesn't send close frame
**File:** src/service.rs
**Fix:** Call ws.close(None) before returning true

Build and test after each change. Run full test suite at the end.
Publish to cargo if all tests pass.