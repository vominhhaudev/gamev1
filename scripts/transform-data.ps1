# Transform PocketBase Data for PostgreSQL Migration
# This script transforms exported PocketBase data to match PostgreSQL schema

param(
    [string]$InputDir = "migration-data",
    [string]$OutputDir = "transformed-data",
    [switch]$ValidateOnly = $false
)

Write-Host "🔄 Starting Data Transformation for PostgreSQL..." -ForegroundColor Green

# Create output directory
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Data transformation mappings
$transformations = @{
    # PocketBase -> PostgreSQL field mappings and transformations
    "users" = @{
        "table" = "players"
        "mappings" = @{
            "id" = "id"  # Keep as-is, will be converted to UUID
            "email" = "email"
            "username" = "username"
            "display_name" = "username"  # Map to username if display_name not exists
            "level" = "skill_rating"  # Map level to skill_rating (0-100 scale)
            "xp" = "total_score"  # Map XP to total_score for now
            "total_games_played" = "games_played"
            "total_wins" = "wins"
            "total_score" = "total_score"
        }
        "defaults" = @{
            "skill_rating" = 1200.00
            "games_played" = 0
            "wins" = 0
            "losses" = 0
            "draws" = 0
            "is_online" = $false
            "region" = "unknown"
        }
        "transforms" = @{
            "created" = "created_at"
            "updated" = "updated_at"
        }
    }

    "matches" = @{
        "table" = "games"
        "mappings" = @{
            "id" = "id"
            "room_id" = "name"  # Use room_id as game name for now
            "game_mode" = "game_mode"
            "map_name" = "map_name"
            "max_players" = "max_players"
            "status" = "status"
            "start_time" = "started_at"
            "end_time" = "finished_at"
            "duration_seconds" = "duration_seconds"  # Will need to calculate
            "total_score" = "total_score"
        }
        "defaults" = @{
            "current_players" = 0
            "settings" = "{}"
        }
    }

    "participants" = @{
        "table" = "game_sessions"
        "mappings" = @{
            "id" = "id"
            "match_id" = "game_id"
            "user_id" = "player_id"
            "username" = "username"  # Will need to lookup from users table
            "score" = "score"
            "kills" = "score"  # Map kills to score for simplicity
            "deaths" = 0
            "assists" = 0
            "accuracy" = 0.0
            "playtime_seconds" = 0
            "joined_at" = "joined_at"
            "left_at" = "left_at"
            "is_winner" = $false
            "stats" = "{}"
        }
        "defaults" = @{
            "health" = 100.00
            "position" = "{}"
            "status" = "finished"
        }
    }

    "user_stats" = @{
        "table" = "player_stats"
        "mappings" = @{
            "user_id" = "player_id"
            "date" = "date"
            "games_played" = "games_played"
            "total_score" = "total_score"
            "total_playtime_seconds" = "playtime_minutes"  # Convert to minutes
            "avg_accuracy" = "average_score"
            "best_streak" = "best_streak"
            "achievements_unlocked" = 0
            "items_acquired" = 0
        }
        "defaults" = @{
            "games_won" = 0
            "average_score" = 0.00
            "region" = "unknown"
        }
    }
}

# Function to transform a single record
function Transform-Record {
    param(
        [string]$CollectionName,
        [PSObject]$Record
    )

    $config = $transformations[$CollectionName]
    if (!$config) {
        Write-Warning "No transformation config for collection: $CollectionName"
        return $null
    }

    $transformed = @{}

    # Apply field mappings
    foreach ($pbField in $config.mappings.Keys) {
        $pgField = $config.mappings[$pbField]

        if ($Record.PSObject.Properties.Name -contains $pbField) {
            $value = $Record.$pbField

            # Apply transformations if needed
            if ($config.transforms.ContainsKey($pbField)) {
                $transformType = $config.transforms[$pbField]
                switch ($transformType) {
                    "created_at" {
                        if ($value) {
                            $transformed[$pgField] = $value.ToString("yyyy-MM-dd HH:mm:ss")
                        }
                    }
                    "updated_at" {
                        if ($value) {
                            $transformed[$pgField] = $value.ToString("yyyy-MM-dd HH:mm:ss")
                        }
                    }
                    default {
                        $transformed[$pgField] = $value
                    }
                }
            }
            else {
                $transformed[$pgField] = $value
            }
        }
    }

    # Apply defaults for missing fields
    foreach ($defaultField in $config.defaults.Keys) {
        if (!$transformed.ContainsKey($defaultField)) {
            $transformed[$defaultField] = $config.defaults[$defaultField]
        }
    }

    return $transformed
}

# Function to transform collection data
function Transform-Collection {
    param(
        [string]$CollectionName
    )

    $inputFile = Join-Path $InputDir "$CollectionName.json"
    if (!(Test-Path $inputFile)) {
        Write-Warning "Input file not found: $inputFile"
        return 0
    }

    Write-Host "🔄 Transforming collection: $CollectionName" -ForegroundColor Yellow

    # Read and parse JSON
    $content = Get-Content $inputFile -Raw
    $records = $content | ConvertFrom-Json

    if (!$records -or $records.Count -eq 0) {
        Write-Warning "No records found in $inputFile"
        return 0
    }

    $transformedRecords = @()
    $successCount = 0
    $errorCount = 0

    foreach ($record in $records) {
        try {
            $transformed = Transform-Record -CollectionName $CollectionName -Record $record
            if ($transformed) {
                $transformedRecords += $transformed
                $successCount++
            }
            else {
                $errorCount++
            }
        }
        catch {
            Write-Warning "Failed to transform record $($record.id): $($_.Exception.Message)"
            $errorCount++
        }
    }

    # Save transformed data
    $outputFile = Join-Path $OutputDir "$($transformations[$CollectionName].table).json"
    $transformedRecords | ConvertTo-Json -Depth 10 | Out-File -FilePath $outputFile -Encoding UTF8

    Write-Host "  ✅ Transformed $successCount records, $errorCount errors" -ForegroundColor Green
    Write-Host "  📁 Output: $outputFile" -ForegroundColor Gray

    return $successCount
}

# Main execution
try {
    $totalTransformed = 0
    $transformSummary = @{}

    # Transform each collection
    foreach ($collection in $transformations.Keys) {
        $count = Transform-Collection -CollectionName $collection
        $transformSummary[$collection] = $count
        $totalTransformed += $count
    }

    # Create transformation summary
    $summaryFile = Join-Path $OutputDir "transform-summary.json"
    $summary = @{
        "total_records_transformed" = $totalTransformed
        "collections" = $transformSummary
        "transformation_date" = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    }
    $summary | ConvertTo-Json -Depth 3 | Out-File -FilePath $summaryFile

    Write-Host ""
    Write-Host "🎉 Transformation completed successfully!" -ForegroundColor Green
    Write-Host "📊 Summary:" -ForegroundColor Cyan
    foreach ($collection in $transformations.Keys) {
        $table = $transformations[$collection].table
        Write-Host "  $collection → $table`: $($transformSummary[$collection]) records" -ForegroundColor White
    }
    Write-Host "  Total: $totalTransformed records transformed" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "📁 Output directory: $OutputDir" -ForegroundColor Green
    Write-Host "📋 Next step: Run 'import-postgresql.ps1' to import data" -ForegroundColor Magenta

}
catch {
    Write-Error "Transformation failed: $($_.Exception.Message)"
    exit 1
}
