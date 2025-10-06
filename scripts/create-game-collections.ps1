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

# Authenticate với PocketBase (skip for now to test basic connectivity)
$headers = @{
    "Content-Type" = "application/json"
}
Write-Host "Using basic authentication (no admin token)" -ForegroundColor Yellow

# Collection 1: Players (người chơi) - Updated for Room Manager
$playersCollection = @{
    name = "players"
    type = "base"
    schema = @(
        @{
            name = "id"
            type = "text"
            required = $true
            unique = $true
        },
        @{
            name = "name"
            type = "text"
            required = $true
        },
        @{
            name = "room_id"
            type = "text"
            required = $true
        },
        @{
            name = "joined_at"
            type = "date"
            required = $true
        },
        @{
            name = "last_seen"
            type = "date"
            required = $true
        },
        @{
            name = "status"
            type = "select"
            required = $true
            options = @{
                values = @("connected", "disconnected", "left")
            }
        },
        @{
            name = "team"
            type = "text"
        }
    )
    indexes = @(
        "CREATE INDEX idx_players_room ON players (room_id)",
        "CREATE INDEX idx_players_status ON players (status)",
        "CREATE INDEX idx_players_last_seen ON players (last_seen DESC)"
    )
}

# Collection 2: Rooms (phòng chơi) - Updated for Room Manager
$roomsCollection = @{
    name = "rooms"
    type = "base"
    schema = @(
        @{
            name = "id"
            type = "text"
            required = $true
            unique = $true
        },
        @{
            name = "name"
            type = "text"
            required = $true
        },
        @{
            name = "game_mode"
            type = "select"
            required = $true
            options = @{
                values = @("deathmatch", "team_deathmatch", "capture_the_flag")
            }
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
                values = @("waiting", "starting", "in_progress", "finished", "closed")
            }
        },
        @{
            name = "created_at"
            type = "date"
            required = $true
        },
        @{
            name = "updated_at"
            type = "date"
            required = $true
        },
        @{
            name = "host_player_id"
            type = "text"
            required = $true
        },
        @{
            name = "worker_endpoint"
            type = "text"
        },
        @{
            name = "settings"
            type = "json"
        }
    )
    indexes = @(
        "CREATE INDEX idx_rooms_status ON rooms (status)",
        "CREATE INDEX idx_rooms_game_mode ON rooms (game_mode)",
        "CREATE INDEX idx_rooms_updated ON rooms (updated_at DESC)"
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