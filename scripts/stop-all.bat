@echo off
echo 🛑 Stopping All GameV1 Services
echo ===============================

echo.
echo Stopping processes...
powershell -Command "Get-Process -Name 'cargo','pocketbase','node' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue"

echo.
echo Checking if any processes are still running...
powershell -Command "Get-Process -Name cargo,node,pocketbase -ErrorAction SilentlyContinue | Select-Object Name,Id"

echo.
echo =================================
echo ✅ All services stopped!
echo =================================
echo.
echo To start services again, run:
echo   .\start-all.bat
echo.
echo Press any key to exit...
pause >nul
