# GAMEV1 - Simple Startup Script
# ==============================
# This script starts the essential game services

param(
    [switch]$Verbose = $false
)

Write-Host "GAMEV1 - Starting Game Services" -ForegroundColor Green
Write-Host "===============================" -ForegroundColor Yellow

# Check environment
Write-Host "Checking environment..." -ForegroundColor Cyan

try {
    $cargoVersion = cargo --version
    Write-Host "Rust: $cargoVersion" -ForegroundColor Green
}
catch {
    Write-Host "ERROR: Rust not found. Install from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

try {
    $nodeVersion = node --version
    Write-Host "Node.js: $nodeVersion" -ForegroundColor Green
}
catch {
    Write-Host "ERROR: Node.js not found. Install from https://nodejs.org/" -ForegroundColor Red
    exit 1
}

# Check ports
Write-Host "Checking ports..." -ForegroundColor Cyan

function Test-Port {
    param($Port)
    $result = Test-NetConnection -ComputerName localhost -Port $Port -WarningAction SilentlyContinue
    return $result.TcpTestSucceeded
}

$port50051 = Test-Port 50051
$port5173 = Test-Port 5173

Write-Host "Port 50051 (Worker): $(if ($port50051) { 'IN USE' } else { 'Available' })" -ForegroundColor $(if ($port50051) { 'Red' } else { 'Green' })
Write-Host "Port 5173 (Client): $(if ($port5173) { 'IN USE' } else { 'Available' })" -ForegroundColor $(if ($port5173) { 'Red' } else { 'Green' })

if ($port50051 -or $port5173) {
    Write-Host "ERROR: Required ports are in use!" -ForegroundColor Red
    exit 1
}

# Stop existing processes
Write-Host "Cleaning up existing processes..." -ForegroundColor Yellow
Get-Process -Name "cargo", "node" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Install client dependencies if needed
Write-Host "Checking client dependencies..." -ForegroundColor Cyan
if (!(Test-Path "client\node_modules")) {
    Write-Host "Installing client dependencies..." -ForegroundColor Yellow
    Set-Location "client"
    npm install
    Set-Location ".."
}

# Start Worker
Write-Host "Starting Worker..." -ForegroundColor Cyan
$workerJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    cargo run --bin worker
} -Name "GameWorker"

Write-Host "Worker started (Job ID: $($workerJob.Id))" -ForegroundColor Green

# Wait for worker
Write-Host "Waiting for Worker..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Start Client
Write-Host "Starting Client..." -ForegroundColor Cyan
Set-Location "client"
$clientProcess = Start-Process "npm" -ArgumentList "run dev" -PassThru
Set-Location ".."

Write-Host "Client started (PID: $($clientProcess.Id))" -ForegroundColor Green

# Success message
Write-Host ""
Write-Host "SUCCESS! All services started!" -ForegroundColor Green
Write-Host "============================" -ForegroundColor Green
Write-Host ""
Write-Host "Access points:" -ForegroundColor Cyan
Write-Host "  Game Client: http://localhost:5173" -ForegroundColor White
Write-Host "  Game:        http://localhost:5173/game" -ForegroundColor White
Write-Host ""
Write-Host "Services running:" -ForegroundColor Cyan
Write-Host "  Worker: localhost:50051 (gRPC)" -ForegroundColor White
Write-Host "  Client: localhost:5173 (Web)" -ForegroundColor White
Write-Host ""
Write-Host "To play: Open http://localhost:5173 in browser" -ForegroundColor Yellow
Write-Host "To stop: Close this window or press Ctrl+C" -ForegroundColor Red

# Keep alive
try {
    while ($true) {
        Start-Sleep -Seconds 5

        # Check if processes are still running
        if ($workerJob.State -ne "Running") {
            Write-Host "Worker process stopped!" -ForegroundColor Yellow
            break
        }

        if ($clientProcess.HasExited) {
            Write-Host "Client process stopped!" -ForegroundColor Yellow
            break
        }
    }
}
catch {
    # Cleanup
    Write-Host "Shutting down..." -ForegroundColor Yellow
    Stop-Job -Job $workerJob -Force
    Stop-Process -Id $clientProcess.Id -Force -ErrorAction SilentlyContinue
    Write-Host "Shutdown complete!" -ForegroundColor Green
}
