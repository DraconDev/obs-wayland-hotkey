# OBS Hotkey — Rust Rewrite

## Overview

Rewrite `obs-wayland-hotkey` (Go) in Rust. Goal: publish to crates.io as `obs-hotkey`, add GitHub Releases with cross-platform binaries.

## Why Rust

- **evdev**: `emberian/evdev` crate (0.13.x) — pure Rust reimplementation of libevdev, same functionality as the Go `gvalkov/golang-evdev`
- **WebSocket**: `tungstenite` (sync, no async runtime needed) — same OBS WebSocket 5.x protocol as Go `gorilla/websocket`
- **Distribute via crates.io** — `cargo install obs-hotkey` for instant installation
- **No external runtime dependencies** — single static binary
- **obws** (dnaka91) is archived (May 2026) and async-only, so we implement the OBS WebSocket protocol ourselves (~200 LOC, same as Go)

## Crate

- **Name**: `obs-hotkey` (available on crates.io)
- **Edition**: 2021
- **License**: MIT
- **Repository**: https://github.com/DraconDev/obs-wayland-hotkey

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `evdev` | 0.13 | Linux evdev keyboard enumeration + event reading |
| `tungstenite` | 0.29 | Sync WebSocket client |
| `serde` + `serde_json` | 1.x | Config file parse/serialize |
| `clap` | 4.x (derive) | CLI subcommands |
| `dirs` | 6.x | XDG config home |
| `anyhow` | 1.x | Error handling |
| `libc` | 0.2 | `getgroups()` for input group check |

## Module Structure

```
src/
├── main.rs      # CLI entry, subcommand dispatch (clap)
├── config.rs    # AppConfig, HotkeyConfig, load/save/ensure
├── obs.rs       # OBSClient: connect, identify, send_request, all 8 actions
├── input.rs     # find_keyboards, key code map, event loop
├── service.rs   # setup, teardown, status, systemd helpers
└── banner.rs    # print_banner
```

## OBS WebSocket 5.x Protocol (op codes)

- Op 0: Hello (server → client) — obsWebSocketVersion, rpcVersion
- Op 1: Identify (client → server) — rpcVersion=1
- Op 2: Identified (server → client) — success
- Op 6: Request (client → server) — requestType, requestId, optional requestData
- Op 7: RequestResponse (server → client) — requestId, requestStatus

## Supported Actions (same as Go version)

| Action | OBS Request | Config Field |
|--------|-------------|--------------|
| Toggle Recording | `ToggleRecord` | `toggle_recording` |
| Toggle Pause/Resume | `ToggleRecordPause` | `toggle_pause` |
| Toggle Streaming | `ToggleStream` | `toggle_streaming` |
| Screenshot | `SaveSourceScreenshot` | `screenshot` |
| Toggle Mic Mute | `ToggleInputMute` | `toggle_mute_mic` |
| Toggle Studio Mode | `SetStudioModeEnabled` | `toggle_studio_mode` |
| Toggle Replay Buffer | `ToggleReplayBuffer` | `toggle_replay_buffer` |
| Save Replay | `SaveReplayBuffer` | `save_replay` |

## Key Code Map (Linux evdev)

`KEY_SCROLLLOCK`, `KEY_PAUSE`, `KEY_HOME`, `KEY_END`, `KEY_PAGEUP`, `KEY_PAGEDOWN`, `KEY_INSERT`, `KEY_DELETE`, `KEY_F1`..`KEY_F24`

## Config File

`~/.config/obs-hotkey/hotkeys.json` (XDG_CONFIG_HOME respected):

```json
{
  "obs_host": "ws://localhost:4455",
  "hotkeys": {
    "toggle_recording": "scroll lock",
    "toggle_pause": "pause",
    "toggle_streaming": "",
    "screenshot": "",
    "toggle_mute_mic": "",
    "toggle_studio_mode": "",
    "toggle_replay_buffer": "",
    "save_replay": ""
  },
  "screenshot_source": "",
  "screenshot_dir": "~/Pictures",
  "mic_name": ""
}
```

## CLI Subcommands

```
obs-hotkey           # run daemon (default)
obs-hotkey setup     # write systemd service + enable + start
obs-hotkey teardown  # stop + disable + remove service
obs-hotkey status    # show autostart/input group/config/OBS state
```

Flags: `--config <path>` (shared), `--purge` (teardown only)

## Build

- **Binary name**: `obs-hotkey`
- **Rust toolchain**: 1.75+ (for async closures in spawn)
- **Cross-compile targets**: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- **Static binary**: `musl` target for fully static Linux binary

## CI/CD

- `.github/workflows/ci.yml` — push to main / PR: test, clippy, fmt, build
- `.github/workflows/release.yml` — tag push `v*`: build + upload `obs-hotkey` binaries for amd64 + arm64 as GitHub Release assets

## Publishing

```bash
cargo publish  # crates.io
```

After publish: `cargo install obs-hotkey` works everywhere with Rust.

## What's Being Deleted (Go version)

- `main.go`, `main_test.go`, `obsclient_test.go`
- `go.mod`, `go.sum`, `vendor/`
- `build.sh`, `obs-hotkey` (binary), `result` (symlink)
- `TODO.md` (Go-specific)

## What's Kept

