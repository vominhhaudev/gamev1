# Script tao collections cho du an game mot cach tu dong

param(
    [string]$AdminEmail = "admin@pocketbase.local",
    [string]$AdminPassword = "123456789",
    [string]$PocketBaseUrl = "http://127.0.0.1:8090"
)

Write-Host "Tao Collections cho du an Game..." -ForegroundColor Cyan
Write-Host "Admin: $AdminEmail" -ForegroundColor Gray

# Test connection
try {
    $health = Invoke-RestMethod -Uri "$PocketBaseUrl/api/health" -Method Get -TimeoutSec 5
    Write-Host "PocketBase hoat dong tot" -ForegroundColor Green
} catch {
    Write-Host "Khong the ket noi PocketBase: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Dang nhap de lay auth token
Write-Host "Dang nhap admin..." -ForegroundColor Yellow
$authData = @{
    identity = $AdminEmail
    password = $AdminPassword
} | ConvertTo-Json

try {
    $authResponse = Invoke-RestMethod -Uri "$PocketBaseUrl/api/admins/auth-with-password" -Method Post -Body $authData -ContentType "application/json"
    $authToken = $authResponse.token
    Write-Host "Dang nhap thanh cong!" -ForegroundColor Green
} catch {
    Write-Host "Dang nhap that bai: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Hay kiem tra:" -ForegroundColor Yellow
    Write-Host "   1. Email/password dung chua?" -ForegroundColor Yellow
    Write-Host "   2. Admin user co ton tai khong?" -ForegroundColor Yellow
    Write-Host "   3. Mo trinh duyet: http://127.0.0.1:8090/_/ de tao admin" -ForegroundColor Yellow
    exit 1
}

# Headers voi auth token
$headers = @{
    "Authorization" = "Bearer $authToken"
    "Content-Type" = "application/json"
}

# Function tao collection neu chua ton tai
function New-GameCollection {
    param($CollectionName, $Schema)

    Write-Host "Tao collection: $CollectionName..." -ForegroundColor Yellow

    try {
        # Kiem tra collection da ton tai chua
        $existing = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections?filter=name='$CollectionName'" -Method Get -Headers $headers -TimeoutSec 5 -ErrorAction SilentlyContinue

        if ($existing -and $existing.Count -gt 0) {
            Write-Host "   Collection '$CollectionName' da ton tai" -ForegroundColor Blue
            return $true
        }

        # Tao collection moi
        $collectionData = @{
            name = $CollectionName
            type = "base"
            schema = $Schema
        }

        $response = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections" -Method Post -Body (ConvertTo-Json $collectionData -Depth 3) -Headers $headers -TimeoutSec 10
        Write-Host "   Da tao collection '$CollectionName'" -ForegroundColor Green
        return $true

    } catch {
        Write-Host "   Loi tao collection '$CollectionName': $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Tao cac collections can thiet cho du an game

# 1. Games Collection
$gamesSchema = @(
    @{
        name = "name"
        type = "text"
        required = $true
        options = @{ max = 100 }
    },
    @{
        name = "max_players"
        type = "number"
        required = $true
        options = @{ min = 2; max = 8 }
    },
    @{
        name = "status"
        type = "select"
        required = $true
        options = @{ values = @("waiting", "playing", "finished") }
    },
    @{
        name = "created"
        type = "date"
        required = $true
    },
    @{
        name = "updated"
        type = "date"
    }
)

New-GameCollection -CollectionName "games" -Schema $gamesSchema

# 2. Users Collection
$usersSchema = @(
    @{
        name = "username"
        type = "text"
        required = $true
        options = @{ max = 50 }
    },
    @{
        name = "email"
        type = "email"
        required = $true
    },
    @{
        name = "score"
        type = "number"
        options = @{ min = 0 }
    },
    @{
        name = "is_online"
        type = "bool"
    },
    @{
        name = "created"
        type = "date"
        required = $true
    }
)

New-GameCollection -CollectionName "users" -Schema $usersSchema

# 3. Game Sessions Collection
$sessionsSchema = @(
    @{
        name = "game_id"
        type = "relation"
        required = $true
        options = @{
            collectionId = "games"
            cascadeDelete = $true
            minSelect = $null
            maxSelect = 1
        }
    },
    @{
        name = "player_id"
        type = "relation"
        required = $true
        options = @{
            collectionId = "users"
            cascadeDelete = $true
            minSelect = $null
            maxSelect = 1
        }
    },
    @{
        name = "position"
        type = "json"
        required = $true
    },
    @{
        name = "session_score"
        type = "number"
        options = @{ min = 0 }
    },
    @{
        name = "status"
        type = "select"
        required = $true
        options = @{ values = @("active", "finished") }
    },
    @{
        name = "created"
        type = "date"
        required = $true
    }
)

New-GameCollection -CollectionName "game_sessions" -Schema $sessionsSchema

# Summary
Write-Host ""
Write-Host "Tom tat:" -ForegroundColor White
Write-Host "   Da tao collections cho du an game" -ForegroundColor Green
Write-Host "   Collections duoc tao:" -ForegroundColor White
Write-Host "      • games (thong tin game)" -ForegroundColor Gray
Write-Host "      • users (thong tin nguoi choi)" -ForegroundColor Gray
Write-Host "      • game_sessions (phien choi)" -ForegroundColor Gray
Write-Host ""
Write-Host "San sang su dung!" -ForegroundColor Cyan
Write-Host "   • Admin Dashboard: http://127.0.0.1:8090/_/" -ForegroundColor Green
Write-Host "   • API Endpoints: http://127.0.0.1:8090/api/collections/" -ForegroundColor Green
Write-Host "   • Rust code co the ket noi va su dung" -ForegroundColor Green

# Test mot vai API calls
Write-Host ""
Write-Host "Test ket noi..." -ForegroundColor Yellow

try {
    $collections = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections" -Method Get -Headers $headers
    Write-Host "   Co the truy cap collections API" -ForegroundColor Green
    Write-Host "   Tong so collections: $($collections.Count)" -ForegroundColor Green
} catch {
    Write-Host "   Khong the truy cap collections API: $($_.Exception.Message)" -ForegroundColor Yellow
}