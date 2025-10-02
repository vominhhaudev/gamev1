# MO TA CHI TIET: QUY TRINH TAO CO SO DU LIEU TU DONG TREN POCKETBASE

## TOM TAT NHANH

Tôi đã tạo **collections** (bảng dữ liệu) tự động bằng **PocketBase migrations**, sau đó thêm **records** (dữ liệu mẫu) qua API.

---

## BUOC 1: TAO FILE MIGRATION

### **File tạo**: `pocketbase/pb_migrations/1759399999_create_game_collections.js`

```javascript
/// <reference path="../pb_data/types.d.ts" />
migrate((app) => {
  // ========== 1. TẠO GAMES COLLECTION ==========
  const gamesCollection = new Collection({
    "name": "games",                    // Tên collection
    "type": "base",                     // Loại: base (thông thường)
    "system": false,                    // Không phải system collection
    "schema": [                         // Định nghĩa các fields
      {
        "name": "name",                 // Field 1: name
        "type": "text",                 // Kiểu: text (chuỗi)
        "required": true,               // Bắt buộc nhập
        "options": {
          "min": 1,                     // Tối thiểu 1 ký tự
          "max": 100                    // Tối đa 100 ký tự
        }
      },
      {
        "name": "max_players",          // Field 2: max_players
        "type": "number",               // Kiểu: number (số)
        "required": true,
        "options": {
          "min": 2,                     // Tối thiểu 2 người
          "max": 8                      // Tối đa 8 người
        }
      },
      {
        "name": "status",               // Field 3: status
        "type": "select",               // Kiểu: select (dropdown)
        "required": true,
        "options": {
          "maxSelect": 1,               // Chỉ chọn 1 giá trị
          "values": ["waiting", "playing", "finished"]  // 3 giá trị có thể
        }
      }
    ],
    "listRule": "",                     // Rule xem danh sách (rỗng = public)
    "viewRule": "",                     // Rule xem chi tiết (rỗng = public)
    "createRule": "",                   // Rule tạo mới (rỗng = public)
    "updateRule": "",                   // Rule cập nhật (rỗng = public)
    "deleteRule": ""                    // Rule xóa (rỗng = public)
  });

  // ========== 2. TẠO PLAYERS COLLECTION ==========
  const playersCollection = new Collection({
    "name": "players",
    "type": "base",
    "system": false,
    "schema": [
      {
        "name": "username",
        "type": "text",
        "required": true,
        "options": { "min": 3, "max": 50 }
      },
      {
        "name": "email",
        "type": "email",                // Kiểu email (tự động validate)
        "required": true,
        "options": {}
      },
      {
        "name": "score",
        "type": "number",
        "required": false,              // Không bắt buộc
        "options": { "min": 0 }
      },
      {
        "name": "is_online",
        "type": "bool",                 // Kiểu boolean (true/false)
        "required": false
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  // ========== 3. TẠO GAME_SESSIONS COLLECTION ==========
  const sessionsCollection = new Collection({
    "name": "game_sessions",
    "type": "base",
    "system": false,
    "schema": [
      {
        "name": "game_id",
        "type": "relation",             // Kiểu relation (liên kết)
        "required": true,
        "options": {
          "collectionId": gamesCollection.id,  // Liên kết đến games
          "cascadeDelete": true,        // Xóa game → xóa sessions
          "minSelect": 1,
          "maxSelect": 1                // Chỉ chọn 1 game
        }
      },
      {
        "name": "player_id",
        "type": "relation",
        "required": true,
        "options": {
          "collectionId": playersCollection.id, // Liên kết đến players
          "cascadeDelete": true,
          "minSelect": 1,
          "maxSelect": 1
        }
      },
      {
        "name": "position",
        "type": "json",                 // Kiểu JSON (object)
        "required": true,
        "options": { "maxSize": 1000 }  // Tối đa 1000 bytes
      },
      {
        "name": "session_score",
        "type": "number",
        "required": false,
        "options": { "min": 0 }
      },
      {
        "name": "status",
        "type": "select",
        "required": true,
        "options": {
          "maxSelect": 1,
          "values": ["active", "finished"]
        }
      }
    ],
    "listRule": "",
    "viewRule": "",
    "createRule": "",
    "updateRule": "",
    "deleteRule": ""
  });

  // ========== 4. LƯU VÀO DATABASE ==========
  app.save(gamesCollection);         // Lưu games collection
  app.save(playersCollection);       // Lưu players collection
  app.save(sessionsCollection);      // Lưu sessions collection

}, (app) => {
  // ========== HÀM ROLLBACK (HOÀN TÁC) ==========
  const games = app.findCollectionByNameOrId("games");
  const players = app.findCollectionByNameOrId("players");
  const sessions = app.findCollectionByNameOrId("game_sessions");

  if (games) app.delete(games);       // Xóa nếu tồn tại
  if (players) app.delete(players);
  if (sessions) app.delete(sessions);
});
```

