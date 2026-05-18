#!/usr/bin/env bash
set -e

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$CARGO_BIN"

# Show versions before doing anything
INSTALLED_VERSION=""
if [ -x "$CARGO_BIN/obs-hotkey" ]; then
    INSTALLED_VERSION=$("$CARGO_BIN/obs-hotkey" --version 2>/dev/null || echo "unknown")
fi
RUNNING_VERSION=""
if systemctl --user is-active obs-hotkey.service &>/dev/null; then
    RUNNING_PID=$(systemctl --user show obs-hotkey.service -p MainPID --value 2>/dev/null || echo "")
    if [ -n "$RUNNING_PID" ] && [ "$RUNNING_PID" != "0" ]; then
        RUNNING_VERSION="pid $RUNNING_PID (active)"
    fi
fi

# Also check for stale binaries
STALE_BIN=""
if [ -x "$HOME/.local/bin/obs-hotkey" ]; then
    STALE_BIN="$HOME/.local/bin/obs-hotkey"
elif pgrep -x obs-hotkey &>/dev/null; then
    # There are running obs-hotkey processes — check if any are not from cargo bin
    CARGO_PID=$(pgrep -f "$CARGO_BIN/obs-hotkey" 2>/dev/null | head -1 || echo "")
    STALE_COUNT=$(pgrep -x obs-hotkey 2>/dev/null | grep -v "^$" | wc -l)
    if [ -n "$STALE_COUNT" ] && [ "$STALE_COUNT" -gt 0 ]; then
        STALE_BIN="$STALE_COUNT stale process(es)"
    fi
fi

echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  obs-hotkey installer"
echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ -n "$INSTALLED_VERSION" ]; then
    echo "  Installed:  $INSTALLED_VERSION"
fi
if [ -n "$RUNNING_VERSION" ]; then
    echo "  Running:   $RUNNING_VERSION"
fi
if [ -n "$STALE_BIN" ]; then
    echo "  Stale:     $STALE_BIN"
fi

# Clean up stale binaries before any install
CLEANED=0
if [ -f "$HOME/.local/bin/obs-hotkey" ]; then
    echo "  Removing stale binary: $HOME/.local/bin/obs-hotkey"
    rm -f "$HOME/.local/bin/obs-hotkey"
    CLEANED=1
fi

# Kill any stale obs-hotkey processes that aren't from the current cargo bin.
# This handles cases where the service was started from an old binary location.
# We identify stale processes by: not running from CARGO_BIN, or not matching
# the current running service's command line.
if pgrep -x obs-hotkey &>/dev/null; then
    SERVICE_PID=$(systemctl --user show obs-hotkey.service -p MainPID --value 2>/dev/null || echo "")
    for pid in $(pgrep -x obs-hotkey 2>/dev/null); do
        # Skip if it's the systemd service PID
        if [ -n "$SERVICE_PID" ] && [ "$pid" = "$SERVICE_PID" ]; then
            continue
        fi
        # Read the command line to check if it's from cargo bin
        CMD=$(cat /proc/$pid/cmdline 2>/dev/null | tr '\0' ' ')
        if echo "$CMD" | grep -q "$CARGO_BIN"; then
            # This is likely the old binary with the same path — still stale
            :
        fi
        echo "  Killing stale process: $pid ($CMD)"
        kill "$pid" 2>/dev/null || true
        CLEANED=1
    done
    sleep 1
fi

# Stop the service first to avoid "text file busy" error
if systemctl --user is-active obs-hotkey.service &>/dev/null; then
    echo "  Stopping obs-hotkey service..."
    systemctl --user stop obs-hotkey.service
    SERVICE_WAS_RUNNING=1
fi

# Also kill any stray processes that started during the stop (race)
if pgrep -x obs-hotkey &>/dev/null; then
    for pid in $(pgrep -x obs-hotkey 2>/dev/null); do
        SERVICE_PID=$(systemctl --user show obs-hotkey.service -p MainPID --value 2>/dev/null || echo "")
        [ -n "$SERVICE_PID" ] && [ "$pid" = "$SERVICE_PID" ] && continue
        kill "$pid" 2>/dev/null || true
    done
    sleep 1
fi

if [ "$CLEANED" = "1" ]; then
    echo "  Cleanup complete."
fi

# Build from source into a temp location, then install to cargo bin
echo "  Building obs-hotkey..."
if ! cargo build --release 2>&1 | tail -1; then
    echo "  Error: build failed!"
    exit 1
fi

# Verify the binary exists before installing
if [ ! -f target/release/obs-hotkey ]; then
    echo "  Error: binary not found after build!"
    exit 1
fi

# Install the freshly-built binary (atomic replacement)
install -DTp target/release/obs-hotkey "$CARGO_BIN/obs-hotkey"

NEW_VERSION=$("$CARGO_BIN/obs-hotkey" --version)
echo "  Installed:  $NEW_VERSION  →  $CARGO_BIN/obs-hotkey"

# Restart the service if it was running
if [ "${SERVICE_WAS_RUNNING:-0}" = "1" ]; then
    echo "  Restarting obs-hotkey service..."
    systemctl --user start obs-hotkey.service
fi

# Run setup (unless skipped with --no-setup flag)
if [ "${1:-}" != "--no-setup" ]; then
    exec "$CARGO_BIN/obs-hotkey" setup "$@"
fi