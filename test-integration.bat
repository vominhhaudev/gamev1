@echo off
echo Starting GameV1 Integration Test
echo =====================================
echo.

REM Wait a moment for user to read
timeout /t 2 /nobreak >nul

echo Ports check skipped for simplicity

REM Start worker in background
echo Starting game worker...
start "GameWorker" cmd /k "cargo run --bin worker"

echo Game worker started
echo Waiting for worker to initialize...
timeout /t 5 /nobreak >nul

REM Start client
echo Starting game client...

REM Install client dependencies if needed
if not exist "client\node_modules" (
    echo Installing client dependencies...
    cd client
    "C:\Program Files\nodejs\node.exe" "C:\Program Files\nodejs\node_modules\npm\bin\npm-cli.js" install
    cd ..
)

REM Start client in new window
start "GameClient" cmd /k "cd /d %~dp0client && "C:\Program Files\nodejs\node.exe" node_modules\.bin\vite dev --host 0.0.0.0 --port 5173"

echo Game client started

echo.
echo Integration test setup complete!
echo =================================
echo.
echo Access points:
echo    Client: http://localhost:5173
echo    Game:   http://localhost:5173/game
echo.
echo Services running:
echo    Worker (gRPC): localhost:50051
echo    Client (Web): localhost:5173
echo.
echo Instructions:
echo    1. Open http://localhost:5173 in your browser
echo    2. Click 'Play Game' to start
echo    3. Click 'Join Game' to connect to worker
echo    4. Use WASD to move, Shift to sprint
echo.
echo To stop:
echo    - Close browser tab
echo    - Close this command window
echo    - Or press any key to exit
echo.
echo Happy gaming!

pause
