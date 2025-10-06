Write-Host "🚀 GAMEV1 - ONE-CLICK STARTUP SCRIPT v2.0" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host "🎮 Complete Game Development Platform" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Green

function Stop-AllServices {
    Write-Host "Stopping all services..." -ForegroundColor Yellow

    $processes = @("gateway", "pocketbase", "node")
    foreach ($process in $processes) {
        Get-Process -Name $process -ErrorAction SilentlyContinue | ForEach-Object {
            Write-Host "  Stopping $($_.ProcessName) (PID: $($_.Id))" -ForegroundColor Yellow
            Stop-Process -Id $_.Id -Force
        }
    }

    Start-Sleep -Seconds 2
    Write-Host "All services stopped" -ForegroundColor Green
}

function Start-AllServices {
    Write-Host "🚀 Starting GameV1 System..." -ForegroundColor Green
    Write-Host "================================================" -ForegroundColor Green

    # 1. Stop existing services first
    Write-Host "🛑 Stopping existing services..." -ForegroundColor Yellow
    Stop-AllServices

    # 2. Start PocketBase (Database)
    Write-Host "Starting PocketBase (port 8090)..." -ForegroundColor Cyan
    $pocketbasePath = Join-Path $PSScriptRoot "pocketbase\pocketbase.exe"
    if (Test-Path $pocketbasePath) {
        Start-Process $pocketbasePath -ArgumentList "serve", "--http=127.0.0.1:8090" -WindowStyle Hidden
        Start-Sleep -Seconds 3
        Write-Host "  ✅ PocketBase ready" -ForegroundColor Green
    } else {
        Write-Host "  ❌ PocketBase binary not found at $pocketbasePath" -ForegroundColor Red
        Write-Host "  💡 Run: pwsh -File scripts\setup-pocketbase.ps1" -ForegroundColor Yellow
    }

    # 3. Start Worker (Game Logic)
    Write-Host "Starting Worker (gRPC 50051)..." -ForegroundColor Cyan
    $workerPath = Join-Path $PSScriptRoot "worker"
    Set-Location $workerPath
    Start-Process "cargo" -ArgumentList "run" -WindowStyle Hidden
    Start-Sleep -Seconds 5
    Write-Host "  ✅ Worker ready" -ForegroundColor Green

    # 4. Start Gateway (HTTP API)
    Write-Host "🌐 Starting Gateway (port 8080)..." -ForegroundColor Cyan
    $gatewayPath = Join-Path $PSScriptRoot "gateway"
    Set-Location $gatewayPath
    Start-Process "cargo" -ArgumentList "run" -WindowStyle Hidden
    Start-Sleep -Seconds 5
    Write-Host "  ✅ Gateway ready" -ForegroundColor Green

    # 5. Start Client (Web UI)
    Write-Host "Starting Client (port 5173)..." -ForegroundColor Cyan
    $clientPath = Join-Path $PSScriptRoot "client"

    # Check if client directory exists
    if (!(Test-Path $clientPath)) {
        Write-Host "  ❌ Client directory not found at $clientPath" -ForegroundColor Red
        Write-Host "  💡 Make sure you're running this script from the gamev1 root directory" -ForegroundColor Yellow
        Write-Host "  📁 Expected structure: gamev1/client/, gamev1/worker/, gamev1/gateway/" -ForegroundColor Yellow
    } else {
        Set-Location $clientPath

        # Check if node_modules exists, install if not
        if (!(Test-Path "node_modules")) {
            Write-Host "  Installing dependencies..." -ForegroundColor Yellow
            & npm install
        }

        # Try to start with batch file first (more reliable)
        $batchFile = Join-Path $clientPath "start-client.bat"
        if (Test-Path $batchFile) {
            Write-Host "  Using start-client.bat for better reliability..." -ForegroundColor Cyan
            Start-Process $batchFile -WindowStyle Hidden
        } else {
            Write-Host "  Using npm run dev..." -ForegroundColor Cyan
            Start-Process "npm" -ArgumentList "run", "dev" -WindowStyle Hidden
        }

        Start-Sleep -Seconds 8  # Give more time for Client to start
        Write-Host "  Client ready at http://localhost:5173" -ForegroundColor Green
    }

    # Back to root directory
    Set-Location $PSScriptRoot

    Write-Host ""
    Show-Status
}

