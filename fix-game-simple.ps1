# Fix GameV1 Issues Script
Write-Host "Fixing GameV1 Issues..." -ForegroundColor Green

# Stop all running processes
Write-Host "Stopping existing services..." -ForegroundColor Yellow
Get-Process -Name "cargo","node","pocketbase","powershell" -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "*gamev1*" } | Stop-Process -Force -ErrorAction SilentlyContinue

# Start services in correct order
Write-Host "Starting services..." -ForegroundColor Green

# 1. Start PocketBase first
Write-Host "Starting PocketBase..." -ForegroundColor Cyan
Start-Process -FilePath "pocketbase\pocketbase.exe" -ArgumentList "serve","--http=127.0.0.1:8090","--dir=./pb_data" -NoNewWindow

# Wait for PocketBase
Start-Sleep -Seconds 3

# 2. Start Worker service
Write-Host "Starting Worker..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-File","scripts\run-service.ps1","-Service","worker" -NoNewWindow

# Wait for Worker
Start-Sleep -Seconds 5

# 3. Start Gateway service
Write-Host "Starting Gateway..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-File","scripts\run-service.ps1","-Service","gateway" -NoNewWindow

# Wait for Gateway
Start-Sleep -Seconds 5

# 4. Start Client service
Write-Host "Starting Client..." -ForegroundColor Cyan
Start-Process -FilePath "powershell" -ArgumentList "-NoProfile","-Command","cd client; npm run dev" -NoNewWindow

Write-Host "All services started!" -ForegroundColor Green
Write-Host "" -ForegroundColor Green
Write-Host "Access Points:" -ForegroundColor Yellow
Write-Host "   Client:     http://localhost:5173" -ForegroundColor White
Write-Host "   Gateway:    http://localhost:8080" -ForegroundColor White
Write-Host "   PocketBase: http://localhost:8090" -ForegroundColor White
Write-Host "" -ForegroundColor Green
Write-Host "Game should now be accessible at: http://localhost:5173/game" -ForegroundColor Yellow
