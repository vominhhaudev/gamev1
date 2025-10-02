# Huong Dan Tao Collections Tu Dong Tren PocketBase

## Collections Da Duoc Tao (Da Sua Loi)

Da tao thanh cong **3 collections** cho du an game bang **PocketBase migrations** voi cu phap dung:

### 1. **games** Collection
- `name` (text, required): Tên game
- `max_players` (number, 2-8): Số người chơi tối đa
- `status` (select): waiting | playing | finished

### 2. **players** Collection
- `username` (text, required): Tên người chơi
- `email` (email, required): Email
- `score` (number): Điểm số
- `is_online` (bool): Trạng thái online

### 3. **game_sessions** Collection
- `game_id` (relation → games): Liên kết đến game
- `player_id` (relation → players): Liên kết đến người chơi
- `position` (json): Vị trí trong game
- `session_score` (number): Điểm trong phiên
- `status` (select): active | finished

---

## Lenh Da Chay (Da Sua Loi Schema)

```powershell
# 1. Tạo file migration với cú pháp ĐÚNG
# File: pocketbase/pb_migrations/1759400001_fix_collections.js
# Luu y: Phai dung "fields" (khong phai "schema")

# 2. Dừng PocketBase
Get-Process -Name "pocketbase" | Stop-Process -Force

# 3. Apply migration
.\pocketbase\pocketbase.exe migrate up
# Output: Applied 1759400001_fix_collections.js 
# 4. Khởi động PocketBase
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090

# 5. Thêm dữ liệu mẫu (với các trường đầy đủ)
$game = @{name="Battle Royale"; max_players=8; status="waiting"} | ConvertTo-Json
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Post -Body $game -ContentType "application/json"
```

**Kết quả**: 
-  Collections được tạo với schema đúng
-  Records có đầy đủ các trường (không chỉ ID)
-  Hiển thị tất cả cột trong admin dashboard

---

## Cach Tao Collections Moi Trong Tuong Lai

### **Luu Y Quan Trong**: Cu Phap Dung

PocketBase sử dụng **`fields`** (KHÔNG PHẢI `schema`)!

### Bước 1: Tạo Migration File Mới
```javascript
// File: pocketbase/pb_migrations/[timestamp]_your_migration.js

/// <reference path="../pb_data/types.d.ts" />
migrate((app) => {
  const collection = new Collection({
    "name": "your_collection",
    "type": "base",
    "fields": [  //  ĐÚNG: Dùng "fields", không phải "schema"
      {
        "autogeneratePattern": "[a-z0-9]{15}",
        "id": "text_id",              //  Bắt buộc: ID field
        "name": "id",
        "type": "text",
        "primaryKey": true,
        "system": true,
        "required": true
      },
      {
        "id": "text_field1",          //  Bắt buộc: ID duy nhất cho mỗi field
        "name": "field_name",
        "type": "text",                // text, number, email, bool, json, select, relation, autodate
        "required": true,
        "presentable": true,           //  Quan trọng: Để hiển thị trong admin UI
        "min": 1,
        "max": 100
      },
      {
        "id": "autodate_created",
        "name": "created",
        "type": "autodate",
        "onCreate": true,
        "onUpdate": false
      },
      {
        "id": "autodate_updated",
        "name": "updated",
        "type": "autodate",
        "onCreate": true,
        "onUpdate": true
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });
  
  app.save(collection);
}, (app) => {
  // Rollback: Xóa collection
  const collection = app.findCollectionByNameOrId("your_collection");
  if (collection) app.delete(collection);
});
```

### **SAI** vs **DUNG**

```javascript
//  SAI (sẽ tạo collection nhưng không lưu fields)
"schema": [
  { name: "title", type: "text" }
]

//  ĐÚNG (lưu đầy đủ fields)
"fields": [
  {
    "id": "text_title",      // Bắt buộc
    "name": "title",
    "type": "text",
    "presentable": true      // Để hiển thị trong UI
  }
]
```

### Bước 2: Apply Migration
```powershell
# Dừng PocketBase trước
Get-Process -Name "pocketbase" | Stop-Process -Force

# Apply migration
.\pocketbase\pocketbase.exe migrate up

# Khởi động lại
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090
```

