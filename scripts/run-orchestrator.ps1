param(
  [string]$ConfigPath = "server/server-settings.sample.json"
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $ConfigPath)) {
  Write-Warning "Config file not found: $ConfigPath. Using defaults/env."
}

cargo run -p server -- --config "$ConfigPath"

