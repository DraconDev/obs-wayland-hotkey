# TODO

## Bugs

- [x] **`read_response()` panics on disconnected conn or binary WS frames** — replaced with `ok_or_else` and `into_text()` with proper error mapping
- [x] **Keyboard thread shutdown race** — now uses `recv_timeout(500ms)` to periodically check close channel instead of blocking in `fetch_events()`
- [x] **Reconnect thread is orphaned on shutdown** — now joined explicitly before `run_daemon` exits; `should_stop` also signaled in Ctrl-C handler
- [x] **Release workflow: ARM64 binary has no arch suffix** — both binaries are now renamed before upload; release assets named `obs-hotkey-x86_64-unknown-linux-gnu` and `obs-hotkey-aarch64-unknown-linux-gnu`
- [x] **`query_studio_mode()` can read wrong response** — now verifies `requestId` matches before processing response

## Cleanup

- [ ] **`OBSClient::clone()` copies `AtomicBool` by value** — each clone gets independent atomics. Works because `Arc<Mutex<Conn>>` is authoritative. Could wrap atomics inside Arc for clarity.

## Improvements

- [ ] **Hardcoded port 4455 in `run_status()`** — status check always probes `127.0.0.1:4455` even if config specifies a different host/port
- [x] **Default log level too verbose** — changed from `info` to `warn` default
- [x] **CI missing `cargo check --locked`** — added lockfile check step in CI

## Future

- [ ] Password/auth support for OBS WebSocket (currently assumes no auth)
- [ ] Hotkey chord support (e.g., Ctrl+Shift+F1)
- [ ] Wayland idle inhibit on recording start
- [x] Publish to crates.io — done, v1.0.24
- [ ] Integration tests — would need a mock OBS WebSocket server