### Bước 3: Rollback Nếu Cần
```powershell
# Rollback migration cuối cùng
.\pocketbase\pocketbase.exe migrate down 1
```

---

## Truy Cap Collections

### Admin Dashboard
- **URL**: http://127.0.0.1:8090/_/
- **Login**: vominhhauviettel@gmail.com / pt123456789

### API Endpoints (PowerShell)

**Lấy danh sách games:**
```powershell
$games = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get
$games.items | Format-Table id, name, max_players, status
```

**Tạo game mới:**
```powershell
$game = @{
  name = "My Game"
  max_players = 4
  status = "waiting"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" `
  -Method Post `
  -Body $game `
  -ContentType "application/json"
```

**Lấy danh sách players:**
```powershell
$players = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/players/records" -Method Get
$players.items | Format-Table id, username, email, score, is_online
```

**Tạo player mới:**
```powershell
$player = @{
  username = "player1"
  email = "player1@example.com"
  score = 0
  is_online = $true
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/players/records" `
  -Method Post `
  -Body $player `
  -ContentType "application/json"
```

**Lấy 1 record cụ thể:**
```powershell
$gameId = "x8jvvucpv26j99t"
$game = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records/$gameId" -Method Get
$game | ConvertTo-Json
```

**Update record:**
```powershell
$gameId = "x8jvvucpv26j99t"
$update = @{status = "playing"} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records/$gameId" `
  -Method Patch `
  -Body $update `
  -ContentType "application/json"
```

**Xóa record:**
```powershell
$gameId = "x8jvvucpv26j99t"
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records/$gameId" -Method Delete
```

---

## Ket Noi Tu Rust Code

```rust
// worker/src/database.rs đã có PocketBaseClient
use worker::database::PocketBaseClient;

// Khởi tạo
let mut client = PocketBaseClient::new();

// Test connection
client.test_connection().await?;

// Authenticate (nếu cần)
client.authenticate("admin@example.com", "password").await?;

// Sử dụng client để save/load data
```

---

## Tom Tat

 **Collections đã được tạo tự động** bằng migrations với cú pháp đúng (`fields`)  
 **PocketBase đang chạy** ở http://127.0.0.1:8090  
 **Admin dashboard** hiển thị đầy đủ các cột (không chỉ ID)  
 **API endpoints** sẵn sàng sử dụng  
 **Rust code** có thể kết nối qua PocketBaseClient  
 **Records có đầy đủ dữ liệu**: name, max_players, status, created, updated

### Dữ Liệu Mẫu Đã Tạo:
```
GAMES:
- Battle Royale FIXED (8 players, waiting)
- Team Deathmatch (4 players, playing)
- Capture the Flag (6 players, finished)

PLAYERS:
- player_alpha (1500 score, online)
- player_beta (2000 score, online)
```  

---

## Buoc Tiep Theo

1. **Mở Admin Dashboard** để xem collections
2. **Thêm dữ liệu mẫu** qua dashboard hoặc API
3. **Test API endpoints** với curl/Postman
4. **Tích hợp vào Rust code** (worker/gateway)
5. **Chạy full system** với `powershell -File scripts/run-dev.ps1`

---

## **BAI HOC QUAN TRONG**

### **Vấn Đề Gặp Phải Ban Đầu**:
- Collections chỉ có cột **ID**, không có các trường khác
- Records được tạo nhưng không lưu data: `{id: "abc", collectionId: "xyz"}` (thiếu name, max_players, status)

### **Nguyên Nhân**:
- Migration file dùng **`schema`** thay vì **`fields`**
- Thiếu các thuộc tính bắt buộc: `id`, `presentable`

### **Giải Pháp**:
1.  Dùng **`fields`** (không phải `schema`)
2.  Mỗi field phải có `id` duy nhất
3.  Set `presentable: true` để hiển thị trong admin UI
4.  Luôn có field `id` (primary key), `created`, `updated` (autodate)

### **Tham Khảo Migration Đúng**:
- File: `pocketbase/pb_migrations/1759400001_fix_collections.js`
- File: `VAN-DE-DA-SUA.md` (giải thích chi tiết lỗi và cách sửa)
- File: `TAO-CSDL-TU-DONG-CHI-TIET.md` (quy trình từng bước)

---

**Chuc mung! Ban da tao co so du lieu thanh cong bang lenh voi schema dung!**

