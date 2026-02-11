#!/bin/bash
# Build script for OBS Hotkey Go binary

set -e

echo "Building OBS Hotkey (Go version)..."

# Check if Go is installed
if ! command -v go &> /dev/null; then
    echo "Error: Go is not installed. Please install Go 1.21 or later."
    echo "Visit: https://go.dev/doc/install"
    exit 1
fi

# Build the binary using vendored dependencies (no network required)
echo "Compiling binary..."
go build -mod=vendor -o obs-hotkey-go main.go

# Make it executable
chmod +x obs-hotkey-go

echo ""
echo "Build successful!"
echo "Binary created: obs-hotkey-go"
echo ""
echo "To run:"
echo "  sudo ./obs-hotkey-go"
echo ""
echo "Or install to /usr/local/bin:"
echo "  sudo cp obs-hotkey-go /usr/local/bin/"
