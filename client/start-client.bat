@echo off
echo Starting GameV1 Client...
cd /d "%~dp0"
"C:\Program Files\Nodejs\node.exe" "node_modules\.bin\vite.cmd" dev --host 0.0.0.0 --port 5173
pause
