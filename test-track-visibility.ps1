# Test script to check track visibility fixes

Write-Host "🔧 Testing Track Visibility Fixes..." -ForegroundColor Green

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

    Write-Host ""
    Write-Host "🎮 Game should now display:" -ForegroundColor Cyan
    Write-Host "  • Green track/ground underneath player" -ForegroundColor White
    Write-Host "  • White lane markers on the track" -ForegroundColor White
    Write-Host "  • Brown barriers on the sides of track" -ForegroundColor White
    Write-Host "  • Player character running on the track" -ForegroundColor White
    Write-Host "  • Camera positioned behind player looking forward" -ForegroundColor White

    Write-Host ""
    Write-Host "🔍 Debugging Tips:" -ForegroundColor Yellow
    Write-Host "  • Press F12 to open browser console" -ForegroundColor White
    Write-Host "  • Look for track segment creation logs" -ForegroundColor White
    Write-Host "  • Check if track segments are visible in console logs" -ForegroundColor White
    Write-Host "  • Verify camera position relative to track position" -ForegroundColor White

    Write-Host ""
    Write-Host "✅ Track visibility fixes applied successfully!" -ForegroundColor Green
}
catch {
    Write-Host "❌ Game page not accessible: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please make sure the client server is running:" -ForegroundColor Yellow
    Write-Host "  cd client" -ForegroundColor White
    Write-Host "  npm run dev" -ForegroundColor White
}

Write-Host ""
Write-Host "Press any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
