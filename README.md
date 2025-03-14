# OBS-Hotkey

A simple Linux utility for controlling OBS Studio with global hotkeys. This tool connects to OBS via its WebSocket API and allows you to control recording, streaming, and other functions with customizable keyboard shortcuts.

## Features

- Global hotkeys that work even when OBS is not in focus
- Toggle recording start/stop with a single key press
- Toggle recording pause/resume
- Easily customizable hotkey configuration
- Support for OBS WebSocket v5 protocol
- No authentication required (works with default OBS WebSocket settings)

## Requirements

- Linux system
- OBS Studio 28+ with WebSocket plugin enabled (built-in since OBS v28)
- Python 3.6+
- Root privileges (required for global keyboard capture on Linux)

## Installation

### Quick Installation

Use the provided installer script to install OBS-Hotkey to your user directory:

```bash
chmod +x install.sh
./install.sh
```

This will:

- Install the application to `~/.local/bin/obs-hotkey` (or custom location)
- Set up a Python virtual environment with dependencies
- Create a desktop entry for easy launching from your application menu

### Manual Installation

1. Clone this repository:

   ```bash
   git clone https://github.com/yourusername/obs-hotkey.git
   cd obs-hotkey
   ```

2. Create and activate a virtual environment:

   ```bash
   python3 -m venv venv
   source venv/bin/activate
   ```

3. Install dependencies:
   ```bash
   pip install websocket-client keyboard
   ```

## Usage

1. Make sure OBS Studio is running with WebSocket server enabled:

   - In OBS, go to Tools → WebSocket Server Settings
   - Enable the WebSocket server
   - Default port is 4455 (no authentication required)

2. Run the script with sudo (required for keyboard input capture on Linux):
   ```bash
   ./run.sh
   ```
3. Use the configured hotkeys to control OBS:
   - `Scroll Lock`: Toggle recording start/stop
   - `Pause`: Toggle recording pause/resume
   - (And any other hotkeys you've configured)

## Configuration

You can easily customize the hotkeys by editing the `hotkeys.py` file:

```python
# Define your hotkeys here
HOTKEYS = {
    'toggle_recording': 'scroll lock',
    'toggle_pause': 'pause',
    # Add more hotkeys by uncommenting and configuring these:
    # 'toggle_streaming': 'home',
    # 'toggle_scene': 'page up',
    # ...etc
}
```

The file includes documentation on available key names and combinations.

### Available Actions

Currently implemented actions:

- `toggle_recording`: Start/stop recording
- `toggle_pause`: Pause/resume recording

More actions will be added in future updates.

## Troubleshooting

- **Keyboard shortcuts don't work**: Make sure you're running the script with sudo permissions using `./run.sh`
- **Connection errors**: Check that OBS is running and the WebSocket server is enabled (Tools → WebSocket Server Settings)
- **"Action not found" warnings**: Make sure the action name in hotkeys.py matches one of the implemented actions

## License

This project is licensed under the MIT License - see the LICENSE file for details.
