# OBS Wayland Hotkey

A lightweight Go binary for controlling OBS Studio with global hotkeys on Wayland (and X11).

**Works on Wayland!** Uses evdev for direct keyboard input capture, bypassing Wayland's security restrictions.

## Features

- ✅ **Wayland & X11 Support** - Works on both display servers
- ✅ **Single Binary** - No dependencies, just 7.7MB
- ✅ **Global Hotkeys** - Works even when OBS is not in focus
- ✅ **Auto-reconnect** - Automatically reconnects to OBS if it restarts
- ✅ **Multi-keyboard** - Monitors all connected keyboards
- ✅ **Low Resource Usage** - ~10-20MB RAM, minimal CPU

## Quick Start

### 1. Build

```bash
chmod +x build.sh
./build.sh
```

This creates the `obs-hotkey-go` binary (~7.7MB).

### 2. Configure OBS

1. Open OBS Studio
2. Go to **Tools → WebSocket Server Settings**
3. Check **"Enable WebSocket server"**
4. Use default port **4455**
5. Disable authentication

### 3. Run

```bash
sudo ./obs-hotkey-go
```

You'll need sudo for keyboard device access (`/dev/input/`).

## Default Hotkeys

- **Scroll Lock** - Toggle recording start/stop
- **Pause** - Toggle recording pause/resume

## Customizing Hotkeys

Edit the `config` struct in [`main.go`](main.go:1):

```go
var config = HotkeyConfig{
	ToggleRecording: "scroll lock",
	TogglePause:     "pause",
}
```

Then rebuild:
```bash
./build.sh
```

### Supported Keys

- Function keys: `f1`, `f2`, `f3`, `f4`, `f5`, `f6`, `f7`, `f8`, `f9`, `f10`, `f11`, `f12`
- Special keys: `scroll lock`, `pause`, `home`, `end`, `page up`, `page down`, `insert`, `delete`

To add more keys, edit the `keyNames` map in [`main.go`](main.go:1).

## Installation

### System-wide Installation

```bash
sudo cp obs-hotkey-go /usr/local/bin/
sudo chmod +x /usr/local/bin/obs-hotkey-go
```

Then run from anywhere:
```bash
sudo obs-hotkey-go
```

### Autostart with Systemd

Create `~/.config/systemd/user/obs-hotkey.service`:

```ini
[Unit]
Description=OBS Hotkey Controller
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/obs-hotkey-go
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=graphical-session.target
```

Enable and start:
```bash
systemctl --user daemon-reload
systemctl --user enable obs-hotkey.service
systemctl --user start obs-hotkey.service
```

**Note**: You'll need passwordless sudo (see below).

### Passwordless Sudo

To avoid entering password every time:

```bash
sudo visudo -f /etc/sudoers.d/obs-hotkey
```

Add (replace `your_username`):
```
your_username ALL=(root) NOPASSWD: /usr/local/bin/obs-hotkey-go
```

## Troubleshooting

### "must be run as root"

Run with sudo:
```bash
sudo ./obs-hotkey-go
```

### "failed to connect to OBS"

1. Make sure OBS is running
2. Enable WebSocket: Tools → WebSocket Server Settings
3. Check port 4455 is not blocked

### "No keyboard devices found"

1. Verify you're running with sudo
2. Check `/dev/input/` permissions:
   ```bash
   ls -l /dev/input/
   ```

### Hotkey not working

1. Check the key is in the `keyNames` map in [`main.go`](main.go:1)
2. Verify OBS is connected (check terminal output)
3. Try a different key

## How It Works

On Wayland, traditional keyboard libraries don't work due to security restrictions. This tool uses **evdev** to read keyboard input directly from `/dev/input/` devices at the kernel level, bypassing Wayland's restrictions.

The program:
1. Scans `/dev/input/` for keyboard devices
2. Monitors all keyboards for configured key presses
3. Sends commands to OBS via WebSocket when hotkeys are pressed
4. Auto-reconnects if OBS restarts or keyboards are unplugged

## Building for Different Architectures

```bash
# Current system
go build -o obs-hotkey-go main.go

# Raspberry Pi (32-bit ARM)
GOARCH=arm GOARM=7 go build -o obs-hotkey-go-arm main.go

# 64-bit ARM
GOARCH=arm64 go build -o obs-hotkey-go-arm64 main.go

# AMD64
GOARCH=amd64 go build -o obs-hotkey-go-amd64 main.go
```

## Adding New Actions

1. Add the OBS command method to `OBSClient`:
   ```go
   func (c *OBSClient) YourAction() {
       log.Println("Doing something...")
       c.SendRequest("YourOBSCommand")
   }
   ```

2. Add to config:
   ```go
   var config = HotkeyConfig{
       YourAction: "f1",
   }
   ```

3. Map in `main()`:
   ```go
   if keyName == config.YourAction {
       hotkeyActions[keyCode] = client.YourAction
   }
   ```

4. Rebuild: `./build.sh`

## Requirements

- Linux (Wayland or X11)
- OBS Studio 28+ with WebSocket enabled
- Go 1.21+ (for building)
- Root/sudo access (for running)

## License

MIT License

## Credits

Built with:
- [gorilla/websocket](https://github.com/gorilla/websocket) - WebSocket client
- [gvalkov/golang-evdev](https://github.com/gvalkov/golang-evdev) - Linux evdev bindings
