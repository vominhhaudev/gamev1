# 🚀 QUICK START - GameV1 System

## 🎯 Một lệnh duy nhất để khởi động toàn bộ hệ thống!

### Cách 1: Sử dụng Batch File (Dễ nhất)
```bash
# Double-click hoặc chạy từ terminal:
run-gamev1.bat
```

### Cách 2: Sử dụng PowerShell Script
```bash
# Từ thư mục gốc của project:
.\restart-all-services.ps1
```

### Cách 3: Thủ công (Nếu cần)
```bash
# Terminal 1 - PocketBase
cd pocketbase && .\pocketbase.exe serve --http=127.0.0.1:8090

# Terminal 2 - Gateway
cd gateway && cargo run

# Terminal 3 - Client
cd client && npm run dev
```

## 🌐 Truy cập hệ thống
- **Client**: http://localhost:5173
- **Gateway API**: http://localhost:8080
- **PocketBase Admin**: http://localhost:8090/_/

## 🔑 Thông tin đăng nhập
- **Email**: admin@pocketbase.local
- **Password**: 123456789

## 📊 Kiểm tra trạng thái
```bash
# Kiểm tra services đang chạy
.\restart-all-services.ps1 -Status

# Dừng tất cả services
.\restart-all-services.ps1 -Stop
```

## 🔧 Các lệnh hữu ích khác
```bash
# Khởi động lại toàn bộ hệ thống
.\restart-all-services.ps1 -Restart

# Chỉ khởi động (không dừng trước)
.\restart-all-services.ps1

# Xem help đầy đủ
.\restart-all-services.ps1  # Sẽ hiển thị help nếu không có tham số
```

## 🚨 Khắc phục sự cố
1. **Đóng tất cả terminals**
2. **Mở terminal mới với quyền Administrator**
3. **Chạy**: `run-gamev1.bat`
4. **Nếu vẫn lỗi**: Kiểm tra logs trong từng terminal

## 🎉 Hệ thống đã sẵn sàng!
Sau khi chạy thành công, bạn sẽ thấy:
- ✅ PocketBase (port 8090)
- ✅ Gateway (port 8080)
- ✅ Client (port 5173)
- ✅ Authentication hoạt động
- ✅ Không còn lỗi CORS

**Chúc bạn phát triển thành công! 🚀**
