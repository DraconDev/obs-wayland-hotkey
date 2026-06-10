# TODO

## Bugs

- [x] **`read_response()` panics on disconnected conn or binary WS frames** — replaced with `ok_or_else` and `into_text()` with proper error mapping
- [x] **Keyboard thread shutdown race** — now uses `recv_timeout(500ms)` to periodically check close channel instead of blocking in `fetch_events()`
- [x] **Reconnect thread is orphaned on shutdown** — now joined explicitly before `run_daemon` exits; `should_stop` also signaled in Ctrl-C handler
- [x] **Release workflow: ARM64 binary has no arch suffix** — both binaries are now renamed before upload; release assets named `obs-hotkey-x86_64-unknown-linux-gnu` and `obs-hotkey-aarch64-unknown-linux-gnu`
- [x] **`query_studio_mode()` can read wrong response** — now verifies `requestId` matches before processing response
- [x] **`OBSClient::clone()` copies `AtomicBool` by value** — wrapped in `Arc<AtomicBool>` so all clones share state (v1.0.39)
- [x] **Deadlock in `send_request_with_data`** — dropped lock before calling `connect()` to avoid lock order reversal (v1.0.40)
- [x] **Wrong WebSocket op code check (op=5 instead of op=2)** — fixed for obs-websocket 5.x Identified response (v1.0.41)
- [x] **Race condition in `connect()`** — set `connected=false` before acquiring lock, not after, to prevent reconnect thread deadlock (v1.0.42)
- [x] **`TcpStream::connect()` had no timeout** — now uses `connect_timeout(5s)` with DNS resolution (v1.0.42)
- [x] **Sync actions block event loop** — hotkey actions now spawn in background threads (v1.0.42)
- [x] **No hotkey debouncing** — added 50ms debounce window per key code (v1.0.42)

## Cleanup

- [x] **`OBSClient::clone()` copies `AtomicBool` by value** — wrapped in `Arc<AtomicBool>` (v1.0.39)
- [x] **Hardcoded port 4455 in `run_status()`** — now reads config to probe the configured port (v1.0.42)
- [x] **`println!` in reconnect thread** — replaced with `log::info!`/`log::warn!` for proper systemd journal capture (v1.0.42)
- [x] **Dead `close_flag` atomic** — removed, consolidated to single `should_stop` flag (v1.0.42)
- [x] **Magic numbers throughout** — added named constants: `CONNECT_TIMEOUT`, `READ_TIMEOUT`, `DEBOUNCE_MS`, `EVENT_LOOP_POLL_MS` (v1.0.42)
- [x] **`authentication: None` serializes as `null`** — added `skip_serializing_if = "Option::is_none"` (v1.0.42)
- [x] **Unused `sha2`/`base64` deps** — removed until auth is implemented (v1.0.36)
- [x] **`in_input_group()` fixed-size buffer** — dynamic `Vec<u32>` with resize loop (v1.0.42)
- [x] **`in_input_group()` silently ignores `getgrouplist` failure** — now logs warning (v1.0.42)
- [x] **`install.sh` swallows build errors** — checks exit code and binary existence (v1.0.42)
- [x] **Unknown config fields silently ignored** — added `deny_unknown_fields` to `AppConfig` and `HotkeyConfig` (v1.0.42)

## Improvements

- [x] **Default log level too verbose** — changed from `info` to `warn` default
- [x] **CI missing `cargo check --locked`** — added lockfile check step in CI
- [x] **`toggle_studio_mode` sends wrong value when state unknown** — now forces `query_studio_mode()` first (v1.0.42)
- [x] **`studio_mode_enabled` state out of sync on failure** — sets `studio_mode_queried=false` on error to force re-query (v1.0.42)
- [x] **`wss://` URLs silently broken** — now explicitly rejected with clear error message (v1.0.36)
- [x] **OBS auth silently fails** — now rejected with clear message when OBS has auth enabled (v1.0.36)
- [x] **Errors logged at `info` level** — changed all toggle error paths to `log::warn!` (v1.0.36)
- [x] **`connect()` didn't verify Identified response** — now parses op code and bails if not op=2 (v1.0.36)
- [x] **`probe_obs_websocket` didn't send close frame** — added `ws.close(None)` (v1.0.36)
- [x] **Reconnect thread doesn't re-check `should_stop` before `connect()`** — added second check (v1.0.42)

## Future

- [ ] Password/auth support for OBS WebSocket (currently assumes no auth)
- [ ] Wayland idle inhibit on recording start
- [x] Publish to crates.io — done, v1.0.24
- [ ] Integration tests — would need a mock OBS WebSocket server
- [ ] Keyboard device hot-plug support — currently only enumerates at startup
- [x] Thread panic catching in keyboard readers — wrap in `panic::catch_unwind`
