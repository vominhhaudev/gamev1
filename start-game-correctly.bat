@echo off
echo ========================================
echo 🚀 GAME V1 - CORRECT STARTUP SCRIPT
echo ========================================
echo.
echo 🔧 Starting the correct game client...
echo.
echo ✅ Correct URL: http://localhost:5173/game
echo ❌ Wrong URL: http://localhost:5173/game3d (doesn't exist)
echo.
echo ========================================
echo.

cd /d "%~dp0"
cd client

echo 🔄 Starting development server...
npm run dev

pause
