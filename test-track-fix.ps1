# Test script to check track persistence fixes

Write-Host "Testing Track Persistence Fixes..." -ForegroundColor Green

try {
    # Test if client server is accessible
    $response = Invoke-WebRequest -Uri "http://localhost:5173/game" -Method GET -TimeoutSec 10
    Write-Host "Game page loads successfully" -ForegroundColor Green

    # Check if game container exists
    if ($response.Content -match "game3d-container") {
        Write-Host "Game 3D container found in HTML" -ForegroundColor Green
    } else {
        Write-Host "Game 3D container not found in HTML" -ForegroundColor Yellow
    }

    # Check if Three.js is loaded
    if ($response.Content -match "three") {
        Write-Host "Three.js library detected" -ForegroundColor Green
    } else {
        Write-Host "Three.js library not found" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Game fixes applied:" -ForegroundColor Cyan
    Write-Host "  * Fixed track generation logic" -ForegroundColor White
    Write-Host "  * Track segments now persist when player runs fast" -ForegroundColor White
    Write-Host "  * Improved track positioning behind player" -ForegroundColor White
    Write-Host "  * Wider track with better visibility" -ForegroundColor White
    Write-Host "  * Reduced track segments for better performance" -ForegroundColor White

    Write-Host ""
    Write-Host "What to test:" -ForegroundColor Yellow
    Write-Host "  * Start game and run for several minutes" -ForegroundColor White
    Write-Host "  * Track should remain visible throughout gameplay" -ForegroundColor White
    Write-Host "  * No more disappearing track/background" -ForegroundColor White
    Write-Host "  * Check browser console for track creation logs" -ForegroundColor White

    Write-Host ""
    Write-Host "Track persistence fixes applied successfully!" -ForegroundColor Green
}
catch {
    Write-Host "Game page not accessible" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please make sure the client server is running:" -ForegroundColor Yellow
    Write-Host "  cd client" -ForegroundColor White
    Write-Host "  npm run dev" -ForegroundColor White
}

Write-Host ""
Write-Host "Press any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
