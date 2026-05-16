# OBS Hotkey

A lightweight Go binary for controlling OBS Studio with global hotkeys on Wayland (and X11).

**Works on Wayland!** Uses evdev for direct keyboard input capture, bypassing Wayland's security restrictions.

## Features

- **Wayland & X11 Support** - Works on both display servers
- **Single Binary** - No dependencies, just ~7MB
- **Global Hotkeys** - Works even when OBS is not in focus
- **Auto-start on Login** - Set it and forget it
- **Auto-reconnect** - Automatically reconnects to OBS if it restarts
- **Multi-keyboard** - Monitors all connected keyboards
- **Low Resource Usage** - ~10-20MB RAM, minimal CPU
- **F13-F24 Support** - Use mouse extra keys as a stream dock

## Quick Install

### NixOS

Add to your flake inputs and configuration:

```nix
inputs.obs-hotkey.url = "path:/path/to/obs-wayland-hotkey";

# In your configuration:
imports = [ inputs.obs-hotkey.nixosModules.default ];

services.obs-hotkey.enable = true;
services.obs-hotkey.user = "your_username";
services.obs-hotkey.configFile = "/home/your_username/.config/obs-hotkey/hotkeys.json";  # optional
```

The module sets up the systemd user service automatically — no extra steps needed.

### Standalone (no module)

If you don't want to import the module, you can still get auto-start:

```bash
# Build:
nix build
./result/bin/obs-hotkey

# Or run without building:
nix run .#

# One-time setup for auto-start as a systemd user service:
nix run .# -- setup
```

### Other Linux

```bash
chmod +x install.sh && ./install.sh
```

This runs `obs-hotkey setup` — writes the systemd service file, enables and starts it.
You can also run `obs-hotkey setup` directly if the binary is already in your PATH.

## Subcommands

```
obs-hotkey          # run the daemon (default)
obs-hotkey setup    # enable auto-start on login
obs-hotkey teardown # undo setup (stop + disable + remove service)
obs-hotkey status   # show service state and config status
```

**`setup`** writes `~/.config/systemd/user/obs-hotkey.service` and enables it.

**`teardown`** stops, disables, and removes the service file. Use `--purge` to also delete the config directory.

**`status`** checks whether the service is enabled, whether you're in the `input` group, whether the config file exists, and whether OBS is reachable on port 4455.

## Default Hotkeys

- **Scroll Lock** - Toggle recording start/stop
- **Pause** - Toggle recording pause/resume

Default hotkeys are set in the config file at `~/.config/obs-hotkey/hotkeys.json`. On first run, a default config is created automatically.

## Manual Setup

### 1. Build

```bash
chmod +x build.sh && ./build.sh
```

This creates the `obs-hotkey` binary (~7MB).

### 2. Configure OBS

1. Open OBS Studio
2. Go to **Tools → WebSocket Server Settings**
3. Check **"Enable WebSocket server"**
4. Use default port **4455**
5. Disable authentication

### 3. Run

```bash
./obs-hotkey           # run as daemon (shows banner with hotkey list)
./obs-hotkey setup     # enable auto-start on login
./obs-hotkey status    # check service state
```

You need to be in the `input` group for keyboard device access (`/dev/input/`):
```bash
sudo usermod -aG input $(whoami)
# Log out and back in for group changes to take effect
```

## System-wide Installation

### Install Binary

```bash
sudo cp obs-hotkey /usr/local/bin/
sudo chmod +x /usr/local/bin/obs-hotkey
```

Then run from anywhere:
```bash
obs-hotkey
```

### Autostart with Systemd (Manual)

If you didn't use the installer, you can manually set up auto-start:

1. **Add your user to the input group:**
   ```bash
   sudo usermod -aG input $(whoami)
   ```
   Log out and back in for this to take effect.

2. **Create systemd service** `~/.config/systemd/user/obs-hotkey.service`:
   ```bash
   mkdir -p ~/.config/systemd/user
   cat > ~/.config/systemd/user/obs-hotkey.service << 'EOF'
   [Unit]
   Description=OBS Hotkey Controller
   After=graphical-session.target

   [Service]
   Type=simple
   ExecStart=/usr/local/bin/obs-hotkey --config /home/YOUR_USERNAME/.config/obs-hotkey/hotkeys.json
   Restart=on-failure
   RestartSec=10s

   [Install]
   WantedBy=graphical-session.target
   EOF
   ```

   Replace `YOUR_USERNAME` with your actual username.

3. **Enable and start:**
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable obs-hotkey.service
   systemctl --user start obs-hotkey.service
   ```

## Managing the Service

```bash
# Check status
systemctl --user status obs-hotkey.service

# View live logs
journalctl --user -u obs-hotkey.service -f

# Start the service
systemctl --user start obs-hotkey.service

# Stop the service
systemctl --user stop obs-hotkey.service

# Restart the service
systemctl --user restart obs-hotkey.service

# Disable auto-start
systemctl --user disable obs-hotkey.service

# Re-enable auto-start
systemctl --user enable obs-hotkey.service
```

## Customizing Hotkeys

Hotkeys are configured via a JSON config file at `~/.config/obs-hotkey/hotkeys.json`.

On first run, a default config is created automatically if none exists.

**To change hotkeys:**

1. Edit `~/.config/obs-hotkey/hotkeys.json`
2. Restart the service: `systemctl --user restart obs-hotkey.service`

**Config file format:**
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

**Config fields:**
- `obs_host` — OBS WebSocket URL (default: `ws://localhost:4455`)
- `hotkeys.*` — Key name for each action (empty string = disabled)
- `screenshot_source` — OBS source name for screenshot (empty = current program scene)
- `screenshot_dir` — Directory to save screenshots (default: `~/Pictures`)
- `mic_name` — OBS input name for mute toggle (e.g., `"Mic/Aux"`)

