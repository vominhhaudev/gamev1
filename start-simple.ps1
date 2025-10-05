Write-Host "🚀 Starting GameV1 Project" -ForegroundColor Green
Write-Host "==========================" -ForegroundColor Yellow

# Start worker
Write-Host "🔧 Starting game worker..." -ForegroundColor Cyan
Start-Job -ScriptBlock {
    Set-Location $using:PWD
    cargo run --bin worker
} -Name "GameWorker" | Out-Null

Write-Host "✅ Game worker started" -ForegroundColor Green

# Wait for worker to start
Write-Host "⏳ Waiting for worker..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Start client
Write-Host "🌐 Starting game client..." -ForegroundColor Cyan
Set-Location "client"
Start-Process "npm" -ArgumentList "run dev" -PassThru | Out-Null
Set-Location ".."

Write-Host "✅ Game client started" -ForegroundColor Green

Write-Host ""
Write-Host "🎮 Project started successfully!" -ForegroundColor Green
Write-Host "============================" -ForegroundColor Yellow
Write-Host ""
Write-Host "📍 Access points:" -ForegroundColor Cyan
Write-Host "   🌐 Client: http://localhost:5173" -ForegroundColor White
Write-Host "   🎯 Game:   http://localhost:5173/game" -ForegroundColor White
Write-Host ""
Write-Host "🔧 Services running:" -ForegroundColor Cyan
Write-Host "   ⚙️  Worker (gRPC): localhost:50051" -ForegroundColor White
Write-Host "   🌐 Client (Web): localhost:5173" -ForegroundColor White
Write-Host ""
Write-Host "🛑 To stop: Close this window or press Ctrl+C" -ForegroundColor Red

# Keep window open
try {
    while ($true) {
        Start-Sleep -Seconds 1
    }
}
catch {
    Write-Host "🛑 Stopping services..." -ForegroundColor Yellow
    Get-Job | Stop-Job
    Get-Job | Remove-Job
}
