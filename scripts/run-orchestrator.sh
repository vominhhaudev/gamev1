#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH=${1:-server/server-settings.sample.json}

if [ ! -f "$CONFIG_PATH" ]; then
  echo "[warn] Config not found: $CONFIG_PATH. Using defaults/env." >&2
fi

cargo run -p server -- --config "$CONFIG_PATH"

