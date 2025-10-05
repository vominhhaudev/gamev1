# Final startup script for GameV1 3D Client
Write-Host "GAME V1 3D CLIENT STARTUP" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""

# Kill any existing processes on port 5173
Write-Host "Checking for existing processes..." -ForegroundColor Yellow
try {
    $processes = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
    if ($processes) {
        Write-Host "WARNING: Found existing processes on port 5173, killing them..." -ForegroundColor Red
        $processes | ForEach-Object {
            $pid = $_.OwningProcess
            if ($pid -and $pid -ne 0) {
                taskkill /F /PID $pid 2>$null
                Write-Host "Killed process PID: $pid" -ForegroundColor Red
            }
        }
    }
} catch {
    Write-Host "No existing processes found or unable to check" -ForegroundColor Yellow
}

# Kill any Node.js processes that might be related
Write-Host "Checking for Node.js processes..." -ForegroundColor Yellow
try {
    $nodeProcesses = Get-Process node -ErrorAction SilentlyContinue
    if ($nodeProcesses) {
        $nodeProcesses | ForEach-Object {
            taskkill /F /PID $_.Id 2>$null
            Write-Host "Killed Node.js process PID: $($_.Id)" -ForegroundColor Red
        }
    }
} catch {
    Write-Host "No Node.js processes found or unable to kill" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Starting fresh GameV1 3D Client..." -ForegroundColor Green
Write-Host ""
Write-Host "Game Features:" -ForegroundColor Yellow
Write-Host "  [OK] 3D Graphics with Three.js" -ForegroundColor Green
Write-Host "  [OK] Endless Runner Gameplay" -ForegroundColor Green
Write-Host "  [OK] Multiple Lanes & Obstacles" -ForegroundColor Green
Write-Host "  [OK] Jump and Collect Mechanics" -ForegroundColor Green
Write-Host "  [OK] Real-time Physics" -ForegroundColor Green
Write-Host ""

# Change to client directory
Set-Location "client"

# Start development server
Write-Host "Starting Vite dev server on port 5173..." -ForegroundColor Cyan
Write-Host "INFO: Access the 3D game at: http://localhost:5173/game" -ForegroundColor Yellow
npm run dev
