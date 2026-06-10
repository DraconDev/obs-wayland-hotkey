# Similar Programs & Extension Plan

## Research method

This document compares obs-hotkey against adjacent OBS control tools, automation plugins, stream-deck / MIDI integrations, and Wayland/X11 hotkey daemons. The goal is not to copy them wholesale; it is to identify what they ship that operators already expect, then decide which pieces fit obs-hotkey's product position:

> obs-hotkey is a lightweight Rust daemon that captures real keyboard events on Wayland/X11 and turns one gesture into one or more OBS WebSocket actions.

Evidence sources used in this pass:

- `pschmitt/obs-cli` README — Python OBS WebSocket v5 CLI using `obsws-python`; scene/item/group/source/input/filter/hotkey/virtualcam/stream/recording/replay commands.
- `muesli/obs-cli` README — Go OBS CLI using the older OBS WebSocket plugin; stream/recording/scene/studio/profile/replay/virtualcam commands plus label countdown.
- `onyx-and-iris/gobs-cli` README — Go OBS WebSocket v5 CLI with host/port/password/timeout/env config, broad scene item transform, input volume, recording chapter/split, stream/replay/virtualcam/studio/profile/media/text commands.
- `onyx-and-iris/obsws-cli` README — Python OBS WebSocket v5 CLI with similar breadth and env config.
- `WarmUpTill/SceneSwitcher` README — OBS automation plugin for Windows/macOS/Linux with runtime conditions/actions and scripting.
- Bitfocus Companion OBS module README/HELP — WebSocket OBS module with actions, feedbacks, variables, custom commands, vendor events, audio meters, disk-space feedback, sequential execution.
- Touch Portal OBS tutorials — OBS control through the OBS WebSocket plugin and Custom Request actions.
- `elgatosf/streamdeck-obs-plugin2` README/source tree — Stream Deck OBS Studio plugin with local JSON-RPC-style handlers for frontend/scene/source/system actions.
- `lebaston100/MIDItoOBS` README — archived MIDI controller mapper for OBS; button/fader mappings, macros by assigning multiple actions to one button, and experimental bidirectional feedback.
- `waycrate/swhkd` README — Rust Wayland/X11 hotkey daemon, sxhkd-compatible config, server/client security split, signal-based pause/resume/reload.
- `baskerville/sxhkd` README — X11 daemon that reacts to input events by executing commands; concise binding syntax.
- `kdotool`, `wev`, and `input-remapper` public project pages/search results — KDE Wayland automation, Wayland event inspection, and input-device remapping.

## Program categories

### 1. OBS WebSocket CLIs

These tools are the closest functional cousins to `obs-hotkey action`, but they are usually manual/script-driven rather than gesture-driven.

| Program | What it does | What it teaches | Fit with obs-hotkey |
| --- | --- | --- | --- |
| `pschmitt/obs-cli` | Python OBS WebSocket v5 CLI with pretty/JSON/TSV output; scene/item/group/source/input/filter/hotkey/virtualcam/stream/record/replay commands. | Operators expect listing, status, toggle/start/stop, source visibility, input mute/volume, filter visibility, screenshots, and hotkey trigger support. | Complements obs-hotkey. obs-hotkey should expose more of these actions as named config actions and HTTP actions, but keep CLI one-shot mode simple. |
| `muesli/obs-cli` | Go CLI for OBS with stream, recording, scenes, scene items, labels/countdown, studio mode, profiles, replay buffer, virtual camera. | Countdown labels, studio mode, profiles, scene collections, and replay status are common operator workflows. | Good source of action ideas. obs-hotkey already has delayed actions, but could add explicit countdown/status helpers. |
| `gobs-cli` / `obsws-cli` | OBS WebSocket v5 CLIs in Go/Python with env config and broad command coverage: scene item transforms, input volume, record split/chapter, media/text, custom request data. | Mature CLIs surface many OBS WebSocket request types and use env config for repeatable automation. | obs-hotkey should not become a full CLI, but config validation and HTTP should learn from this breadth. |

### 2. OBS automation plugins

| Program | What it does | What it teaches | Fit with obs-hotkey |
| --- | --- | --- | --- |
| Advanced Scene Switcher | OBS plugin automation with conditions/actions, runtime scripting, cross-platform packages. | Operators with complex scene logic expect event-driven conditions, variables, and OBS-native feedback. | Mostly non-goal. obs-hotkey should not become an OBS plugin or full automation engine. It can hand off complex automation to Companion/Advanced Scene Switcher via HTTP/custom requests. |

### 3. Stream Deck / Companion / Touch Portal / MIDI integrations

