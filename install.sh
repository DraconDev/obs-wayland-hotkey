#!/usr/bin/env bash
set -e

if [ ! -f "./obs-hotkey" ]; then
    echo "Binary not found. Building..."
    ./build.sh
fi

exec ./obs-hotkey setup "$@"