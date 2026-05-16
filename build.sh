#!/usr/bin/env bash
set -e

echo "Building OBS Hotkey..."

if ! command -v go &> /dev/null; then
    echo "Error: Go is not installed. Please install Go 1.21 or later."
    echo "Visit: https://go.dev/doc/install"
    exit 1
fi

echo "Compiling binary..."
go build -mod=vendor -o obs-hotkey main.go

chmod +x obs-hotkey

echo ""
echo "Build successful!"
echo "Binary created: obs-hotkey"
echo ""
echo "To run:"
echo "  ./obs-hotkey"
echo ""
echo "Or install with auto-start:"
echo "  ./install.sh"
echo ""
echo "Config file (auto-created on first run):"
echo "  ~/.config/obs-hotkey/hotkeys.json"
