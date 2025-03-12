#!/bin/bash

# Create the autostart directory if it doesn't exist
mkdir -p ~/.config/autostart/

# Create the desktop entry file
cat > ~/.config/autostart/obs-hotkey.desktop << EOF
[Desktop Entry]
Type=Application
Name=OBS Hotkeys Service Starter
Comment=Starts OBS Hotkeys after login
Exec=bash -c "sleep 10 && systemctl --user restart obs-hotkey.service"
Terminal=false
Hidden=false
X-GNOME-Autostart-Delay=10
StartupNotify=false
EOF

chmod +x ~/.config/autostart/obs-hotkey.desktop

echo "Created autostart entry to restart OBS Hotkey service with a 10 second delay after login"
echo "The file is located at: ~/.config/autostart/obs-hotkey.desktop"
