#!/usr/bin/env bash
set -e

if command -v obs-hotkey >/dev/null 2>&1; then
    exec obs-hotkey setup "$@"
elif [ -f "./obs-hotkey" ]; then
    exec ./obs-hotkey setup "$@"
elif [ -f "target/release/obs-hotkey" ]; then
    exec target/release/obs-hotkey setup "$@"
else
    echo "obs-hotkey not found. Building from source..."
    cargo build --release
    exec target/release/obs-hotkey setup "$@"
fi