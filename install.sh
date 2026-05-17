#!/usr/bin/env bash
set -e

BINARY="${OBS_HOTKEY_BINARY:-obs-hotkey}"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found at '$BINARY'. Please build or install first:"
    echo "  cargo build --release"
    echo "  # binary will be at target/release/obs-hotkey"
    exit 1
fi

exec ./"$BINARY" setup "$@"