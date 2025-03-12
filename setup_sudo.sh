#!/bin/bash

# Get current username
USERNAME=$(whoami)

# Path to obs-hotkey Python script
PYTHON_PATH="/home/dracon/.local/bin/obs-hotkey/venv/bin/python"
SCRIPT_PATH="/home/dracon/.local/bin/obs-hotkey/main.py"

echo "Setting up passwordless sudo for OBS-Hotkey..."
echo "This will allow the script to run with sudo permissions without prompting for a password."

# Create the sudoers file
echo "Creating sudo configuration file (will ask for your sudo password)..."
sudo bash -c "cat > /etc/sudoers.d/obs-hotkey << EOF
# Allow $USERNAME to run the obs-hotkey script without password
$USERNAME ALL=(root) NOPASSWD: $PYTHON_PATH $SCRIPT_PATH
EOF"

# Set appropriate permissions
sudo chmod 440 /etc/sudoers.d/obs-hotkey

echo "Sudo configuration set up successfully!"
echo "You should no longer need to enter password when running obs-hotkey."
echo "Try restarting the service with: systemctl --user restart obs-hotkey.service"
