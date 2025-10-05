@echo off
REM ==========================================
REM 🚀 GAMEV1 - EASY STARTUP BATCH FILE
REM ==========================================
echo 🚀 Starting GameV1 System...
echo.

REM Check if PowerShell is available
powershell -Command "Write-Host 'PowerShell is available'" >nul 2>&1
if errorlevel 1 (
    echo ❌ PowerShell is not available. Please install PowerShell or run manually.
    pause
    exit /b 1
)

echo 📋 Choose an option:
echo [1] Start all services (Default)
echo [2] Stop all services
echo [3] Check status
echo [4] Show help
echo.

set /p choice="Enter your choice (1-4): "

if "%choice%"=="1" goto start_all
if "%choice%"=="2" goto stop_all
if "%choice%"=="3" goto check_status
if "%choice%"=="4" goto show_help

:start_all
echo 🚀 Starting all services...
powershell -ExecutionPolicy Bypass -File "%~dp0restart-all-services-simple.ps1"
goto end

:stop_all
echo 📴 Stopping all services...
powershell -ExecutionPolicy Bypass -File "%~dp0restart-all-services-simple.ps1" -Stop
goto end

:check_status
echo 📊 Checking system status...
powershell -ExecutionPolicy Bypass -File "%~dp0restart-all-services-simple.ps1" -Status
goto end

:show_help
echo.
echo 🚀 GAMEV1 - ONE-CLICK STARTUP SCRIPT
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo.
echo USAGE:
echo   run-gamev1.bat              - Start all services
echo   run-gamev1.bat stop         - Stop all services
echo   run-gamev1.bat status       - Check status
echo.
echo ALTERNATIVE COMMANDS:
echo   .\restart-all-services-simple.ps1    - PowerShell version
echo   .\run-game-client-integration.ps1   - Only worker + client
echo.
echo MANUAL COMMANDS (if needed):
echo   cd pocketbase ^&^& pocketbase.exe serve --http=127.0.0.1:8090
echo   cd gateway ^&^& cargo run
echo   cd worker ^&^& cargo run
echo   cd client ^&^& npm run dev
echo.
echo TROUBLESHOOTING:
echo   1. Close all terminals and PowerShell windows
echo   2. Open new terminal in project root
echo   3. Run: run-gamev1.bat
echo   4. If Node.js errors: Run 'npm install' in client folder
echo   5. If Rust errors: Run 'cargo build' in each service folder
echo   6. If port conflicts: Use .\restart-all-services-simple.ps1 -Stop first
echo.
pause
goto end

:end
echo.
echo Press any key to exit...
pause >nul
