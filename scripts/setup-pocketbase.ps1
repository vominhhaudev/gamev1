# Setup PocketBase for Windows
param(
    [string]$Version = "0.30.0",
    [string]$InstallPath = "pocketbase"
)

Write-Host "Setting up PocketBase $Version..."

# Create directory if it doesn't exist
if (-not (Test-Path $InstallPath)) {
    New-Item -ItemType Directory -Path $InstallPath | Out-Null
}

# Download PocketBase
$PocketBaseUrl = "https://github.com/pocketbase/pocketbase/releases/download/v$Version/pocketbase_$Version_windows_amd64.zip"
$ZipPath = "$InstallPath/pocketbase_$Version.zip"
$BinaryPath = "$InstallPath/pocketbase.exe"

if (-not (Test-Path $BinaryPath)) {
    Write-Host "Downloading PocketBase..."
    Invoke-WebRequest -Uri $PocketBaseUrl -OutFile $ZipPath

    Write-Host "Extracting..."
    Expand-Archive -Path $ZipPath -DestinationPath $InstallPath -Force

    # Move binary to root of pocketbase directory
    Move-Item "$InstallPath/pocketbase.exe" $BinaryPath -Force

    # Cleanup
    Remove-Item $ZipPath -Force
    Remove-Item "$InstallPath/pb_migrations" -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "PocketBase installed at: $BinaryPath"
Write-Host "To start PocketBase: .\pocketbase\pocketbase.exe serve"
Write-Host "Admin UI will be available at: http://127.0.0.1:8090/_/"
