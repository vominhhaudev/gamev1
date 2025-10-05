# Test script to verify game fixes are working correctly

Write-Host "🎮 Testing GameV1 Fixes..." -ForegroundColor Green

# Test if client server is accessible
try {
    $response = Invoke-WebRequest -Uri "http://localhost:5173/game" -Method GET -TimeoutSec 10
    Write-Host "✅ Game page loads successfully" -ForegroundColor Green

    # Check if game container exists
    if ($response.Content -match "game3d-container") {
        Write-Host "✅ Game 3D container found in HTML" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Game 3D container not found in HTML" -ForegroundColor Yellow
    }

    # Check if Three.js is loaded
    if ($response.Content -match "three") {
        Write-Host "✅ Three.js library detected" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Three.js library not found" -ForegroundColor Yellow
    }

    # Check if EndlessRunner3D component is referenced
    if ($response.Content -match "EndlessRunner3D") {
        Write-Host "✅ EndlessRunner3D component found" -ForegroundColor Green
    } else {
        Write-Host "⚠️  EndlessRunner3D component not found" -ForegroundColor Yellow
    }

} catch {
    Write-Host "❌ Game page not accessible: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please make sure the client server is running:" -ForegroundColor Yellow
    Write-Host "  cd client" -ForegroundColor White
    Write-Host "  npm run dev" -ForegroundColor White
}

Write-Host ""
Write-Host "🔧 Troubleshooting Tips:" -ForegroundColor Yellow
Write-Host "  • Open http://localhost:5173/game in your browser" -ForegroundColor White
Write-Host "  • Press F12 to open browser console and check for errors" -ForegroundColor White
Write-Host "  • Look for 'emissive' errors which should now be fixed" -ForegroundColor White
Write-Host "  • Check if you can see the 3D track and player character" -ForegroundColor White

Write-Host ""
Write-Host "Press any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
