# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.58] - 2026-06-10

### Changed
- **Product positioning clarified.** The README now leads with an explicit "What obs-hotkey is for" section and a "OBS Native Hotkeys vs obs-hotkey" comparison table. The tool's value on X11 is the multi-action-per-gesture features (action combos, delayed actions, push-to-release, switch_scene with parameter) and the single-config-file workflow. The tool's value on Wayland is global hotkey capture (which OBS itself cannot do). Simple single-key hotkeys such as `Scroll Lock → toggle recording` are explicitly noted as redundant with OBS native hotkeys on X11. No code or config changes — the feature set is unchanged, only the documentation is more honest about where obs-hotkey adds value.
- **Macro documentation clarified.** Reusable macros are documented as implemented for hotkey, CLI, and HTTP invocation, and HTTP behavior now explicitly distinguishes action dispatch success from OBS WebSocket request failures.

### Fixed
- **License metadata unified.** `Cargo.toml` and the `License` section of the README now match the actual `LICENSE` file (`AGPL-3.0-or-later`) and `CONTRIBUTING.md`. Previously `Cargo.toml` declared `MIT` while the rest of the project was AGPLv3.
- **Doctor diagnostics more informative.** The `Notify config` and `HTTP config` checks now show the actual reason instead of a static `ok` when they fail.

### Added
- Key chord support for hotkeys, including generic modifiers such as `ctrl`, `shift`, `alt`, `super`, and `meta`.
- `hotkey_combos` config entries for triggering multiple OBS actions from one key chord.
- `set_mic_volume` action with `mic_volume` config for setting OBS input volume as part of a combo.
- Recommended combo workflows and non-goals in the README.
- Config validation for duplicate combo names, unknown combo actions, and missing `mic_name` when a combo uses `set_mic_volume`.
- `switch_scene` action with a per-combo `scene` parameter for fast scene switching.
- `release_actions` and `release_action_delays_ms` on `hotkey_combos` for push-to-record / push-to-talk semantics.
- `allowed_devices` config field for restricting hotkey capture to specific /dev/input devices in multi-keyboard setups.
- `obs-hotkey action <name>` one-shot CLI subcommand for triggering a named OBS action or macro from scripts and systemd timers.
- Keyboard reader threads wrapped in `catch_unwind` so a panic in one device cannot kill the daemon.
- Parameterized action items: `actions` entries may now be either a bare action name or an object `{"action": "switch_scene", "scene": "Gaming"}` for actions that need arguments.
- A `Professional Workflows` section in the README covering stream-deck scene switching, push-to-mute, live recording with volume preset, countdown recording, and multi-keyboard shows.
- A `Roadmap & Non-Goals` section in the README that states explicitly what obs-hotkey is and is not for.

### Changed
- Existing single-action hotkeys remain backward-compatible and can also use chord syntax.

### Added (Tier 1 Observability)
- `obs-hotkey doctor` subcommand that runs a startup diagnostic checklist (config, combos, chords, macros, input group, keyboards, OBS WebSocket, notify, http).
- Reusable named macros that can be invoked from hotkey combos, `obs-hotkey action <name>`, and the HTTP listener.
- Richer `obs-hotkey status` output: it now also queries OBS for recording / streaming / replay buffer status, current program scene, and mic mute + volume.
- Desktop notification on every action trigger (configurable command, defaults to `notify-send`).
- `obs-hotkey action <name>` now also fires the configured desktop notification.
- Optional localhost-only HTTP listener (`http` config block) exposing `GET /health`, `GET /status`, `POST /actions`, `POST /actions/<name>`, `POST /macros`, and `POST /macros/<name>` for Companion / Touch Portal / Home Assistant / MIDI-controller bridges.
- HTTP listener supports an optional `token`; non-loopback binds require a token; `127.0.0.1` binds are accepted without a token for local automation.
- New `docs/tier1-observability.md` describing the Tier 1 design, behavior, and safety boundaries.
- New `docs/competitors-and-extensions.md` comparing obs-hotkey with OBS WebSocket CLIs, Companion / Touch Portal / Stream Deck integrations, MIDI-to-OBS tools, Advanced Scene Switcher, and Wayland/X11 hotkey daemons; it turns the research into a must-add / consider / non-goal extension plan.
- New `docs/value-add.md` distilling the research into a practical recommendation: the biggest next win is custom OBS request + feedback-friendly status JSON, followed by a practical action library, macro integration recipes, discovery helpers, and integration examples.
- New `docs/macros.md` describing reusable macro configuration, execution semantics, HTTP invocation, and safety boundaries.

### Changed (Tier 1 Observability)
- `obs-hotkey status` now reports the live OBS state when reachable, and prints a single-line failure summary when OBS is offline.

## [1.0.56] - 2026-05-20

### Added
- CHANGELOG.md for release tracking
- Improved crates.io metadata (homepage, documentation URLs)
- GitHub Release workflow now publishes to crates.io automatically

### Changed
- Release workflow creates published (non-draft) releases
- Release workflow generates release notes from commits

## [1.0.55] - 2026-05-20

### Changed
- Updated dependencies (obs, Cargo)

### Fixed
- Correct guard clearing by using single dereference

## [1.0.31] - 2026-05-18

### Changed
- Add contents:write permission to release workflow

## [1.0.30] - 2026-05-18

### Changed
- Reformat with latest rustfmt

## [1.0.29] - 2026-05-18

### Fixed
- Fix CI — pin dtolnay/rust-toolchain@stable

## [1.0.28] - 2026-05-18

### Changed
- Clean CI/CD workflows (Swatinem/rust-cache, reduced steps)

## [1.0.27] - 2026-05-18

### Fixed
- Keyword limit fix (crates.io max 5)

## [1.0.26] - 2026-05-18

### Changed
- Improved package metadata and GitHub repo topics

## [1.0.25] - 2026-05-18

### Fixed
- Fix test_expand_home under sandboxed HOME

## [1.0.24] - 2026-05-18

### Changed
- Docs and crates.io metadata sync

## [1.0.22-1.0.23] - 2026-05-18

### Fixed
- OBS WebSocket connection (ws:// scheme in TcpStream + missing rpcVersion serde rename)
- Continuous background reconnection, no 10-attempt cap