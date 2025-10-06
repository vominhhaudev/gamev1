@echo off
REM 🚀 GAMEV1 - Ultimate Startup Script (Batch Version)
REM ==================================================
REM This batch file provides a simple way to start the entire game system
REM Run this single file to start all services with proper error handling

echo 🚀 GAMEV1 - Complete Startup Script
echo =================================
echo.

echo 📋 Starting initialization...
powershell -ExecutionPolicy Bypass -File "%~dp0start-project-complete.ps1" %*

if errorlevel 1 (
    echo.
    echo ❌ Startup failed. Check the error messages above.
    echo 🔧 Try running: powershell -ExecutionPolicy Bypass -File "start-project-complete.ps1" -Verbose
    pause
    exit /b 1
)

echo.
echo ✅ Startup completed successfully!
echo Press any key to exit...
pause > nul
