Fix ALL findings from the code review of /home/dracon/Dev/obs-wayland-hotkey

## CRITICAL
- [ ] C1: Race condition in connect() — set connected=false while holding lock, not after
- [ ] C2: Sync actions block event loop — spawn actions in background thread
- [ ] C3: No hotkey debouncing — add per-key debounce window

## HIGH
- [ ] H1: println! in reconnect thread → log::info!/warn!
- [ ] H2: Status always probes port 4455 → use configured port
- [ ] H3: should_stop not re-checked before connect() in reconnect thread
- [ ] H4: Remove dead close_flag atomic

## MEDIUM
- [ ] M1: skip_serializing_if on authentication field
- [ ] M2: toggle_studio_mode re-queries when state unknown
- [ ] M3: studio_mode state sync on failure — re-query
- [ ] M4: in_input_group dynamic buffer instead of fixed [0u32; 64]
- [ ] M5: in_input_group log warning on getgrouplist failure
- [ ] M6: Magic numbers → named constants
- [ ] M7: deny_unknown_fields on AppConfig
- [ ] M8: install.sh build error handling

After all fixes: cargo clippy, cargo test, install, publish