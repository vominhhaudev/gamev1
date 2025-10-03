# ==========================================
# GAMEV1 - ONE-CLICK STARTUP SCRIPT
# ==========================================
param(
    [switch]$Stop,
    [switch]$Restart,
    [switch]$Status
)

# Thiet lap mau sac cho output
$Green = "Green"
$Yellow = "Yellow"
$Cyan = "Cyan"
$Red = "Red"

function Write-ColorOutput {
    param([string]$Message, [string]$Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Stop-AllServices {
    Write-ColorOutput "Dung tat ca services..." $Yellow

    # Dung cac process theo ten
    $processes = @("gateway", "pocketbase", "node")
    foreach ($process in $processes) {
        Get-Process -Name $process -ErrorAction SilentlyContinue | ForEach-Object {
            Write-ColorOutput "  Dung $($_.ProcessName) (PID: $($_.Id))" $Yellow
            Stop-Process -Id $_.Id -Force
        }
    }

    Start-Sleep -Seconds 2
    Write-ColorOutput "Da dung tat ca services" $Green
}

function Start-AllServices {
    Write-ColorOutput "Khoi dong he thong GameV1..." $Green

    # 1. Dung services hien tai (neu co)
    Stop-AllServices

    # 2. Khoi dong PocketBase
    Write-ColorOutput "Khoi dong PocketBase (port 8090)..." $Cyan
    try {
        Start-Process "C:\Users\Fit\Downloads\gamev1\pocketbase\pocketbase.exe" -ArgumentList "serve", "--http=127.0.0.1:8090" -WindowStyle Hidden
        Start-Sleep -Seconds 3
        Write-ColorOutput "  PocketBase da san sang" $Green
    }
    catch {
        Write-ColorOutput "  Loi khoi dong PocketBase: $($_.Exception.Message)" $Red
        exit 1
    }

    # 3. Khoi dong Gateway
    Write-ColorOutput "Khoi dong Gateway (port 8080)..." $Cyan
    try {
        $env:PATH = "C:\Program Files\nodejs;$env:PATH"
        Set-Location "C:\Users\Fit\Downloads\gamev1\gateway"
        Start-Process "cargo" -ArgumentList "run" -WindowStyle Hidden
        Start-Sleep -Seconds 5
        Write-ColorOutput "  Gateway da san sang" $Green
    }
    catch {
        Write-ColorOutput "  Loi khoi dong Gateway: $($_.Exception.Message)" $Red
        exit 1
    }

    # 4. Khoi dong Client
    Write-ColorOutput "Khoi dong Client (port 5173)..." $Cyan
    try {
        $env:PATH = "C:\Program Files\nodejs;$env:PATH"
        Set-Location "C:\Users\Fit\Downloads\gamev1\client"
        Start-Process "npm" -ArgumentList "run", "dev" -WindowStyle Hidden
        Start-Sleep -Seconds 5
        Write-ColorOutput "  Client da san sang" $Green
    }
    catch {
        Write-ColorOutput "  Loi khoi dong Client: $($_.Exception.Message)" $Red
        exit 1
    }

    Show-Status
}

function Show-Status {
    Write-ColorOutput "" $Green
    Write-ColorOutput "TRANG THAI HE THONG:" $Green
    Write-ColorOutput "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" $Green

    # Kiểm tra các services
    $services = @(
        @{Name="PocketBase"; URL="http://localhost:8090/api/health"; Port=8090},
        @{Name="Gateway"; URL="http://localhost:8080/healthz"; Port=8080},
        @{Name="Client"; URL="http://localhost:5173"; Port=5173}
    )

    foreach ($service in $services) {
        try {
            $response = Invoke-WebRequest -Uri $service.URL -Method GET -TimeoutSec 3
            $status = if ($response.StatusCode -eq 200) { "HOAT DONG" } else { "LOI" }
            $color = if ($response.StatusCode -eq 200) { $Green } else { $Red }
        }
        catch {
            $status = "KHONG PHAN HOI"
            $color = $Red
        }

        $servicePort = $service.Port
        Write-ColorOutput "  $($service.Name) ($servicePort): $status" $color
    }

    Write-ColorOutput "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" $Green
    Write-ColorOutput "Truy cap: http://localhost:5173" $Cyan
    Write-ColorOutput "Dang nhap: admin@pocketbase.local / 123456789" $Cyan
    Write-ColorOutput "Xem logs: Mo terminal va chay lenh nay de theo doi" $Yellow
}

function Show-Help {
    Write-ColorOutput "GAMEV1 - ONE-CLICK STARTUP SCRIPT" $Green
    Write-ColorOutput "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" $Green
    Write-ColorOutput "Cach su dung:" $Yellow
    Write-ColorOutput "  .\restart-all-services.ps1       # Khoi dong toan bo he thong" $Cyan
    Write-ColorOutput "  .\restart-all-services.ps1 -Stop    # Dung tat ca services" $Cyan
    Write-ColorOutput "  .\restart-all-services.ps1 -Restart # Khoi dong lai toan bo" $Cyan
    Write-ColorOutput "  .\restart-all-services.ps1 -Status  # Kiem tra trang thai" $Cyan
    Write-ColorOutput "" $Yellow
    Write-ColorOutput "Cac lenh thu cong neu can:" $Yellow
    Write-ColorOutput "  # Khoi dong tung service rieng:" $Cyan
    Write-ColorOutput "  cd pocketbase && .\pocketbase.exe serve --http=127.0.0.1:8090" $Cyan
    Write-ColorOutput "  cd gateway && cargo run" $Cyan
    Write-ColorOutput "  cd client && npm run dev" $Cyan
    Write-ColorOutput "" $Yellow
    Write-ColorOutput "De khac phuc su co:" $Yellow
    Write-ColorOutput "  1. Dong tat ca terminals" $Cyan
    Write-ColorOutput "  2. Mo terminal moi" $Cyan
    Write-ColorOutput "  3. Chay: .\restart-all-services.ps1" $Cyan
}

# Main logic
if ($Stop) {
    Stop-AllServices
}
elseif ($Restart) {
    Start-AllServices
}
elseif ($Status) {
    Show-Status
}
else {
    # Default: khởi động toàn bộ hệ thống
    Start-AllServices
}
