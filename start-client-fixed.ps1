# Fixed PowerShell script for starting client
Write-Host "🚀 Starting GameV1 Client..." -ForegroundColor Green
Write-Host "================================" -ForegroundColor Cyan

# Change to client directory
Set-Location "client"

# Start development server
Write-Host "🔄 Starting Vite dev server on port 5173..." -ForegroundColor Yellow
npm run dev
