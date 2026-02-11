#!/usr/bin/env bash
# Installation script for OBS Wayland Hotkey

set -e

echo "=== OBS Wayland Hotkey Installer ==="
echo ""

# Check if binary exists
if [ ! -f "obs-hotkey-go" ]; then
    echo "Binary not found. Building..."
    ./build.sh
fi

# Install to /usr/local/bin
echo "Installing to /usr/local/bin..."
sudo cp obs-hotkey-go /usr/local/bin/
sudo chmod +x /usr/local/bin/obs-hotkey-go

echo "✓ Binary installed to /usr/local/bin/obs-hotkey-go"
echo ""

# Setup passwordless sudo
echo "Setting up passwordless sudo..."
SUDOERS_FILE="/etc/sudoers.d/obs-hotkey"
CURRENT_USER=$(whoami)

sudo tee "$SUDOERS_FILE" > /dev/null <<EOF
# Allow $CURRENT_USER to run obs-hotkey-go without password
$CURRENT_USER ALL=(root) NOPASSWD: /usr/local/bin/obs-hotkey-go
EOF

sudo chmod 0440 "$SUDOERS_FILE"
echo "✓ Passwordless sudo configured"
echo ""

# Create systemd service
echo "Creating systemd service..."
SERVICE_FILE="$HOME/.config/systemd/user/obs-hotkey.service"
mkdir -p "$HOME/.config/systemd/user"

cat > "$SERVICE_FILE" <<EOF
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

echo "✓ Systemd service created"
echo ""

# Enable and start service
echo "Enabling systemd service..."
systemctl --user daemon-reload
systemctl --user enable obs-hotkey.service

echo "✓ Service enabled"
echo ""

echo "=== Installation Complete! ==="
echo ""
echo "The service will start automatically on login."
echo ""
echo "To start now:"
echo "  systemctl --user start obs-hotkey.service"
echo ""
echo "To check status:"
echo "  systemctl --user status obs-hotkey.service"
echo ""
echo "To view logs:"
echo "  journalctl --user -u obs-hotkey.service -f"
echo ""
echo "To stop:"
echo "  systemctl --user stop obs-hotkey.service"
echo ""
