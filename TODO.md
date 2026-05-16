# TODO: Subcommand CLI + Banner

## Goal

Replace `--install-service` flag with proper subcommands (`setup`, `teardown`, `status`)
and add a startup banner that shows hotkeys + autostart state. The binary becomes
self-documenting — running it with no args shows what to do next.

## Current State

- `main()` uses `flag.Parse()` with `--config` and `--install-service` flags
- `--install-service` writes systemd unit + enables it, then exits
- `install.sh` is a 140-line script that duplicates logic (input group check,
  service writing, next-steps output, old service migration)
- `flake.nix` has a separate `obs-hotkey-install-service` wrapper binary
- No banner — just `log.Println("OBS Hotkey Controller - Wayland compatible")`
- Running the binary gives no indication of how to set up auto-start

## Target State

```
obs-hotkey [flags]       → run daemon with banner (default)
obs-hotkey setup         → one-time: write systemd service + enable auto-start
obs-hotkey teardown      → undo setup (stop + disable + remove service file)
obs-hotkey status        → show service state + config + group membership
```

Banner shown on every daemon start:
```
OBS Hotkey Controller - Wayland compatible

  scroll lock → Toggle Recording
  pause       → Toggle Pause/Resume

  Auto-start: not configured (run 'obs-hotkey setup' to enable)

Listening for hotkeys... (Ctrl+C to exit)
```

---

## Tasks

### Phase 1: Refactor main.go — extract functions

- [ ] 1.1 Extract current daemon body (lines 593-768) into `func runDaemon(configPath string)`
- [ ] 1.2 Extract current `--install-service` body (lines 556-591) into `func runSetup(configPath string)`
- [ ] 1.3 Add `func runTeardown(purge bool)` — stop, disable, remove service, daemon-reload; if purge, also remove config dir
- [ ] 1.4 Add `func runStatus(configPath string)` — check service enabled, input group, config exists, OBS port reachable
- [ ] 1.5 Add `func isAutostartEnabled() bool` — `systemctl --user is-enabled obs-hotkey.service` exit code
- [ ] 1.6 Add `func printBanner(cfg AppConfig, bindings []hotkeyBinding, autostart bool)` — formatted hotkey list + setup state

### Phase 2: Subcommand parsing

- [ ] 2.1 Replace `flag.Parse()` with subcommand switch on `os.Args[1:]`
- [ ] 2.2 Parse `--config` flag before subcommand switch (shared by daemon + setup + status)
- [ ] 2.3 Route: no subcommand → `runDaemon()`, `setup` → `runSetup()`, `teardown` → `runTeardown()`, `status` → `runStatus()`
- [ ] 2.4 Unknown subcommand → print usage and exit 1
- [ ] 2.5 Remove `--install-service` flag entirely
- [ ] 2.6 Add `--purge` flag for `teardown` subcommand only

### Phase 3: Enhance `runSetup()`

Absorb logic currently in `install.sh`:
- [ ] 3.1 Check `input` group membership and print warning if missing (currently only in install.sh)
- [ ] 3.2 Migrate old `obs-wayland-hotkey.service` if present (currently only in install.sh)
- [ ] 3.3 Print next-steps after setup (OBS WebSocket enable, hotkey list, service commands)
- [ ] 3.4 Start the service after setup (currently only `enable`, not `start`)

### Phase 4: Banner in `runDaemon()`

