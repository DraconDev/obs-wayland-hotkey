#!/bin/bash

# Define installation locations
DEFAULT_INSTALL_DIR="$HOME/.local/bin/obs-hotkey"
DESKTOP_ENTRY_DIR="$HOME/.local/share/applications"

echo "OBS-Hotkey Installer"
echo "===================="

# Ask for installation directory
read -p "Installation directory [$DEFAULT_INSTALL_DIR]: " INSTALL_DIR
INSTALL_DIR=${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}

# Create installation directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Copy all necessary files
echo "Copying files to $INSTALL_DIR..."
cp -r main.py hotkeys.py README.md LINUX_USAGE.md "$INSTALL_DIR/"

# Create the virtual environment in the installation directory
echo "Creating virtual environment..."
python3 -m venv "$INSTALL_DIR/venv"
source "$INSTALL_DIR/venv/bin/activate"
pip install websocket-client keyboard
deactivate

# Create run script in the installation directory
cat > "$INSTALL_DIR/run.sh" << EOF
#!/bin/bash

# Get the directory where the script is located
SCRIPT_DIR="\$( cd "\$( dirname "\${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Use the absolute path to the Python in the virtual environment
VENV_PYTHON="\${SCRIPT_DIR}/venv/bin/python"

echo "Running with sudo as keyboard input capture requires root privileges on Linux"
sudo "\${VENV_PYTHON}" "\${SCRIPT_DIR}/main.py" "\$@"
EOF

# Make the run script executable
chmod +x "$INSTALL_DIR/run.sh"

# Create a desktop entry for easy launching
echo "Creating desktop entry..."
mkdir -p "$DESKTOP_ENTRY_DIR"
cat > "$DESKTOP_ENTRY_DIR/obs-hotkey.desktop" << EOF
[Desktop Entry]
Type=Application
Name=OBS Hotkeys
Comment=Control OBS Studio with global hotkeys
Exec=$INSTALL_DIR/run.sh
Terminal=true
Categories=AudioVideo;Utility;
EOF

# Make the desktop entry executable
chmod +x "$DESKTOP_ENTRY_DIR/obs-hotkey.desktop"

echo ""
echo "Installation complete!"
echo "You can now run OBS-Hotkey with:"
echo "  $INSTALL_DIR/run.sh"
echo ""
echo "Or launch it from your application menu as 'OBS Hotkeys'"
echo ""
echo "You can edit your hotkeys by modifying:"
echo "  $INSTALL_DIR/hotkeys.py"
