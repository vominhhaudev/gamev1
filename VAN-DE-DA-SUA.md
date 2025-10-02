# DA SUA LOI: COLLECTIONS CHI CO TRUONG ID

## **VAN DE**

Khi xem collections trong admin dashboard, **chỉ thấy cột ID**, không thấy các trường khác như `name`, `max_players`, `status`.

**Kiểm tra qua API**:
```json
{
  "collectionId": "pbc_879072730",
  "collectionName": "games",
  "id": "t4bv3afrusskukx"
  // Khong co: name, max_players, status
}
```

---

## **NGUYEN NHAN**

Migration file ban đầu sử dụng **`schema`** (sai):

```javascript
const collection = new Collection({
  "name": "games",
  "type": "base",
  "schema": [  //  SAI: PocketBase dùng "fields", không phải "schema"
    { name: "name", type: "text", required: true }
  ]
});
```

**Cú pháp đúng của PocketBase**:
```javascript
const collection = new Collection({
  "name": "games",
  "type": "base",
  "fields": [  //  ĐÚNG: Phải dùng "fields"
    {
      "id": "text_name",          // Phải có ID duy nhất
      "name": "name",
      "type": "text",
      "required": true,
      "min": 1,
      "max": 100,
      "presentable": true,        // Hiển thị trong dashboard
      "system": false
    }
  ]
});
```

**Khác biệt quan trọng**:
- `schema` = Cách tưởng tượng (giống ORM)
- `fields` = Cú pháp thực tế của PocketBase (giống table definition)

---

## **CACH SUA**

### **Bước 1: Tạo Migration Sửa Lỗi**

File: `pocketbase/pb_migrations/1759400001_fix_collections.js`

```javascript
migrate((app) => {
  // Xóa collections cũ bị lỗi
  const oldGames = app.findCollectionByNameOrId("games");
  if (oldGames) app.delete(oldGames);

  // Tạo lại với cú pháp ĐÚNG
  const gamesCollection = new Collection({
    "name": "games",
    "type": "base",
    "fields": [  //  Đúng: dùng "fields"
      {
        "autogeneratePattern": "[a-z0-9]{15}",
        "id": "text_id",         //  ID field
        "name": "id",
        "type": "text",
        "primaryKey": true,
        "system": true,
        "required": true
      },
      {
        "id": "text_name",       //  Unique ID cho field
        "name": "name",          //  Tên field
        "type": "text",
        "required": true,
        "min": 1,
        "max": 100,
        "presentable": true      //  Hiển thị trong admin UI
      },
      {
        "id": "number_max_players",
        "name": "max_players",
        "type": "number",
        "required": true,
        "min": 2,
        "max": 8
      },
      {
        "id": "select_status",
        "name": "status",
        "type": "select",
        "required": true,
        "maxSelect": 1,
        "values": ["waiting", "playing", "finished"]
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
    ]
  });

  app.save(gamesCollection);
});
```

### **Bước 2: Apply Migration**

```powershell
# Dừng PocketBase
Get-Process -Name "pocketbase" | Stop-Process -Force

# Apply migration
.\pocketbase\pocketbase.exe migrate up
# Output: Applied 1759400001_fix_collections.js

# Khởi động lại
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090
```

### **Bước 3: Test Tạo Record**

```powershell
$game = @{
  name = "Battle Royale FIXED"
  max_players = 8
  status = "waiting"
} | ConvertTo-Json

$result = Invoke-RestMethod `
  -Uri "http://127.0.0.1:8090/api/collections/games/records" `
  -Method Post `
  -Body $game `
  -ContentType "application/json"

$result | ConvertTo-Json
```

**Output (ĐÚNG)**:
```json
{
  "collectionId": "pbc_879072730",
  "collectionName": "games",
  "id": "x8jvvucpv26j99t",
  "name": "Battle Royale FIXED",    //  CÓ RỒI!
  "max_players": 8,                  //  CÓ RỒI!
  "status": "waiting",               //  CÓ RỒI!
  "created": "2025-10-02T02:53:57.909Z",
  "updated": "2025-10-02T02:53:57.909Z"
}
```

