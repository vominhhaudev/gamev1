# GAMEV1 - COMPLETE PROJECT STARTUP SCRIPT
# ============================================
# One-click startup for the entire GameV1 project
# Run this single file to start all services

param(
    [switch]$SkipGateway = $false,
    [switch]$SkipPocketBase = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Write-Host "GAMEV1 - Complete Project Startup" -ForegroundColor Green
Write-Host "====================================" -ForegroundColor Green
Write-Host "Multiplayer Game Development Platform" -ForegroundColor Cyan
Write-Host ""

# Function to check if service is running
function Test-ServiceRunning {
    param($ProcessName, $ServiceName)
    $process = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
    if ($process) {
        Write-Host "OK $ServiceName is running (PID: $($process.Id))" -ForegroundColor Green
        return $true
    }
    return $false
}

# Function to start a service
function Start-ServiceProcess {
    param(
        $ServiceName,
        $Command,
        $WorkingDirectory = ".",
        $Arguments = @()
    )

    Write-Host "Starting $ServiceName..." -ForegroundColor Cyan

    try {
        Push-Location $WorkingDirectory

        if ($Arguments.Count -gt 0) {
            $process = Start-Process "cmd.exe" -ArgumentList "/c $Command $($Arguments -join ' ')" -PassThru -WindowStyle Hidden
        } else {
            $process = Start-Process "cmd.exe" -ArgumentList "/c $Command" -PassThru -WindowStyle Hidden
        }

        Pop-Location

        # Wait a moment for process to start
        Start-Sleep -Seconds 2

        if (!$process.HasExited) {
            Write-Host "OK $ServiceName started successfully (PID: $($process.Id))" -ForegroundColor Green
            return $process
        } else {
            Write-Host "ERROR $ServiceName failed to start" -ForegroundColor Red
            return $null
        }
    }
    catch {
        Write-Host "ERROR Failed to start $ServiceName : $($_.Exception.Message)" -ForegroundColor Red
        Pop-Location
        return $null
    }
}

# Function to check environment
function Test-Environment {
    Write-Host "Checking Environment..." -ForegroundColor Cyan

    # Check Rust
    try {
        $rustVersion = & cargo --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "OK Rust: $rustVersion" -ForegroundColor Green
        } else {
            throw "Rust not found"
        }
    }
    catch {
        Write-Host "ERROR Rust not found. Please install from https://rustup.rs/" -ForegroundColor Red
        return $false
    }

    # Check Node.js
    try {
        $nodeVersion = & node --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "OK Node.js: $nodeVersion" -ForegroundColor Green
        } else {
            throw "Node.js not found"
        }
    }
    catch {
        Write-Host "ERROR Node.js not found. Please install from https://nodejs.org/" -ForegroundColor Red
        return $false
    }

    return $true
}

# Function to install client dependencies
function Install-ClientDependencies {
    Write-Host "Installing client dependencies..." -ForegroundColor Cyan

    if (!(Test-Path "client\node_modules")) {
        Push-Location "client"
        try {
            Write-Host "  Running npm install..." -ForegroundColor Yellow
            & npm install 2>$null

            if ($LASTEXITCODE -eq 0) {
                Write-Host "OK Dependencies installed successfully" -ForegroundColor Green
            } else {
                Write-Host "WARN Standard install failed, trying legacy peer deps..." -ForegroundColor Yellow
                & npm install --legacy-peer-deps 2>$null

                if ($LASTEXITCODE -eq 0) {
                    Write-Host "OK Dependencies installed with legacy peer deps" -ForegroundColor Green
                } else {
                    throw "npm install failed"
                }
            }
        }
        catch {
            Write-Host "ERROR Failed to install dependencies: $($_.Exception.Message)" -ForegroundColor Red
        }
        finally {
            Pop-Location
        }
    } else {
        Write-Host "OK Client dependencies already installed" -ForegroundColor Green
    }
}

