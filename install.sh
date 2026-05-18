#!/usr/bin/env bash
set -e

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$CARGO_BIN"

# Stop the service first to avoid "text file busy" error
if systemctl --user is-active obs-hotkey.service &>/dev/null; then
    echo "Stopping obs-hotkey service..."
    systemctl --user stop obs-hotkey.service
    SERVICE_WAS_RUNNING=1
fi

# Build from source into a temp location, then install to cargo bin
echo "Building obs-hotkey..."
cargo build --release

# Install the freshly-built binary (atomic replacement)
install -DTp target/release/obs-hotkey "$CARGO_BIN/obs-hotkey"

echo "Installed to $CARGO_BIN/obs-hotkey"

# Restart the service if it was running
if [ "${SERVICE_WAS_RUNNING:-0}" = "1" ]; then
    echo "Restarting obs-hotkey service..."
    systemctl --user start obs-hotkey.service
fi

# Run setup (unless skipped with --no-setup flag)
if [ "${1:-}" != "--no-setup" ]; then
    exec "$CARGO_BIN/obs-hotkey" setup "$@"
fi