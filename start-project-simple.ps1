# GameV1 Project - Simple Startup Script
Write-Host "Starting GameV1 Project..." -ForegroundColor Green

# Stop existing processes
Write-Host "Stopping existing services..." -ForegroundColor Yellow
Get-Process -Name "cargo","node","pocketbase" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# Start services in order
Write-Host "Starting PocketBase..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-File","scripts\run-service.ps1","-Service","pocketbase" -NoNewWindow

Start-Sleep -Seconds 3

Write-Host "Starting Worker..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-File","scripts\run-service.ps1","-Service","worker" -NoNewWindow

Start-Sleep -Seconds 5

Write-Host "Starting Gateway..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-File","scripts\run-service.ps1","-Service","gateway" -NoNewWindow

Start-Sleep -Seconds 5

Write-Host "Starting Client..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-NoProfile","-Command","cd client; npm run dev" -NoNewWindow

Write-Host "All services started!" -ForegroundColor Green
Write-Host ""
Write-Host "Access Points:" -ForegroundColor Yellow
Write-Host "  Client:     http://localhost:5173" -ForegroundColor White
Write-Host "  Gateway:    http://localhost:8080" -ForegroundColor White
Write-Host "  PocketBase: http://localhost:8090/_/" -ForegroundColor White
Write-Host ""
Write-Host "Quick Access:" -ForegroundColor Yellow
Write-Host "  - Game:        http://localhost:5173/game" -ForegroundColor White
Write-Host "  - Network Test: http://localhost:5173/net-test" -ForegroundColor White
Write-Host "  - Admin Panel:  http://localhost:8090/_/" -ForegroundColor White
Write-Host ""
Write-Host "Admin Login:" -ForegroundColor Yellow
Write-Host "  Email: vominhhauviettel@gmail.com" -ForegroundColor White
Write-Host "  Password: pt123456789" -ForegroundColor White