---

## **KET QUA SAU KHI SUA**

### **Games Collection**:
```
id              name                max_players status   created
--              ----                ----------- ------   -------
x8jvvucpv26j99t Battle Royale FIXED           8 waiting  2025-10-02 02:53:57
vtexzsgs0xs4ebd Team Deathmatch               4 playing  2025-10-02 02:55:52
84t5z8o5hm79xi3 Capture the Flag              6 finished 2025-10-02 02:55:52
```

### **Players Collection**:
```
id              username     email          score is_online
--              --------     -----          ----- ---------
v4null9iv8czmt5 player_alpha alpha@game.com  1500      True
1likcys2z6l731n player_beta  beta@game.com   2000      True
```

---

## **BAI HOC**

### **1. Cú pháp PocketBase Migration**

```javascript
//  SAI (giống ORM)
new Collection({
  schema: [{ name: "field1", type: "text" }]
})

//  ĐÚNG (PocketBase syntax)
new Collection({
  fields: [
    {
      id: "unique_id",      // Bắt buộc
      name: "field1",
      type: "text",
      required: true,
      presentable: true     // Để hiển thị trong admin UI
    }
  ]
})
```

### **2. Các thuộc tính quan trọng**

| Thuộc tính | Mô tả | Bắt buộc |
|------------|-------|----------|
| `id` | ID duy nhất cho field |  Yes |
| `name` | Tên field (tên cột) |  Yes |
| `type` | Kiểu dữ liệu (text, number, email, bool, select, relation, json, autodate) |  Yes |
| `required` | Bắt buộc nhập | No |
| `presentable` | Hiển thị trong admin dashboard | No |
| `system` | Field hệ thống (id, created, updated) | No |
| `primaryKey` | Primary key (chỉ dùng cho field `id`) | No |

### **3. Các kiểu field thường dùng**

```javascript
// Text field
{ id: "text1", name: "title", type: "text", min: 1, max: 100 }

// Number field
{ id: "num1", name: "score", type: "number", min: 0, max: 1000 }

// Email field
{ id: "email1", name: "email", type: "email" }

// Bool field
{ id: "bool1", name: "is_active", type: "bool" }

// Select field (dropdown)
{ 
  id: "select1", 
  name: "status", 
  type: "select",
  maxSelect: 1,
  values: ["pending", "approved", "rejected"]
}

// JSON field
{ id: "json1", name: "metadata", type: "json", maxSize: 1000 }

// Autodate field (tự động update)
{ 
  id: "date1", 
  name: "created", 
  type: "autodate",
  onCreate: true,
  onUpdate: false
}

// Relation field (foreign key)
{
  id: "rel1",
  name: "user_id",
  type: "relation",
  collectionId: "users_collection_id",
  cascadeDelete: true,
  minSelect: 1,
  maxSelect: 1
}
```

---

## **TOM TAT**

 **Đã sửa**: Migration dùng `schema` → `fields`  
 **Đã test**: Records bây giờ có đầy đủ fields  
 **Đã thêm**: 3 games + 2 players với dữ liệu đầy đủ  
 **Hiển thị**: Admin dashboard bây giờ show tất cả các cột  

**Mở browser**: http://127.0.0.1:8090/_/#/collections  
→ Vào **games** hoặc **players**  
→ Bạn sẽ thấy **TẤT CẢ các cột**: id, name, max_players, status, created, updated!

---

## **LENH DA CHAY DE SUA**

```powershell
# 1. Dừng PocketBase
Get-Process -Name "pocketbase" | Stop-Process -Force

# 2. Apply migration sửa lỗi
.\pocketbase\pocketbase.exe migrate up

# 3. Khởi động lại
.\pocketbase\pocketbase.exe serve --http=127.0.0.1:8090

# 4. Thêm dữ liệu mới (đúng)
$game = @{name="Battle Royale"; max_players=8; status="waiting"} | ConvertTo-Json
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Post -Body $game -ContentType "application/json"
```

**Hoan thanh!**

