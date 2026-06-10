# Tier 1 Professional Observability Design

## Product decision

Tier 1 closes the biggest professional gap in obs-hotkey: the tool can trigger OBS actions, but it does not yet give operators enough feedback to trust it during a live show. Tier 1 adds observability and safe external integration without turning obs-hotkey into a full OBS control surface or web UI.

Tier 1 is four features:

1. **Richer status output** — `obs-hotkey status` reports service/config/input/OBS connectivity plus OBS state when OBS WebSocket is reachable.
2. **Doctor diagnostic flow** — `obs-hotkey doctor` runs a startup checklist and explains exactly what is wrong and how to fix it.
3. **Desktop notifications** — optional best-effort desktop toasts when hotkeys or one-shot actions fire.
4. **Localhost HTTP listener** — optional HTTP API for Bitfocus Companion, Touch Portal, Home Assistant, and shell scripts.

These features are intentionally lightweight. The daemon remains a Rust binary with minimal dependencies. The HTTP listener is localhost-only by default and supports an optional bearer token. Notifications are implemented by spawning a configurable command, defaulting to `notify-send` when enabled.

## User workflows

### Pre-show check

A professional operator runs:

```bash
obs-hotkey doctor
```

Expected result:

- config exists and parses
- required fields are present for configured actions
- user is in the `input` group
- configured keyboard devices exist and are readable
- OBS WebSocket is reachable
- OBS WebSocket authentication/TLS requirements are compatible
- hotkey/chord parsing succeeds
- HTTP listener configuration is safe

If something is wrong, `doctor` prints the exact failing check and a fix hint.

### Status at a glance

A professional operator runs:

```bash
obs-hotkey status
```

Expected result:

- systemd autostart state
- config path and config directory
- input group membership
- OBS WebSocket reachability
- OBS recording/streaming/replay state when OBS is reachable
- current program scene when OBS is reachable
- configured mic mute/volume when `mic_name` is set
- daemon active state

If OBS is not running, the command still reports local checks and marks OBS state as unavailable.

### Action confirmation

With notifications enabled, pressing `Ctrl+Shift+R` shows a desktop toast:

```text
obs-hotkey
Triggered Ctrl+Shift+R → Toggle Recording + Set Mic Volume
```

The notification is best-effort. If `notify-send` is missing, the action still succeeds and the failure is logged.

### Companion / Touch Portal integration

With HTTP enabled:

```json
{
  "http": {
    "enabled": true,
    "bind": "127.0.0.1:7999",
    "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBianhCNDlIV2N2UlByMFY2WHd0djkrYUdoWGFiKzJieWwvbVFuS0NKbTNVCkFlZmlQNlBsS1FMY2Iyd2U3T1BUakMwZHNGUHg3SHVkdC9mMFRaUDFDQk0KLT4gWDI1NTE5IGoyOUUvb21tWUkyVnVzUDJMKy9OVnZvKzZ4L0pZVS92dXlJK3lGanBTUm8KS05Pcncxb2x6UnR2RWhIKzhvRFZuMC8rbVBqYUR0UVhlN0RHamI4TElPawotPiBYMjU1MTkgQlY5QWIwRWt5ZURKVStPdFZhSmZEV2x2eFBzV1lBeW0zNUpjV09DYW9CawpwVXhHSmxzeVdrS3Z2dzQ3dzhVbnZTV0Z5OWJmM0dGWlJtLzVoUXA1OStBCi0+IFgyNTUxOSB2a3E3K2h4L0ZXSFVPa3kySmlFb2lDZlFTZzdZYk1aeVp2WkswanFHMVdRClhtRzBhSElueWNQNVJqTDVHTG4yWEpyUkRkMmV1cDlUMGU4VVY0ZjkvRkEKLT4gWDI1NTE5IDVKRlBsTkdEdWdud0pNQWNJREx1T3ZHS0JFZE1yVnlObFFiVE4xanpDeVEKQ0pLUE8waFdudUlObGdHN3grL2ZsK0lxTDFVbE9FUlNINUJ3RkFrdzRvcwotPiBlQk9ObC1ncmVhc2UgL3IgI0o+bHQgbHxiKwo5WFdFSUxSRS9kWHdQeEd3Y291ZWNTYS9JeTR2WE4vMWVNdlhXZlV1ZUlnN1JEaThBNkxmaGNsVEpYN2h4MDZ2Cm5SNVl5TFEKLS0tIGE5UFpPV0YxS2lDRjg4L3FqVTJyaWtxZXh4azZvWnFMdllzTG9lZjN4MGcKh9M7aznN8IjP2bSVDx4TegcnkVfkD+e1Lefs3889vgjR8YCtxbfrzJNeQUH0tWpnvOonzkWwh1e6MK9fuqIQfKrrKxXRFN/yVvk=]
  }
}
```

