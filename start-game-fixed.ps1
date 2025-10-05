# 🚀 GAMEV1 - Ultimate Startup Script (Fixed Version)
# ===================================================
# This script ensures complete project startup with proper error handling
# Run this single file to start the entire game system

param(
    [switch]$SkipGateway = $false,
    [switch]$SkipPocketBase = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Write-Host "🚀 GAMEV1 - Complete Startup Script" -ForegroundColor Green
Write-Host "===================================" -ForegroundColor Yellow
Write-Host ""

# Function to test port availability
function Test-Port {
    param($Port, $ServiceName)
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $tcpClient.Connect("localhost", $Port)
        $tcpClient.Close()
        Write-Host "❌ Port $Port ($ServiceName) is already in use!" -ForegroundColor Red
        return $false
    }
    catch {
        Write-Host "✅ Port $Port ($ServiceName) is available" -ForegroundColor Green
        return $true
    }
}

# Function to check process status
function Get-ProcessStatus {
    param($ProcessName, $ServiceName)
    $process = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
    if ($process) {
        Write-Host "🟡 $ServiceName is already running (PID: $($process.Id))" -ForegroundColor Yellow
        return $true
    }
    return $false
}

# Function to start service with retry
function Start-ServiceWithRetry {
    param(
        $ServiceName,
        $Command,
        $WorkingDirectory = ".",
        $MaxRetries = 3
    )

    Write-Host "🔄 Starting $ServiceName..." -ForegroundColor Cyan

    for ($i = 1; $i -le $MaxRetries; $i++) {
        try {
            Push-Location $WorkingDirectory
            if ($Verbose) {
                Write-Host "   Command: $Command" -ForegroundColor Gray
            }

            $process = Start-Process "cmd.exe" -ArgumentList "/c $Command" -PassThru -WindowStyle Hidden
            Pop-Location

            # Wait a moment and check if process is still running
            Start-Sleep -Seconds 2
            if (!$process.HasExited) {
                Write-Host "✅ $ServiceName started successfully (PID: $($process.Id))" -ForegroundColor Green
                return $process
            }
        }
        catch {
            Write-Host "⚠️  Attempt $i failed for $ServiceName : $($_.Exception.Message)" -ForegroundColor Yellow
        }
        Pop-Location

        if ($i -lt $MaxRetries) {
            Write-Host "⏳ Retrying in 3 seconds..." -ForegroundColor Yellow
            Start-Sleep -Seconds 3
        }
    }

    Write-Host "❌ Failed to start $ServiceName after $MaxRetries attempts" -ForegroundColor Red
    throw "Service startup failed: $ServiceName"
}

# Function to check Rust toolchain
function Test-RustEnvironment {
    try {
        $rustVersion = & cargo --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Rust toolchain: $rustVersion" -ForegroundColor Green
            return $true
        }
    }
    catch {
        Write-Host "❌ Rust toolchain not found" -ForegroundColor Red
        Write-Host "   Please install Rust from: https://rustup.rs/" -ForegroundColor Yellow
        return $false
    }
}

# Function to check Node.js
function Test-NodeEnvironment {
    try {
        $nodeVersion = & node --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Node.js: $nodeVersion" -ForegroundColor Green
            return $true
        }
    }
    catch {
        Write-Host "❌ Node.js not found" -ForegroundColor Red
        Write-Host "   Please install Node.js from: https://nodejs.org/" -ForegroundColor Yellow
        return $false
    }
}

