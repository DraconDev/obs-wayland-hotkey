# obs-hotkey

A lightweight Rust daemon for controlling OBS Studio with global hotkeys on Wayland and X11.

**Works on Wayland!** Uses evdev for direct keyboard input capture, bypassing Wayland's security restrictions.

## Features

- **Wayland & X11 Support** - Works on both display servers
- **Single Static Binary** - No runtime dependencies
- **Global Hotkeys** - Works even when OBS is not in focus
- **Auto-start on Login** - systemd user service integration
- **Auto-reconnect** - Automatically reconnects to OBS if it restarts
- **Multi-keyboard** - Monitors all connected keyboards
- **Low Resource Usage** - Minimal RAM and CPU
- **F13-F24 Support** - Use mouse extra keys as stream deck buttons

## Installation

### From crates.io

```bash
cargo install obs-hotkey
```

### From GitHub Releases

Download a pre-built binary from the [Releases page](https://github.com/DraconDev/obs-wayland-hotkey/releases):

```bash
# amd64
curl -L https://github.com/DraconDev/obs-wayland-hotkey/releases/latest/download/obs-hotkey-x86_64-unknown-linux-gnu -o obs-hotkey
chmod +x obs-hotkey
sudo cp obs-hotkey /usr/local/bin/
```

### From source

```bash
cargo build --release
./target/release/obs-hotkey
```

## Setup

### 1. Enable OBS WebSocket Server

1. Open OBS Studio
2. Go to **Tools → WebSocket Server Settings**
3. Check **"Enable WebSocket server"**
4. Use default port **4455**
5. Disable authentication

### 2. Add yourself to the input group

```bash
sudo usermod -aG input $(whoami)
# Log out and back in for changes to take effect
```

### 3. Run setup

```bash
obs-hotkey setup
```

This writes and enables the systemd user service.

## Usage

```
obs-hotkey            # run the daemon (default)
obs-hotkey setup      # enable auto-start on login
obs-hotkey teardown   # undo setup (stop + disable + remove service)
obs-hotkey status     # show service state and config status
```

### Global flags

- `--config <path>` - Path to config file (all subcommands)

### Teardown options

- `obs-hotkey teardown --purge` - Also remove config directory

## Default Hotkeys

| Key | Action |
|-----|--------|
| **Scroll Lock** | Toggle recording start/stop |
| **Pause** | Toggle recording pause/resume |

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
  "mic_name": ""
}
```

### Supported Keys

- Function keys: `f1` through `f24`
- Special keys: `scroll lock`, `pause`, `home`, `end`, `page up`, `page down`, `insert`, `delete`

### Available Actions

| Action | OBS Request | Notes |
|--------|-------------|-------|
| `toggle_recording` | `ToggleRecord` | Start/stop recording |
| `toggle_pause` | `ToggleRecordPause` | Pause/resume recording |
| `toggle_streaming` | `ToggleStream` | Start/stop streaming |
| `screenshot` | `SaveSourceScreenshot` | Saves PNG to `screenshot_dir` |
| `toggle_mute_mic` | `ToggleInputMute` | Requires `mic_name` in config |
| `toggle_studio_mode` | `SetStudioModeEnabled` | Toggles studio mode |
| `toggle_replay_buffer` | `ToggleReplayBuffer` | Requires replay buffer enabled |
| `save_replay` | `SaveReplayBuffer` | Saves current replay buffer |

## Managing the Service

```bash
# Check status
systemctl --user status obs-hotkey.service

# View live logs
journalctl --user -u obs-hotkey.service -f

# Restart
systemctl --user restart obs-hotkey.service

# Stop
systemctl --user stop obs-hotkey.service
```

## Building

```bash
# Build release binary
cargo build --release

# Build with all optimizations
cargo build --release --all

# Cross-compile for ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

## Requirements

- Linux (Wayland or X11)
- OBS Studio 28+ with WebSocket enabled
- Membership in the `input` group

## Uninstall

```bash
# Stop and disable
obs-hotkey teardown

# Remove binary
sudo rm /usr/local/bin/obs-hotkey
```

## License

MIT