# Reusable Macros Design

## Product decision

Reusable macros make obs-hotkey more useful without changing its core product: a lightweight Rust daemon that turns keyboard gestures into ordered OBS WebSocket actions.

A macro is a named action sequence. It can be invoked from:

- a `hotkey_combos` action,
- `obs-hotkey action <macro-name>`,
- the HTTP listener via `POST /macros/<name>` or `POST /macros`.

This gives professional operators a way to reuse workflows such as “switch to intro scene, wait 10 seconds, start recording” across hotkeys, timers, Companion, Touch Portal, Home Assistant, or MIDI bridges.

## Config shape

Macros live in a new top-level `macros` array:

```json
{
  "macros": [
    {
      "name": "countdown_record",
      "actions": [
        {"action": "switch_scene", "scene": "Intro"},
        {"action": "start_recording"}
      ],
      "action_delays_ms": [0, 10000]
    }
  ]
}
```

Rules:

- `name` must be non-empty and unique.
- `actions` must contain at least one action or macro reference.
- `action_delays_ms` is optional. If present, its length must match `actions`.
- Each delay is capped at 10 minutes, matching hotkey combo delays.
- A macro may reference another macro.
- Recursive macro cycles are rejected at config load and runtime.

## Execution semantics

Macros run synchronously in the caller’s action thread:

1. For each action index, sleep for that action’s delay.
2. If the action is another macro, run that macro.
3. Otherwise run the OBS action runner.

For hotkey combos, the macro runs in the same background trigger thread as a normal combo. For `obs-hotkey action`, the macro runs before the command exits. For HTTP, the request waits until the macro finishes.

This intentionally keeps macros simple. They are not a full rule engine, event subscription system, or OBS state machine.

## Professional value

The most useful macro pattern is deterministic start/stop:

```json
{
  "macros": [
    {
      "name": "go_live",
      "actions": [
        {"action": "switch_scene", "scene": "Starting Soon"},
        {"action": "start_streaming"},
        {"action": "start_recording"}
      ]
    },
    {
      "name": "end_show",
      "actions": [
        {"action": "stop_recording"},
        {"action": "stop_streaming"},
        {"action": "switch_scene", "scene": "BRB"}
      ]
    }
  ]
}
```

Prefer `start_recording` / `stop_recording` and `start_streaming` / `stop_streaming` in macros when you want deterministic behavior. The older `toggle_recording` and `toggle_streaming` actions remain available for simple toggles.

## HTTP API

With HTTP enabled:

```json
{
  "http": {
    "enabled": true,
    "bind": "127.0.0.1:7999"
  }
}
```

Run a macro with either form:

```bash
curl -X POST http://127.0.0.1:7999/macros/countdown_record

curl -X POST http://127.0.0.1:7999/macros \
  -H 'Content-Type: application/json' \
  -d '{"macro":"countdown_record"}'
```

Successful macro execution returns 200. Unknown macros, recursive references, and invalid requests return 400. As with hotkey actions, OBS WebSocket request failures are logged by the action runner rather than converted into HTTP errors.

## Safety boundaries

- Macros do not add conditionals or event subscriptions.
- Macros do not make OBS requests atomic.
- Macros do not replace Companion, Advanced Scene Switcher, or a native Stream Deck plugin.
- Macros preserve existing hotkey combo behavior; old configs without `macros` continue to load unchanged.
