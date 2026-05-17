# OBS Hotkey — Rust Rewrite (Complete)

## Overview

Lightweight Rust daemon for controlling OBS Studio with global hotkeys on Wayland and X11.

## Why Rust

- **evdev**: `emberian/evdev` crate (0.13.x) — pure Rust implementation
- **WebSocket**: `tungstenite` (sync, no async runtime needed)
- **Single static binary** — no external runtime dependencies

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
| `ctrlc` | 3.x | Signal handling |

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

## Supported Actions

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

```bash
cargo build --release
```

## CI/CD

- `.github/workflows/ci.yml` — push to main / PR: test, clippy, fmt, build
- `.github/workflows/release.yml` — tag push `v*`: build + upload `obs-hotkey` binaries for amd64 + arm64

## Publishing

```bash
cargo publish  # crates.io
```

## Implementation Status

### Phase 0: Pre-flight cleanup
- [x] 0a. Remove tracked Go binary from git
- [x] 0b. Add obs-hotkey to .gitignore
- [x] 0c. Add LICENSE file (MIT)
- [x] 0d. Delete Go source files
- [x] 0e. Delete build artifacts
- [x] 0f. Update .gitignore

### Phase 1: Project scaffolding
- [x] 1a. Cargo.toml: name=obs-hotkey, version, edition=2021, license=MIT
- [x] 1b. Add dependencies
- [x] 1c. Add package metadata
- [x] 1d. Add [[bin]] target
- [x] 1e. Release profile optimizations

### Phase 2: Core modules
- [x] 2a. src/main.rs — clap subcommands + dispatch
- [x] 2b. src/config.rs — AppConfig, load/save/ensure/defaults
- [x] 2c. src/obs.rs — OBSClient
- [x] 2d. src/input.rs — keyboard discovery + events
- [x] 2e. src/service.rs — systemd service management
- [x] 2f. src/banner.rs — print_banner

### Phase 3: Config module
- [x] 3a. Structs with serde derive
- [x] 3b. default_config()
- [x] 3c. load_config()
- [x] 3d. ensure_config()
- [x] 3e. expand_home()
- [x] 3f. config_path()

### Phase 4: OBS WebSocket client
- [x] 4a. Handshake (op 0→1→2)
- [x] 4b. send_request()
- [x] 4c. send_request_with_data()
- [x] 4d. Auto-reconnect
- [x] 4e. All 8 OBS actions
- [x] 4f. QueryStudioMode
- [x] 4g. Mutex-protected connection

### Phase 5: Keyboard input (evdev)
- [x] 5a. find_keyboards()
- [x] 5b. Key code map
- [x] 5c. Thread per device, fetch_events()
- [x] 5d. mpsc channels to main loop
- [x] 5e. Device disconnection handling

### Phase 6: Daemon main event loop
- [x] 6a. run_daemon()
- [x] 6b. Banner printing
- [x] 6c. OBS connection with retries
- [x] 6d. Main event loop (key events → OBS actions)
- [x] 6e. Periodic reconnect ticker
- [x] 6f. SIGINT/SIGTERM handling

### Phase 7: Service subcommands
- [x] 7a. run_setup()
- [x] 7b. run_teardown(purge)
- [x] 7c. run_status()
- [x] 7d. write_service_file()
- [x] 7e. service_unit_path()
- [x] 7f. is_autostart_enabled()
- [x] 7g. in_input_group()

### Phase 8: CLI (clap derive)
- [x] 8a. Subcommands: daemon (default), setup, teardown, status
- [x] 8b. Global --config flag
- [x] 8c. teardown --purge flag
- [x] 8d. --help on all
- [x] 8e. Unknown subcommand error

### Phase 9: Tests
- [x] 9a. Config tests (8 tests)
- [x] 9b. OBS protocol tests (2 tests)
- [x] 9c. Service file tests (4 tests)
- [x] 9d. Input/key code tests (2 tests)
- [x] 9e. CLI parsing tests (11 tests)
- [x] 9f. Banner tests (2 tests)

### Phase 11: CI/CD
- [x] 11a. .github/workflows/ci.yml
- [x] 11b. .github/workflows/release.yml
- [x] 11c. Cross-compilation targets configured

### Phase 12: Docs + install
- [x] 12a. Rewrite README.md
- [x] 12b. Update install.sh

### Phase 13: crates.io
- [x] 13a. Verify metadata
- [x] 13b. cargo publish --dry-run passes

## Open Questions / Future Ideas

1. **Keyboard hotplug** — Devices are scanned once at startup. Could add `inotify` for plug/unplug.
2. **Config hot-reload** — Could add `notify` crate to watch config file.
3. **`--verbose` flag** — For debug logging.
4. **Shell completions** — clap can generate these.