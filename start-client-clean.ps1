# Clean startup script for GameV1 Client
Write-Host "🧹 CLEAN STARTUP: GameV1 Client" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""

# Kill any existing processes on port 5173
Write-Host "🔍 Checking for existing processes..." -ForegroundColor Yellow
$processes = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
if ($processes) {
    Write-Host "⚠️ Found existing processes on port 5173, killing them..." -ForegroundColor Red
    $processes | ForEach-Object {
        $pid = $_.OwningProcess
        taskkill /F /PID $pid 2>$null
        Write-Host "Killed process PID: $pid" -ForegroundColor Red
    }
}

# Kill any Node.js processes that might be related
Write-Host "🔍 Checking for Node.js processes..." -ForegroundColor Yellow
$nodeProcesses = Get-Process node -ErrorAction SilentlyContinue
if ($nodeProcesses) {
    $nodeProcesses | ForEach-Object {
        taskkill /F /PID $_.Id 2>$null
        Write-Host "Killed Node.js process PID: $($_.Id)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "🚀 Starting fresh GameV1 Client..." -ForegroundColor Green
Write-Host ""
Write-Host "🎮 Game Features:" -ForegroundColor Yellow
Write-Host "  ✅ 3D Graphics with Three.js" -ForegroundColor Green
Write-Host "  ✅ Endless Runner Gameplay" -ForegroundColor Green
Write-Host "  ✅ Multiple Lanes & Obstacles" -ForegroundColor Green
Write-Host "  ✅ Jump and Collect Mechanics" -ForegroundColor Green
Write-Host ""

# Change to client directory
Set-Location "client"

# Start development server
Write-Host "🔄 Starting Vite dev server on port 5173..." -ForegroundColor Cyan
Write-Host "📍 Access the game at: http://localhost:5173/game" -ForegroundColor Yellow
npm run dev
