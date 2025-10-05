# Test script to verify all game services are running correctly

Write-Host "🧪 Testing GameV1 Services..." -ForegroundColor Green

# Test if all required ports are accessible
$services = @(
    @{Name="Gateway Health"; Port=8080; Url="http://localhost:8080/healthz"},
    @{Name="Gateway Test"; Port=8080; Url="http://localhost:8080/test"},
    @{Name="Client Server"; Port=5173; Url="http://localhost:5173"},
    @{Name="PocketBase"; Port=8090; Url="http://localhost:8090"},
    @{Name="Worker"; Port=50051; Url="http://localhost:50051"}
)

$allServicesRunning = $true

foreach ($service in $services) {
    try {
        Write-Host "Testing $($service.Name)..." -ForegroundColor Yellow
        $response = Invoke-WebRequest -Uri $service.Url -Method GET -TimeoutSec 5
        Write-Host "✅ $($service.Name): Running (Port $($service.Port)) - Status: $($response.StatusCode)" -ForegroundColor Green

        # Check CORS headers for gateway endpoints
        if ($service.Port -eq 8080) {
            $corsOrigin = $response.Headers["Access-Control-Allow-Origin"]
            $corsMethods = $response.Headers["Access-Control-Allow-Methods"]
            if ($corsOrigin -and $corsMethods) {
                Write-Host "  CORS Headers: ✅ Origin: $corsOrigin, Methods: $corsMethods" -ForegroundColor Green
            } else {
                Write-Host "  CORS Headers: ❌ Missing" -ForegroundColor Red
                $allServicesRunning = $false
            }
        }
    }
    catch {
        Write-Host "❌ $($service.Name): Not accessible (Port $($service.Port)) - Error: $($_.Exception.Message)" -ForegroundColor Red
        $allServicesRunning = $false
    }
}

Write-Host ""
if ($allServicesRunning) {
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
    Write-Host "⚠️  Some services are not running. Please check the logs above." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Press any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
