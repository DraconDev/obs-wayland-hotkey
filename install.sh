#!/usr/bin/env bash
set -e

# Build from source into a temp location, then install to cargo bin
echo "Building obs-hotkey..."
cargo build --release

# Find or install to cargo bin
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$CARGO_BIN"

# Copy the freshly-built binary
cp target/release/obs-hotkey "$CARGO_BIN/obs-hotkey"
chmod +x "$CARGO_BIN/obs-hotkey"

echo "Installed to $CARGO_BIN/obs-hotkey"

# Run setup
exec "$CARGO_BIN/obs-hotkey" setup "$@"