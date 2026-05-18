# fix-review-findings

## Status: COMPLETE

## Issues Fixed (from code review)

### C1 - CRITICAL: OBSClient::clone() copies atomics by value -- reconnection broken ✅
- **File:** src/obs.rs, lines 9-11
- **Fix:** Wrapped AtomicBool fields in Arc<AtomicBool> so all clones share state
- **Verification:** `grep "Arc<AtomicBool>" src/obs.rs`

### C2 - CRITICAL: wss:// TLS URLs silently broken ✅
- **File:** src/obs.rs, lines 61-64
- **Fix:** Added explicit check rejecting wss:// with clear error message
- **Verification:** `grep "wss://" src/obs.rs`

### C3 - CRITICAL: No OBS WebSocket auth -- silently fails with password ✅
- **File:** src/obs.rs, lines 82-86
- **Fix:** Reject connection with clear message if OBS has auth enabled
- **Verification:** Auth check in connect() function

### H2 - HIGH: Errors logged at info level -- invisible with default warn ✅
- **File:** src/obs.rs, all toggle methods
- **Fix:** Changed all toggle/screenshot error paths to log::warn!
- **Verification:** `grep -c "log::warn!" src/obs.rs` = 11

### H5 - HIGH: connect() doesn't verify Identified (op 5) response ✅
- **File:** src/obs.rs, lines 108-112
- **Fix:** Parse response op code, bail if not op 5
- **Verification:** op check in connect()

### M4 - MEDIUM: Unused sha2/base64 deps ✅
- **File:** Cargo.toml
- **Fix:** Removed sha2 and base64 until auth is implemented
- **Verification:** `grep sha2 Cargo.toml` returns empty

### M7 - MEDIUM: probe_obs_websocket doesn't send close frame ✅
- **File:** src/service.rs, line 346
- **Fix:** Added ws.close(None) before returning true
- **Verification:** `grep "ws.close" src/service.rs`

---

## Additional: Freezing/Hotkey Deadlock Fix

### Root Cause
Two deadlock-prone code paths in `OBSClient`:

1. **send_request_with_data**: checked `is_connected()` BEFORE acquiring the lock, but then held the lock while calling `connect()` — which also needs the lock → **classic lock order reversal deadlock**.

2. **connect()**: set `connected.store(false, Ordering::SeqCst)` WHILE holding the lock. The reconnect thread could see `connected=false` and re-enter `connect()` while the original caller still held the lock → **self-deadlock**.

### Fixes Applied

1. **send_request_with_data** (lines 143-172):
   - Now acquires the lock FIRST, checks if conn is None inside the lock
   - If None: drops guard, calls connect() (which re-acquires lock), re-acquires lock
   - **Lock order: send_request_with_data → connect()** (consistent, no reversal)

2. **connect()** (lines 51-130):
   - Dropped guard BEFORE setting `connected.store(false)` on error paths
   - `connected=false` now happens AFTER releasing the lock, so the reconnect thread won't re-enter while we hold the lock

### Test Results
- `cargo test`: 33/33 passed ✅
- `cargo clippy -- -D warnings`: 0 warnings ✅

## Release
- Version: 1.0.40 (local commit on sync'd remotes)
- Remotes sync'd (origin, github, gitlab, codeberg) ✅
- crates.io: v1.0.40 (already published with deadlock fix) ✅