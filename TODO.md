# TODO

## Bugs

- [x] **ARM64 cross-compilation broken in CI** — fixed `release.yml` with cross-toolchain install + linker env
- [x] **`upload-release-asset@v1` deprecated** — replaced with `actions/upload-artifact` + `actions/download-artifact` workflow

## Cleanup

- [x] **Drop `once_cell` dependency** — replaced with `std::sync::LazyLock`
- [x] **Deduplicate `real_home()`** — moved to `config.rs`, re-exported for `service.rs`
- [x] **Replace `in_input_group()` subprocess** — now uses `libc::getgrouplist` directly

## Improvements

- [x] **Ensure screenshot directory exists** — `screenshot()` now creates `save_dir` via `create_dir_all`
- [x] **Add safety comment on `OBSClient` concurrency** — documented in `read_response()` method
- [x] **Normalize repo/binary naming** — README already consistent (binary `obs-hotkey`, no lingering references)

## Future

- [ ] Password/auth support for OBS WebSocket (currently assumes no auth)
- [ ] Hotkey chord support (e.g., Ctrl+Shift+F1)
- [ ] Wayland idle inhibit on recording start
- [ ] Publish to crates.io (metadata already set up)