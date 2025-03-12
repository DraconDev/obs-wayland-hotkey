# OBS Hotkey

Control OBS Studio recording with global hotkeys using Python and the OBS WebSocket.

## Setup

### First-time setup

1. Make the setup script executable:

   ```
   chmod +x setup.sh
   ```

2. Run the setup script to create a virtual environment and install dependencies:
   ```
   ./setup.sh
   ```

### Running the application

After setting up, you can run the application in two ways:

1. Using the run script (recommended):

   ```
   chmod +x run.sh
   ./run.sh
   ```

2. Manually:
   ```
   source venv/bin/activate
   python main.py
   ```

> **Note for Linux users:** Due to the way keyboard input is captured, this application requires root privileges to run on Linux systems. The run script will automatically use sudo when needed.

## Default Hotkeys

- `Insert`: Toggle Recording (Start/Stop)
- `Pause`: Toggle Pause/Resume Recording

## Requirements

- OBS Studio with WebSocket plugin enabled on port 4455
- Python 3.6+
- On Linux: Root privileges for keyboard capture
