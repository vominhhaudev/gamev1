@echo off
echo 🔍 Monitoring GameV1 Logs...
echo Press Ctrl+C to stop monitoring
echo.

REM Monitor worker output in real-time
powershell -Command "Get-Content worker\worker_output.log -Tail 0 -Wait"

REM Alternative: Monitor both logs
REM powershell -Command "Get-Content worker\worker_output.log -Tail 0 -Wait; Get-Content worker\worker_error.log -Tail 0 -Wait"
