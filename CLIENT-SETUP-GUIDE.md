# 🚀 Hướng Dẫn Chạy Client UI Chi Tiết

## 📋 Yêu cầu hệ thống

- **Node.js** 18.x hoặc 20.x (tải tại: https://nodejs.org/)
- **npm** (được cài cùng Node.js)

## 🔧 Các bước thực hiện

### Bước 1: Kiểm tra Node.js
```bash
node --version
```
Nếu không có, tải và cài đặt Node.js từ trang chủ.

### Bước 2: Điều hướng đến thư mục client
```bash
cd client
```

### Bước 3: Cài đặt dependencies
```bash
npm install
```
Lệnh này sẽ cài đặt tất cả packages cần thiết cho SvelteKit.

### Bước 4: Khởi động development server
```bash
npm run dev
```

Bạn sẽ thấy output như sau:
```
➜  Local:   http://localhost:5173/
➜  Network:  http://192.168.x.x:5173/
➜  Press h + enter to show help
```

### Bước 5: Truy cập ứng dụng
Mở trình duyệt và truy cập:
```
http://localhost:5173/net-test
```

## 🔑 Đăng nhập và sử dụng

### Thông tin đăng nhập:
- **Email:** `admin@pocketbase.local`
- **Password:** `123456789`

### Các tính năng có thể test:

1. **Authentication UI**
   - Form đăng nhập đẹp với validation
   - Hiển thị trạng thái đăng nhập
   - Tự động refresh token

2. **Network Testing**
   - WebSocket connection với ping/pong
   - Real-time RTT measurement
   - Connection status monitoring

3. **Authentication Status**
   - Hiển thị thông tin user hiện tại
   - Thời gian hết hạn token
   - Trạng thái kết nối

## 🖥️ Giao diện người dùng

- **Dark theme** đẹp mắt phù hợp gaming
- **Responsive design** hoạt động trên mọi thiết bị
- **Real-time updates** với Svelte reactivity
- **Modern UI components** với smooth animations

## 🌐 Backend Services (Đã chạy sẵn)

| Service | Port | URL | Trạng thái |
|---------|------|-----|------------|
| **Gateway** | 8080 | http://127.0.0.1:8080 | ✅ Đang chạy |
| **Worker** | 50051 | http://127.0.0.1:50051 | ✅ Đang chạy |
| **PocketBase** | 8090 | http://127.0.0.1:8090 | ✅ Đang chạy |

## 🔍 Monitoring & Debug

### Các trang hữu ích:
- **Gateway Metrics:** http://127.0.0.1:8080/metrics
- **PocketBase Admin:** http://127.0.0.1:8090/_/
- **Client Dev Server:** http://localhost:5173 (sẽ hiện sau khi chạy)

### Logs để debug:
- **Gateway logs:** Terminal đang chạy gateway
- **Worker logs:** Terminal đang chạy worker (PERF STATS)
- **Client logs:** Terminal đang chạy `npm run dev`

## 🚨 Troubleshooting

### Nếu gặp lỗi "Port already in use":
```bash
# Tìm process đang dùng port
netstat -ano | findstr :5173

# Kill process (thay PID bằng số tìm được)
taskkill /PID <PID> /F
```

### Nếu gặp lỗi Node.js:
- Đảm bảo Node.js được thêm vào PATH
- Khởi động lại Command Prompt sau khi cài Node.js

### Nếu client không load:
- Xóa thư mục `node_modules` và chạy lại `npm install`
- Kiểm tra firewall không block port 5173

## ✅ Đã hoàn thành

- ✅ **Authentication System** hoàn chỉnh với JWT
- ✅ **Rate Limiting** framework (sẵn sàng)
- ✅ **WebSocket** connections với fallback
- ✅ **Beautiful UI** với dark theme
- ✅ **Real-time** monitoring và metrics
- ✅ **Session Management** với localStorage

## 🎯 Sẵn sàng cho Week 4+

Hệ thống đã sẵn sàng để tiếp tục với:
- **WebRTC Implementation**
- **Game Simulation** nâng cao
- **Multiplayer Features**

---

**🎮 Chúc bạn test vui vẻ! Có vấn đề gì cứ hỏi nhé! 🚀**
