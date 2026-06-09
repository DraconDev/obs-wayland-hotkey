# obs-hotkey

[![Crates.io](https://img.shields.io/crates/v/obs-hotkey?style=for-the-badge)](https://crates.io/crates/obs-hotkey)
[![CI](https://img.shields.io/github/actions/workflow/status/DraconDev/obs-wayland-hotkey/ci.yml?style=for-the-badge&label=CI)](https://github.com/DraconDev/obs-wayland-hotkey/actions)
[![License](https://img.shields.io/crates/l/obs-hotkey?style=for-the-badge)](https://github.com/DraconDev/obs-wayland-hotkey/blob/main/LICENSE)

> A lightweight Rust daemon for controlling OBS Studio with global hotkeys on Wayland and X11.

**Works on Wayland!** Uses evdev for direct keyboard input capture, bypassing Wayland's security restrictions.

---

## Features

- **Wayland & X11 Support** — works on both display servers
- **Single Static Binary** — no runtime dependencies
- **Global Hotkeys** — works even when OBS is not focused
- **Chord Hotkeys** — use combinations like `ctrl + shift + f1`
- **Action Combos** — trigger multiple OBS actions from one key chord
- **Mic Volume Presets** — set input volume as part of a combo
- **Auto-start on Login** — systemd user service integration
- **Auto-reconnect** — automatically reconnects if OBS restarts
- **Multi-keyboard** — monitors all connected keyboards
- **F13-F24 Support** — use extra keys as stream deck buttons
- **Hotkey Debouncing** — 50ms debounce prevents accidental double-toggles
- **Non-blocking Actions** — hotkeys stay responsive during network I/O
- **Config Validation** — typos in config are rejected with clear errors

---

## Quick Start

```bash
# 1. Install
cargo install obs-hotkey

# 2. Run once — shows the setup guide
obs-hotkey

# 3. Follow the on-screen steps:
#    - Enable OBS WebSocket Server
#    - Add yourself to the input group
#    - Run: obs-hotkey setup
```

---

## Installation

### From crates.io

```bash
cargo install obs-hotkey
```

### From GitHub Releases

```bash
# amd64
curl -L https://github.com/DraconDev/obs-wayland-hotkey/releases/latest/download/obs-hotkey-x86_64-unknown-linux-gnu -o obs-hotkey
chmod +x obs-hotkey
sudo cp obs-hotkey /usr/local/bin/

# ARM64
curl -L https://github.com/DraconDev/obs-wayland-hotkey/releases/latest/download/obs-hotkey-aarch64-unknown-linux-gnu -o obs-hotkey
chmod +x obs-hotkey
sudo cp obs-hotkey /usr/local/bin/
```

### From source

```bash
git clone https://github.com/DraconDev/obs-wayland-hotkey.git
cd obs-wayland-hotkey
./install.sh    # builds and runs setup
```

---

## Setup

### 1. Enable OBS WebSocket Server

1. Open OBS Studio
2. Go to **Tools → WebSocket Server Settings**
3. Check **Enable WebSocket server**
4. Port: **4455** (default)
5. Authentication: **disabled**

### 2. Add yourself to the input group

```bash
sudo usermod -aG input $(whoami)
# Log out and back in for changes to take effect
```

### 3. Run setup

```bash
obs-hotkey setup
```

This writes the systemd user service and enables it to start on login.

---

## Usage

```
obs-hotkey              # Show quickstart guide (interactive setup help)
obs-hotkey daemon      # Run the hotkey daemon
obs-hotkey setup        # Install systemd user service
obs-hotkey teardown     # Remove service and binaries
obs-hotkey status       # Check service, config, and OBS connectivity
```

### Global flags

| Flag | Description |
|------|-------------|
| `--config <path>` | Use a custom config file |
| `--version` | Show version |
| `--help` | Show full help |

### Teardown options

| Command | Description |
|---------|-------------|
| `obs-hotkey teardown` | Stop service, remove service files and binaries |
| `obs-hotkey teardown --purge` | Above + remove config directory |

---

## Default Hotkeys

| Key | Action |
|-----|--------|
| **Scroll Lock** | Toggle recording start/stop |
| **Pause** | Toggle recording pause/resume |

---

## Customizing Hotkeys

Edit `~/.config/obs-hotkey/hotkeys.json`:

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
  "mic_name": "",
  "mic_volume": 1.0,
  "hotkey_combos": []
}
```

Existing single-action hotkeys still work. To trigger multiple OBS actions from one key chord, add entries to `hotkey_combos`:

```json
{
  "obs_host": "ws://localhost:4455",
  "hotkeys": {
    "toggle_recording": "",
    "toggle_pause": "",
    "toggle_streaming": "",
    "screenshot": "",
    "toggle_mute_mic": "",
    "toggle_studio_mode": "",
    "toggle_replay_buffer": "",
    "save_replay": ""
  },
  "screenshot_source": "",
  "screenshot_dir": "~/Pictures",
  "mic_name": "Microphone",
  "mic_volume": 0.75,
  "hotkey_combos": [
    {
      "name": "record_and_set_mic",
      "keys": ["ctrl", "shift", "r"],
      "actions": ["toggle_recording", "set_mic_volume"]
    }
  ]
}
```

A combo entry may use either `"key": "ctrl + f1"` or `"keys": ["ctrl", "shift", "f1"]`. Actions run in the order listed.

### Key Combos

Key combos are written as physical keys separated by `+`:

```json
"ctrl + shift + f1"
```

Generic modifier names match either left or right key:

- `ctrl` / `control`
- `shift`
- `alt` / `option`
- `super` / `command` / `win`
- `meta`

Left/right-specific names are also supported, for example `left ctrl` or `right shift`.

### Action Combos

`hotkey_combos` lets one chord run multiple actions. This is useful for workflows OBS does not support natively, such as starting recording and setting your mic volume in the same gesture.

```json
{
  "name": "record_and_set_mic",
  "key": "ctrl + f1",
  "actions": ["toggle_recording", "set_mic_volume"]
}
```

### Supported Keys

- Function keys: `f1` – `f24`
- Special keys: `scroll lock`, `pause`, `home`, `end`, `page up`, `page down`, `insert`, `delete`
- Modifiers: `ctrl`, `shift`, `alt`, `super`, `meta`, plus left/right-specific variants

### Available Actions

| Action | OBS Request | Notes |
|--------|-------------|-------|
| `toggle_recording` | `ToggleRecord` | Start/stop recording |
| `toggle_pause` | `ToggleRecordPause` | Pause/resume recording |
| `toggle_streaming` | `ToggleStream` | Start/stop streaming |
| `screenshot` | `SaveSourceScreenshot` | Saves PNG to `screenshot_dir` |
| `toggle_mute_mic` | `ToggleInputMute` | Requires `mic_name` in config |
| `set_mic_volume` | `SetInputVolume` | Requires `mic_name` and `mic_volume` |
| `toggle_studio_mode` | `SetStudioModeEnabled` | Toggles studio mode |
| `toggle_replay_buffer` | `ToggleReplayBuffer` | Requires replay buffer enabled |
| `save_replay` | `SaveReplayBuffer` | Saves current replay buffer |

---

## Managing the Service

```bash
# Check status
systemctl --user status obs-hotkey.service

# View live logs
journalctl --user -u obs-hotkey.service -f

# Restart (after config changes)
systemctl --user restart obs-hotkey.service

# Stop
systemctl --user stop obs-hotkey.service
```

---

## Building

```bash
# Build release binary
cargo build --release

# Cross-compile for ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

---

## Requirements

- Linux (Wayland or X11)
- OBS Studio 28+ with WebSocket enabled
- Membership in the `input` group

---

## Uninstall

```bash
# Stop service and remove everything
obs-hotkey teardown --purge
```

---

## License

MIT