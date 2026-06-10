# obs-hotkey

[![Crates.io](https://img.shields.io/crates/v/obs-hotkey?style=for-the-badge)](https://crates.io/crates/obs-hotkey)
[![CI](https://img.shields.io/github/actions/workflow/status/DraconDev/obs-wayland-hotkey/ci.yml?style=for-the-badge&label=CI)](https://github.com/DraconDev/obs-wayland-hotkey/actions)
[![License](https://img.shields.io/crates/l/obs-hotkey?style=for-the-badge)](https://github.com/DraconDev/obs-wayland-hotkey/blob/main/LICENSE)

> A lightweight Rust daemon for controlling OBS Studio with global hotkeys on Wayland and X11.

**Works on Wayland!** Uses evdev for direct keyboard input capture, bypassing Wayland's security restrictions.

## What obs-hotkey is for

obs-hotkey was originally written as a Wayland helper because OBS native hotkeys are limited to OBS-window focus on Wayland. It does two things:

1. **Global OBS hotkeys on Wayland** (where OBS native hotkeys cannot reach the global desktop), and on X11 as a portability / single-config-file alternative.
2. **Multi-action key gestures that OBS itself cannot do** — one chord runs several OBS WebSocket calls in order, optionally with delays, optionally with release-side actions.

If you only need a single `F12` to toggle recording, OBS native hotkeys will do that on X11 and on Windows/macOS. If you need `Ctrl+Shift+R` to start recording **and** set your mic volume **and** schedule a replay save five seconds later, obs-hotkey is the tool for the job. See the comparison table below for the full breakdown.

---

## Features

- **Wayland & X11 Support** — works on both display servers
- **Single Static Binary** — no runtime dependencies
- **Global Hotkeys** — works even when OBS is not focused (the original Wayland use case)
- **Chord Hotkeys** — use combinations like `ctrl + shift + f1`
- **Action Combos** — trigger multiple OBS actions from one key chord (the unique value on X11)
- **Delayed Actions** — schedule actions in a combo with per-step delays (e.g. start recording 3 seconds after a hotkey)
- **Push-to-Release Actions** — run a second set of actions when the chord is released (push-to-record / push-to-talk)
- **Scene Switching** — dedicated `switch_scene` action for the most common pro workflow
- **Keyboard Allowlist** — restrict hotkey capture to specific /dev/input devices in multi-keyboard setups
- **One-shot CLI** — `obs-hotkey action <name>` triggers a single action from scripts and systemd timers
- **Mic Volume Presets** — set input volume as part of a combo
- **Auto-start on Login** — systemd user service integration
- **Auto-reconnect** — automatically reconnects if OBS restarts
- **Multi-keyboard** — monitors all connected keyboards
- **F13-F24 Support** — use extra keys as stream deck buttons
- **Hotkey Debouncing** — 50ms debounce prevents accidental double-toggles
- **Non-blocking Actions** — hotkeys stay responsive during network I/O
- **Panic-safe Reader Threads** — a panic in one keyboard device cannot kill the daemon
- **Config Validation** — typos in config are rejected with clear errors

---

## OBS Native Hotkeys vs obs-hotkey

Use this table to decide whether you need obs-hotkey at all, and which features to enable.

| Workflow | OBS native hotkey | obs-hotkey |
|----------|-------------------|------------|
| `F12` → toggle recording on X11 | ✅ works | ✅ works (redundant on X11) |
| `F12` → toggle recording on Wayland | ⚠️ only when OBS is focused | ✅ works globally |
| `Ctrl+Shift+R` → start recording **and** set mic volume | ❌ one hotkey = one action | ✅ multi-action combo |
| `Ctrl+Shift+S` → start streaming **and** set mic volume | ❌ | ✅ multi-action combo |
| Push `F13` to record, release to stop | ❌ OBS does not have a push-to-record action | ✅ `release_actions` |
| Press once, recording starts in 10 seconds | ❌ | ✅ `action_delays_ms` |
| Switch between multiple scenes via hotkey | ✅ "Switch to scene" hotkey per scene, configured in OBS | ✅ `switch_scene` in one config file |
| Multi-keyboard, exclude guest USB | ❌ global, catches everything | ✅ `allowed_devices` allowlist |
| Trigger an action from a systemd timer or shell script | ❌ | ✅ `obs-hotkey action <name>` |

The two obs-hotkey features with no OBS equivalent are **action combos** (multiple OBS calls per chord) and **delayed actions**. If you need either of those, obs-hotkey earns its keep even on X11. If you only need a single key to start recording, OBS native hotkeys are enough and obs-hotkey is mostly redundant on X11.

---

## Quick Start

```bash
# 1. Install
cargo install obs-hotkey

# 2. Run once — shows the setup guide
obs-hotkey

# 3. Follow the on-screen steps:
#    - Enable OBS WebSocket Server
#    - Add yourself to the input group
#    - Run: obs-hotkey setup
```

---

## Installation

### From crates.io

```bash
cargo install obs-hotkey
```

### From GitHub Releases

```bash
# amd64
curl -L https://github.com/DraconDev/obs-wayland-hotkey/releases/latest/download/obs-hotkey-x86_64-unknown-linux-gnu -o obs-hotkey
chmod +x obs-hotkey
sudo cp obs-hotkey /usr/local/bin/

# ARM64
curl -L https://github.com/DraconDev/obs-wayland-hotkey/releases/latest/download/obs-hotkey-aarch64-unknown-linux-gnu -o obs-hotkey
chmod +x obs-hotkey
sudo cp obs-hotkey /usr/local/bin/
```

### From source

```bash
git clone https://github.com/DraconDev/obs-wayland-hotkey.git
cd obs-wayland-hotkey
./install.sh    # builds and runs setup
```

---

## Setup

### 1. Enable OBS WebSocket Server

1. Open OBS Studio
2. Go to **Tools → WebSocket Server Settings**
3. Check **Enable WebSocket server**
4. Port: **4455** (default)
5. Authentication: **disabled**

### 2. Add yourself to the input group

```bash
sudo usermod -aG input $(whoami)
# Log out and back in for changes to take effect
```

### 3. Run setup

```bash
obs-hotkey setup
```

This writes the systemd user service and enables it to start on login.

---

## Usage

```
obs-hotkey              # Show quickstart guide (interactive setup help)
obs-hotkey daemon       # Run the hotkey daemon
obs-hotkey setup        # Install systemd user service
obs-hotkey teardown     # Remove service and binaries
obs-hotkey status       # Check service, config, and OBS connectivity
obs-hotkey action NAME  # Trigger a single OBS action once (no daemon)
```

### Global flags

| Flag | Description |
|------|-------------|
| `--config <path>` | Use a custom config file |
| `--version` | Show version |
| `--help` | Show full help |

### Teardown options

| Command | Description |
|---------|-------------|
| `obs-hotkey teardown` | Stop service, remove service files and binaries |
| `obs-hotkey teardown --purge` | Above + remove config directory |

---

## Default Hotkeys

| Key | Action |
|-----|--------|
| **Scroll Lock** | Toggle recording start/stop |
| **Pause** | Toggle recording pause/resume |

---

## Customizing Hotkeys

Edit `~/.config/obs-hotkey/hotkeys.json`:

```json
{
  "obs_host": "ws://localhost:4455",
  "hotkeys": {
    "toggle_recording": "scroll lock",
    "toggle_pause": "pause",
    "toggle_streaming": "",
    "screenshot": "",
    "toggle_mute_mic": "",
    "toggle_studio_mode": "",
    "toggle_replay_buffer": "",
    "save_replay": ""
  },
  "screenshot_source": "",
  "screenshot_dir": "~/Pictures",
  "mic_name": "",
  "mic_volume": 1.0,
  "allowed_devices": [],
  "hotkey_combos": []
}
```

Existing single-action hotkeys still work. To trigger multiple OBS actions from one key chord, add entries to `hotkey_combos`:

```json
{
  "obs_host": "ws://localhost:4455",
  "hotkeys": {
    "toggle_recording": "",
    "toggle_pause": "",
    "toggle_streaming": "",
    "screenshot": "",
    "toggle_mute_mic": "",
    "toggle_studio_mode": "",
    "toggle_replay_buffer": "",
    "save_replay": ""
  },
  "screenshot_source": "",
  "screenshot_dir": "~/Pictures",
  "mic_name": "Microphone",
  "mic_volume": 0.75,
  "allowed_devices": ["AT Translated Set 2 keyboard"],
  "hotkey_combos": [
    {
      "name": "record_and_set_mic",
      "keys": ["ctrl", "shift", "r"],
      "actions": ["toggle_recording", "set_mic_volume"]
    },
    {
      "name": "to_gaming",
      "key": "f13",
      "actions": [{"action": "switch_scene", "scene": "Gaming"}]
    },
    {
      "name": "push_to_mute",
      "key": "ctrl + space",
      "actions": ["toggle_mute_mic"],
      "release_actions": ["toggle_mute_mic"]
    }
  ]
}
```

A combo entry may use either `"key": "ctrl + f1"` or `"keys": ["ctrl", "shift", "f1"]`. Actions run in the order listed.

### Key Combos

Key combos are written as physical keys separated by `+`:

```json
"ctrl + shift + f1"
```

Generic modifier names match either left or right key:

- `ctrl` / `control`
- `shift`
- `alt` / `option`
- `super` / `command` / `win`
- `meta`

Left/right-specific names are also supported, for example `left ctrl` or `right shift`.

### Action Combos

`hotkey_combos` lets one chord run multiple actions. This is useful for workflows OBS does not support natively, such as starting recording and setting your mic volume in the same gesture.

```json
{
  "name": "record_and_set_mic",
  "key": "ctrl + f1",
  "actions": ["toggle_recording", "set_mic_volume"]
}
```

Action combos are best thought of as one gesture that triggers an ordered OBS request sequence. They are not atomic transactions: if one OBS request fails, obs-hotkey logs the failure and continues with the remaining actions.

#### Recommended Combos

These are the safest high-value workflows for this tool:

- **Record + set mic volume**: `toggle_recording` + `set_mic_volume` for a consistent recording preset.
- **Stream + set mic volume**: `toggle_streaming` + `set_mic_volume` when going live.
- **Replay + screenshot**: `save_replay` + `screenshot` to capture a moment and save the replay buffer together.
- **Mute + volume preset**: `toggle_mute_mic` + `set_mic_volume` when you want to unmute at a known level.

#### Combos to Avoid

Avoid combining stateful toggles that can fight each other or depend on OBS state that obs-hotkey does not track:

- `toggle_recording` + `toggle_pause` can pause/resume at awkward times.
- `toggle_streaming` + `toggle_recording` is usable, but both are toggles, so the result depends on current OBS state.
- Multiple `set_mic_volume` actions in one combo are redundant; use one `mic_volume` preset.
- Studio mode, scene switching, and media controls are possible through OBS WebSocket, but they need more state tracking before they are good combo candidates for this lightweight daemon.

### Delayed Actions

`hotkey_combos` can schedule each action with its own delay using the optional `action_delays_ms` field. The array length must match `actions`; each entry is the milliseconds to wait *before* running that action. A value of `0` runs immediately.

```json
{
  "name": "start_recording_after_3s",
  "key": "ctrl + f1",
  "actions": ["toggle_recording", "set_mic_volume", "screenshot"],
  "action_delays_ms": [0, 3000, 6000]
}
```

In the example above, pressing `ctrl + f1` will:

1. Toggle recording immediately.
2. After 3 seconds, set the mic volume to the configured `mic_volume`.
3. After another 3 seconds, save a screenshot.

Delays only apply to the combo that triggered them. Other hotkeys remain responsive. The maximum delay per action is 10 minutes (600,000 ms); values above that are rejected at config load.

If `action_delays_ms` is omitted, the combo runs all its actions immediately, like before. A combo with a single delayed step is the easiest way to get a “start record after a countdown” workflow:

```json
{
  "name": "record_in_5",
  "key": "ctrl + shift + r",
  "actions": ["toggle_recording"],
  "action_delays_ms": [5000]
}
```

This is one gesture that gives you a 5-second countdown before recording actually starts.

### Push-to-Release Actions (Push-to-Record / Push-to-Talk)

`hotkey_combos` can declare an optional `release_actions` list. The actions in `actions` run on press; the actions in `release_actions` run when the chord is released. This is the professional pattern for transient controls:

```json
{
  "name": "push_to_mute",
  "key": "ctrl + space",
  "actions": ["toggle_mute_mic"],
  "release_actions": ["toggle_mute_mic"]
}
```

The example above mutes the mic while `ctrl + space` is held, and unmutes it on release — the classic push-to-talk pattern. `release_action_delays_ms` works the same way as `action_delays_ms` for the release side.

Push-to-release is best used with idempotent toggle-style actions. If OBS state is not what you expect, both the press and the release will be toggles, so the result depends on the current state. For deterministic start/stop, prefer the `toggle_*` action together with OBS's own state.

### Scene Switching

The `switch_scene` action calls OBS WebSocket `SetCurrentProgramScene` and accepts the scene name via the parameter object form:

```json
{
  "name": "to_gaming",
  "key": "f13",
  "actions": [{"action": "switch_scene", "scene": "Gaming"}]
}
```

Use this to map extra function keys (`F13`–`F24`) or your macro pad to the scenes you switch between most. Config validation rejects `switch_scene` without a scene name so misconfigurations fail at startup.

### Keyboard Allowlist

`allowed_devices` restricts which `/dev/input/event*` devices obs-hotkey monitors. The default is an empty list, which means “monitor every detected keyboard.” In setups with multiple keyboards (laptop + dock + stream deck + guest USB + drawing tablet), restrict hotkey capture to a specific device so guests cannot accidentally start your stream:

```json
{
  "allowed_devices": ["AT Translated Set 2 keyboard", "Stream Deck XL"]
}
```

The names are the kernel-assigned device names reported by evdev. To find yours, run `cat /sys/class/input/event*/device/name` or read the daemon logs after `obs-hotkey daemon` enumerates them.

### One-shot Action CLI

`obs-hotkey action <name>` connects to OBS once, runs a single action, and exits. It does not start the event loop or watch any keyboards.

```bash
obs-hotkey action toggle_recording
obs-hotkey action switch_scene --scene "Gaming"
```

This is useful for systemd timers, shell scripts, and integrations where the daemon would be overkill:

```ini
# ~/.config/systemd/user/auto-record.timer
[Unit]
Description=Auto-start recording at 20:00

[Timer]
OnCalendar=*-*-* 20:00:00
Persistent=true

[Install]
WantedBy=default.target
```

```ini
# ~/.config/systemd/user/auto-record.service
[Service]
Type=oneshot
ExecStart=%h/.cargo/bin/obs-hotkey action toggle_recording
```

### Supported Keys

- Function keys: `f1` – `f24`
- Special keys: `scroll lock`, `pause`, `home`, `end`, `page up`, `page down`, `insert`, `delete`
- Modifiers: `ctrl`, `shift`, `alt`, `super`, `meta`, plus left/right-specific variants

### Available Actions

| Action | OBS Request | Notes |
|--------|-------------|-------|
| `toggle_recording` | `ToggleRecord` | Start/stop recording |
| `toggle_pause` | `ToggleRecordPause` | Pause/resume recording |
| `toggle_streaming` | `ToggleStream` | Start/stop streaming |
| `screenshot` | `SaveSourceScreenshot` | Saves PNG to `screenshot_dir` |
| `toggle_mute_mic` | `ToggleInputMute` | Requires `mic_name` in config |
| `set_mic_volume` | `SetInputVolume` | Requires `mic_name`; `mic_volume` defaults to `1.0` if omitted |
| `toggle_studio_mode` | `SetStudioModeEnabled` | Toggles studio mode |
| `toggle_replay_buffer` | `ToggleReplayBuffer` | Requires replay buffer enabled |
| `save_replay` | `SaveReplayBuffer` | Saves current replay buffer |
| `switch_scene` | `SetCurrentProgramScene` | Requires a `scene` parameter in the action object form |

---

## Professional Workflows

The following configurations cover the workflows a professional streamer, broadcast operator, or live-events engineer is most likely to want.

### Stream Deck / Function-Key Scene Switching

Map `F13`–`F18` to the scenes you actually use. Because the keys are out of the way, this works on a laptop or a minimal keyboard.

```json
{
  "hotkey_combos": [
    {"name": "scene_gaming",   "key": "f13", "actions": [{"action": "switch_scene", "scene": "Gaming"}]},
    {"name": "scene_brb",      "key": "f14", "actions": [{"action": "switch_scene", "scene": "BRB"}]},
    {"name": "scene_chatting", "key": "f15", "actions": [{"action": "switch_scene", "scene": "Just Chatting"}]},
    {"name": "scene_starting", "key": "f16", "actions": [{"action": "switch_scene", "scene": "Starting Soon"}]},
    {"name": "scene_ending",   "key": "f17", "actions": [{"action": "switch_scene", "scene": "Stream Ending"}]}
  ]
}
```

### Push-to-Mute Mic

```json
{
  "hotkey_combos": [
    {
      "name": "push_to_mute",
      "key": "ctrl + space",
      "actions": ["toggle_mute_mic"],
      "release_actions": ["toggle_mute_mic"]
    }
  ]
}
```

Combine with `set_mic_volume` to enforce a known level every time you release:

```json
{
  "hotkey_combos": [
    {
      "name": "push_to_talk_with_level",
      "key": "ctrl + space",
      "actions": ["toggle_mute_mic"],
      "release_actions": ["toggle_mute_mic", "set_mic_volume"],
      "release_action_delays_ms": [0, 200]
    }
  ]
}
```

### Live Recording with Volume Preset + Replay + Screenshot

```json
{
  "hotkey_combos": [
    {
      "name": "go_live_record",
      "key": "ctrl + shift + r",
      "actions": ["set_mic_volume", "toggle_recording", "toggle_streaming", "save_replay"],
      "action_delays_ms": [0, 0, 0, 5000]
    }
  ]
}
```

This sets the mic volume, starts recording, starts streaming, and 5 seconds later saves a replay buffer. Adjust the delays to match your actual scene-transition timing.

### Countdown Recording Start

```json
{
  "hotkey_combos": [
    {
      "name": "record_in_10",
      "key": "ctrl + alt + r",
      "actions": ["toggle_recording"],
      "action_delays_ms": [10000]
    }
  ]
}
```

A 10-second countdown before recording actually starts. Use this to give yourself a verbal “starting in 3, 2, 1” runway without having to time it manually.

### Multi-Keyboard Show

When your machine has the laptop keyboard, a dock keyboard, a Stream Deck, a guest USB keyboard, and a drawing tablet:

```json
{
  "allowed_devices": ["AT Translated Set 2 keyboard", "Elgato Stream Deck XL"]
}
```

Only those two devices will be able to trigger hotkeys. The guest's USB keyboard and the drawing tablet's keys are ignored.

---

## Roadmap & Non-Goals

What obs-hotkey already does well for a professional operator:

- Reliable OBS WebSocket v5 connection with auto-reconnect, op-code checking, and DNS-aware connect timeouts.
- 50 ms debounce + chord tracking so evdev autorepeat never double-fires an action.
- Non-blocking actions: every OBS request runs on a background thread, so the event loop stays responsive even during a slow request.
- Per-action delays up to 10 minutes for countdown workflows.
- Push-to-release semantics for transient controls.
- Device allowlist for multi-keyboard setups.
- One-shot CLI for script and timer integration.
- Panic-safe reader threads so a single bad device cannot take down the daemon.
- Clear, fail-fast config validation.

What is intentionally out of scope:

- **Atomic state machine for OBS**: this daemon does not query the live recording/streaming state on startup, so toggles are best-effort. If you need deterministic start/stop, the right tool is an OBS plugin that subscribes to events and tracks state.
- **Scene transitions, animations, source visibility toggles**: these need richer OBS WebSocket event handling and are not a good fit for a lightweight chord-driven daemon.
- **Media control / studio mode choreography**: use OBS's native hotkeys or an OBS plugin.
- **Native Stream Deck protocol**: the daemon accepts the Stream Deck's keyboard-emulation mode and treat it as a normal keyboard. A native HID integration would add significant complexity for a small win.
- **OBS WebSocket authentication**: this daemon explicitly rejects authenticated OBS WebSocket connections. Set OBS's WebSocket to no-auth, or front it with an auth-aware proxy.
- **TLS (`wss://`)**: not implemented. Rejected at startup with a clear message.
- **Mouse buttons, gamepads, MIDI**: keyboard-only by design. Use a tool like `input-remapper` if you need to remap other input devices to keyboard keys.
- **Multi-OBS orchestration**: this daemon talks to a single OBS instance. A multi-instance setup needs a small supervisor.

---

## Managing the Service

```bash
# Check status
systemctl --user status obs-hotkey.service

# View live logs
journalctl --user -u obs-hotkey.service -f

# Restart (after config changes)
systemctl --user restart obs-hotkey.service

# Stop
systemctl --user stop obs-hotkey.service
```

---

## Building

```bash
# Build release binary
cargo build --release

# Cross-compile for ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

---

## Requirements

- Linux (Wayland or X11)
- OBS Studio 28+ with WebSocket enabled
- Membership in the `input` group

---

## Uninstall

```bash
# Stop service and remove everything
obs-hotkey teardown --purge
```

---

## License

MIT