Companion sends:

```http
POST /actions/switch_scene
Authorization: [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBQazhmdUJMNXVZazM4V2FtMElGb2dTOTVoNjVOOVBSeU5ORmZDTW9CcFFFCmxCekhtcDF0dTB0ZXBzc01aMm0xaG12VDJ0NjFJZmc5akR2RGdVbjZ2UXcKLT4gWDI1NTE5IHA4bllHN2pXZXlkR1NoYW52Wk03bUZrWWdJMVFFZ0ozbnlXNHZWeXl2VUEKd0lybzVudmJETXVxcHF1eG11OURsS0pIdGJyeEtQNEVlQnlqSEJ6MkhMcwotPiBYMjU1MTkgVjNnOGF3ZEc5VmNyaHlFVHFXWWgwTWVkNTFMTHlUakVuK05qTDI3M21GawpnN09VSEd1Q2tpaVpqOVIvQncyL09vRk8vTkp0eGg2bDZuRDluenhaKzV3Ci0+IFgyNTUxOSBjU0RtUEM5a0h1SHJSdGdtOUtTY01UbmNOdHBsRE1MaFc4RHdrNG9YdHhvCnJkbUx5WElLMmNWSDFCNkpBS0dVaitmZE5JZS9OamY1VXRYV2dYVGwvWWcKLT4gWDI1NTE5IFFWYURWVUxpVGw0YVE4OFNROGgxRkp0Z1BRRmREMHJzdjkyTzFqSUdUelUKRUVRMEdNakRGZmViamNxYW1mUW11YnNOWndFMTgzbjgvSTFCN2QwY1BCOAotPiBTQlQtZ3JlYXNlICVmZ0QoY18gTCgzL0dzIE14UjAKeGFkaVVZMXovNWVPM3FzMlBkOWNMYXZmUEpGYzhRci9PY3REMUdTSVpDaWdlUU5qcHo5a29CWFQ0V2JzU3Q0ZApaS0VxNVk4VGdpTC92ZjFSUkpxMEk5cTJzQ0JQbzRzCi0tLSBjWmk5cG90UERvazNJbHQyb2JyNE02VEFBTFpzTzBReVlEakFvSWVaTkxzCgj6WkD2/8enMUbWpm/nkdFd0iljpEGNxF6zNUlVNE9a1n51+KThd/sPFLhLW0wTEKNxo1O4HLtvdJn+2rDcHyFH2fUP+pS1]
Content-Type: application/json

{"scene": "Gaming"}
```

obs-hotkey responds with JSON:

```json
{"ok": true, "message": "action switch_scene triggered"}
```

The API does not expose the keyboard event loop. It triggers configured OBS actions or named macros and returns health/status data.

## Configuration shape

Add these top-level config fields. All are optional and default to disabled/off, preserving existing behavior.

```json
{
  "notify": {
    "enabled": false,
    "command": ["notify-send", "obs-hotkey", "{message}"]
  },
  "http": {
    "enabled": false,
    "bind": "127.0.0.1:7999",
    "token": null
  }
}
```

### `notify`

- `enabled`: boolean. Default `false`.
- `command`: string array. Default `["notify-send", "obs-hotkey", "{message}"]`.
- `{message}` is replaced with the human-readable action label.
- Notification failures are logged and never block OBS actions.

### `http`

- `enabled`: boolean. Default `false`.
- `bind`: string. Default `127.0.0.1:7999`.
- `token`: optional string. Recommended whenever the API is exposed outside the current user session or when used by external tools.

Validation rules:

- If `http.enabled` is true and `bind` is not loopback (`127.0.0.1`, `localhost`, `::1`), a token is required.
- If `token` is empty or whitespace-only, treat it as absent.
- `bind` must parse as an address and port.
- `command` must contain at least one element.

