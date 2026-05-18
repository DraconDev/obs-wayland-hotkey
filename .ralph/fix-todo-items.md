# Fix TODO items for obs-hotkey

## Bugs

### 1. ARM64 cross-compilation broken in CI ✅
File: `.github/workflows/release.yml`
- Added `apt-get install gcc-aarch64-linux-gnu` step in `build-arm64` job
- Set `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc` env var

### 2. upload-release-asset@v1 deprecated ✅
File: `.github/workflows/release.yml`
- Replaced with `actions/upload-artifact@v4` → `actions/download-artifact@v4` → single `softprops/action-gh-release@v2` upload

## Cleanup

### 3. Drop once_cell, use std::sync::LazyLock ✅
Files: `src/input.rs`, `Cargo.toml`
- Removed `once_cell` from Cargo.toml dependencies
- Changed `use once_cell::sync::Lazy` → `use std::sync::LazyLock`
- Changed all `Lazy::new()` → `LazyLock::new()`

### 4. Deduplicate real_home() ✅
Files: `src/config.rs`, `src/service.rs`
- Moved `real_home()` to `src/config.rs` as `pub fn`
- Removed from `src/service.rs`, now imports via `use crate::config::real_home`

### 5. Replace in_input_group() subprocess ✅
Files: `src/service.rs`
- Now uses `libc::getgrouplist` instead of shelling out to `groups` command
- Uses `libc::getgrgid` to get group name and check for "input"

## Improvements

### 6. Ensure screenshot directory exists ✅
File: `src/obs.rs` (`screenshot` function)
- Added `std::fs::create_dir_all(save_dir)` before calling OBS request

### 7. Add concurrency safety comment on OBSClient ✅
File: `src/obs.rs`
- Added `# Safety` comment above `read_response()` explaining single-threaded assumption

### 8. Normalize repo/binary naming ✅
File: `README.md`
- Already consistent (binary `obs-hotkey`), no changes needed

---

## Status: COMPLETE

All 8 TODO items addressed. Committed as `06db093`.
- Tests: 29 passed
- Clippy: clean