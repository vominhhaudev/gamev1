# Script khởi động client đúng cách cho PowerShell
Write-Host "🚀 GAME V1 - CORRECT CLIENT STARTUP SCRIPT" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "🔧 Starting the correct game client..." -ForegroundColor Yellow
Write-Host ""

Write-Host "✅ Correct URL: http://localhost:5173/game" -ForegroundColor Green
Write-Host "❌ Wrong URL: http://localhost:5173/game3d (doesn't exist)" -ForegroundColor Red
Write-Host ""

# Thay đổi thư mục và chạy server
Set-Location "client"
Write-Host "🔄 Starting development server..." -ForegroundColor Cyan
npm run dev