- `flake.nix` (replaced Go module with Rust build)
- `nix/module.nix` (NixOS module — binary name same, no changes needed)
- `README.md` (rewritten for Rust version)
- `install.sh` (updated to support `cargo install` or GitHub binary fallback)
- `.gitignore`, `.gitattributes`, `.github/` (if present)
- All remotes (GitHub, GitLab, Codeberg)

## Phases

### Phase 0: Pre-flight cleanup
- [x] 0a. Remove tracked Go binary from git
- [x] 0b. Add obs-wayland-hotkey to .gitignore
- [ ] 0c. Add LICENSE file (MIT)
- [ ] 0d. Delete Go source files
- [ ] 0e. Delete build artifacts (build.sh, binaries, result symlink)
- [ ] 0f. Update .gitignore

### Phase 1: Project scaffolding
- [ ] 1a. Cargo.toml: name=obs-hotkey, version=1.0.0, edition=2021, license=MIT
- [ ] 1b. Add dependencies
- [ ] 1c. Add package metadata (description, repository, keywords, categories)
- [ ] 1d. Add [[bin]] target
- [ ] 1e. Release profile optimizations

### Phase 2: Core modules (stubs first, then flesh out)
- [ ] 2a. src/main.rs — clap subcommands + dispatch
- [ ] 2b. src/config.rs — AppConfig, load/save/ensure/defaults
- [ ] 2c. src/obs.rs — OBSClient
- [ ] 2d. src/input.rs — keyboard discovery + events
- [ ] 2e. src/service.rs — systemd service management
- [ ] 2f. src/banner.rs — print_banner

### Phase 3: Config module
- [ ] 3a. Structs with serde derive (same JSON schema as Go)
- [ ] 3b. default_config()
- [ ] 3c. load_config()
- [ ] 3d. ensure_config()
- [ ] 3e. expand_home()
- [ ] 3f. config_path()

### Phase 4: OBS WebSocket client
- [ ] 4a. Handshake (op 0→1→2)
- [ ] 4b. send_request()
- [ ] 4c. send_request_with_data()
- [ ] 4d. Auto-reconnect
- [ ] 4e. All 8 OBS actions
- [ ] 4f. QueryStudioMode
- [ ] 4g. Mutex-protected connection

### Phase 5: Keyboard input (evdev)
- [ ] 5a. find_keyboards()
- [ ] 5b. Key code map
- [ ] 5c. Thread per device, fetch_events()
- [ ] 5d. mpsc channels to main loop
- [ ] 5e. Device disconnection handling

### Phase 6: Daemon main event loop
- [ ] 6a. run_daemon()
- [ ] 6b. Banner printing
- [ ] 6c. OBS connection with retries
- [ ] 6d. Main event loop (key events → OBS actions)
- [ ] 6e. Periodic reconnect ticker
- [ ] 6f. SIGINT/SIGTERM handling

### Phase 7: Service subcommands
- [ ] 7a. run_setup()
- [ ] 7b. run_teardown(purge)
- [ ] 7c. run_status()
- [ ] 7d. write_service_file()
- [ ] 7e. service_unit_path()
- [ ] 7f. is_autostart_enabled()
- [ ] 7g. in_input_group()

### Phase 8: CLI (clap derive)
- [ ] 8a. Subcommands: daemon (default), setup, teardown, status
- [ ] 8b. Global --config flag
- [ ] 8c. teardown --purge flag
- [ ] 8d. --help on all
- [ ] 8e. Unknown subcommand error

### Phase 9: Tests
- [ ] 9a. Config tests
- [ ] 9b. OBS protocol tests (mock WebSocket)
- [ ] 9c. Service file tests
- [ ] 9d. Input/key code tests
- [ ] 9e. CLI parsing tests
- [ ] 9f. Banner tests
- [ ] 9g. run_status integration test

### Phase 10: Nix/Flakes
- [ ] 10a. Update flake.nix (Rust build)
- [ ] 10b. Compute cargoHash
- [ ] 10c. Verify nix/module.nix still works
- [ ] 10d. nix build + nix run

### Phase 11: CI/CD
- [ ] 11a. .github/workflows/ci.yml
- [ ] 11b. .github/workflows/release.yml
- [ ] 11c. Cross-compilation setup

### Phase 12: Docs + install
- [ ] 12a. Rewrite README.md
- [ ] 12b. Update install.sh
- [ ] 12c. Delete build.sh, TODO.md

### Phase 13: crates.io
- [ ] 13a. Verify metadata
- [ ] 13b. cargo publish --dry-run
- [ ] 13c. cargo publish

### Phase 14: Final
- [ ] 14a. cargo test + clippy + fmt
- [ ] 14b. nix build
- [ ] 14c. Manual smoke test
- [ ] 14d. Tag v1.0.0 and push
- [ ] 14e. Verify GitHub Release

## Open Questions

1. **Keyboard hotplug** — Go version doesn't support it (devices are scanned once at startup). Rust version could add `inotify` on `/dev/input/` for plug/unplug. Not in initial scope.
2. **Config hot-reload** — Go version requires restart to pick up config changes. Could add `notify` crate to watch the config file. Not in initial scope.
3. **`--verbose` flag** — Could add `-v` for debug logging. Not in initial scope.
4. **Shell completions** — clap can generate these. Not in initial scope.
5. **Rust toolchain file** — `rust-toolchain.toml` for pinned nightly/stable? Not needed for pure stable.
6. **deny.toml / clippy.toml** — Security linting config. Could add later.