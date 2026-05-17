# Fix TODO items for obs-hotkey

## Bugs

### 1. ARM64 cross-compilation broken in CI
File: `.github/workflows/release.yml`
- Add `apt-get install gcc-aarch64-linux-gnu` step in `build-arm64` job
- OR set `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc` env var

### 2. upload-release-asset@v1 deprecated
File: `.github/workflows/release.yml`
- Remove separate `Upload` step from both `build-amd64` and `build-arm64`
- Add `files:` parameter to `softprops/action-gh-release@v2` step in `create-release` job instead

## Cleanup

### 3. Drop once_cell, use std::sync::LazyLock
Files: `src/input.rs`
- Remove `once_cell` from Cargo.toml dependencies
- Change `use once_cell::sync::Lazy` → `use std::sync::LazyLock`
- Change `Lazy::<HashMap<u16, &'static str>>::new(...)` → `LazyLock::<...>::new(...)`
- Change `Lazy::<String, u16>::new(...)` → `LazyLock::<...>::new(...)`

### 4. Deduplicate real_home()
Files: `src/config.rs`, `src/service.rs`
- Move `real_home()` to `src/config.rs` (keep it there)
- Remove it from `src/service.rs`
- In `src/service.rs`, either import it from config or call `config::real_home()` (make it pub)

### 5. Replace in_input_group() subprocess
Files: `src/service.rs`, `Cargo.toml`
- Use `libc::getgroups()` to check group membership
- Remove shelling out to `groups` command

## Improvements

### 6. Ensure screenshot directory exists
File: `src/obs.rs` (`screenshot` function)
- Before calling OBS request, check if `save_dir` exists
- If not, create it with `std::fs::create_dir_all`

### 7. Add concurrency safety comment on OBSClient
File: `src/obs.rs`
- Add comment above `read_response()` explaining single-threaded assumption

### 8. Normalize repo/binary naming
File: `README.md`
- Binary name is already `obs-hotkey`, consistent with Cargo.toml
- Just remove or update any lingering `obs-wayland-hotkey` references in docs