## API shape

### `GET /health`

No auth required. Returns:

```json
{
  "ok": true,
  "service": "obs-hotkey",
  "version": "1.0.58"
}
```

### `GET /status`

Auth required when a token is configured. Returns the same rich status data as `obs-hotkey status`, where available. If OBS is unreachable, OBS fields are marked unavailable and the response remains `200 OK` because the HTTP server itself is healthy.

### `POST /actions`

Auth required when a token is configured. Body:

```json
{
  "action": "switch_scene",
  "scene": "Gaming"
}
```

or:

```json
{
  "action": "toggle_recording"
}
```

Response:

```json
{"ok": true, "message": "action toggle_recording triggered"}
```

or:

```json
{"ok": false, "error": "unknown action 'foo'"}
```

### `POST /actions/{name}`

Auth required when a token is configured. Query/body parameters are optional action parameters. For now, only `switch_scene` uses parameters:

```http
POST /actions/switch_scene?scene=Gaming
```

or JSON body:

```json
{"scene": "Gaming"}
```

Unsupported parameters are rejected with `400 Bad Request`.

### `POST /macros`

Auth required when a token is configured. Body:

```json
{"macro": "countdown_record"}
```

Response:

```json
{"ok": true, "message": "macro countdown_record triggered"}
```

or:

```json
{"ok": false, "error": "macro 'missing' not found"}
```

### `POST /macros/{name}`

Auth required when a token is configured. Runs a named macro without a body.

```http
POST /macros/countdown_record
```

## Failure modes

### OBS WebSocket unavailable

- `status` still succeeds and marks OBS state as unavailable.
- `doctor` fails that check and prints a fix hint.
- `action` and HTTP action calls fail for unknown actions/macros, missing parameters, and invalid config. OBS WebSocket request failures are logged by the action runner and are not converted into HTTP errors.
- Existing hotkeys remain responsive and will retry OBS on the next action trigger.

### Notification command missing

- Log warning.
- Do not fail the action.
- Do not block the hotkey event loop.

### HTTP listener port already in use

- Log error.
- Daemon continues running.
- Hotkeys continue working.
- `doctor` reports HTTP listener unavailable.

### Invalid HTTP auth token

- Return `401 Unauthorized`.
- Do not log the token value.
- Log only that authentication failed.

### Invalid HTTP request

- Return `400 Bad Request` with a short error.
- Do not execute any action.

### Non-loopback bind without token

- Reject config validation with a clear error.
- Do not start the daemon.

## Security posture

Tier 1 intentionally avoids broad network exposure.

- HTTP listener defaults to `127.0.0.1:7999`.
- Non-loopback bind requires a token.
- Bearer tokens are compared with constant-time-ish string comparison where feasible; at minimum, never log tokens.
- The HTTP API exposes only action triggers, macro triggers, and status. It does not expose arbitrary WebSocket requests.
- The HTTP API does not bypass config validation. Unknown actions are rejected.
- Notifications spawn a user-configured command. The default is `notify-send`; users who customize it accept that command execution responsibility.

## Non-goals

Tier 1 does not add:

- A web UI for editing hotkeys.
- Native Stream Deck protocol support.
- MIDI, gamepad, or mouse-button capture.
- Arbitrary OBS WebSocket request execution from HTTP.
- OBS state subscriptions or state-aware action gating.
- TLS for the HTTP listener.
- Multi-user auth.
- Remote control over the network without an explicit token.

## Implementation plan

1. Add config structs and validation for `notify` and `http`.
2. Add OBS status helpers in `src/obs.rs` for recording, streaming, replay buffer, current program scene, and input mute/volume.
3. Replace/augment `service::run_status` with richer local and OBS state output.
4. Add `service::run_doctor` and `Commands::Doctor`.
5. Add notification runner in `src/main.rs` and call it from hotkey triggers, release triggers, and one-shot actions.
6. Add a minimal `src/http_api.rs` module for localhost HTTP health/status/action endpoints.
7. Spawn the HTTP listener in daemon mode when `http.enabled` is true.
8. Add tests for config parsing, request parsing, auth, status formatting, and CLI parsing.
9. Update README and CHANGELOG.
