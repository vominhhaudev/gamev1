@echo off
echo 🚀 Starting GameV1 Project - Complete Startup Script
echo ==================================================

echo 🛑 Stopping existing services...
powershell -Command "Get-Process -Name 'cargo','node','pocketbase' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue"

echo.
echo 🗄️ Starting PocketBase (Database)...
powershell -Command "& '.\scripts\run-service.ps1' -Service pocketbase"
timeout /t 3 /nobreak > nul

echo.
echo ⚙️ Starting Worker (Game Logic)...
powershell -Command "& '.\scripts\run-service.ps1' -Service worker"
timeout /t 5 /nobreak > nul

echo.
echo 🌐 Starting Gateway (API Server)...
powershell -Command "& '.\scripts\run-service.ps1' -Service gateway"
timeout /t 5 /nobreak > nul

echo.
echo 🖥️ Starting Client (Web UI)...
cd client
if not exist node_modules (
    echo Installing client dependencies...
    npm install
)
start /B npm run dev
cd ..

echo.
echo ==================================================
echo ✅ ALL SERVICES STARTED SUCCESSFULLY!
echo ==================================================
echo.
echo 🌐 Access Points:
echo   🏠 Client:     http://localhost:5173
echo   🌐 Gateway:    http://localhost:8080
echo   🗄️ PocketBase: http://localhost:8090/_/
echo.
echo 🎮 Quick Access:
echo   - Game:        http://localhost:5173/game
echo   - Network Test: http://localhost:5173/net-test
echo   - Admin Panel:  http://localhost:8090/_/
echo.
echo 🔑 Admin Login:
echo   Email: vominhhauviettel@gmail.com
echo   Password: pt123456789
echo.
echo 📋 Troubleshooting:
echo   - If any service fails, run this script again
echo   - Check logs in terminal windows for errors
echo   - Make sure no other apps use ports 8080, 8090, 5173
echo.
echo Press any key to close this window...
pause > nul
