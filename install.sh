#!/bin/bash

# Define installation locations
DEFAULT_INSTALL_DIR="$HOME/.local/bin/obs-hotkey"
DESKTOP_ENTRY_DIR="$HOME/.local/share/applications"

echo "OBS-Hotkey Installer"
echo "===================="

# Ask for installation directory with a timeout
read -t 30 -p "Installation directory [$DEFAULT_INSTALL_DIR] (30s timeout): " INSTALL_DIR || echo ""
INSTALL_DIR=${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}

echo "Installing to: $INSTALL_DIR"

# Create installation directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Copy all necessary files
echo "Copying files to $INSTALL_DIR..."
cp -v main.py hotkeys.py README.md LINUX_USAGE.md "$INSTALL_DIR/" 2>/dev/null || cp -v *.py *.md "$INSTALL_DIR/" 2>/dev/null
if [ $? -ne 0 ]; then
    echo "Warning: Some files could not be copied. Continuing anyway..."
fi

# Create requirements.txt if not exists for pip installation
if [ ! -f "requirements.txt" ]; then
    echo "Creating requirements.txt..."
    echo "websocket-client==1.6.1" > "$INSTALL_DIR/requirements.txt"
    echo "keyboard==0.13.5" >> "$INSTALL_DIR/requirements.txt"
else
    echo "Copying requirements.txt..."
    cp -v requirements.txt "$INSTALL_DIR/"
fi

# Create the virtual environment in the installation directory
echo "Creating virtual environment..."
python3 -m venv "$INSTALL_DIR/venv"
source "$INSTALL_DIR/venv/bin/activate"
pip install -r "$INSTALL_DIR/requirements.txt"
deactivate

# Create run script in the installation directory
echo "Creating run script..."
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
