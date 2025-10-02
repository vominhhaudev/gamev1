#!/usr/bin/env bash
set -euo pipefail

VERSION=${1:-"0.22.0"}
INSTALL_PATH=${2:-"pocketbase"}

echo "Setting up PocketBase $VERSION..."

# Create directory if it doesn't exist
mkdir -p "$INSTALL_PATH"

# Download PocketBase for Linux
POCKETBASE_URL="https://github.com/pocketbase/pocketbase/releases/download/v$VERSION/pocketbase_$VERSION_linux_amd64.zip"
ZIP_PATH="$INSTALL_PATH/pocketbase_$VERSION.zip"
BINARY_PATH="$INSTALL_PATH/pocketbase"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Downloading PocketBase..."
    wget -O "$ZIP_PATH" "$POCKETBASE_URL"

    echo "Extracting..."
    cd "$INSTALL_PATH"
    unzip -o "$ZIP_PATH"

    # Move binary to root of pocketbase directory
    mv pocketbase "$BINARY_PATH"
    chmod +x "$BINARY_PATH"

    # Cleanup
    rm -f "$ZIP_PATH"
    rm -rf pb_migrations
    cd ..
fi

echo "PocketBase installed at: $BINARY_PATH"
echo "To start PocketBase: ./$BINARY_PATH serve"
echo "Admin UI will be available at: http://127.0.0.1:8090/_/"

