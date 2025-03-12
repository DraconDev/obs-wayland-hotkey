"""
OBS Hotkey Configuration File

This file contains all keyboard shortcuts used to control OBS via the obs-hokkey script.
Edit this file to customize your hotkeys according to your preferences.

Available actions:
- toggle_recording: Start/stop recording
- toggle_pause: Pause/resume recording
- (more actions can be added as needed)

For a full list of key names you can use, see:
https://github.com/boppreh/keyboard#api

Common key names:
- Function keys: 'f1', 'f2', ..., 'f12'
- Special keys: 'esc', 'enter', 'space', 'tab', 'backspace', 'insert', 'delete', 'home', 'end', 'page up', 'page down'
- Arrow keys: 'up', 'down', 'left', 'right'
- Modifiers: 'shift', 'ctrl', 'alt', 'windows'
- Combinations: 'ctrl+shift+t', 'alt+tab', etc.
"""

# Define your hotkeys here
HOTKEYS = {
    # Format: 'action_name': 'key_name'
    'toggle_recording': 'insert',
    'toggle_pause': 'pause',
    # Add more hotkeys as needed
    # 'toggle_streaming': 'home',
    # 'toggle_scene': 'end',
}

# Map actions to their display names
ACTION_DESCRIPTIONS = {
    'toggle_recording': 'Toggle Recording',
    'toggle_pause': 'Toggle Pause/Resume Recording',
    # Add more descriptions as needed
    # 'toggle_streaming': 'Toggle Streaming',
    # 'toggle_scene': 'Switch to Next Scene',
}
