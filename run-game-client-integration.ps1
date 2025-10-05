Write-Host "🚀 Starting GameV1 Integration Test" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Yellow

# Đợi một chút để user có thể đọc
Start-Sleep -Seconds 2

Write-Host "📋 Checking port availability..." -ForegroundColor Cyan

# Function để kiểm tra port available
function Test-PortAvailable {
    param($Port)
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $tcpClient.Connect("localhost", $Port)
        $tcpClient.Close()
        return $false
    }
    catch {
        return $true
    }
}

# Kiểm tra ports cần thiết
$workerPort = 50051
$clientPort = 5173

if (-not (Test-PortAvailable -Port $workerPort)) {
    Write-Host "❌ Port $workerPort (gRPC) is already in use!" -ForegroundColor Red
    Write-Host "Please stop any existing worker process and try again." -ForegroundColor Yellow
    exit 1
}

if (-not (Test-PortAvailable -Port $clientPort)) {
    Write-Host "❌ Port $clientPort (Client) is already in use!" -ForegroundColor Red
    Write-Host "Please stop any existing client process and try again." -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ All required ports are available" -ForegroundColor Green

# Start worker in background
Write-Host "🔧 Starting game worker..." -ForegroundColor Cyan

$workerJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    cargo run --bin worker
} -Name "GameWorker"

Write-Host "✅ Game worker started (PID: $($workerJob.Id))" -ForegroundColor Green

# Wait for worker to start
Write-Host "⏳ Waiting for worker to initialize..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Start client
Write-Host "🌐 Starting game client..." -ForegroundColor Cyan

# Install client dependencies if needed
if (-not (Test-Path "client\node_modules")) {
    Write-Host "📦 Installing client dependencies..." -ForegroundColor Yellow
    Set-Location "client"
    npm install
    Set-Location ".."
}

# Start client in new window
Set-Location "client"
$clientProcess = Start-Process "npm" -ArgumentList "run dev" -PassThru
Set-Location ".."

Write-Host "✅ Game client started (PID: $($clientProcess.Id))" -ForegroundColor Green

Write-Host ""
Write-Host "🎮 Integration test setup complete!" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Yellow
Write-Host ""
Write-Host "📍 Access points:" -ForegroundColor Cyan
Write-Host "   🌐 Client: http://localhost:5173" -ForegroundColor White
Write-Host "   🎯 Game:   http://localhost:5173/game" -ForegroundColor White
Write-Host ""
Write-Host "🔧 Services running:" -ForegroundColor Cyan
Write-Host "   ⚙️  Worker (gRPC): localhost:50051" -ForegroundColor White
Write-Host "   🌐 Client (Web): localhost:5173" -ForegroundColor White
Write-Host ""
Write-Host "📋 Instructions:" -ForegroundColor Cyan
Write-Host "   1. Open http://localhost:5173 in your browser" -ForegroundColor White
Write-Host "   2. Click '🎮 Play Game' to start" -ForegroundColor White
Write-Host "   3. Click 'Join Game' to connect to worker" -ForegroundColor White
Write-Host "   4. Use WASD to move, Shift to sprint" -ForegroundColor White
Write-Host ""
Write-Host "🛑 To stop:" -ForegroundColor Red
Write-Host "   - Close browser tab" -ForegroundColor White
Write-Host "   - Press Ctrl+C in this terminal" -ForegroundColor White
Write-Host "   - Or close this PowerShell window" -ForegroundColor White
Write-Host ""
Write-Host "🚀 Happy gaming!" -ForegroundColor Green

# Wait for user input to stop
try {
    while ($true) {
        $input = Read-Host
        if ($input -eq "stop" -or $input -eq "quit" -or $input -eq "exit") {
            break
        }
    }
}
catch {
    # User pressed Ctrl+C
}
finally {
    # Cleanup
    Write-Host ""
    Write-Host "🛑 Stopping services..." -ForegroundColor Yellow

    # Stop worker
    if ($workerJob) {
        Stop-Job -Job $workerJob
        Remove-Job -Job $workerJob -Force
        Write-Host "✅ Worker stopped" -ForegroundColor Green
    }

    # Stop client
    if ($clientProcess -and !$clientProcess.HasExited) {
        Stop-Process -Id $clientProcess.Id -Force
        Write-Host "✅ Client stopped" -ForegroundColor Green
    }

    Write-Host "👋 Integration test completed!" -ForegroundColor Green
}
