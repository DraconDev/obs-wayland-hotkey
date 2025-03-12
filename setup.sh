#!/bin/bash

echo "Setting up OBS Hotkey virtual environment..."

# Check if python3-venv is installed
if ! dpkg -l | grep -q python3-venv; then
    echo "Installing python3-venv..."
    sudo apt-get update
    sudo apt-get install -y python3-venv
fi

# Create virtual environment if it doesn't exist
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
fi

# Activate virtual environment and install dependencies
echo "Installing dependencies..."
source venv/bin/activate
pip install -r requirements.txt

echo "Setup complete. Use the following commands to run the script:"
echo ""
echo "  source venv/bin/activate   # Activate the virtual environment"
echo "  python main.py             # Run the script"
echo ""
echo "Or use the run.sh script:"
echo "  ./run.sh"
