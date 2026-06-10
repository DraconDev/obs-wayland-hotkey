# What Would Be a Real Value Add for obs-hotkey?

## Short answer

The highest-value add is **not** another way to press a hotkey. obs-hotkey already wins by capturing real keyboard events on Wayland and turning one gesture into multiple OBS actions. The real value add is making obs-hotkey a **reliable local OBS automation bridge**:

1. more useful named OBS actions,
2. a safe custom OBS request escape hatch,
3. feedback-friendly status JSON,
4. macro integration recipes for the reusable macros that are now implemented,
5. discovery helpers for scenes/inputs/sources.

That combination makes obs-hotkey more useful for professional operators and gives Companion, Touch Portal, Home Assistant, MIDI bridges, and shell scripts a stable target without turning obs-hotkey into a full OBS plugin, web UI, or generic hotkey daemon.

## What is genuinely worth building

### 1. A practical action library

**Highest value.**

obs-hotkey currently has a small action set. Similar tools and integrations repeatedly expose these as day-one controls:

- source visibility,
- source mute/unmute/toggle,
- input volume,
- filter visibility/settings,
- media play/pause/stop,
- virtual camera,
- profile switching,
- scene collection switching,
- transition helpers,
- recording/streaming/replay controls beyond the basics.

**Why this is value-add:** it lets one obs-hotkey chord or HTTP button do the same practical OBS tasks that Companion, Stream Deck, MIDItoOBS, and OBS CLIs already support.

**Why obs-hotkey is the right place:** these are still just action primitives. They fit the existing model: "trigger an action" or "run a sequence of actions."

**Risk:** low to medium. The main risk is scope creep if we try to implement every OBS request as a first-class action.

---

### 2. Custom OBS request support

**Highest value.**

Add a small escape hatch:

```json
{
  "action": "obs_request",
  "request": "SetSceneItemRender",
  "data": {
    "sceneName": "Gaming",
    "sceneItemId": 123,
    "sceneItemRender": true
  }
}
```

And the same over HTTP:

```http
POST /obs/request
Content-Type: application/json

{"request":"SetSceneItemRender","data":{}}
```

**Why this is value-add:** Companion has Custom Command, Touch Portal has Custom Request, and CLIs expose broad WebSocket coverage. A custom request escape hatch means obs-hotkey does not need to hard-code every possible OBS feature to remain useful.

**Why obs-hotkey is the right place:** it preserves the lightweight bridge model. obs-hotkey does not become a full GUI or plugin; it just forwards a vetted request safely.

**Risk:** medium. We need validation, clear errors, and documentation so users do not treat this as a full scripting engine.

---

### 3. Feedback-friendly status JSON

**Highest value.**

Companion is not just about actions. It is about feedbacks: button colors, variables, recording status, scene active/preview state, source visibility, audio meters, disk space, and media status.

obs-hotkey should make `GET /status` useful as a feedback source:

```json
{
  "ok": true,
  "recording": {
    "active": true,
    "paused": false,
    "timecode": "00:12:34"
  },
  "streaming": {
    "active": false
  },
  "replay": {
    "active": false
  },
  "scene": {
    "current": "Gaming",
    "preview": null
  },
  "inputs": [
    {
      "name": "Mic",
      "muted": false,
      "volume": 0.8
    }
  ]
}
```

**Why this is value-add:** it makes obs-hotkey a better bridge for Companion, Touch Portal, Home Assistant, and MIDI/Stream Deck tools.

**Why obs-hotkey is the right place:** status is already part of Tier 1. Turning it into stable JSON is a natural extension.

**Risk:** low to medium. The risk is over-promising event freshness; without event subscription, this is polling/status, not a full state machine.

---

### 4. Reusable macros / named sequences

**Implemented.**

obs-hotkey now supports named macros that can be invoked from hotkey combos, `obs-hotkey action <name>`, and the HTTP listener. This makes the HTTP bridge cleaner because a Companion button or Touch Portal action can call one macro instead of duplicating a long action list.

The implemented shape is intentionally simple:

```json
{
  "macros": [
    {
      "name": "start_gaming",
      "actions": [
        {"action": "switch_scene", "scene": "Gaming"},
        {"action": "set_mic_volume"},
        {"action": "start_recording"}
      ]
    }
  ]
}
```

**Why this was value-add:** it reduces config duplication and makes obs-hotkey easier to use as a shared automation target.

**Why obs-hotkey was the right place:** macros are still just action sequences. That is exactly the product's core.

**Remaining risk:** keep macros simple. Conditional macros are still not a good fit because they drift toward event-driven automation.

---

### 5. Discovery helpers

**Medium value.**

Add commands like:

```bash
obs-hotkey list scenes
obs-hotkey list inputs
obs-hotkey list sources
obs-hotkey list scene-items --scene Gaming
```

**Why this is value-add:** it reduces guesswork when writing config. CLIs already do this, and it makes integration setup easier.

**Why obs-hotkey is the right place:** discovery helpers are still lightweight and CLI-shaped.

**Risk:** low. The main risk is making the CLI too broad; keep these commands narrow and useful.

---

## What is useful but lower priority

### 1. Conditional macros

Example:

> "Start recording only if not already recording."

This is useful, but it starts to look like automation logic. I would only build this after status JSON and custom requests are solid.

### 2. Bidirectional device feedback

Examples:

- MIDI LEDs showing recording/streaming state
- Stream Deck icons reflecting scene state

This is valuable, but device-specific. obs-hotkey should expose the status JSON first; let bridges handle device feedback.

### 3. A Companion module wrapping obs-hotkey

This could be nice, but it adds packaging and maintenance. Start with generic HTTP.

### 4. A GUI/config generator

Useful for some users, but it risks breaking the single-binary/CLI-first philosophy.

### 5. OBS WebSocket authentication support

Worth considering, but it changes the current security posture. Treat it as a deliberate product decision, not a silent compatibility shim.

---

## What is not worth building

These are not good value-adds for obs-hotkey:

1. **Do not become Advanced Scene Switcher.**
   - That means event-driven automation, conditions, scripts, timers, and OBS-native state management. That is a different product.

2. **Do not become a native Stream Deck plugin.**
   - Too much packaging and platform maintenance. The lightweight path is HTTP/generic HTTP.

3. **Do not become a generic hotkey daemon.**
   - swhkd/sxhkd/input-remapper already own that space. obs-hotkey should stay OBS-specific.

4. **Do not speak every MIDI/HID protocol.**
   - MIDItoOBS and Stream Deck plugins already own this. obs-hotkey can expose a bridge, not every device protocol.

5. **Do not build a web UI.**
   - That adds auth, sessions, CSRF, and frontend maintenance. The HTTP API is enough.

---

## Best next build order

If we want the biggest bang for the buck, build in this order:

1. **Custom OBS request + status JSON**
   - This unlocks the most value with the least scope creep.
2. **Expand the named action library**
   - Focus on the common actions people actually use in Companion and CLIs.
3. **Document macro integration recipes**
   - Companion, Touch Portal, Home Assistant, and MIDI bridge examples should now target the implemented macro endpoints.
4. **Add discovery helpers**
   - This reduces config friction.
5. **Write integration recipes**
   - Companion, Touch Portal, Home Assistant, and MIDI bridge examples.

## Bottom line

The real value-add is to make obs-hotkey the **lightweight local bridge** between keyboard gestures and OBS automation, not to turn it into a full OBS plugin or a generic control surface.

If I had to pick one thing to build first, I would pick:

> **custom OBS request + feedback-friendly status JSON**

That gives the biggest practical lift while staying closest to obs-hotkey's existing strengths.
