@echo off
echo 🚀 GAMEV1 - Complete Startup Script v2.1
echo =========================================
echo.

echo 🛑 Stopping existing services...
powershell -Command "Get-Process -Name 'cargo','pocketbase','node' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue"

echo.
echo 🗄️ Starting PocketBase (Database)...
start /B powershell -Command "& '.\scripts\run-service.ps1' -Service pocketbase"
timeout /t 3 /nobreak > nul

echo.
echo ⚙️ Starting Worker (Game Logic)...
start /B powershell -Command "& '.\scripts\run-service.ps1' -Service worker"
timeout /t 5 /nobreak > nul

echo.
echo 🌐 Starting Gateway (HTTP API)...
start /B powershell -Command "& '.\scripts\run-service.ps1' -Service gateway"
timeout /t 5 /nobreak > nul

echo.
echo 🖥️ Starting Client (Web UI)...
cd client
if not exist node_modules (
    echo Installing client dependencies...
    npm install
    if errorlevel 1 (
        echo ❌ Failed to install dependencies
        echo Try running: npm install --legacy-peer-deps
        pause
        exit /b 1
    )
)
echo Starting client...
start /B npm run dev
cd ..

echo.
echo =========================================
echo 🎉 ALL SERVICES STARTED SUCCESSFULLY!
echo =========================================
echo.
echo 🌐 Access Points:
echo   🖥️ Client:     http://localhost:5173
echo   🔗 Gateway:    http://localhost:8080
echo   🗄️ PocketBase: http://localhost:8090/_/
echo.
echo 🔧 Troubleshooting:
echo   - If client fails: cd client ^&^& npm run dev
echo   - If ports busy: Check with 'netstat -an ^| findstr :port'
echo   - If proto errors: Make sure you're in the gamev1 root folder
echo.
echo Press any key to close this window...
pause > nul
