# Script để dừng tất cả các dịch vụ development

Write-Host "Đang dừng tất cả các dịch vụ..."

# Dừng PocketBase
Write-Host "Dừng PocketBase..."
Get-Process -Name "pocketbase" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# Dừng các tiến trình Cargo/Rust
Write-Host "Dừng các tiến trình Rust/Cargo..."
Get-Process -Name "cargo" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# Dừng các tiến trình khác có thể liên quan
Get-Process | Where-Object {
    $_.ProcessName -like "*rust*" -or
    $_.ProcessName -like "*target*" -or
    $_.CommandLine -like "*gamev1*"
} -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# Sử dụng taskkill như backup
Write-Host "Sử dụng taskkill để đảm bảo..."
taskkill /IM "pocketbase.exe" /F 2>$null | Out-Null
taskkill /IM "cargo.exe" /F 2>$null | Out-Null

Write-Host "Đã dừng tất cả các dịch vụ!"
Write-Host "Bạn có thể chạy lại với: powershell -File scripts/run-dev.ps1"