# Cleanup function
function Stop-ExistingProcesses {
    Write-Host "🛑 Cleaning up existing processes..." -ForegroundColor Yellow

    $processes = @("cargo", "node", "pocketbase")
    foreach ($processName in $processes) {
        $process = Get-Process -Name $processName -ErrorAction SilentlyContinue
        if ($process) {
            Write-Host "   Stopping $processName (PID: $($process.Id))..." -ForegroundColor Gray
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
    }

    # Wait for processes to fully stop
    Start-Sleep -Seconds 2
}

# Main startup sequence
try {
    Write-Host "📋 Pre-flight Checks" -ForegroundColor Cyan
    Write-Host "===================" -ForegroundColor Cyan

    # Check environment
    if (!(Test-RustEnvironment)) {
        throw "Rust environment check failed"
    }

    if (!(Test-NodeEnvironment)) {
        throw "Node.js environment check failed"
    }

    # Check ports
    Write-Host ""
    Write-Host "🔌 Checking Ports" -ForegroundColor Cyan
    Write-Host "================" -ForegroundColor Cyan

    $portsOk = $true
    $portsOk = $portsOk -and (Test-Port -Port 50051 -ServiceName "Worker (gRPC)")
    $portsOk = $portsOk -and (Test-Port -Port 5173 -ServiceName "Client (Web)")

    if (!$SkipGateway) {
        $portsOk = $portsOk -and (Test-Port -Port 8080 -ServiceName "Gateway (HTTP)")
    }

    if (!$SkipPocketBase) {
        $portsOk = $portsOk -and (Test-Port -Port 8090 -ServiceName "PocketBase (DB)")
    }

    if (!$portsOk) {
        Write-Host ""
        Write-Host "❌ Port conflicts detected. Please resolve them and try again." -ForegroundColor Red
        exit 1
    }

    # Stop existing processes
    Write-Host ""
    Stop-ExistingProcesses

    # Install client dependencies if needed
    Write-Host ""
    Write-Host "📦 Checking Dependencies" -ForegroundColor Cyan
    Write-Host "=======================" -ForegroundColor Cyan

    if (!(Test-Path "client\node_modules")) {
        Write-Host "📥 Installing client dependencies..." -ForegroundColor Yellow
        Push-Location "client"
        try {
            npm install
            Write-Host "✅ Dependencies installed successfully" -ForegroundColor Green
        }
        catch {
            Write-Host "⚠️  Standard install failed, trying with legacy peer deps..." -ForegroundColor Yellow
            npm install --legacy-peer-deps
            Write-Host "✅ Dependencies installed with legacy peer deps" -ForegroundColor Green
        }
        Pop-Location
    } else {
        Write-Host "✅ Client dependencies already installed" -ForegroundColor Green
    }

    # Start services
    Write-Host ""
    Write-Host "🚀 Starting Services" -ForegroundColor Cyan
    Write-Host "===================" -ForegroundColor Cyan

    # Start Worker (essential)
    $workerProcess = Start-ServiceWithRetry -ServiceName "Worker" -Command "cargo run --bin worker" -WorkingDirectory "."

    # Wait for worker to initialize
    Write-Host "⏳ Waiting for Worker to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5

    # Start Client (essential)
    $clientProcess = Start-ServiceWithRetry -ServiceName "Client" -Command "npm run dev" -WorkingDirectory "client"

    # Start optional services
    $gatewayProcess = $null
    $pbProcess = $null

    if (!$SkipGateway) {
        Write-Host ""
        Write-Host "🌐 Starting Gateway (optional)..." -ForegroundColor Yellow
        try {
            $gatewayProcess = Start-ServiceWithRetry -ServiceName "Gateway" -Command "cargo run -p gateway" -WorkingDirectory "."
        }
        catch {
            Write-Host "⚠️  Gateway startup failed (continuing without it)" -ForegroundColor Yellow
        }
    }

    if (!$SkipPocketBase) {
        Write-Host ""
        Write-Host "🗄️ Starting PocketBase (optional)..." -ForegroundColor Yellow
        try {
            $pbProcess = Start-ServiceWithRetry -ServiceName "PocketBase" -Command ".\pocketbase\pocketbase.exe serve" -WorkingDirectory "."
        }
        catch {
            Write-Host "⚠️  PocketBase startup failed (continuing without it)" -ForegroundColor Yellow
        }
    }

    # Success summary
    Write-Host ""
    Write-Host "🎉 ALL SERVICES STARTED SUCCESSFULLY!" -ForegroundColor Green
    Write-Host "=====================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "📍 Access Points:" -ForegroundColor Cyan
    Write-Host "   🌐 Game Client: http://localhost:5173" -ForegroundColor White
    Write-Host "   🎯 Game:        http://localhost:5173/game" -ForegroundColor White
    Write-Host ""
    Write-Host "🔧 Services Running:" -ForegroundColor Cyan
    Write-Host "   ⚙️  Worker:     localhost:50051 (gRPC)" -ForegroundColor White
    Write-Host "   🌐 Client:     localhost:5173 (Web)" -ForegroundColor White

    if ($gatewayProcess -and !$gatewayProcess.HasExited) {
        Write-Host "   🔗 Gateway:    localhost:8080 (HTTP API)" -ForegroundColor White
    }

    if ($pbProcess -and !$pbProcess.HasExited) {
        Write-Host "   🗄️ PocketBase: localhost:8090 (Database)" -ForegroundColor White
    }

    Write-Host ""
    Write-Host "🎮 How to Play:" -ForegroundColor Cyan
    Write-Host "   1. Open http://localhost:5173 in your browser" -ForegroundColor White
    Write-Host "   2. Click '🎮 Play Game' to start" -ForegroundColor White
    Write-Host "   3. Click 'Join Game' to connect to server" -ForegroundColor White
    Write-Host "   4. Use WASD to move, Shift to sprint" -ForegroundColor White
    Write-Host ""
    Write-Host "🛑 To Stop:" -ForegroundColor Red
    Write-Host "   - Close this PowerShell window" -ForegroundColor White
    Write-Host "   - Or press Ctrl+C" -ForegroundColor White
    Write-Host ""
    Write-Host "🚀 Happy gaming!" -ForegroundColor Green

    # Keep window alive and monitor processes
    try {
        while ($true) {
            Start-Sleep -Seconds 5

            # Check if essential processes are still running
            $essentialRunning = (!$workerProcess.HasExited) -and (!$clientProcess.HasExited)

            if (!$essentialRunning) {
                Write-Host ""
                Write-Host "⚠️  Essential services have stopped unexpectedly!" -ForegroundColor Yellow
                break
            }
        }
    }
    catch {
        # Ctrl+C or other interruption
    }
    finally {
        # Cleanup
        Write-Host ""
        Write-Host "🛑 Shutting down services..." -ForegroundColor Yellow

        $allProcesses = @($workerProcess, $clientProcess)
        if ($gatewayProcess) { $allProcesses += $gatewayProcess }
        if ($pbProcess) { $allProcesses += $pbProcess }

        foreach ($proc in $allProcesses) {
            if (!$proc.HasExited) {
                Write-Host "   Stopping service (PID: $($proc.Id))..." -ForegroundColor Gray
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            }
        }

        Write-Host "👋 Shutdown complete!" -ForegroundColor Green
    }

}
catch {
    Write-Host ""
    Write-Host "❌ FATAL ERROR: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "🔧 Troubleshooting:" -ForegroundColor Yellow
    Write-Host "   1. Check if Rust is installed: cargo --version" -ForegroundColor White
    Write-Host "   2. Check if Node.js is installed: node --version" -ForegroundColor White
    Write-Host "   3. Check port availability: netstat -an | findstr :port" -ForegroundColor White
    Write-Host "   4. Run with -Verbose flag for detailed output" -ForegroundColor White
    Write-Host ""
    Write-Host "📞 Need help? Check the troubleshooting guide." -ForegroundColor Cyan

    exit 1
}