### Supported Keys

- Function keys: `f1` through `f24` (including extra keys like `f13`-`f24` for mouse buttons)
- Special keys: `scroll lock`, `pause`, `home`, `end`, `page up`, `page down`, `insert`, `delete`

### Available Actions

| Action | OBS Request | Notes |
|--------|-------------|-------|
| `toggle_recording` | `ToggleRecord` | Start/stop recording |
| `toggle_pause` | `ToggleRecordPause` | Pause/resume recording |
| `toggle_streaming` | `ToggleStream` | Start/stop streaming |
| `screenshot` | `SaveSourceScreenshot` | Saves PNG to `screenshot_dir` |
| `toggle_mute_mic` | `ToggleInputMute` | Requires `mic_name` in config |
| `toggle_studio_mode` | `SetStudioModeEnabled` | Toggles studio mode state |
| `toggle_replay_buffer` | `ToggleReplayBuffer` | Requires replay buffer enabled |
| `save_replay` | `SaveReplayBuffer` | Saves current replay buffer |

## Troubleshooting

### Service not starting

Check the logs:
```bash
journalctl --user -u obs-hotkey.service -n 50
```

Common issues:
- **"failed to connect to OBS"** - OBS not running or WebSocket not enabled
- **"No keyboard devices found"** - Not in the `input` group
- **"unknown key"** - Key name in config doesn't match a supported key

### "No keyboard devices found"

You need to be in the `input` group to access `/dev/input/` devices:
```bash
groups $(whoami)
```

If `input` is not listed:
```bash
sudo usermod -aG input $(whoami)
```
Then log out and back in.

### "failed to connect to OBS"

1. Make sure OBS is running
2. Enable WebSocket: Tools → WebSocket Server Settings
3. Check port 4455 is not blocked
4. The service will auto-reconnect when OBS starts

### Hotkey not working

1. Check the service is running: `systemctl --user status obs-hotkey.service`
2. Check the key is set in your config file (`~/.config/obs-hotkey/hotkeys.json`)
3. Verify OBS is connected (check logs)
4. Try a different key
5. Check that F13-F24 are supported (they require `f13`, `f14`, etc. in the config)

### Manual testing

Stop the service and run manually to see output:
```bash
systemctl --user stop obs-hotkey.service
/usr/local/bin/obs-hotkey --config ~/.config/obs-hotkey/hotkeys.json
```

Press Ctrl+C to stop, then restart the service:
```bash
systemctl --user start obs-hotkey.service
```

## How It Works

On Wayland, traditional keyboard libraries don't work due to security restrictions. This tool uses **evdev** to read keyboard input directly from `/dev/input/` devices at the kernel level, bypassing Wayland's restrictions.

The program:
1. Scans `/dev/input/event*` for keyboard devices
2. Monitors all keyboards for configured key presses
3. Sends commands to OBS via WebSocket when hotkeys are pressed
4. Auto-reconnects if OBS restarts or keyboards are unplugged

## Building for Different Architectures

```bash
# Current system
go build -mod=vendor -o obs-hotkey main.go

# Raspberry Pi (32-bit ARM)
GOARCH=arm GOARM=7 go build -mod=vendor -o obs-hotkey-arm main.go

# 64-bit ARM
GOARCH=arm64 go build -mod=vendor -o obs-hotkey-arm64 main.go

# AMD64
GOARCH=amd64 go build -mod=vendor -o obs-hotkey-amd64 main.go
```

## Adding New Actions

To add a new OBS WebSocket action:

1. Add a new method to `OBSClient` in [`main.go`](main.go):
   ```go
   func (c *OBSClient) StartRecording() {
       log.Println("Starting recording...")
       c.SendRequest("StartRecord")
   }
   ```

   For actions that need request data (see the [OBS WebSocket 5.x Protocol](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md)):
   ```go
   func (c *OBSClient) SetScene(sceneName string) {
       reqData := map[string]interface{}{
           "sceneName": sceneName,
       }
       c.SendRequestWithData("SetCurrentProgramScene", reqData)
   }
   ```

2. Add the hotkey binding in `main()` under the `bindings` slice.

3. Rebuild and reinstall:
    ```bash
    ./build.sh && ./install.sh
    ```

For the full list of available OBS WebSocket requests, see the [Available Actions](#available-actions) table or the [OBS WebSocket 5.x Protocol](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md) documentation.

## Requirements

- Linux (Wayland or X11)
- OBS Studio 28+ with WebSocket enabled
- Go 1.22+ (for building)
- Membership in the `input` group (for keyboard device access)

## Uninstall

```bash
# Stop and disable the service
systemctl --user stop obs-hotkey.service
systemctl --user disable obs-hotkey.service

# Remove systemd user service
rm ~/.config/systemd/user/obs-hotkey.service
systemctl --user daemon-reload

# Remove binary (check which location was used)
[ -f ~/.local/bin/obs-hotkey ] && rm ~/.local/bin/obs-hotkey
[ -f /usr/local/bin/obs-hotkey ] && sudo rm /usr/local/bin/obs-hotkey

# Optional: remove config
# rm -rf ~/.config/obs-hotkey
```

## License

MIT

## Credits

Built with:
- [gorilla/websocket](https://github.com/gorilla/websocket) - WebSocket client
- [gvalkov/golang-evdev](https://github.com/gvalkov/golang-evdev) - Linux evdev bindings