- [ ] 4.1 Replace `log.Println("OBS Hotkey Controller...")` with `printBanner()`
- [ ] 4.2 `printBanner` shows: title, hotkey bindings (from config), autostart state
- [ ] 4.3 Detect if running under systemd (check `INVOCATION_ID` or `JOURNAL_STREAM` env vars)
      → if yes, skip the "run obs-hotkey setup" hint (it's already set up)

### Phase 5: Tests

- [ ] 5.1 `TestSubcommandParsing` — verify os.Args routing for each subcommand
- [ ] 5.2 `TestSetupWritesServiceFile` — temp HOME, run runSetup(), verify unit file contents + ExecStart path
- [ ] 5.3 `TestTeardownRemovesServiceFile` — create service file, run runTeardown(), verify removed
- [ ] 5.4 `TestIsAutostartEnabled` — mock systemctl behavior (enabled/disabled)
- [ ] 5.5 `TestPrintBanner` — verify output format with known config
- [ ] 5.6 `TestRunStatusOutput` — verify status checks (at minimum: config exists check)
- [ ] 5.7 Keep all existing 24 tests passing (no regressions)

### Phase 6: Simplify install.sh

- [ ] 6.1 Reduce to thin wrapper: build if needed, then `exec ./obs-hotkey setup "$@"`
- [ ] 6.2 Remove all duplicated logic (input group check, service writing, next-steps, migration)
- [ ] 6.3 Keep as convenience for curl-to-bash / zero-doc users

### Phase 7: Update flake.nix

- [ ] 7.1 Remove `obs-hotkey-install-service` wrapper from postInstall
- [ ] 7.2 Remove `"install-service"` app entry from apps
- [ ] 7.3 Default app already passes args through: `nix run .# -- setup` works
- [ ] 7.4 Verify `nix build` still produces `result/bin/obs-hotkey`

### Phase 8: Update README.md

- [ ] 8.1 Replace `nix run .#install-service` with `nix run .# -- setup`
- [ ] 8.2 Add `setup`, `teardown`, `status` to usage docs
- [ ] 8.3 Update "Other Linux" section: `./install.sh` still works but mention `./obs-hotkey setup`
- [ ] 8.4 Update "Managing the Service" section with subcommand examples
- [ ] 8.5 Remove `--install-service` references

### Phase 9: Verify nix/module.nix

- [ ] 9.1 No changes needed — `ExecStart = .../bin/obs-hotkey --config ...` is still the daemon mode
- [ ] 9.2 Verify: module does NOT need to call `setup` (it writes the service file directly)

### Phase 10: Final verification

- [ ] 10.1 `go test -mod=vendor -count=1 ./...` — all tests pass
- [ ] 10.2 `go vet -mod=vendor ./...` — no warnings
- [ ] 10.3 `gofmt -l *.go` — no formatting issues
- [ ] 10.4 `go build -mod=vendor -o /dev/null .` — builds clean
- [ ] 10.5 `nix build .#default` — nix build succeeds
- [ ] 10.6 `nix flake show` — apps look correct

---

## Open Questions

1. **`status` subcommand complexity** — worth implementing, or just tell users
   `systemctl --user status obs-hotkey.service`? It's convenient but adds
   ~50 lines of code for checks that are trivially done manually.

2. **Banner under systemd** — when running as a systemd service, the banner
   adds noise to journalctl. Detect `INVOCATION_ID` env var (set by systemd)
   and suppress the "run obs-hotkey setup" hint in that case?

3. **`teardown --purge` removing binary** — should teardown also remove the
   binary from `~/.local/bin`? No — the user may have installed via nix/pkg.
   `--purge` only removes config dir. Binary removal is the user's responsibility.

4. **`install.sh` as thin wrapper vs removal** — keep it for curl-to-bash
   convenience, or remove entirely and rely on `obs-hotkey setup`?

---

## Files Changed (summary)

| File | Lines | Change |
|------|-------|--------|
| `main.go` | 768→~850 | Refactor main() into subcommands, add setup/teardown/status/banner functions |
| `main_test.go` | 263→~350 | Add tests for subcommands, setup, teardown, banner |
| `install.sh` | 140→~15 | Simplify to thin wrapper (build + exec obs-hotkey setup) |
| `flake.nix` | 59→~45 | Remove install-service wrapper + app entry |
| `README.md` | 390→~380 | Update Quick Install, add subcommand docs |
| `nix/module.nix` | 37 | No changes |
| `obsclient_test.go` | 330 | No changes |
| `build.sh` | 28 | No changes |
