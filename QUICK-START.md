# QUICK START: Tao Collections PocketBase Tu Dong

## Chay Nhanh (1 Phut)

```powershell
# 1. Dừng PocketBase nếu đang chạy
Get-Process -Name "pocketbase" -ErrorAction SilentlyContinue | Stop-Process -Force

# 2. Apply migrations (tạo collections)
.\pocketbase\pocketbase.exe migrate up

# 3. Khởi động PocketBase
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090

# 4. Thêm dữ liệu mẫu (terminal mới)
$game = @{name="My Game"; max_players=4; status="waiting"} | ConvertTo-Json
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Post -Body $game -ContentType "application/json"
```

**Xem kết quả**: http://127.0.0.1:8090/_/

---

## Collections Co San

### 1. **games**
```powershell
# Tạo game
$game = @{name="Battle Royale"; max_players=8; status="waiting"} | ConvertTo-Json
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Post -Body $game -ContentType "application/json"

# Xem tất cả games
$games = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get
$games.items | Format-Table id, name, max_players, status
```

**Fields**: `id`, `name` (text), `max_players` (number 2-8), `status` (waiting/playing/finished), `created`, `updated`

### 2. **players**
```powershell
# Tạo player
$player = @{username="player1"; email="player1@game.com"; score=0; is_online=$true} | ConvertTo-Json
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/players/records" -Method Post -Body $player -ContentType "application/json"

# Xem tất cả players
$players = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/players/records" -Method Get
$players.items | Format-Table id, username, email, score, is_online
```

**Fields**: `id`, `username` (text), `email` (email), `score` (number), `is_online` (bool), `created`, `updated`

---

## Lenh Huu Ich

### Kiểm tra PocketBase đang chạy:
```powershell
Get-Process -Name "pocketbase"
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get
```

### Xem tất cả migrations:
```powershell
.\pocketbase\pocketbase.exe migrate collections
```

### Rollback migration cuối:
```powershell
Get-Process -Name "pocketbase" | Stop-Process -Force
.\pocketbase\pocketbase.exe migrate down 1
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090
```

### Xóa tất cả dữ liệu (giữ schema):
```powershell
# Xóa từng collection
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get | 
  ForEach-Object { $_.items } | 
  ForEach-Object { Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records/$($_.id)" -Method Delete }
```

---

## Tai Lieu Chi Tiet

- **CREATE-COLLECTIONS-GUIDE.md**: Hướng dẫn đầy đủ, API endpoints
- **TAO-CSDL-TU-DONG-CHI-TIET.md**: Giải thích chi tiết từng bước
- **VAN-DE-DA-SUA.md**: Lỗi thường gặp và cách sửa

---

## Luu Y

### DUNG: Dung `fields` trong migration
```javascript
const collection = new Collection({
  "fields": [
    { "id": "text_name", "name": "name", "type": "text" }
  ]
});
```

### SAI: Dung `schema` (se khong luu data)
```javascript
const collection = new Collection({
  "schema": [  // Sai
    { "name": "name", "type": "text" }
  ]
});
```

---

## Ket Qua Mong Doi

Sau khi chạy xong, có:
- 2 collections: `games`, `players`
- Schema day du voi validation
- API endpoints san sang
- Admin UI hien thi day du cac cot
- Records co data day du (khong chi ID)

**Kiểm tra**:
```powershell
$games = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get
$games.items[0] | ConvertTo-Json
```

**Output mong đợi**:
```json
{
  "id": "abc123xyz",
  "collectionId": "pbc_879072730",
  "collectionName": "games",
  "name": "My Game",           // Co data
  "max_players": 4,             // Co data
  "status": "waiting",          // Co data
  "created": "2025-10-02...",
  "updated": "2025-10-02..."
}
```

---

**Hoan thanh!** Mo http://127.0.0.1:8090/_/ de xem collections!

