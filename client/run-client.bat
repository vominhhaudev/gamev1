@echo off
echo Starting GameV1 Client...
echo.

REM Change to client directory
cd /d "%~dp0"

REM Check if node_modules exists
if not exist "node_modules" (
    echo Installing dependencies...
    "C:\Program Files\nodejs\node.exe" "C:\Program Files\nodejs\node_modules\npm\bin\npm-cli.js" install
)

REM Start development server
echo Starting development server...
echo Server will be available at: http://localhost:5173/net-test
echo.
"C:\Program Files\nodejs\node.exe" "node_modules/vite/bin/vite.js" dev --host 0.0.0.0 --port 5173

pause
