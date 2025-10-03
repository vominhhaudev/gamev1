# 📦 Hướng Dẫn Cài Đặt Node.js Trên Windows

## 🎯 Thông tin phiên bản hiện tại

- **Latest LTS:** v22.20.0 (Khuyên dùng)
- **Latest Release:** v24.9.0 (Mới nhất nhưng có thể chưa ổn định)

## 🖼️ Hướng dẫn trực quan từng bước

### Bước 1: Truy cập trang chủ
```
🌐 Trình duyệt → https://nodejs.org/
```

### Bước 2: Tải Node.js
- ✅ Click nút **"LTS"** (khuyên dùng)
- ✅ Chọn **"Windows Installer (.msi)"**
- 📁 File sẽ tải về thư mục Downloads

### Bước 3: Cài đặt
- 🔍 Tìm file `node-v22.20.0-x64.msi` trong Downloads
- 🖱️ Double-click để mở
- ⏭️ Click **"Next"** qua các bước:
  - ✅ License Agreement
  - ✅ Installation Folder (mặc định)
  - ✅ Install
- ⏳ Đợi quá trình cài đặt hoàn tất

### Bước 4: Kiểm tra cài đặt
```bash
# Mở Command Prompt hoặc PowerShell
node --version
npm --version

# Kết quả mong đợi:
# v22.20.0
# 10.x.x
```

## 🚨 Troubleshooting

### Nếu gặp lỗi "node is not recognized"
- 🔄 **Khởi động lại Command Prompt/PowerShell**
- 🔍 Đảm bảo Node.js được thêm vào PATH
- 🛠️ Nếu vẫn lỗi, thử cài lại với quyền Administrator

### Nếu npm không hoạt động
```bash
# Cài lại npm
npm install -g npm@latest
```

### Nếu gặp lỗi quyền truy cập
- 🛡️ Chạy Command Prompt với quyền **Administrator**
- 📁 Cài đặt ở thư mục khác (không phải Program Files)

## ✅ Kiểm tra thành công

Khi thấy:
```bash
C:\Users\Fit> node --version
v22.20.0

C:\Users\Fit> npm --version
10.x.x
```

→ **THÀNH CÔNG!** Bạn đã sẵn sàng chạy client UI.

## 🎮 Tiếp theo: Chạy Client UI

```bash
cd "C:\Users\Fit\Downloads\gamev1\client"
npm install
npm run dev
```

Truy cập: http://localhost:5173/net-test

---

**📚 Tham khảo:** [nodejs.org](https://nodejs.org/) - Trang chủ chính thức của Node.js với đầy đủ tài liệu và hướng dẫn.

**💡 Mẹo:** Luôn chọn phiên bản **LTS** để đảm bảo tính ổn định cho dự án production.