| Program | What it does | What it teaches | Fit with obs-hotkey |
| --- | --- | --- | --- |
| Bitfocus Companion OBS module | WebSocket OBS module with actions, feedbacks, variables, custom commands, vendor events, audio meters, disk-space feedback, sequential execution. | Professional operators expect buttons that both trigger actions and reflect OBS state. Feedbacks/variables are as important as actions. | obs-hotkey's Tier 1 HTTP listener should become a bridge target for Companion. Add richer status, custom request, and feedback-friendly JSON. |
| Touch Portal OBS tutorials | Uses OBS WebSocket and Custom Request actions to control OBS from a tablet/mobile surface. | Tablet/mobile operators want simple HTTP/JSON request patterns and readable state. | obs-hotkey HTTP API should stay simple enough for generic HTTP buttons. |
| Elgato Stream Deck OBS plugin | Native Stream Deck OBS plugin with handlers for OBS frontend, scene, source, and system actions. | Dedicated hardware wants tight OBS state feedback and scene/source-specific actions. | Native Stream Deck plugin is heavy; obs-hotkey should remain keyboard-emulation/HTTP-first. |
| MIDItoOBS | Archived Python script mapping MIDI buttons/faders to OBS actions; supports macros by assigning multiple actions to one button and experimental bidirectional feedback. | MIDI workflows care about device mapping, fader scaling, and feedback LEDs. | obs-hotkey should not speak MIDI natively, but HTTP can let a MIDI controller bridge trigger obs-hotkey actions. |

### 4. Wayland/X11 hotkey daemons and input mappers

| Program | What it does | What it teaches | Fit with obs-hotkey |
| --- | --- | --- | --- |
| `swhkd` | Rust Wayland/X11 hotkey daemon; sxhkd-compatible config; server/client split; signal-based pause/resume/reload. | Hotkey daemons need simple config, reload semantics, and clear privilege boundaries. | Complements obs-hotkey for general desktop hotkeys. obs-hotkey should stay OBS-specific and multi-action. |
| `sxhkd` | X11 daemon that reacts to input events by executing shell commands with concise binding syntax. | The config syntax is terse and beloved by tiling-window-manager users. | obs-hotkey should not copy sxhkd syntax; it should keep JSON config and add better validation/examples. |
| `input-remapper` | Remaps keyboard/mouse input devices on Linux. | Device-level remapping is separate from app-level automation. | obs-hotkey can rely on input-remapper for remapping and keep its own OBS action semantics. |
| `kdotool` / `wev` | KDE Wayland automation and Wayland event inspection. | Wayland users need tool-specific automation and event debugging. | obs-hotkey's value is not generic input simulation; it is OBS WebSocket control from evdev chords. |

## What similar programs do that obs-hotkey does not yet do

1. **Broad OBS action library** — CLIs and Companion expose source visibility, filter visibility/settings, input volume/fade, media controls, virtual camera, profiles, scene collections, transitions, recording split/chapter, and custom OBS requests.
2. **Feedbacks and variables** — Companion maintains button feedbacks, audio meters, disk-space feedback, recording/streaming timecodes, scene active/preview state, source visibility, and media status.
3. **Sequences/macros** — MIDItoOBS allows multiple actions on one button; obs-hotkey has action combos and now has named reusable macros, but still avoids conditional sequences.
4. **Config discovery and ergonomics** — CLIs list scenes/items/sources/inputs/filters; obs-hotkey currently requires the user to know names and edit JSON.
5. **Integration surfaces** — Companion, Touch Portal, Stream Deck, MIDI, and CLIs all need either direct OBS WebSocket or a simple HTTP/JSON bridge. obs-hotkey now has a Tier 1 HTTP listener, but it is still minimal.
6. **Diagnostics** — obs-hotkey's `doctor` is newer than most general CLIs; keep it as a differentiator.
7. **Event-driven automation** — Advanced Scene Switcher and Companion can react to OBS events and maintain state. obs-hotkey should avoid becoming a full event engine.

## Current obs-hotkey feature surface

As of this repo, obs-hotkey already has:

- Wayland global hotkey capture via evdev, plus X11 support.
- Chord hotkeys and generic modifiers (`ctrl`, `shift`, `alt`, `super`, `meta`).
- `hotkey_combos` for multiple OBS actions from one gesture.
- `action_delays_ms` for delayed/countdown workflows.
- `release_actions` and `release_action_delays_ms` for push-to-release patterns.
- `switch_scene` with a per-combo `scene` parameter.
- `allowed_devices` for multi-keyboard setups.
- `obs-hotkey action <name>` one-shot CLI, including named macro invocation.
- Richer `obs-hotkey status`.
- `obs-hotkey doctor`.
- Desktop notifications.
- Optional localhost HTTP listener for Companion/Touch Portal-style integrations, including named macro endpoints.

## Extension plan

### Must add

These are high-value, low-risk extensions grounded in what similar programs already ship.

1. **Expand the named action library**
   - Add actions for source visibility (`set_source_visibility`, `toggle_source_visibility`), input mute/volume/fade, filter visibility, media play/pause/stop, virtual camera, profile, scene collection, and transition helpers.
   - Justification: pschmitt/obs-cli, gobs-cli/obsws-cli, Companion, MIDItoOBS, and Stream Deck all expose these as common operator actions. obs-hotkey should let a chord or HTTP button trigger them without becoming a full CLI.

2. **Add a small “custom OBS request” action and HTTP endpoint**
   - Config action: `{"action": "obs_request", "request": "SetSceneItemRender", "data": {...}}`.
   - HTTP endpoint: `POST /obs/request` with JSON `{ "request": "...", "data": {} }`.
   - Justification: Companion's Custom Command and Touch Portal Custom Request are high-value escape hatches. This avoids maintaining every OBS request as a first-class obs-hotkey action.

