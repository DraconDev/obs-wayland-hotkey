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

echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  obs-hotkey installer"
echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ -n "$INSTALLED_VERSION" ]; then
    echo "  Installed:  $INSTALLED_VERSION"
fi
if [ -n "$RUNNING_VERSION" ]; then
    echo "  Running:   $RUNNING_VERSION"
fi

# Stop the service first to avoid "text file busy" error
if systemctl --user is-active obs-hotkey.service &>/dev/null; then
    echo "  Stopping obs-hotkey service..."
    systemctl --user stop obs-hotkey.service
    SERVICE_WAS_RUNNING=1
fi

# Build from source into a temp location, then install to cargo bin
echo "  Building obs-hotkey..."
cargo build --release 2>&1 | tail -1

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