### **Giải thích cấu trúc Migration**:

```javascript
migrate(
  (app) => { /* Hàm UP: Tạo collections */ },
  (app) => { /* Hàm DOWN: Xóa collections (rollback) */ }
)
```

- **Hàm UP**: Chạy khi apply migration (`migrate up`)
- **Hàm DOWN**: Chạy khi rollback migration (`migrate down`)

---

## BUOC 2: DUNG POCKETBASE

### **Lệnh chạy**:
```powershell
Get-Process -Name "pocketbase" | Stop-Process -Force
```

### **Tại sao phải dừng?**
- PocketBase đang mở file database SQLite (`pb_data/data.db`)
- SQLite không cho phép nhiều process ghi đồng thời
- Dừng để tránh **database locked** error

### **Điều gì xảy ra**:
1. PowerShell tìm process có tên `pocketbase`
2. Gửi lệnh `SIGKILL` (Force stop)
3. PocketBase đóng kết nối database
4. File `data.db` được unlock

---

## BUOC 3: APPLY MIGRATION

### **Lệnh chạy**:
```powershell
.\pocketbase\pocketbase.exe migrate up
```

### **Điều gì xảy ra bên trong PocketBase**:

#### **1. Đọc migrations folder**
```
pocketbase/pb_migrations/
  ├── 1759370067_created_gamev1.js         (đã apply)
  ├── 1759370110_updated_gamev1.js         (đã apply)
  ├── 1759370140_updated_users.js          (đã apply)
  ├── 1759370829_created_user_max.js       (đã apply)
  └── 1759399999_create_game_collections.js (chưa apply) ← MỚI
```

#### **2. Kiểm tra migrations đã apply**
PocketBase lưu log migrations trong bảng `_migrations`:
```sql
SELECT name FROM _migrations;
-- Kết quả:
-- 1759370067_created_gamev1
-- 1759370110_updated_gamev1
-- 1759370140_updated_users
-- 1759370829_created_user_max
```

#### **3. Tìm migrations chưa apply**
- So sánh files trong folder vs bảng `_migrations`
- Tìm thấy: `1759399999_create_game_collections.js`

#### **4. Load và execute JavaScript**
```javascript
// PocketBase engine tải file .js
const migrationCode = fs.readFileSync("1759399999_create_game_collections.js");

// Chạy hàm migrate()
migrate((app) => {
  // Tạo Collection objects
  const gamesCollection = new Collection({...});
  
  // Convert sang SQL CREATE TABLE
  app.save(gamesCollection);
  // → Thực thi: CREATE TABLE games (id TEXT, name TEXT, max_players INT, ...)
  
  const playersCollection = new Collection({...});
  app.save(playersCollection);
  // → Thực thi: CREATE TABLE players (...)
  
  const sessionsCollection = new Collection({...});
  app.save(sessionsCollection);
  // → Thực thi: CREATE TABLE game_sessions (...)
}, (app) => { /* rollback function */ });
```

#### **5. Lưu log migration**
```sql
INSERT INTO _migrations (name, applied_at) 
VALUES ('1759399999_create_game_collections', '2025-10-02 02:40:00');
```

