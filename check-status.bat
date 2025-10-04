@echo off
echo 🔍 Checking GameV1 System Status
echo ================================

echo.
echo 🌐 Checking Ports:
netstat -an | findstr :50051 >nul && echo ✅ Worker (50051): Running || echo ❌ Worker (50051): Not running
netstat -an | findstr :8080 >nul && echo ✅ Gateway (8080): Running || echo ❌ Gateway (8080): Not running
netstat -an | findstr :5173 >nul && echo ✅ Client (5173): Running || echo ❌ Client (5173): Not running
netstat -an | findstr :8090 >nul && echo ✅ PocketBase (8090): Running || echo ❌ PocketBase (8090): Not running

echo.
echo 🔧 Checking Processes:
powershell -Command "Get-Process -Name cargo,node,pocketbase -ErrorAction SilentlyContinue | Select-Object Name,Id"

echo.
echo 🌐 Testing Endpoints:
powershell -Command "
try { Invoke-WebRequest -Uri http://localhost:8080/healthz -TimeoutSec 3 -ErrorAction Stop; Write-Host '✅ Gateway Health: OK' }
catch { Write-Host '❌ Gateway Health: Failed' }

try { Invoke-WebRequest -Uri http://localhost:8080/api/rooms/list -TimeoutSec 3 -ErrorAction Stop; Write-Host '✅ Gateway Rooms API: OK' }
catch { Write-Host '❌ Gateway Rooms API: Failed' }

try { Invoke-WebRequest -Uri http://localhost:5173 -TimeoutSec 3 -ErrorAction Stop; Write-Host '✅ Client: OK' }
catch { Write-Host '❌ Client: Failed' }

try { Invoke-WebRequest -Uri http://localhost:5173/rooms -TimeoutSec 5 -ErrorAction Stop; Write-Host '✅ Rooms Route: OK' }
catch { Write-Host '❌ Rooms Route: Failed (500 error)' }
"

echo.
echo ========================================
echo 📋 Quick Actions:
echo   Start all:     .\start-all.bat
echo   Stop all:      .\scripts\stop-all.bat
echo   Restart:       .\restart-all-services-simple.ps1 -Restart
echo ========================================

echo.
echo Press any key to exit...
pause >nul
