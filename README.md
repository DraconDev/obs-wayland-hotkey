# OBS Wayland Hotkey

A lightweight Go binary for controlling OBS Studio with global hotkeys on Wayland (and X11).

**Works on Wayland!** Uses evdev for direct keyboard input capture, bypassing Wayland's security restrictions.

## Features

- ✅ **Wayland & X11 Support** - Works on both display servers
- ✅ **Single Binary** - No dependencies, just 7.7MB
- ✅ **Global Hotkeys** - Works even when OBS is not in focus
- ✅ **Auto-start on Login** - Set it and forget it
- ✅ **Auto-reconnect** - Automatically reconnects to OBS if it restarts
- ✅ **Multi-keyboard** - Monitors all connected keyboards
- ✅ **Low Resource Usage** - ~10-20MB RAM, minimal CPU

## Quick Install (Recommended)

Run the installer to set up auto-start:

```bash
chmod +x install.sh
./install.sh
```

This will:
- Build the binary (if needed)
- Install to `/usr/local/bin/`
- Configure passwordless sudo
- Create a systemd service that starts automatically on login
- Enable and start the service

**After installation, the hotkey controller will start automatically when you log in!**

## Default Hotkeys

- **Scroll Lock** - Toggle recording start/stop
- **Pause** - Toggle recording pause/resume

## Manual Setup

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

### 3. Run Manually

```bash
sudo ./obs-hotkey-go
```

You'll need sudo for keyboard device access (`/dev/input/`).

## System-wide Installation

### Install Binary

```bash
sudo cp obs-hotkey-go /usr/local/bin/
sudo chmod +x /usr/local/bin/obs-hotkey-go
```

Then run from anywhere:
```bash
sudo obs-hotkey-go
```

### Autostart with Systemd (Manual)

If you didn't use the installer, you can manually set up auto-start:

1. **Configure passwordless sudo:**
   ```bash
   sudo tee /etc/sudoers.d/obs-hotkey > /dev/null << 'EOF'
   # Allow your_username to run obs-hotkey-go without password
   your_username ALL=(root) NOPASSWD: /usr/local/bin/obs-hotkey-go
   EOF
   ```
   
   Replace `your_username` with your actual username.

2. **Create systemd service** `~/.config/systemd/user/obs-hotkey.service`:
   ```bash
   mkdir -p ~/.config/systemd/user
   cat > ~/.config/systemd/user/obs-hotkey.service << 'EOF'
   [Unit]
   Description=OBS Hotkey Controller (Wayland)
   After=graphical-session.target

   [Service]
   Type=simple
   ExecStart=/usr/bin/sudo /usr/local/bin/obs-hotkey-go
   Restart=on-failure
   RestartSec=10s

   [Install]
   WantedBy=graphical-session.target
   EOF
   ```

3. **Enable and start:**
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable obs-hotkey.service
   systemctl --user start obs-hotkey.service
   ```

## Managing the Service

After installation, use these commands:

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
sudo cp obs-hotkey-go /usr/local/bin/
systemctl --user restart obs-hotkey.service
```

### Supported Keys

- Function keys: `f1`, `f2`, `f3`, `f4`, `f5`, `f6`, `f7`, `f8`, `f9`, `f10`, `f11`, `f12`
- Special keys: `scroll lock`, `pause`, `home`, `end`, `page up`, `page down`, `insert`, `delete`

To add more keys, edit the `keyNames` map in [`main.go`](main.go:1).

## Troubleshooting

### Service not starting

Check the logs:
```bash
journalctl --user -u obs-hotkey.service -n 50
```

Common issues:
- **"must be run as root"** - Passwordless sudo not configured correctly
- **"failed to connect to OBS"** - OBS not running or WebSocket not enabled
- **"No keyboard devices found"** - Permission issue with `/dev/input/`

### "must be run as root"

Make sure passwordless sudo is configured:
```bash
sudo cat /etc/sudoers.d/obs-hotkey
```

Should show:
```
your_username ALL=(root) NOPASSWD: /usr/local/bin/obs-hotkey-go
```

### "failed to connect to OBS"

1. Make sure OBS is running
2. Enable WebSocket: Tools → WebSocket Server Settings
3. Check port 4455 is not blocked
4. The service will auto-reconnect when OBS starts

### Hotkey not working

1. Check the service is running: `systemctl --user status obs-hotkey.service`
2. Check the key is in the `keyNames` map in [`main.go`](main.go:1)
3. Verify OBS is connected (check logs)
4. Try a different key

### Manual testing

Stop the service and run manually to see output:
```bash
systemctl --user stop obs-hotkey.service
sudo /usr/local/bin/obs-hotkey-go
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
go build -o obs-hotkey-go main.go

# Raspberry Pi (32-bit ARM)
GOARCH=arm GOARM=7 go build -o obs-hotkey-go-arm main.go

# 64-bit ARM
GOARCH=arm64 go build -o obs-hotkey-go-arm64 main.go

# AMD64
GOARCH=amd64 go build -o obs-hotkey-go-amd64 main.go
```

## Adding New Actions

1. Add the OBS command method to `OBSClient` in [`main.go`](main.go:1):
   ```go
   func (c *OBSClient) YourAction() {
       log.Println("Doing something...")
       c.SendRequest("YourOBSCommand")
   }
   ```

2. Add to config:
   ```go
   var config = HotkeyConfig{
       ToggleRecording: "scroll lock",
       TogglePause:     "pause",
       YourAction:      "f1",
   }
   ```

3. Map in `main()`:
   ```go
   for keyCode, keyName := range keyNames {
       if keyName == config.ToggleRecording {
           hotkeyActions[keyCode] = client.ToggleRecording
       } else if keyName == config.TogglePause {
           hotkeyActions[keyCode] = client.TogglePause
       } else if keyName == config.YourAction {
           hotkeyActions[keyCode] = client.YourAction
       }
   }
   ```

4. Rebuild and reinstall:
   ```bash
   ./build.sh
   sudo cp obs-hotkey-go /usr/local/bin/
   systemctl --user restart obs-hotkey.service
   ```

## Requirements

- Linux (Wayland or X11)
- OBS Studio 28+ with WebSocket enabled
- Go 1.21+ (for building)
- Root/sudo access (for running)

## Uninstall

```bash
# Stop and disable the service
systemctl --user stop obs-hotkey.service
systemctl --user disable obs-hotkey.service

# Remove files
rm ~/.config/systemd/user/obs-hotkey.service
sudo rm /usr/local/bin/obs-hotkey-go
sudo rm /etc/sudoers.d/obs-hotkey

# Reload systemd
systemctl --user daemon-reload
```

## License

MIT License

## Credits

Built with:
- [gorilla/websocket](https://github.com/gorilla/websocket) - WebSocket client
- [gvalkov/golang-evdev](https://github.com/gvalkov/golang-evdev) - Linux evdev bindings