### **Output**:
```
Applied 1759399999_create_game_collections.js
```

### **Kết quả trong database** (`pb_data/data.db`):

```sql
-- Bảng _collections (metadata về collections)
INSERT INTO _collections (id, name, type, schema) VALUES
  ('pbc_879072730', 'games', 'base', '[{"name":"name","type":"text",...}]'),
  ('pbc_123456789', 'players', 'base', '[...]'),
  ('pbc_987654321', 'game_sessions', 'base', '[...]');

-- Bảng thực tế (rỗng, chưa có dữ liệu)
CREATE TABLE games (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  max_players INTEGER NOT NULL,
  status TEXT NOT NULL,
  created TEXT NOT NULL,
  updated TEXT
);

CREATE TABLE players (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL,
  email TEXT NOT NULL,
  score INTEGER,
  is_online INTEGER,
  created TEXT,
  updated TEXT
);

CREATE TABLE game_sessions (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL,
  player_id TEXT NOT NULL,
  position TEXT NOT NULL,
  session_score INTEGER,
  status TEXT NOT NULL,
  created TEXT,
  updated TEXT,
  FOREIGN KEY (game_id) REFERENCES games(id),
  FOREIGN KEY (player_id) REFERENCES players(id)
);
```

**LUU Y**: Cac bang nay **RONG** (0 records), chi co cau truc!

---

## BUOC 4: KHOI DONG LAI POCKETBASE

### **Lệnh chạy**:
```powershell
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090
```

### **Điều gì xảy ra**:
1. PocketBase đọc `pb_data/data.db`
2. Load các collections đã tạo vào memory
3. Khởi động HTTP server
4. Expose REST API endpoints:
   ```
   GET    /api/collections/games/records
   POST   /api/collections/games/records
   GET    /api/collections/games/records/:id
   PATCH  /api/collections/games/records/:id
   DELETE /api/collections/games/records/:id
   ```

### **Output**:
```
2025/10/02 09:40:38 Server started at http://127.0.0.1:8090
├─ REST API:  http://127.0.0.1:8090/api/
└─ Dashboard: http://127.0.0.1:8090/_/
```

---

## BUOC 5: THEM DU LIEU MAU (RECORDS)

### **Tại sao collections rỗng?**

**Collections ≠ Records**:
- **Collections** = Cấu trúc bảng (schema)
- **Records** = Dữ liệu trong bảng

**So sánh với SQL**:
```sql
-- Bước 3 (Migration): Tạo bảng
CREATE TABLE games (...);  -- Bảng rỗng 
-- Bước 5: Thêm dữ liệu
INSERT INTO games VALUES (...);  -- Thêm records
```

### **Lệnh thêm dữ liệu**:

```powershell
# 1. Thêm game thứ nhất
$game1 = @{
  name = "Battle Royale"
  max_players = 8
  status = "waiting"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" `
  -Method Post `
  -Body $game1 `
  -ContentType "application/json"
```

### **Điều gì xảy ra**:

#### **1. Client gửi HTTP POST**
```http
POST /api/collections/games/records HTTP/1.1
Host: 127.0.0.1:8090
Content-Type: application/json

{
  "name": "Battle Royale",
  "max_players": 8,
  "status": "waiting"
}
```

#### **2. PocketBase xử lý**:

**a. Validate dữ liệu theo schema**:
```javascript
// Kiểm tra "name"
if (!data.name || data.name.length < 1 || data.name.length > 100) {
  return error("name invalid");
}

// Kiểm tra "max_players"
if (!data.max_players || data.max_players < 2 || data.max_players > 8) {
  return error("max_players invalid");
}