# Main startup sequence
try {
    Write-Host "System Check" -ForegroundColor Cyan
    Write-Host "==============" -ForegroundColor Cyan

    # Check environment
    if (!(Test-Environment)) {
        throw "Environment check failed"
    }

    # Stop existing processes to avoid conflicts
    Write-Host ""
    Write-Host "Cleaning up existing processes..." -ForegroundColor Yellow

    $processesToStop = @("cargo", "node", "pocketbase")
    foreach ($processName in $processesToStop) {
        $process = Get-Process -Name $processName -ErrorAction SilentlyContinue
        if ($process) {
            Write-Host "  Stopping $processName (PID: $($process.Id))..." -ForegroundColor Gray
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
    }

    Start-Sleep -Seconds 2

    # Install client dependencies
    Write-Host ""
    Install-ClientDependencies

    # Start services
    Write-Host ""
    Write-Host "Starting Services" -ForegroundColor Cyan
    Write-Host "===================" -ForegroundColor Cyan

    # Start Worker (essential)
    Write-Host "Starting Worker (gRPC 50051)..." -ForegroundColor Cyan
    $workerProcess = Start-ServiceProcess -ServiceName "Worker" -Command "cargo run --bin worker" -WorkingDirectory "."
    if (!$workerProcess) {
        throw "Failed to start Worker"
    }

    # Wait for worker to initialize
    Write-Host "Waiting for Worker to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5

    # Start Client (essential)
    Write-Host "Starting Client (port 5173)..." -ForegroundColor Cyan
    $clientProcess = Start-ServiceProcess -ServiceName "Client" -Command "npm run dev" -WorkingDirectory "client"
    if (!$clientProcess) {
        throw "Failed to start Client"
    }

    # Start Gateway (optional)
    if (!$SkipGateway) {
        Write-Host ""
        Write-Host "Starting Gateway (port 8080)..." -ForegroundColor Yellow
        $gatewayProcess = Start-ServiceProcess -ServiceName "Gateway" -Command "cargo run -p gateway" -WorkingDirectory "."
    }

    # Start PocketBase (optional)
    if (!$SkipPocketBase) {
        Write-Host ""
        Write-Host "Starting PocketBase (port 8090)..." -ForegroundColor Yellow
        $pbProcess = Start-ServiceProcess -ServiceName "PocketBase" -Command ".\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090" -WorkingDirectory "."
    }

    # Success summary
    Write-Host ""
    Write-Host "ALL SERVICES STARTED SUCCESSFULLY!" -ForegroundColor Green
    Write-Host "=====================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "Access Points:" -ForegroundColor Cyan
    Write-Host "   Game Client: http://localhost:5173" -ForegroundColor White
    Write-Host "   Game:        http://localhost:5173/game" -ForegroundColor White
    Write-Host ""
    Write-Host "Services Running:" -ForegroundColor Cyan
    Write-Host "   Worker:     localhost:50051 (gRPC)" -ForegroundColor White
    Write-Host "   Client:     localhost:5173 (Web)" -ForegroundColor White

    if (!$SkipGateway -and $gatewayProcess) {
        Write-Host "   Gateway:    localhost:8080 (HTTP API)" -ForegroundColor White
    }

    if (!$SkipPocketBase -and $pbProcess) {
        Write-Host "   PocketBase: localhost:8090 (Database)" -ForegroundColor White
    }

    Write-Host ""
    Write-Host "How to Play:" -ForegroundColor Cyan
    Write-Host "   1. Open http://localhost:5173 in your browser" -ForegroundColor White
    Write-Host "   2. Click 'Play Game' to start" -ForegroundColor White
    Write-Host "   3. Click 'Login' for multiplayer features" -ForegroundColor White
    Write-Host "   4. Use SPACE, A/D, S keys to play" -ForegroundColor White
    Write-Host ""
    Write-Host "To Stop:" -ForegroundColor Red
    Write-Host "   - Close this PowerShell window" -ForegroundColor White
    Write-Host "   - Or press Ctrl+C" -ForegroundColor White
    Write-Host ""
    Write-Host "Happy gaming!" -ForegroundColor Green

    # Keep window alive and monitor processes
    try {
        while ($true) {
            Start-Sleep -Seconds 10

            # Check if essential processes are still running
            $essentialProcesses = @($workerProcess, $clientProcess)
            $running = 0

            foreach ($proc in $essentialProcesses) {
                if (!$proc.HasExited) {
                    $running++
                }
            }

            if ($running -lt 2) {
                Write-Host ""
                Write-Host "WARN Essential services have stopped!" -ForegroundColor Yellow
                Write-Host "   Worker running: $(!$workerProcess.HasExited)" -ForegroundColor White
                Write-Host "   Client running: $(!$clientProcess.HasExited)" -ForegroundColor White
                break
            }

            if ($Verbose) {
                Write-Host "." -NoNewline -ForegroundColor Gray
            }
        }
    }
    catch {
        # Ctrl+C or other interruption
    }
    finally {
        # Cleanup
        Write-Host ""
        Write-Host "Shutting down services..." -ForegroundColor Yellow

        $allProcesses = @($workerProcess, $clientProcess)
        if ($gatewayProcess) { $allProcesses += $gatewayProcess }
        if ($pbProcess) { $allProcesses += $pbProcess }

        foreach ($proc in $allProcesses) {
            if (!$proc.HasExited) {
                Write-Host "   Stopping $($proc.ProcessName)..." -ForegroundColor Gray
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            }
        }

        Write-Host "Shutdown complete!" -ForegroundColor Green
    }

}
catch {
    Write-Host ""
    Write-Host "FATAL ERROR: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "   1. Check if Rust is installed: cargo --version" -ForegroundColor White
    Write-Host "   2. Check if Node.js is installed: node --version" -ForegroundColor White
    Write-Host "   3. Check firewall/antivirus settings" -ForegroundColor White
    Write-Host "   4. Run with -Verbose flag for detailed output" -ForegroundColor White
    Write-Host "   5. Try running individual services manually" -ForegroundColor White
    Write-Host ""
    Write-Host "Need help? Check the troubleshooting guide." -ForegroundColor Cyan
    exit 1
}
