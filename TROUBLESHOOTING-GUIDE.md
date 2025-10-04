# 🚨 GameV1 Troubleshooting Guide

## Mục lục
- [Các lỗi thường gặp](#các-lỗi-thường-gặp)
- [Khắc phục lỗi khởi động](#khắc-phục-lỗi-khởi-động)
- [Khắc phục lỗi kết nối](#khắc-phục-lỗi-kết-nối)
- [Khắc phục lỗi client](#khắc-phục-lỗi-client)
- [Kiểm tra trạng thái hệ thống](#kiểm-tra-trạng-thái-hệ-thống)

---

## Các lỗi thường gặp

### ❌ Lỗi 1: "Cannot find path 'client' because it does not exist"

**Nguyên nhân:** Đang chạy lệnh từ thư mục sai

**Giải pháp:**
```bash
# Đúng: Chạy từ thư mục gốc gamev1
cd C:\Users\Fit\Downloads\gamev1
.\start-all.bat

# Sai: Chạy từ thư mục con
cd client
npm run dev  # ❌ Sẽ bị lỗi
```

### ❌ Lỗi 2: "Failed to connect to game worker: ENOENT: no such file or directory, open 'proto\worker.proto'"

**Nguyên nhân:** Client không tìm thấy file proto

**Giải pháp:** Đã được sửa tự động trong code, nhưng nếu vẫn lỗi:
```bash
# Đảm bảo đang ở thư mục gốc
cd C:\Users\Fit\Downloads\gamev1

# Kiểm tra file tồn tại
dir proto\worker.proto

# Nếu không có, kiểm tra cấu trúc thư mục
dir
```

### ❌ Lỗi 3: "Gateway not responding" hoặc port 8080 không hoạt động

**Nguyên nhân:** Thứ tự khởi động sai hoặc Worker service chưa sẵn sàng

**Giải pháp:**
```bash
# Khởi động thủ công theo đúng thứ tự:

# 1. Worker trước (cần thời gian khởi động)
powershell -File scripts\run-service.ps1 worker
timeout /t 10

# 2. Gateway sau khi Worker đã chạy
powershell -File scripts\run-service.ps1 gateway

# 3. Client cuối cùng
cd client && npm run dev
```

### ❌ Lỗi 4: Client không khởi động được (port 5173)

**Nguyên nhân:** Dependencies chưa được cài hoặc Node.js lỗi

**Giải pháp:**
```bash
# 1. Cài đặt dependencies
cd client
rm -rf node_modules package-lock.json
npm install

# 2. Nếu vẫn lỗi, thử với legacy peer deps
npm install --legacy-peer-deps

# 3. Khởi động client
npm run dev

# 4. Nếu vẫn lỗi, kiểm tra Node.js version
node --version
npm --version
```

---

## Khắc phục lỗi khởi động

### Cách 1: Sử dụng script tự động (Khuyến nghị)
```bash
# Từ thư mục gốc gamev1
.\start-all.bat
```

### Cách 2: Khởi động thủ công từng service
```bash
# Terminal 1: Database
powershell -File scripts\run-service.ps1 pocketbase

# Terminal 2: Worker (chờ 10 giây sau khi khởi động)
powershell -File scripts\run-service.ps1 worker

# Terminal 3: Gateway (chờ 5 giây sau khi Worker sẵn sàng)
powershell -File scripts\run-service.ps1 gateway

# Terminal 4: Client
cd client && npm run dev
```

### Cách 3: Sử dụng PowerShell script chính
```powershell
# Từ thư mục gốc
.\restart-all-services-simple.ps1
```

---

## Khắc phục lỗi kết nối

### Kiểm tra các port đang chạy
```bash
netstat -an | findstr :50051  # Worker
netstat -an | findstr :8080   # Gateway
netstat -an | findstr :5173   # Client
netstat -an | findstr :8090   # PocketBase
```

### Kiểm tra tiến trình đang chạy
```bash
# PowerShell
Get-Process -Name cargo, node, pocketbase

# Command Prompt
tasklist | findstr /I "cargo node pocketbase"
```

### Dừng tất cả services và khởi động lại
```powershell
# Dừng tất cả
Get-Process -Name 'cargo','node','pocketbase' -ErrorAction SilentlyContinue | Stop-Process -Force

# Khởi động lại
.\start-all.bat
```

---

## Khắc phục lỗi client

### 1. Lỗi dependencies
```bash
cd client
rm -rf node_modules package-lock.json
npm install
```

### 2. Lỗi build
```bash
cd client
npm run build
npm run preview  # Thay vì dev để kiểm tra
```

### 3. Lỗi port bị chiếm
```bash
# Kiểm tra port nào đang dùng 5173
netstat -an | findstr :5173

# Dừng tiến trình chiếm port
Get-Process -Id <PID> | Stop-Process -Force
```

### 4. Lỗi cache
```bash
cd client
npm run clean  # Nếu có script này
rm -rf .svelte-kit node_modules/.vite
npm install
```

---

## Kiểm tra trạng thái hệ thống

### Script kiểm tra nhanh
```bash
# Tạo file check-status.bat
@echo off
echo 🔍 Checking GameV1 System Status
echo ================================

echo.
echo 🌐 Checking Ports:
netstat -an | findstr :50051 && echo ✅ Worker: Running || echo ❌ Worker: Not running
netstat -an | findstr :8080 && echo ✅ Gateway: Running || echo ❌ Gateway: Not running
netstat -an | findstr :5173 && echo ✅ Client: Running || echo ❌ Client: Not running
netstat -an | findstr :8090 && echo ✅ PocketBase: Running || echo ❌ PocketBase: Not running

echo.
echo 🔧 Checking Processes:
powershell -Command "Get-Process -Name cargo,node,pocketbase -ErrorAction SilentlyContinue | Select-Object Name,Id"

echo.
echo 🌐 Testing Endpoints:
powershell -Command "
try { Invoke-WebRequest -Uri http://localhost:8080/healthz -TimeoutSec 3 -ErrorAction Stop; Write-Host '✅ Gateway API: OK' }
catch { Write-Host '❌ Gateway API: Failed' }

try { Invoke-WebRequest -Uri http://localhost:5173 -TimeoutSec 3 -ErrorAction Stop; Write-Host '✅ Client: OK' }
catch { Write-Host '❌ Client: Failed' }
"

echo.
echo Press any key to exit...
pause >nul
```

### Kiểm tra logs
```bash
# Kiểm tra logs của các service
dir *.log
type worker_error.log  # Nếu có lỗi
```

---

## Cấu trúc thư mục đúng

```
gamev1/                    # Thư mục gốc
├── client/               # Frontend (SvelteKit)
│   ├── src/
│   ├── package.json
│   └── node_modules/
├── worker/               # Game logic (Rust)
│   ├── src/
│   └── Cargo.toml
├── gateway/              # API gateway (Rust)
│   ├── src/
│   └── Cargo.toml
├── proto/                # Protocol definitions
│   └── worker.proto
├── scripts/              # PowerShell scripts
└── pocketbase/           # Database
    └── pocketbase.exe
```

---

## Lưu ý quan trọng

1. **Luôn chạy từ thư mục gốc** `C:\Users\Fit\Downloads\gamev1`
2. **Thứ tự khởi động quan trọng**: Worker → Gateway → Client
3. **Đợi đủ thời gian** giữa các service để chúng kết nối được
4. **Kiểm tra logs** nếu có lỗi để xác định nguyên nhân
5. **Sử dụng script tự động** thay vì chạy thủ công để tránh lỗi

---

## Nếu vẫn không khắc phục được

1. Đóng tất cả terminal và PowerShell windows
2. Mở terminal mới ở thư mục gốc
3. Chạy: `.\start-all.bat`
4. Nếu vẫn lỗi, kiểm tra:
   - Node.js đã cài đặt chưa: `node --version`
   - Rust đã cài đặt chưa: `cargo --version`
   - Cấu trúc thư mục đúng chưa