// Kiểm tra "status"
if (!["waiting", "playing", "finished"].includes(data.status)) {
  return error("status invalid");
}
```

**b. Tạo ID và timestamps**:
```javascript
const id = generateRandomId(15);  // "t4bv3afrusskukx"
const created = new Date().toISOString();  // "2025-10-02T02:40:00Z"
const updated = created;
```

**c. Insert vào database**:
```sql
INSERT INTO games (id, name, max_players, status, created, updated) 
VALUES ('t4bv3afrusskukx', 'Battle Royale', 8, 'waiting', '2025-10-02T02:40:00Z', '2025-10-02T02:40:00Z');
```

#### **3. Trả về kết quả**:
```json
{
  "id": "t4bv3afrusskukx",
  "collectionId": "pbc_879072730",
  "collectionName": "games",
  "name": "Battle Royale",
  "max_players": 8,
  "status": "waiting",
  "created": "2025-10-02T02:40:00.000Z",
  "updated": "2025-10-02T02:40:00.000Z"
}
```

### **Kết quả sau khi thêm 3 games và 3 players**:

```powershell
Total Games: 3
Total Players: 3
```

**Database bây giờ**:
```sql
-- Bảng games
SELECT * FROM games;
-- id              | name                | max_players | status
-- t4bv3afrusskukx | Battle Royale       | 8           | waiting
-- abc123xyz       | Team Deathmatch     | 4           | playing
-- def456uvw       | Capture the Flag    | 6           | waiting

-- Bảng players
SELECT * FROM players;
-- id              | username      | email              | score | is_online
-- p1a2b3c4d5e     | player_alpha  | alpha@game.com     | 1500  | 1
-- p6f7g8h9i0j     | player_beta   | beta@game.com      | 2000  | 1
-- p1k2l3m4n5o     | player_gamma  | gamma@game.com     | 1200  | 0
```

---

## TOM TAT QUY TRINH

```
┌─────────────────────────────────────────────────────────────┐
│ BƯỚC 1: Tạo Migration File                                  │
│   File: pb_migrations/1759399999_create_game_collections.js│
│   Nội dung: JavaScript code định nghĩa 3 collections       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ BƯỚC 2: Dừng PocketBase                                     │
│   Lệnh: Get-Process -Name "pocketbase" | Stop-Process      │
│   Lý do: Unlock database file                              │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ BƯỚC 3: Apply Migration                                     │
│   Lệnh: .\pocketbase\pocketbase.exe migrate up             │
│   Kết quả:                                                  │
│   • Đọc migration file                                      │
│   • Chạy JavaScript code                                    │
│   • Tạo 3 tables trong SQLite: games, players, sessions    │
│   • Lưu schema vào _collections                            │
│   • Ghi log vào _migrations                                │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ BƯỚC 4: Khởi động lại PocketBase                            │
│   Lệnh: .\pocketbase\pocketbase.exe serve                  │
│   Kết quả:                                                  │
│   • Load collections từ database                           │
│   • Khởi động HTTP server                                  │
│   • Expose REST API endpoints                              │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ BƯỚC 5: Thêm dữ liệu mẫu (Records)                          │
│   Lệnh: Invoke-RestMethod POST /api/collections/.../records│
│   Kết quả:                                                  │
│   • Validate dữ liệu theo schema                           │
│   • INSERT vào SQLite tables                               │
│   • Collections không còn rỗng                             │
│   • 3 games + 3 players created                            │
└─────────────────────────────────────────────────────────────┘
```

---

## COLLECTIONS VS RECORDS

| Khái niệm | Giải thích | SQL tương đương | Tạo bằng |
|-----------|------------|-----------------|----------|
| **Collection** | Cấu trúc bảng (schema) | `CREATE TABLE` | Migration |
| **Record** | Dòng dữ liệu | `INSERT INTO` | API POST |
| **Field** | Cột trong bảng | Column | Schema definition |
| **Schema** | Định nghĩa các fields | Table structure | JavaScript object |

---

## KET LUAN

**Những gì đã tạo TỰ ĐỘNG**:
 3 Collections (games, players, game_sessions)  
 Schema đầy đủ (fields, types, validation)  
 Relations giữa collections  
 API endpoints tự động  
 3 games + 3 players records (dữ liệu mẫu)  

**Cách xem**:
- Admin dashboard: http://127.0.0.1:8090/_/
- Vào "Collections" → Click "games" hoặc "players"
- Bạn sẽ thấy 3 records trong mỗi collection

**Bay gio collections KHONG con rong!**

