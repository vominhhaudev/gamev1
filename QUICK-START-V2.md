# 🚀 QUICK START - GameV1 System (v2.0)

## ⚡ Một Lệnh Khởi Động Toàn Bộ Hệ Thống

```powershell
.\restart-all-services-simple.ps1
```

## 📋 Quy Trình Khởi Động Chi Tiết

### 🎯 Khởi Động Nhanh (Khuyến Nghị)

**Bước 1:** Mở PowerShell trong thư mục gốc dự án
**Bước 2:** Chạy lệnh:
```powershell
.\restart-all-services-simple.ps1
```

**Hệ thống sẽ tự động:**
- ✅ Dừng các services cũ (nếu có)
- ✅ Khởi động PocketBase (Database) trên port 8090
- ✅ Khởi động Worker (Game Logic) với gRPC server
- ✅ Khởi động Gateway (HTTP API) trên port 8080
- ✅ Khởi động Client (Web UI) trên port 5173
- ✅ Hiển thị trạng thái tất cả services

### 🔧 Khởi Động Thủ Công (Để Debug)

Nếu cần debug từng service riêng lẻ:

```powershell
# Terminal 1: Database
powershell -File scripts/run-service.ps1 pocketbase

# Terminal 2: Game Logic (Worker)
powershell -File scripts/run-service.ps1 worker

# Terminal 3: HTTP API (Gateway)
powershell -File scripts/run-service.ps1 gateway

# Terminal 4: Web Client
cd client && npm run dev
```

### 📊 Kiểm Tra Trạng Thái

```powershell
# Kiểm tra trạng thái tất cả services
.\restart-all-services-simple.ps1 -Status

# Hoặc kiểm tra thủ công từng service
Invoke-RestMethod -Uri "http://localhost:8080/healthz" -Method Get
Invoke-RestMethod -Uri "http://localhost:8090/api/health" -Method Get
```

### 🛑 Dừng Hệ Thống

```powershell
# Dừng tất cả services
.\restart-all-services-simple.ps1 -Stop
```

## 🌐 Điểm Truy Cập Hệ Thống

| Service | URL | Mô Tả |
|---------|-----|-------|
| **🖥️ Client Web** | http://localhost:5173 | Giao diện người dùng chính |
| **🔗 Gateway API** | http://localhost:8080 | REST API chính |
| **📊 Metrics** | http://localhost:8080/metrics | Thống kê hệ thống |
| **❤️ Health Check** | http://localhost:8080/healthz | Kiểm tra hoạt động |
| **🗄️ PocketBase Admin** | http://localhost:8090/_/ | Quản lý database |
| **📡 WebSocket** | ws://localhost:8080/ws | Real-time communication |

## 👤 Thông Tin Đăng Nhập

**PocketBase Admin:**
- **Email:** admin@pocketbase.local
- **Password:** 123456789

## 🔧 Các Lệnh Hữu Ích

```powershell
# Khởi động lại toàn bộ hệ thống
.\restart-all-services-simple.ps1 -Restart

# Xem hướng dẫn đầy đủ
.\restart-all-services-simple.ps1  # (chạy không có tham số)

# Cài đặt dependencies cho Client (nếu cần)
cd client && npm install

# Build tất cả Rust services (nếu cần)
cargo build --release
```

## ⚠️ Xử Lý Lỗi Thường Gặp

### 1. Lỗi "Address already in use"
```powershell
# Đóng tất cả terminals và chạy lại
.\restart-all-services-simple.ps1 -Stop
.\restart-all-services-simple.ps1
```

### 2. Lỗi Node.js dependencies
```powershell
cd client
npm install
cd ..
.\restart-all-services-simple.ps1
```

### 3. Lỗi Rust compilation
```powershell
# Build từng service
cd gateway && cargo build && cd ..
cd worker && cargo build && cd ..
cd ..
.\restart-all-services-simple.ps1
```

## 📞 Hỗ Trợ

Nếu gặp vấn đề, hãy:
1. Chạy `.\restart-all-services-simple.ps1 -Status` để kiểm tra trạng thái
2. Kiểm tra logs trong các thư mục services
3. Đóng tất cả terminals và thử lại từ đầu

---

**🎉 Chúc bạn khởi động hệ thống thành công!**
