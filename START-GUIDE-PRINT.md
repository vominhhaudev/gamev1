# 🚀 GameV1 Startup Guide (Phiên Bản In)

## ⚡ KHỞI ĐỘNG NHANH NHẤT

### Cách 1: PowerShell (Đầy đủ tính năng)
```powershell
.\restart-all-services-simple.ps1
```

### Cách 2: Batch File (Đơn giản nhất)
```batch
.\start-all.bat
```

## 📋 QUY TRÌNH CHI TIẾT

### Bước 1: Backend (Terminal 1)
```powershell
powershell -File scripts/run-dev.ps1
```

### Bước 2: Client (Terminal 2)
```batch
cd client && .\start-client.bat
```

## 🌐 ĐIỂM TRUY CẬP

- **Client:** http://localhost:5173
- **Gateway:** http://localhost:8080
- **PocketBase:** http://localhost:8090/_/

## 🔐 ĐĂNG NHẬP

**Admin:** admin@pocketbase.local / 123456789

## ✅ KIỂM TRA

```powershell
.\restart-all-services-simple.ps1 -Status
```

## 🛑 DỪNG HỆ THỐNG

```powershell
.\restart-all-services-simple.ps1 -Stop
```

## 💡 LƯU Ý QUAN TRỌNG

- Luôn chạy BACKEND trước CLIENT
- start-client.bat ưu tiên port 5173
- Nếu lỗi → Đóng tất cả terminals và thử lại

---
**🎮 Chúc bạn chơi game vui vẻ!**
