# Fix TODO items for obs-hotkey v2

## Bugs

### 1. read_response() panics on disconnected conn or binary WS frames
File: `src/obs.rs` (`read_response`)
- Replace `guard.as_mut().unwrap()` with `guard.as_mut().ok_or_else(...)` 
- Replace `msg.to_text().unwrap()` with `msg.to_text().map_err(...)` or check `msg.is_text()`

### 2. Keyboard thread shutdown race
File: `src/input.rs` (`spawn_keyboard_reader`)
- Use `Device::grab()` instead of `Device::open()` to avoid needing close channel
- OR use `std::os::fd::AsRawFd` + `poll()` with timeout so close_rx can interrupt
- Simplest fix: set `O_NONBLOCK` on the fd before opening, then poll with timeout in the loop

### 3. Reconnect thread is orphaned on shutdown
File: `src/main.rs` (`run_daemon`)
- Change `_reconnect_handle` to `reconnect_handle` (drop the underscore)
- Join the thread before exiting `run_daemon`
- Signal the thread to stop immediately on Ctrl+C

### 4. Release workflow: ARM64 binary has no arch suffix
File: `.github/workflows/release.yml` (`create-release`)
- Add `mv artifacts/obs-hotkey artifacts/obs-hotkey-aarch64-unknown-linux-gnu` step after downloading arm64 artifact

### 5. query_studio_mode() can read wrong response
File: `src/obs.rs` (`query_studio_mode`)
- Use `send_request_with_data` pattern like other methods
- OR store the request_id and verify response matches

## Cleanup

### 6. Move tempfile to [dev-dependencies]
File: `Cargo.toml`
- Move `tempfile` from `[dependencies]` to `[dev-dependencies]`

### 7. OBSClient::clone() copies AtomicBool by value
File: `src/obs.rs`
- This is tricky — the Arc<Mutex<Conn>> is the truth. The atomic copies don't matter in practice.
- Add a comment explaining this is intentional, or wrap atomics inside the Arc

## Improvements

### 8. Hardcoded port 4455 in run_status()
File: `src/service.rs` (`run_status`)
- Parse the host from config path to get the actual port
- OR use the same sanitize_obs_host logic to determine the endpoint

### 9. Default log level too verbose
File: `src/main.rs` (env_logger init)
- Change default filter from "info" to "warn"

### 10. CI missing cargo check --locked
File: `.github/workflows/ci.yml`
- Add `cargo check --locked` step after format check