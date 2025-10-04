# Script tạo collections cho game trong PocketBase
param(
    [string]$PocketBaseUrl = "http://localhost:8090",
    [string]$AdminEmail = "admin@pocketbase.local",
    [string]$AdminPassword = "123456789"
)

Write-Host "Creating Game Collections in PocketBase..." -ForegroundColor Green
Write-Host "Target: $PocketBaseUrl" -ForegroundColor Cyan

# Import helper functions
. "$PSScriptRoot\common.ps1"

# Authenticate với PocketBase
try {
    $authResponse = Invoke-RestMethod -Uri "$PocketBaseUrl/api/admins/auth-with-password" `
        -Method Post `
        -ContentType "application/json" `
        -Body (@{
            identity = $AdminEmail
            password = $AdminPassword
        } | ConvertTo-Json)

    $adminToken = $authResponse.token
    Write-Host "Authenticated as admin" -ForegroundColor Green

    $headers = @{
        "Authorization" = "Bearer $adminToken"
        "Content-Type" = "application/json"
    }
}
catch {
    Write-Host "Failed to authenticate: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Collection 1: Players (người chơi)
$playersCollection = @{
    name = "players"
    type = "base"
    schema = @(
        @{
            name = "username"
            type = "text"
            required = $true
            unique = $true
        },
        @{
            name = "email"
            type = "email"
            required = $true
        },
        @{
            name = "avatar"
            type = "file"
        },
        @{
            name = "score"
            type = "number"
            required = $true
        },
        @{
            name = "level"
            type = "number"
            required = $true
        },
        @{
            name = "is_online"
            type = "bool"
            required = $true
        },
        @{
            name = "last_seen"
            type = "date"
        },
        @{
            name = "sticky_token"
            type = "text"
            required = $true
        }
    )
    indexes = @(
        "CREATE UNIQUE INDEX idx_players_username ON players (username)",
        "CREATE INDEX idx_players_online ON players (is_online)",
        "CREATE INDEX idx_players_score ON players (score DESC)"
    )
}

# Collection 2: Rooms (phòng chơi)
$roomsCollection = @{
    name = "rooms"
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
            name = "current_players"
            type = "number"
            required = $true
        },
        @{
            name = "status"
            type = "select"
            required = $true
            options = @{
                values = @("waiting", "starting", "in_progress", "finished")
            }
        },
        @{
            name = "game_mode"
            type = "select"
            required = $true
            options = @{
                values = @("battle_royale", "team_deathmatch", "capture_flag")
            }
        },
        @{
            name = "map"
            type = "text"
            required = $true
        },
        @{
            name = "created_by"
            type = "relation"
            required = $true
            options = @{
                collectionId = "players"
            }
        },
        @{
            name = "created_at"
            type = "date"
            required = $true
        },
        @{
            name = "started_at"
            type = "date"
        },
        @{
            name = "finished_at"
            type = "date"
        },
        @{
            name = "settings"
            type = "json"
        }
    )
    indexes = @(
        "CREATE INDEX idx_rooms_status ON rooms (status)",
        "CREATE INDEX idx_rooms_created ON rooms (created_at DESC)",
        "CREATE INDEX idx_rooms_players ON rooms (current_players, max_players)"
    )
}

# Collection 3: Matches (trận đấu)
$matchesCollection = @{
    name = "matches"
    type = "base"
    schema = @(
        @{
            name = "room_id"
            type = "relation"
            required = $true
            options = @{
                collectionId = "rooms"
            }
        },
        @{
            name = "status"
            type = "select"
            required = $true
            options = @{
                values = @("preparing", "in_progress", "finished", "cancelled")
            }
        },
        @{
            name = "winner"
            type = "relation"
            options = @{
                collectionId = "players"
            }
        },
        @{
            name = "duration"
            type = "number"
        },
        @{
            name = "total_score"
            type = "number"
        },
        @{
            name = "started_at"
            type = "date"
        },
        @{
            name = "finished_at"
            type = "date"
        },
        @{
            name = "game_state"
            type = "json"
        }
    )
    indexes = @(
        "CREATE INDEX idx_matches_room ON matches (room_id)",
        "CREATE INDEX idx_matches_status ON matches (status)",
        "CREATE INDEX idx_matches_finished ON matches (finished_at DESC)"
    )
}

# Collection 4: Participants (người tham gia trận đấu)
$participantsCollection = @{
    name = "participants"
    type = "base"
    schema = @(
        @{
            name = "match_id"
            type = "relation"
            required = $true
            options = @{
                collectionId = "matches"
            }
        },
        @{
            name = "player_id"
            type = "relation"
            required = $true
            options = @{
                collectionId = "players"
            }
        },
        @{
            name = "team"
            type = "text"
        },
        @{
            name = "position"
            type = "number"
        },
        @{
            name = "score"
            type = "number"
            required = $true
        },
        @{
            name = "kills"
            type = "number"
            required = $true
        },
        @{
            name = "deaths"
            type = "number"
            required = $true
        },
        @{
            name = "joined_at"
            type = "date"
            required = $true
        },
        @{
            name = "left_at"
            type = "date"
        }
    )
    indexes = @(
        "CREATE INDEX idx_participants_match ON participants (match_id)",
        "CREATE INDEX idx_participants_player ON participants (player_id)",
        "CREATE INDEX idx_participants_score ON participants (score DESC)"
    )
}

# Tạo collections
$collections = @($playersCollection, $roomsCollection, $matchesCollection, $participantsCollection)

foreach ($collection in $collections) {
    $collectionName = $collection.name
    Write-Host "Creating collection: $collectionName" -ForegroundColor Yellow

    try {
        $response = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections" `
            -Method Post `
            -Headers $headers `
            -ContentType "application/json" `
            -Body ($collection | ConvertTo-Json -Depth 10)

        Write-Host "  Collection $($collectionName) created successfully" -ForegroundColor Green

        # Tạo indexes nếu có
        if ($collection.indexes) {
            foreach ($indexSql in $collection.indexes) {
                try {
                    # Note: PocketBase không hỗ trợ tạo indexes qua API, cần chạy SQL trực tiếp
                    Write-Host "  Index: $indexSql" -ForegroundColor Cyan
                }
                catch {
                    Write-Host "  Index creation note: $indexSql" -ForegroundColor Yellow
                }
            }
        }
    }
    catch {
        if ($_.Exception.Response.StatusCode -eq 400) {
            Write-Host "  Collection $($collectionName) already exists" -ForegroundColor Yellow
        }
        else {
            Write-Host "  Failed to create collection $($collectionName): $($_.Exception.Message)" -ForegroundColor Red
        }
    }
}

Write-Host ""
Write-Host "Game collections created successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "  • players: User accounts and stats" -ForegroundColor White
Write-Host "  • rooms: Game rooms/sessions" -ForegroundColor White
Write-Host "  • matches: Individual game matches" -ForegroundColor White
Write-Host "  • participants: Player participation in matches" -ForegroundColor White
Write-Host ""
Write-Host "Access your PocketBase admin at: $($PocketBaseUrl)/_/" -ForegroundColor Yellow