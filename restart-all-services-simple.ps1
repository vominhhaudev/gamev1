param(
    [switch]$Stop,
    [switch]$Restart,
    [switch]$Status
)

Write-Host "GAMEV1 - ONE-CLICK STARTUP SCRIPT" -ForegroundColor Green
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
    Write-Host "Starting GameV1 System..." -ForegroundColor Green

    # 1. Stop existing services
    Stop-AllServices

    # 2. Start PocketBase
    Write-Host "Starting PocketBase (port 8090)..." -ForegroundColor Cyan
    try {
        Start-Process "C:\Users\Fit\Downloads\gamev1\pocketbase\pocketbase.exe" -ArgumentList "serve", "--http=127.0.0.1:8090" -WindowStyle Hidden
        Start-Sleep -Seconds 3
        Write-Host "  PocketBase ready" -ForegroundColor Green
    }
    catch {
        Write-Host "  Error starting PocketBase: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }

    # 3. Start Gateway
    Write-Host "Starting Gateway (port 8080)..." -ForegroundColor Cyan
    try {
        $env:PATH = "C:\Program Files\nodejs;$env:PATH"
        Set-Location "C:\Users\Fit\Downloads\gamev1\gateway"
        Start-Process "cargo" -ArgumentList "run" -WindowStyle Hidden
        Start-Sleep -Seconds 5
        Write-Host "  Gateway ready" -ForegroundColor Green
    }
    catch {
        Write-Host "  Error starting Gateway: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }

    # 4. Start Client
    Write-Host "Starting Client (port 5173)..." -ForegroundColor Cyan
    try {
        $env:PATH = "C:\Program Files\nodejs;$env:PATH"
        Set-Location "C:\Users\Fit\Downloads\gamev1\client"
        Start-Process "npm" -ArgumentList "run", "dev" -WindowStyle Hidden
        Start-Sleep -Seconds 5
        Write-Host "  Client ready" -ForegroundColor Green
    }
    catch {
        Write-Host "  Error starting Client: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }

    Show-Status
}

function Show-Status {
    Write-Host ""
    Write-Host "SYSTEM STATUS:" -ForegroundColor Green
    Write-Host "=========================================" -ForegroundColor Green

    $services = @(
        @{Name="PocketBase"; URL="http://localhost:8090/api/health"},
        @{Name="Gateway"; URL="http://localhost:8080/healthz"},
        @{Name="Client"; URL="http://localhost:5173"}
    )

    foreach ($service in $services) {
        try {
            $response = Invoke-WebRequest -Uri $service.URL -Method GET -TimeoutSec 3
            $status = if ($response.StatusCode -eq 200) { "RUNNING" } else { "ERROR" }
            $color = if ($response.StatusCode -eq 200) { "Green" } else { "Red" }
        }
        catch {
            $status = "NOT RESPONDING"
            $color = "Red"
        }

        Write-Host "  $($service.Name): $status" -ForegroundColor $color
    }

    Write-Host "=========================================" -ForegroundColor Green
    Write-Host "Access: http://localhost:5173" -ForegroundColor Cyan
    Write-Host "Login: admin@pocketbase.local / 123456789" -ForegroundColor Cyan
}

function Show-Help {
    Write-Host "GAMEV1 - ONE-CLICK STARTUP SCRIPT" -ForegroundColor Green
    Write-Host "=========================================" -ForegroundColor Green
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  .\restart-all-services-simple.ps1       # Start all services" -ForegroundColor Cyan
    Write-Host "  .\restart-all-services-simple.ps1 -Stop    # Stop all services" -ForegroundColor Cyan
    Write-Host "  .\restart-all-services-simple.ps1 -Restart # Restart all services" -ForegroundColor Cyan
    Write-Host "  .\restart-all-services-simple.ps1 -Status  # Check status" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Manual commands if needed:" -ForegroundColor Yellow
    Write-Host "  cd pocketbase && .\pocketbase.exe serve --http=127.0.0.1:8090" -ForegroundColor Cyan
    Write-Host "  cd gateway && cargo run" -ForegroundColor Cyan
    Write-Host "  cd client && npm run dev" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  1. Close all terminals" -ForegroundColor Cyan
    Write-Host "  2. Open new terminal" -ForegroundColor Cyan
    Write-Host "  3. Run: .\restart-all-services-simple.ps1" -ForegroundColor Cyan
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
