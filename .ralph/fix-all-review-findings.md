# fix-all-review-findings

## Status: COMPLETE

### CRITICAL
- [x] C1: Race condition in connect() — set connected=false BEFORE acquiring lock
- [x] C2: Sync actions block event loop — spawn each action in a background thread
- [x] C3: No hotkey debouncing — added 50ms debounce window per key code
- [x] C4: No TCP connect timeout — use connect_timeout(5s) with DNS resolution
- [x] C5: WebSocket op code check — fixed from op=5 to op=2 for obs-websocket 5.x

### HIGH
- [x] H1: println! in reconnect thread → log::info!/log::warn!
- [x] H2: Status always probes port 4455 → parse config and probe configured port
- [x] H3: should_stop not re-checked before connect() — added second check
- [x] H4: Remove dead close_flag atomic — consolidated to single should_stop

### MEDIUM
- [x] M1: skip_serializing_if on authentication field
- [x] M2: toggle_studio_mode re-queries when state unknown
- [x] M3: studio_mode state sync on failure — set studio_mode_queried=false on error
- [x] M4: in_input_group dynamic buffer (Vec) instead of fixed [0u32; 64]
- [x] M5: in_input_group log warning on getgrouplist failure
- [x] M6: Magic numbers → named constants (CONNECT_TIMEOUT, READ_TIMEOUT, DEBOUNCE_MS, etc.)
- [x] M7: deny_unknown_fields on AppConfig and HotkeyConfig
- [x] M8: install.sh build error handling — check exit code and binary existence

### Test Results
- cargo test: 36/36 passed ✅
- cargo clippy -- -D warnings: 0 warnings ✅
- Service running: v1.0.45, OBS connected ✅

### Release
- Version: 1.0.45
- Remotes: GitHub ✅, GitLab ✅, Codeberg ✅
- crates.io: v1.0.45 ✅