3. **Make status feedback useful for buttons**
   - Extend `GET /status` to include stable booleans and timecodes for recording, streaming, replay, current scene, preview scene if available, studio mode, virtual camera, profile, scene collection, disk space, FPS/CPU/memory, and input mute/volume.
   - Justification: Companion's value is not just actions; it is feedbacks and variables. obs-hotkey's HTTP listener should be Companion-friendly.

4. **Document macro integration recipes**
   - Config example:
     ```json
     {
       "macros": {
         "pre_record": [
           {"action": "switch_scene", "scene": "Intro"},
           {"action": "set_input_volume", "input": "Mic", "volume": 0.8},
           {"action": "start_recording"}
         ]
       }
     }
     ```
   - Justification: obs-hotkey now supports named macros. The next ergonomic win is showing operators how to wire them into Companion generic HTTP, Touch Portal, Home Assistant REST command, and MIDI/Touch Portal bridge patterns.

5. **Add config discovery helpers**
   - Add `obs-hotkey list scenes`, `obs-hotkey list inputs`, `obs-hotkey list sources`, `obs-hotkey list scene-items --scene <name>`.
   - Justification: CLIs already list these objects. obs-hotkey should not become a full CLI, but discovery helpers reduce JSON guesswork and make config examples easier to generate.

6. **Document integration recipes**
   - Add examples for Companion generic HTTP, Touch Portal Custom Request, Home Assistant REST command, and MIDI/Touch Portal bridge patterns using `obs-hotkey` HTTP.
   - Justification: Similar tools compete on ergonomics. obs-hotkey's advantage is a single local bridge between keyboard gestures and OBS.

### Consider

These are valuable but heavier or wider in scope.

1. **Companion module wrapping obs-hotkey**
   - Pros: native buttons, feedbacks, variables, discovery.
   - Cons: Node.js packaging and maintenance burden. Start with generic HTTP first.

2. **Bidirectional device feedback**
   - Examples: MIDI LEDs or Stream Deck icons reflecting recording/streaming/scene state.
   - Pros: Matches MIDItoOBS and Companion feedbacks.
   - Cons: Requires device-specific protocols or a bridge. Keep behind an integration doc or optional plugin.

3. **Conditional macros**
   - Example: “start recording only if not already recording” or “switch to scene A when streaming, scene B when not”.
   - Pros: Bridges the gap toward Advanced Scene Switcher.
   - Cons: Can drift into event-driven automation. Prefer simple status checks first, not a full rule engine.

4. **GUI/config generator**
   - Pros: Makes scene/input discovery and JSON editing easier.
   - Cons: Breaks the single-binary/CLI-first philosophy unless implemented as a separate optional tool.

5. **OBS WebSocket authentication**
   - Pros: Matches modern OBS defaults and Companion.
   - Cons: Current repo explicitly documents no-auth as the supported mode. Treat as a deliberate product/security decision, not a silent compatibility shim.

### Explicit non-goals

These should remain out of scope unless the product direction changes.

1. **Do not become Advanced Scene Switcher**
   - obs-hotkey should not implement a full event-driven OBS automation engine with arbitrary conditions, timers, and scripts.

2. **Do not become a native Stream Deck plugin**
   - The lightweight path is HTTP/generic HTTP. Native plugins add packaging, UI, and platform maintenance.

3. **Do not replace swhkd/sxhkd/input-remapper**
   - obs-hotkey should not become a general-purpose hotkey daemon or input remapper. It should stay OBS-specific.

4. **Do not implement generic MIDI/HID device protocols**
   - MIDItoOBS and Stream Deck plugins own this space. obs-hotkey can expose a bridge, not speak every device protocol.

5. **Do not build a web UI**
   - The Tier 1 HTTP listener is enough for integrations. A web UI would add auth, sessions, CSRF, and frontend maintenance.

## Recommended next implementation order

1. Expand named actions from Companion/CLI patterns: source visibility, input volume/fade, media, virtual camera, profile, scene collection, transition helpers.
2. Add `custom OBS request` action and `/obs/request` endpoint.
3. Enrich `GET /status` into Companion-friendly feedback JSON.
4. Add named macros / reusable sequences.
5. Add `list` discovery helpers.
6. Write integration recipes for Companion, Touch Portal, Home Assistant, and MIDI/Touch Portal bridges.

## Bottom line

obs-hotkey should not chase every feature in Companion, Advanced Scene Switcher, MIDItoOBS, or Stream Deck plugins. Its strongest extension path is:

- keep the daemon lightweight and OBS-specific;
- make every action trigger auditable and visible;
- expose a safe local HTTP bridge;
- add the small set of named actions and discovery helpers that professional operators already expect;
- let Companion/Touch Portal/Home Assistant/MIDI bridges handle device-specific UI and feedback.

That preserves obs-hotkey's unique value: Wayland global hotkey capture plus multi-action-per-gesture OBS control.
