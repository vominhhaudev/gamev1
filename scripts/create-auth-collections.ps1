# Script tạo collections cho Authentication & Security system

param(
    [string]$PocketBaseUrl = "http://127.0.0.1:8090"
)

Write-Host "🔐 Creating Authentication Collections..." -ForegroundColor Cyan
Write-Host "PocketBase URL: $PocketBaseUrl" -ForegroundColor Gray

# Test connection first
try {
    $health = Invoke-RestMethod -Uri "$PocketBaseUrl/api/health" -Method Get -TimeoutSec 5
    Write-Host "✅ PocketBase is responding" -ForegroundColor Green
} catch {
    Write-Host "❌ Cannot connect to PocketBase: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Collection 1: Users (Enhanced for authentication)
$usersCollection = @{
    name = "users"
    type = "base"
    schema = @(
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
            name = "password_hash"
            type = "text"
            required = $true
            options = @{ max = 255 }
        },
        @{
            name = "email_verified"
            type = "bool"
            required = $true
        },
        @{
            name = "role"
            type = "select"
            required = $true
            options = @{ values = @("user", "admin", "moderator") }
        },
        @{
            name = "is_active"
            type = "bool"
            required = $true
        },
        @{
            name = "last_login"
            type = "date"
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
    rules = @{
        createRule = ""
        updateRule = "@request.auth.id = id"
        deleteRule = "@request.auth.role = ""admin"""
        viewRule = "@request.auth.role = ""admin"""
    }
}

# Collection 2: Authentication Tokens
$authTokensCollection = @{
    name = "auth_tokens"
    type = "base"
    schema = @(
        @{
            name = "user_id"
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
            name = "token_hash"
            type = "text"
            required = $true
            options = @{ max = 255 }
        },
        @{
            name = "token_type"
            type = "select"
            required = $true
            options = @{ values = @("access", "refresh") }
        },
        @{
            name = "expires_at"
            type = "date"
            required = $true
        },
        @{
            name = "is_revoked"
            type = "bool"
            required = $true
        },
        @{
            name = "created"
            type = "date"
            required = $true
        }
    )
    rules = @{
        createRule = "@request.auth.id != """""
        updateRule = "@request.auth.id = user_id"
        deleteRule = "@request.auth.role = ""admin"""
        viewRule = "@request.auth.id = user_id"
    }
}

# Collection 3: Login Sessions
$sessionsCollection = @{
    name = "login_sessions"
    type = "base"
    schema = @(
        @{
            name = "user_id"
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
            name = "session_id"
            type = "text"
            required = $true
            options = @{ max = 255 }
        },
        @{
            name = "ip_address"
            type = "text"
            required = $true
            options = @{ max = 45 }
        },
        @{
            name = "user_agent"
            type = "text"
            options = @{ max = 500 }
        },
        @{
            name = "expires_at"
            type = "date"
            required = $true
        },
        @{
            name = "is_active"
            type = "bool"
            required = $true
        },
        @{
            name = "created"
            type = "date"
            required = $true
        }
    )
    rules = @{
        createRule = "@request.auth.id != """""
        updateRule = "@request.auth.role = ""admin"""
        deleteRule = "@request.auth.role = ""admin"""
        viewRule = "@request.auth.id = user_id"
    }
}

# Collection 4: Password Reset Tokens
$passwordResetCollection = @{
    name = "password_reset_tokens"
    type = "base"
    schema = @(
        @{
            name = "user_id"
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
            name = "token_hash"
            type = "text"
            required = $true
            options = @{ max = 255 }
        },
        @{
            name = "expires_at"
            type = "date"
            required = $true
        },
        @{
            name = "used"
            type = "bool"
            required = $true
        },
        @{
            name = "created"
            type = "date"
            required = $true
        }
    )
    rules = @{
        createRule = ""
        updateRule = "@request.auth.role = ""admin"""
        deleteRule = "@request.auth.role = ""admin"""
        viewRule = "@request.auth.role = ""admin"""
    }
}

# Function to create collection if not exists
function Create-Collection {
    param($CollectionData)

    $collectionName = $CollectionData.name
    Write-Host "📁 Creating collection: $collectionName" -ForegroundColor Yellow

    try {
        # Check if collection exists
        $existing = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections?filter=name='$collectionName'" -Method Get -TimeoutSec 5 -ErrorAction SilentlyContinue

        if ($existing -and $existing.Count -gt 0) {
            Write-Host "   ℹ️  Collection '$collectionName' already exists" -ForegroundColor Blue
            return $true
        }

        # Create new collection
        $response = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections" -Method Post -Body (ConvertTo-Json $CollectionData -Depth 4) -ContentType "application/json" -TimeoutSec 10
        Write-Host "   ✅ Created collection '$collectionName'" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "   ❌ Failed to create collection '$collectionName': $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Create authentication collections
$authCollections = @($usersCollection, $authTokensCollection, $sessionsCollection, $passwordResetCollection)
$successCount = 0

foreach ($collection in $authCollections) {
    if (Create-Collection -CollectionData $collection) {
        $successCount++
    }
}

# Summary
Write-Host ""
Write-Host "📊 Authentication Collections Summary:" -ForegroundColor White
Write-Host "   ✅ Successfully created: $successCount collections" -ForegroundColor Green
Write-Host "   📋 Collections created:" -ForegroundColor White
Write-Host "      • users (enhanced for authentication)" -ForegroundColor Gray
Write-Host "      • auth_tokens (JWT token storage)" -ForegroundColor Gray
Write-Host "      • login_sessions (session tracking)" -ForegroundColor Gray
Write-Host "      • password_reset_tokens (password reset functionality)" -ForegroundColor Gray
Write-Host ""
Write-Host "🔐 Authentication System Ready!" -ForegroundColor Cyan
Write-Host "   You can now:" -ForegroundColor White
Write-Host "   • Access admin dashboard: http://127.0.0.1:8090/_/" -ForegroundColor Gray
Write-Host "   • Create/edit user records in the collections" -ForegroundColor Gray
Write-Host "   • Test authentication API endpoints" -ForegroundColor Gray
