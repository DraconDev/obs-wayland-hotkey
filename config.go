package main

/*
OBS Hotkey Configuration

Edit this file to customize your hotkeys.
After making changes, rebuild with: ./build.sh
*/

// HotkeyConfig defines which keys trigger which actions
type HotkeyConfig struct {
	ToggleRecording string
	TogglePause     string
	// Add more actions here as needed:
	// ToggleStreaming string
	// TakeScreenshot  string
}

// config - Edit these values to change your hotkeys
var config = HotkeyConfig{
	ToggleRecording: "scroll lock", // Start/stop recording
	TogglePause:     "pause",       // Pause/resume recording
	// Add more hotkeys here:
	// ToggleStreaming: "f9",
	// TakeScreenshot:  "f10",
}

/*
Available key names:
- Function keys: f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12
- Special keys: scroll lock, pause, home, end, page up, page down, insert, delete

To add more keys, edit the keyNames map in main.go
*/
