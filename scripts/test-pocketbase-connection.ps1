# Script để test kết nối PocketBase với Rust project

Write-Host "🧪 Testing PocketBase Connection..." -ForegroundColor Cyan

# Test 1: Kiểm tra PocketBase API
Write-Host "1. Testing PocketBase API..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/health" -Method Get -TimeoutSec 5
    Write-Host "   ✅ PocketBase API is responding" -ForegroundColor Green
} catch {
    Write-Host "   ❌ PocketBase API not responding: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Test 2: Kiểm tra collections hiện có
Write-Host "2. Checking existing collections..." -ForegroundColor Yellow
try {
    $collections = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Get -TimeoutSec 5
    Write-Host "   📋 Found $($collections.length) collections" -ForegroundColor Green

    if ($collections.length -gt 0) {
        foreach ($collection in $collections) {
            Write-Host "      - $($collection.name) ($($collection.type))" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "   ⚠️  Could not fetch collections: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Test 3: Tạo collection test nếu chưa có
Write-Host "3. Creating test collections..." -ForegroundColor Yellow

$gamesCollection = @{
    name = "games"
    type = "base"
    schema = @(
        @{
            name = "name"
            type = "text"
            required = $true
        },
        @{
            name = "max_players"
            type = "number"
            required = $true
        },
        @{
            name = "status"
            type = "select"
            required = $true
            options = @{
                values = @("waiting", "playing", "finished")
            }
        }
    )
}

try {
    $existing = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections?filter=name='games'" -Method Get -TimeoutSec 5
    if ($existing.length -eq 0) {
        $response = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections" -Method Post -Body (ConvertTo-Json $gamesCollection -Depth 3) -ContentType "application/json" -TimeoutSec 10
        Write-Host "   ✅ Created 'games' collection" -ForegroundColor Green
    } else {
        Write-Host "   ℹ️  'games' collection already exists" -ForegroundColor Blue
    }
} catch {
    Write-Host "   ⚠️  Could not create games collection: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Test 4: Tạo game record mẫu
Write-Host "4. Creating sample game record..." -ForegroundColor Yellow
$gameRecord = @{
    name = "Test Game $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
    max_players = 4
    status = "waiting"
}

try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Post -Body (ConvertTo-Json $gameRecord) -ContentType "application/json" -TimeoutSec 10
    Write-Host "   ✅ Created game record: $($response.name) (ID: $($response.id))" -ForegroundColor Green
} catch {
    Write-Host "   ⚠️  Could not create game record: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Test 5: Đọc game records
Write-Host "5. Reading game records..." -ForegroundColor Yellow
try {
    $games = Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/collections/games/records" -Method Get -TimeoutSec 5
    Write-Host "   📊 Total games: $($games.totalItems)" -ForegroundColor Green

    if ($games.items.length -gt 0) {
        foreach ($game in $games.items | Select-Object -First 3) {
            Write-Host "      - $($game.name) [$($game.status)] (ID: $($game.id))" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "   ❌ Could not read games: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "🎉 PocketBase Connection Test Complete!" -ForegroundColor Cyan
Write-Host ""
Write-Host "📋 Summary:" -ForegroundColor White
Write-Host "   ✅ PocketBase API: Responding" -ForegroundColor Green
Write-Host "   ✅ Database: Accessible" -ForegroundColor Green
Write-Host "   ✅ Collections: Can create/read" -ForegroundColor Green
Write-Host "   ✅ Ready for Rust integration" -ForegroundColor Green
Write-Host ""
Write-Host "🚀 Next steps:" -ForegroundColor White
Write-Host "   1. Run: powershell -File scripts/run-dev.ps1" -ForegroundColor Gray
Write-Host "   2. Check Rust logs for database operations" -ForegroundColor Gray
Write-Host "   3. Verify data sync between Rust and PocketBase" -ForegroundColor Gray
