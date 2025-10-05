# 🚀 QUICK START - GameV1 System

## 🎯 Một lệnh duy nhất để khởi động toàn bộ hệ thống!

### Cách 1: Sử dụng Batch File (Dễ nhất)
```bash
# Double-click hoặc chạy từ terminal:
run-gamev1.bat
```

### Cách 2: Sử dụng PowerShell Script (Khuyến nghị)
```bash
# Từ thư mục gốc của project - khởi động toàn bộ hệ thống:
.\restart-all-services-simple.ps1

# Hoặc chỉ backend services:
.\restart-all-services.ps1

# Hoặc chỉ worker + client để test:
.\run-game-client-integration.ps1
```

### Cách 3: Thủ công (Để debug từng service)
```bash
# Terminal 1 - PocketBase
cd pocketbase && .\pocketbase.exe serve --http=127.0.0.1:8090

# Terminal 2 - Gateway
cd gateway && cargo run

# Terminal 3 - Worker
cd worker && cargo run

# Terminal 4 - Client
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
# Kiểm tra services đang chạy (sử dụng script tốt nhất)
.\restart-all-services-simple.ps1 -Status

# Dừng tất cả services
.\restart-all-services-simple.ps1 -Stop

# Khởi động lại toàn bộ hệ thống
.\restart-all-services-simple.ps1 -Restart
```

## 🔧 Các lệnh hữu ích khác
```bash
# Kiểm tra trạng thái với script đơn giản
.\restart-all-services-simple.ps1 -Status

# Chỉ khởi động worker + client để test nhanh
.\run-game-client-integration.ps1

# Script cũ (vẫn hoạt động nhưng khuyến nghị dùng -simple)
.\restart-all-services.ps1

# Xem help đầy đủ
.\restart-all-services-simple.ps1  # Sẽ hiển thị help nếu không có tham số
```

## 🚨 Khắc phục sự cố

### Nếu gặp lỗi khi khởi động:
1. **Đóng tất cả terminals và PowerShell windows**
2. **Mở terminal mới với quyền Administrator**
3. **Chạy lệnh phù hợp**:
   ```bash
   # Khuyến nghị - khởi động toàn bộ hệ thống:
   .\restart-all-services-simple.ps1

   # Hoặc sử dụng batch file:
   run-gamev1.bat

   # Nếu vẫn lỗi Node.js:
   cd client && npm install && cd ..

   # Nếu vẫn lỗi Rust:
   cargo build --release
   ```
4. **Kiểm tra trạng thái**: `.\restart-all-services-simple.ps1 -Status`
5. **Xem logs** trong từng terminal để tìm lỗi cụ thể

### Các lỗi thường gặp:
- **Port đã được sử dụng**: Dùng `.\restart-all-services-simple.ps1 -Stop` trước
- **Node.js dependencies**: Chạy `npm install` trong thư mục client
- **Rust compilation**: Chạy `cargo build` trong các thư mục services
- **CORS errors**: Đã được khắc phục với Vite proxy

## 🎉 Hệ thống đã sẵn sàng!
Sau khi chạy thành công, bạn sẽ thấy:
- ✅ **PocketBase** (port 8090) - Database với admin UI
- ✅ **Gateway** (port 8080) - HTTP API và WebSocket
- ✅ **Client** (port 5173) - Giao diện người dùng
- ✅ **Worker** (gRPC 50051) - Game logic và physics
- ✅ **Authentication hoạt động** với JWT
- ✅ **Không còn lỗi CORS** nhờ Vite proxy

**Chúc bạn phát triển thành công! 🚀**