function Show-Status {
    Write-Host ""
    Write-Host "SYSTEM STATUS:" -ForegroundColor Green
    Write-Host "=========================================" -ForegroundColor Green

    $services = @(
        @{Name="PocketBase"; URL="http://localhost:8090/api/health"; Port="8090"},
        @{Name="Gateway"; URL="http://localhost:8080/healthz"; Port="8080"},
        @{Name="Client"; URL="http://localhost:5173"; Port="5173"},
        @{Name="Client_Alt"; URL="http://localhost:5174"; Port="5174"}
    )

    foreach ($service in $services) {
        # Skip Client_Alt in main display
        if ($service.Name -eq "Client_Alt") { continue }

        try {
            $response = Invoke-WebRequest -Uri $service.URL -Method GET -TimeoutSec 5
            $status = if ($response.StatusCode -eq 200) { "✅ RUNNING" } else { "❌ ERROR" }
            $color = if ($response.StatusCode -eq 200) { "Green" } else { "Red" }
        }
        catch {
            $status = "⏳ NOT RESPONDING"
            $color = "Yellow"
        }

        Write-Host "  $($service.Name) (port $($service.Port)): $status" -ForegroundColor $color
    }

    # Check alternative Client port if main port not responding
    $mainClient = $services | Where-Object { $_.Name -eq "Client" }
    $altClient = $services | Where-Object { $_.Name -eq "Client_Alt" }

    try {
        $response = Invoke-WebRequest -Uri $mainClient.URL -Method GET -TimeoutSec 3
    }
    catch {
        try {
            $response = Invoke-WebRequest -Uri $altClient.URL -Method GET -TimeoutSec 3
            Write-Host "  Client (port $($altClient.Port)): ✅ RUNNING (auto-switched)" -ForegroundColor Green
        }
        catch {
            Write-Host "  Client: ❌ NOT RUNNING" -ForegroundColor Red
        }
    }

    Write-Host ""
    Write-Host "ACCESS POINTS:" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "  Client Web:     http://localhost:5173 (or 5174 if 5173 is busy)" -ForegroundColor White
    Write-Host "  Gateway API:    http://localhost:8080" -ForegroundColor White
    Write-Host "  Metrics:        http://localhost:8080/metrics" -ForegroundColor White
    Write-Host "  Health Check:   http://localhost:8080/healthz" -ForegroundColor White
    Write-Host "  PocketBase:     http://localhost:8090/_/" -ForegroundColor White
    Write-Host "  WebSocket:      ws://localhost:8080/ws" -ForegroundColor White
    Write-Host "" -ForegroundColor Cyan
    Write-Host "PocketBase Admin: admin@pocketbase.local / 123456789" -ForegroundColor Yellow
}

function Show-Help {
    Write-Host "🚀 GAMEV1 - ONE-CLICK STARTUP SCRIPT v2.1" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "📖 USAGE:" -ForegroundColor Yellow
    Write-Host "  .\restart-all-services-simple.ps1       # Start all services" -ForegroundColor Cyan
    Write-Host "  .\restart-all-services-simple.ps1 -Stop    # Stop all services" -ForegroundColor Cyan
    Write-Host "  .\restart-all-services-simple.ps1 -Restart # Restart all services" -ForegroundColor Cyan
    Write-Host "  .\restart-all-services-simple.ps1 -Status  # Check status" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "🔧 MANUAL STARTUP:" -ForegroundColor Yellow
    Write-Host "  Terminal 1: pwsh -File scripts/run-service.ps1 pocketbase" -ForegroundColor Cyan
    Write-Host "  Terminal 2: pwsh -File scripts/run-service.ps1 worker" -ForegroundColor Cyan
    Write-Host "  Terminal 3: pwsh -File scripts/run-service.ps1 gateway" -ForegroundColor Cyan
    Write-Host "  Terminal 4: cd client; .\start-client.bat" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "🔧 ALTERNATIVE STARTUP (if script fails):" -ForegroundColor Yellow
    Write-Host "  1. pwsh -File scripts/run-service.ps1 worker" -ForegroundColor Cyan
    Write-Host "  2. pwsh -File scripts/run-service.ps1 gateway" -ForegroundColor Cyan
    Write-Host "  3. cd client; npm run dev" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "🔍 ACCESS POINTS:" -ForegroundColor Yellow
    Write-Host "  Client:     http://localhost:5173" -ForegroundColor White
    Write-Host "  Gateway:    http://localhost:8080" -ForegroundColor White
    Write-Host "  PocketBase: http://localhost:8090/_/" -ForegroundColor White
    Write-Host ""
    Write-Host "🛠️ TROUBLESHOOTING:" -ForegroundColor Yellow
    Write-Host "  1. Close all PowerShell terminals" -ForegroundColor Cyan
    Write-Host "  2. Open new terminal in project root" -ForegroundColor Cyan
    Write-Host "  3. Run: .\restart-all-services-simple.ps1" -ForegroundColor Cyan
    Write-Host "  4. If Node.js errors: Run 'npm install' in client folder" -ForegroundColor Cyan
    Write-Host "  5. If Client doesn't start: Try cd client; .\start-client.bat" -ForegroundColor Cyan
    Write-Host "  6. If port 5173 busy: Client auto-switches to 5174" -ForegroundColor Cyan
    Write-Host "  7. If Rust errors: Run 'cargo build' in each service folder" -ForegroundColor Cyan
    Write-Host "  8. If proto file not found: Make sure you're in the correct directory" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "📁 EXPECTED PROJECT STRUCTURE:" -ForegroundColor Yellow
    Write-Host "  gamev1/" -ForegroundColor White
    Write-Host "  ├── client/     # Frontend (SvelteKit)" -ForegroundColor White
    Write-Host "  ├── worker/     # Game logic (Rust)" -ForegroundColor White
    Write-Host "  ├── gateway/    # API gateway (Rust)" -ForegroundColor White
    Write-Host "  ├── proto/      # Protocol definitions" -ForegroundColor White
    Write-Host "  └── scripts/    # PowerShell scripts" -ForegroundColor White
}

# Main logic

if ($Stop) {
    Stop-AllServices
}
elseif ($Restart) {
    Start-AllServices
}
elseif ($Status) {
    Show-Status
}
else {
    Start-AllServices
}
}
