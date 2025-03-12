#!/bin/bash

# Set colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default installation directory
DEFAULT_INSTALL_DIR="$HOME/.local/bin/obs-hokkey"

# Ask for installation directory if not the default
if [ -n "$1" ]; then
    INSTALL_DIR="$1"
else
    read -p "Installation directory to check [$DEFAULT_INSTALL_DIR]: " INSTALL_DIR
    INSTALL_DIR=${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}
fi

echo -e "${YELLOW}Validating OBS-Hokkey installation at $INSTALL_DIR${NC}"
echo "=================================================="

# Check if installation directory exists
if [ -d "$INSTALL_DIR" ]; then
    echo -e "${GREEN}✓ Installation directory exists${NC}"
else
    echo -e "${RED}✗ Installation directory not found at $INSTALL_DIR${NC}"
    exit 1
fi

# Check for required files
REQUIRED_FILES=("main.py" "hotkeys.py" "run.sh")
ALL_FILES_PRESENT=true

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$INSTALL_DIR/$file" ]; then
        echo -e "${GREEN}✓ $file exists${NC}"
    else
        echo -e "${RED}✗ $file is missing${NC}"
        ALL_FILES_PRESENT=false
    fi
done

# Check virtual environment
if [ -d "$INSTALL_DIR/venv" ]; then
    echo -e "${GREEN}✓ Virtual environment exists${NC}"
    
    # Check for required packages
    source "$INSTALL_DIR/venv/bin/activate" 2>/dev/null
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Virtual environment can be activated${NC}"
        
        if pip list | grep -q websocket-client && pip list | grep -q keyboard; then
            echo -e "${GREEN}✓ Required packages installed${NC}"
        else
            echo -e "${RED}✗ Some required packages are missing${NC}"
        fi
        
        deactivate
    else
        echo -e "${RED}✗ Virtual environment cannot be activated${NC}"
    fi
else
    echo -e "${RED}✗ Virtual environment not found${NC}"
fi

# Check desktop entry
DESKTOP_ENTRY="$HOME/.local/share/applications/obs-hokkey.desktop"
if [ -f "$DESKTOP_ENTRY" ]; then
    echo -e "${GREEN}✓ Desktop entry exists${NC}"
    
    if grep -q "$INSTALL_DIR" "$DESKTOP_ENTRY"; then
        echo -e "${GREEN}✓ Desktop entry points to correct installation${NC}"
    else
        echo -e "${RED}✗ Desktop entry doesn't point to $INSTALL_DIR${NC}"
    fi
else
    echo -e "${YELLOW}! Desktop entry not found at $DESKTOP_ENTRY${NC}"
fi

# Check systemd user service
SERVICE_FILE="$HOME/.config/systemd/user/obs-hokkey.service"
if [ -f "$SERVICE_FILE" ]; then
    echo -e "${GREEN}✓ Systemd user service file exists${NC}"
    
    if systemctl --user is-enabled obs-hokkey.service &>/dev/null; then
        echo -e "${GREEN}✓ Service is enabled${NC}"
    else
        echo -e "${YELLOW}! Service is not enabled${NC}"
    fi
    
    if systemctl --user is-active obs-hokkey.service &>/dev/null; then
        echo -e "${GREEN}✓ Service is running${NC}"
    else
        echo -e "${YELLOW}! Service is not running${NC}"
    fi
else
    echo -e "${YELLOW}! Systemd service file not found (optional)${NC}"
fi

# Check sudo configuration
SUDO_CONFIG="/etc/sudoers.d/obs-hokkey"
if [ -f "$SUDO_CONFIG" ]; then
    echo -e "${GREEN}✓ Sudo config exists for passwordless execution${NC}"
else
    echo -e "${YELLOW}! No passwordless sudo config found (optional)${NC}"
    echo -e "  You may need to enter password each time obs-hokkey runs"
    echo -e "  To set up passwordless execution, see background.md"
fi

# Final assessment
echo -e "\n${YELLOW}Installation Assessment:${NC}"
if $ALL_FILES_PRESENT && [ -d "$INSTALL_DIR/venv" ]; then
    echo -e "${GREEN}✓ OBS-Hokkey appears to be installed correctly!${NC}"
    echo -e "  You can run it with: $INSTALL_DIR/run.sh"
else
    echo -e "${RED}✗ OBS-Hokkey installation appears incomplete or damaged${NC}"
    echo -e "  Try running the installer again: ./install.sh"
fi

# Test if OBS is running and WebSocket server is accessible
echo -e "\n${YELLOW}Connectivity Test:${NC}"
if pgrep -x "obs" >/dev/null; then
    echo -e "${GREEN}✓ OBS Studio is running${NC}"
    
    # Try to connect to WebSocket (basic check)
    if command -v curl &>/dev/null; then
        # Add a timeout to avoid hanging
        if curl --max-time 2 -s http://localhost:4455 &>/dev/null; then
            echo -e "${GREEN}✓ OBS WebSocket port is accessible${NC}"
        else
            echo -e "${YELLOW}! OBS WebSocket port 4455 is not responding${NC}"
            echo -e "  Check that WebSocket Server is enabled in OBS Studio"
            echo -e "  Tools → WebSocket Server Settings → Enable WebSocket Server"
        fi
    else
        echo -e "${YELLOW}! Cannot test WebSocket connection (curl not installed)${NC}"
    fi
else
    echo -e "${RED}✗ OBS Studio is not running${NC}"
    echo -e "  Start OBS Studio before running obs-hokkey"
fi

echo -e "\n${YELLOW}For more information on running in background:${NC}"
echo -e "  See the background.md file for detailed instructions"
