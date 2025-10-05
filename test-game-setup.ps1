image.png# Test script to verify all game services are running correctly

Write-Host "🎮 Testing GameV1 Setup..." -ForegroundColor Green

# Test if all required ports are accessible
$services = @(
    @{Name="Client Server"; Port=5173; Url="http://localhost:5173"},
    @{Name="Gateway"; Port=8080; Url="http://localhost:8080/healthz"},
    @{Name="PocketBase"; Port=8090; Url="http://localhost:8090"},
    @{Name="Worker"; Port=50051; Url="http://localhost:50051"}
)

$allServicesRunning = $true

foreach ($service in $services) {
    try {
        $response = Invoke-WebRequest -Uri $service.Url -Method GET -TimeoutSec 5
        Write-Host "✅ $($service.Name): Running (Port $($service.Port))" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ $($service.Name): Not accessible (Port $($service.Port))" -ForegroundColor Red
        $allServicesRunning = $false
    }
}

if ($allServicesRunning) {
    Write-Host ""
    Write-Host "🎉 All services are running correctly!" -ForegroundColor Green
    Write-Host ""
    Write-Host "🌐 Access your game at: http://localhost:5173/game" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "📊 Services Status:" -ForegroundColor Yellow
    Write-Host "  • Client Server (Frontend): http://localhost:5173" -ForegroundColor White
    Write-Host "  • Gateway (API): http://localhost:8080" -ForegroundColor White
    Write-Host "  • PocketBase (Database): http://localhost:8090" -ForegroundColor White
    Write-Host "  • Worker (Game Logic): http://localhost:50051" -ForegroundColor White
    Write-Host ""
    Write-Host "🔧 Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  • If you see CORS errors, try hard refresh (Ctrl+F5)" -ForegroundColor White
    Write-Host "  • If connection fails, check that all services are running" -ForegroundColor White
    Write-Host "  • Check browser console (F12) for detailed error messages" -ForegroundColor White
}
else {
    Write-Host ""
    Write-Host "⚠️  Some services are not running. Please check the logs above." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "To start all services:" -ForegroundColor Cyan
    Write-Host "  1. Start PocketBase: .\pocketbase\pocketbase.exe serve" -ForegroundColor White
    Write-Host "  2. Start Worker: .\worker\target\debug\worker.exe" -ForegroundColor White
    Write-Host "  3. Start Gateway: .\gateway\target\debug\gateway.exe" -ForegroundColor White
    Write-Host "  4. Start Client: .\client\npm run dev" -ForegroundColor White
}

Write-Host ""
Write-Host "Press any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

# Test game functionality if requested
if ($TestGame) {
    Write-Host ""
    Write-Host "🎯 Testing game functionality..." -ForegroundColor Magenta

    try {
        $gameResponse = Invoke-WebRequest -Uri "http://localhost:5173/game" -Method GET -TimeoutSec 10
        Write-Host "✅ Game page loads successfully" -ForegroundColor Green

        # Check if game container exists
        if ($gameResponse.Content -match "game3d-container") {
            Write-Host "✅ Game 3D container found in HTML" -ForegroundColor Green
        } else {
            Write-Host "⚠️  Game 3D container not found in HTML" -ForegroundColor Yellow
        }

    } catch {
        Write-Host "❌ Game page not accessible: $($_.Exception.Message)" -ForegroundColor Red
    }
}