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
    "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAyOC9aWVdZMnRBaGtTSUgxUmpTZXlDcGxKcENyT0RCUzVMdGxqb1dOTGtBClpPeThQendqbURyZ0hPRGFrZTZRd1BKc1BDaW9LcTkvczU5UWliOUVWMUkKLT4gWDI1NTE5IEhrMlM2V0FHeEh2aU41L3FnS2hJR1NsWC9PYVJsbGEvTUR5bHhuNVFtRGcKKzdsS1QwSXlYNlhRQ1g1MENsdlUrUmhNMWNoSzZ1YVM5TzVxMGZuMzhhawotPiBYMjU1MTkgY0RsV0wxazR6R1lvOUZFWVppQWpROE9WZFFSdzlZbWptUlZKRklJVE9BTQoyeFlsSURHQ1hMSDgzVDlqV3hwUURta241S1JWRFpCc25oV3lnS0sweE5jCi0+IFgyNTUxOSBScDRZc1lYS1p4WVh1akF6aXRwcjQva3k2ZjZ0Z0lYQWNiV0VsQ1lORXhjClg0OXFkQit5N1IzWGxHK1hiYmM2cis3MEhZVGY1NTNZVlFIZTljZEFCZHcKLT4gWDI1NTE5IFJQQTFlOTZ0eFIvSWtWMFNUR1JwMmNjMjlFVUZqN2dtbWEyWSs1RzN6Rm8KRVcrUVM0TzRVaTEyb3pzbVFNeG51ZnJPbjlCN3dtdVdrLzlJaGFCRkdMVQotPiBULnh0ay1ncmVhc2UKSWNVKzJpMUEzREhtZGZ4ZGQyY3VWb0Vkdjk0MC9VWUhhaW9TTWpLaU5hQXh3OVE2QVJGZUdVd1JlZWZFOThEUwp3aklnWjlhMTF1ZkRQV0R2dXFOYU1lMldWeVpqUnlCNkdqb1dZQVpudlRsanNKUVhmQ1ZGNWNENHRYcDkwdwotLS0gWThOVVNJNkJlakY1S3A2WkdQU2tDd1I4Zk5Ed2hpUEZzRmZUNkdnNWQ2TQohr/pRqSIA69wlofEXzgodZvnoJOdaIOYyCRcAV97iv21lRFa6QOS5f9yyMZHkUbwuCPlltO3pJblUP8DY0a+BnbxNVbM+0x04Eg==]
  }
}
```

Companion sends:

```http
POST /actions/switch_scene
Authorization: [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB0Mkh4R1BFa29lczlOeUw3dVpoQlE0VGVwa0FaZFZla3pEWWM2ZnFDUjAwClI1d09wZjk5VzZtd2poQk1FYzFPcGgrZWRtZHkyMlc5TEk1V2NaTEpiTG8KLT4gWDI1NTE5IFAvbHpuYmtLVnVpWTZGVXNSRG52QkZZYmdDdnluMEF2cmFKR3FzK3dmQW8KYnp1TDAxWEhwdUoySTV1TEdxWVFLZGtBUzN1Y0hPV3VYc0I2d0pjQ2RFUQotPiBYMjU1MTkgdk1nTnhxaFRJck93ZEZhYVlOeVZDSXdtMDh2bis0b1IzWEw5eXdpWHhrOAplaExBLzZqWEswZ2xmT29PS2pFRStJM1ZPcERSMFoyZ3RVN3pKWFlFVlRRCi0+IFgyNTUxOSBLRDR0cG5kVWtTQWlZakNReHdSbXZMQVRTMGdleTQ2ZnF0QmNqSGlVV3lzCnJPUU5tLzlWRHNmaVhxNmlRT1BFN2JUdkVuTmx6YnpVMGg4VXFqZHJXVjQKLT4gWDI1NTE5IGZrcFJPVjl4TEhZWGhaUnNwSCtYMWxPQnFzSjdTUXMvU0tIM0YyaEpOSGcKUFNpVFI5aWN3czhVRUphVG1yZ1I4YzBGbVNXT2VVWFZsZGtkYUtCak5aYwotPiBmJj9oY11FLWdyZWFzZSArekAyTiAoJ05BRjFuKiBoMH0+KiBpfQprVm84NjFJNmZMaVRIc1J2UmplTFRKRTZUdwotLS0gN1JXTUNxRUkxb1ZQb25oc3RVQVZJUGh2Y2xtWHV6OUIvQzhCaEVCdWJ3awo2r0KXBNnCvXCMrLW7qDyT5J4kdNU3QOm+rEPAfpN/WFIOr/syWh84FHk3INzWRZK3orSREuMpUh37bhdJ2k3OQEtFygNZWw==]
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
  "version": "1.0.59"
}
```

### `GET /status`

Auth required when a token is configured. Returns a feedback-friendly OBS status envelope for controllers and scripts. The legacy `status` field is retained for compatibility; the new `obs` field uses stable nested objects.

When OBS is reachable:

```json
{
  "ok": true,
  "service": "obs-hotkey",
  "obs": {
    "reachable": true,
    "recording": {"active": true, "paused": false, "timecode": "00:12:34"},
    "streaming": {"active": false, "timecode": null},
    "replay_buffer": {"active": true},
    "current_scene": "Live",
    "input": {"name": "Mic", "muted": false, "volume_mul": 1.0}
  },
  "status": {
    "stream_active": false,
    "stream_timecode": null,
    "record_active": true,
    "record_paused": false,
    "record_timecode": "00:12:34",
    "replay_active": true,
    "current_scene": "Live",
    "input_muted": false,
    "input_volume_mul": 1.0
  }
}
```

When OBS is unreachable, the HTTP server remains healthy and returns `200 OK` with `obs.reachable = false`:

```json
{
  "ok": true,
  "service": "obs-hotkey",
  "obs": {
    "reachable": false,
    "error": "GetStreamStatus: Connection refused (os error 111)",
    "recording": {"active": false, "paused": false, "timecode": null},
    "streaming": {"active": false, "timecode": null},
    "replay_buffer": {"active": false},
    "current_scene": null,
    "input": {"name": "Mic", "muted": null, "volume_mul": null}
  },
  "status": {"unavailable": true, "error": "GetStreamStatus: Connection refused (os error 111)"}
}
```

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
