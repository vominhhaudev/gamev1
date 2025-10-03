@echo off
echo Starting GameV1 Client...
echo.
echo Changing to client directory...
cd client

echo Checking dependencies...
if not exist "node_modules" (
    echo Installing dependencies...
    "C:\Program Files\nodejs\node.exe" "C:\Program Files\nodejs\node_modules\npm\bin\npm-cli.js" install
)

echo.
echo Starting development server...
echo Server will be available at: http://localhost:5173/net-test
echo.
"C:\Program Files\nodejs\node.exe" "node_modules/vite/bin/vite.js" dev --host 0.0.0.0 --port 5173

pause
