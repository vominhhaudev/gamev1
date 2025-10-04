# 🚀 QUICK START - GameV1 Project

## ⚡ Khởi Động Nhanh Nhất (1 lệnh duy nhất)

```powershell
.\restart-all-services-simple.ps1
```

## 📋 Quy Trình Hoàn Chỉnh (Đã Kiểm Chứng)

### Bước 1: Khởi Động Backend (Terminal 1)
```powershell
powershell -File scripts/run-dev.ps1
```

### Bước 2: Khởi Động Client (Terminal 2)
```powershell
cd client && .\start-client.bat
```

## 🌐 Truy Cập Hệ Thống

| Service | URL | Trạng Thái |
|---------|-----|-----------|
| **🖥️ Client Web** | http://localhost:5173 | ✅ **SẴN SÀNG** |
| **🔗 Gateway API** | http://localhost:8080 | ✅ **ĐANG CHẠY** |
| **📊 Metrics** | http://localhost:8080/metrics | ✅ **ĐANG CHẠY** |
| **❤️ Health Check** | http://localhost:8080/healthz | ✅ **ĐANG CHẠY** |
| **🗄️ PocketBase Admin** | http://localhost:8090/_/ | ✅ **ĐANG CHẠY** |

## 🔧 Các Lệnh Thay Thế

### Nếu Muốn Debug Từng Service:
```powershell
# Terminal 1: Database
powershell -File scripts/run-service.ps1 pocketbase

# Terminal 2: Game Logic
powershell -File scripts/run-service.ps1 worker

# Terminal 3: HTTP API
powershell -File scripts/run-service.ps1 gateway

# Terminal 4: Web Client
cd client && .\start-client.bat
```

## ✅ Kiểm Tra Hoạt Động

```powershell
# Kiểm tra trạng thái tổng thể
.\restart-all-services-simple.ps1 -Status

# Kiểm tra trực tiếp
Invoke-RestMethod -Uri "http://localhost:8080/healthz" -Method Get
Invoke-RestMethod -Uri "http://localhost:8090/api/health" -Method Get
```

## 🛑 Dừng Hệ Thống

```powershell
.\restart-all-services-simple.ps1 -Stop
```

## 💡 Mẹo Quan Trọng

- **✅ Luôn chạy BACKEND trước** (run-dev.ps1) để đảm bảo database và API sẵn sàng
- **✅ Rồi mới chạy CLIENT** (start-client.bat) để kết nối với backend
- **✅ File start-client.bat** ưu tiên port 5173 (ổn định hơn npm run dev)
- **✅ Nếu port 5173 bị chiếm** → Client tự động chuyển sang port 5174

## 🔐 Thông Tin Đăng Nhập

**PocketBase Admin:**
- **Email:** `admin@pocketbase.local`
- **Password:** `123456789`

---

**🎉 Hệ thống GameV1 của bạn đã sẵn sàng để phát triển và chơi game!**
