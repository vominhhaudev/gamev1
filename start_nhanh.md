# 🚀 KHỞI ĐỘNG NHANH TOÀN BỘ HỆ THỐNG GAMEV1
# ======================================================================

# ⚡ CÁCH NHANH NHẤT - CHỈ 1 LỆNH DUY NHẤT! (Khuyến nghị)

# PowerShell (Đầy đủ tính năng):
.\restart-all-services-simple.ps1

# HOẶC Batch File (Đơn giản nhất - chỉ cần double-click):
.\start-all.bat

# =============================================================================
# QUY TRÌNH CHI TIẾT (Đã được kiểm chứng hoạt động ổn định):
# =============================================================================

# 1️⃣ KHỞI ĐỘNG BACKEND SERVICES (Terminal 1 - Chạy lệnh này trước)
powershell -File scripts/run-dev.ps1

# 2️⃣ KHỞI ĐỘNG CLIENT (Terminal 2 - Chạy lệnh này sau khi backend đã chạy)
cd client && .\start-client.bat

# =============================================================================
# CÁC LỆNH THAY THẾ (Nếu cần debug từng service riêng lẻ):
# =============================================================================

# ✅ BACKEND SERVICES (Chạy trong terminal riêng để debug):
powershell -File scripts/run-service.ps1 pocketbase  # Database (port 8090)
powershell -File scripts/run-service.ps1 worker     # Game Logic (gRPC)
powershell -File scripts/run-service.ps1 gateway    # HTTP API (port 8080)

# ✅ CLIENT (Chạy trong terminal riêng để debug):
cd client && .\start-client.bat                    # Luôn dùng port 5173 (ổn định)
# HOẶC nếu muốn tự động chuyển port:
cd client && npm run dev                          # Port 5174 nếu 5173 bị chiếm

# =============================================================================
# KIỂM TRA HOẠT ĐỘNG (Sau khi khởi động xong):
# =============================================================================

# Kiểm tra trạng thái tổng thể:
.\restart-all-services-simple.ps1 -Status

# Test trực tiếp từng service:
Invoke-RestMethod -Uri "http://localhost:8080/healthz" -Method Get  # Gateway
Invoke-RestMethod -Uri "http://localhost:8090/api/health" -Method Get # PocketBase

# Client sẽ hiển thị trên trình duyệt tự động

# =============================================================================
# DỪNG TOÀN BỘ HỆ THỐNG:
# =============================================================================
.\restart-all-services-simple.ps1 -Stop

# =============================================================================
# CÁC ĐIỂM TRUY CẬP SAU KHI KHỞI ĐỘNG THÀNH CÔNG:
# =============================================================================
🖥️ Client Web:     http://localhost:5173 (Trang chủ game)
🔗 Gateway API:    http://localhost:8080 (API backend)
📊 Metrics:        http://localhost:8080/metrics (Thống kê hệ thống)
❤️ Health Check:   http://localhost:8080/healthz (Kiểm tra hoạt động)
🗄️ PocketBase:     http://localhost:8090/_/ (Quản lý database)
📡 WebSocket:      ws://localhost:8080/ws (Real-time communication)

# =============================================================================
# THÔNG TIN ĐĂNG NHẬP:
# =============================================================================
👤 PocketBase Admin: admin@pocketbase.local / 123456789

# =============================================================================
# MẸO KHỞI ĐỘNG HIỆU QUẢ:
# =============================================================================
✅ Luôn chạy BACKEND trước (run-dev.ps1) để đảm bảo database và API sẵn sàng
✅ Rồi mới chạy CLIENT (start-client.bat) để kết nối với backend
✅ File start-client.bat ưu tiên port 5173 (ổn định hơn npm run dev)
✅ File start-all.bat là cách đơn giản nhất - chỉ cần double-click
✅ Nếu gặp lỗi, đóng tất cả terminals và chạy lại từ đầu
✅ Có thể mở nhiều terminals để chạy/debug từng service riêng lẻ