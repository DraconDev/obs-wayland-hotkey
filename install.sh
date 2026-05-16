#!/usr/bin/env bash
set -e

echo "=== OBS Hotkey Installer ==="
echo ""

CURRENT_USER=$(whoami)

# Check if binary exists
if [ ! -f "obs-hotkey" ]; then
    echo "Binary not found. Building..."
    ./build.sh
fi

# Detect install location — prefer ~/.local/bin (no sudo needed), fall back to /usr/local/bin
INSTALL_DIR=""
if [ -d "$HOME/.local/bin" ] || can_write_dir "$HOME/.local/bin" 2>/dev/null; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
elif [ -w /usr/local/bin ]; then
    INSTALL_DIR="/usr/local/bin"
else
    echo "Error: Cannot write to ~/.local/bin or /usr/local/bin."
    echo "Please ensure ~/.local/bin exists or you have write access to /usr/local/bin."
    exit 1
fi

echo "Installing to $INSTALL_DIR..."
if [ "$INSTALL_DIR" = "/usr/local/bin" ]; then
    sudo cp obs-hotkey "$INSTALL_DIR/"
    sudo chmod +x "$INSTALL_DIR/obs-hotkey"
else
    cp obs-hotkey "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/obs-hotkey"
fi
echo "✓ Binary installed to $INSTALL_DIR/obs-hotkey"
echo ""

# Create config directory
echo "Creating config directory..."
CONFIG_DIR="$HOME/.config/obs-hotkey"
mkdir -p "$CONFIG_DIR"
echo "✓ Config directory: $CONFIG_DIR"
echo ""

# Ensure user is in input group (needed for /dev/input/ access)
echo "Checking input group membership..."
if groups "$CURRENT_USER" | grep -q '\binput\b'; then
    echo "✓ Already in input group"
else
    echo "Adding $CURRENT_USER to input group..."
    sudo usermod -aG input "$CURRENT_USER"
    echo "✓ Added to input group"
    echo "  Note: You must log out and back in for this to take effect."
fi
echo ""

# Migrate from old service if present
if systemctl --user is-enabled obs-wayland-hotkey.service &>/dev/null; then
    echo "Found old obs-wayland-hotkey.service, migrating..."
    systemctl --user stop obs-wayland-hotkey.service 2>/dev/null || true
    systemctl --user disable obs-wayland-hotkey.service 2>/dev/null || true
    echo "✓ Old service stopped and disabled"
    echo ""
fi

# Create systemd service (no sudo needed - uses input group)
echo "Creating systemd service..."
SERVICE_FILE="$HOME/.config/systemd/user/obs-hotkey.service"
mkdir -p "$HOME/.config/systemd/user"

cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=OBS Hotkey Controller
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/obs-hotkey --config $HOME/.config/obs-hotkey/hotkeys.json
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=graphical-session.target
EOF

echo "✓ Service created"
echo ""

# Enable and start service
echo "Enabling systemd service..."
systemctl --user daemon-reload
systemctl --user enable obs-hotkey.service
systemctl --user start obs-hotkey.service
echo "✓ Service enabled and started"
echo ""

echo "=== Installation Complete! ==="
echo ""
echo "Config file: $HOME/.config/obs-hotkey/hotkeys.json"
echo ""
echo "Manage the service:"
echo "  systemctl --user status obs-hotkey.service"
echo "  systemctl --user restart obs-hotkey.service"
echo "  systemctl --user stop obs-hotkey.service"
echo ""
echo "View logs:"
echo "  journalctl --user -u obs-hotkey.service -f"
echo ""
echo "Edit hotkeys:"
echo "  ~/.config/obs-hotkey/hotkeys.json"
