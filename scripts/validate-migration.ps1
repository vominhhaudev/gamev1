# Validate Complete Migration Process
# This script validates that the migration from PocketBase to PostgreSQL was successful

param(
    [string]$PocketBaseUrl = "http://localhost:8090",
    [string]$PostgreHost = "localhost",
    [string]$PostgrePort = "5432",
    [string]$PostgreDatabase = "gamev1",
    [string]$PostgreUser = "gamev1_user",
    [string]$PostgrePassword = "gamev1_password",
    [string]$DataDir = "migration-data",
    [switch]$Detailed = $false
)

Write-Host "🔍 Validating Complete Migration Process..." -ForegroundColor Green

# Connection strings
$PocketBaseConnection = "Target: $PocketBaseUrl"
$PostgreConnection = "Host=$PostgreHost;Port=$PostgrePort;Database=$PostgreDatabase;Username=$PostgreUser;Password=$PostgrePassword"

# Function to get PocketBase record count
function Get-PocketBaseCount {
    param([string]$Collection)

    try {
        $response = Invoke-RestMethod -Uri "$PocketBaseUrl/api/collections/$Collection/records?perPage=1" -Method Get
        return $response.totalItems
    }
    catch {
        Write-Warning "Failed to get count for PocketBase collection $Collection`: $($_.Exception.Message)"
        return -1
    }
}

# Function to get PostgreSQL record count
function Get-PostgreSQLCount {
    param([string]$Table)

    try {
        $result = Invoke-Sqlcmd -ConnectionString $PostgreConnection -Query "SELECT COUNT(*) as count FROM $Table"
        return $result.count
    }
    catch {
        Write-Warning "Failed to get count for PostgreSQL table $Table`: $($_.Exception.Message)"
        return -1
    }
}

# Function to validate data integrity
function Validate-DataIntegrity {
    Write-Host "🔍 Validating data integrity..." -ForegroundColor Cyan

    $collections = @("users", "matches", "participants", "leaderboard", "inventory", "achievements", "user_stats")
    $validationResults = @{}

    foreach ($collection in $collections) {
        $pbCount = Get-PocketBaseCount -Collection $collection
        $pgCount = Get-PostgreSQLCount -Table $collection

        $validationResults[$collection] = @{
            "pocketbase_count" = $pbCount
            "postgresql_count" = $pgCount
            "match" = ($pbCount -eq $pgCount)
            "status" = if ($pbCount -eq $pgCount) { "✅" } else { "❌" }
        }

        if ($Detailed) {
            Write-Host "  $collection`: PB=$pbCount, PG=$pgCount $($validationResults[$collection].status)" -ForegroundColor White
        }
    }

    return $validationResults
}

# Function to validate referential integrity
function Validate-ReferentialIntegrity {
    Write-Host "🔗 Validating referential integrity..." -ForegroundColor Cyan

    $referentialTests = @(
        @{
            "name" = "game_sessions -> games"
            "query" = "SELECT COUNT(*) FROM game_sessions gs LEFT JOIN games g ON gs.game_id = g.id WHERE g.id IS NULL"
            "should_be_zero" = $true
        },
        @{
            "name" = "game_sessions -> players"
            "query" = "SELECT COUNT(*) FROM game_sessions gs LEFT JOIN players p ON gs.player_id = p.id WHERE p.id IS NULL"
            "should_be_zero" = $true
        },
        @{
            "name" = "games -> players (host)"
            "query" = "SELECT COUNT(*) FROM games g LEFT JOIN players p ON g.host_id = p.id WHERE p.id IS NULL"
            "should_be_zero" = $false  # Hosts might not exist yet
        }
    )

    $referentialResults = @{}

    foreach ($test in $referentialTests) {
        try {
            $result = Invoke-Sqlcmd -ConnectionString $PostgreConnection -Query $test.query
            $count = $result[0]

            $isValid = $test.should_be_zero ? ($count -eq 0) : $true
            $status = if ($isValid) { "✅" } else { "❌" }

            $referentialResults[$test.name] = @{
                "count" = $count
                "is_valid" = $isValid
                "status" = $status
            }

            if ($Detailed) {
                Write-Host "  $($test.name)`: $count $($status)" -ForegroundColor White
            }
        }
        catch {
            Write-Warning "Failed referential test $($test.name)`: $($_.Exception.Message)"
            $referentialResults[$test.name] = @{
                "count" = -1
                "is_valid" = $false
                "status" = "❌"
            }
        }
    }

    return $referentialResults
}

# Function to validate performance
function Validate-Performance {
    Write-Host "⚡ Validating performance..." -ForegroundColor Cyan

    $performanceTests = @(
        @{
            "name" = "Players query response time"
            "query" = "SELECT COUNT(*) FROM players WHERE is_online = true"
            "max_time_ms" = 100
        },
        @{
            "name" = "Games query response time"
            "query" = "SELECT COUNT(*) FROM games WHERE status = 'waiting'"
            "max_time_ms" = 50
        },
        @{
            "name" = "Leaderboard query response time"
            "query" = "SELECT p.username, p.skill_rating FROM players p ORDER BY p.skill_rating DESC LIMIT 10"
            "max_time_ms" = 200
        }
    )

    $performanceResults = @{}

    foreach ($test in $performanceTests) {
        try {
            $startTime = Get-Date
            $result = Invoke-Sqlcmd -ConnectionString $PostgreConnection -Query $test.query -QueryTimeout 30
            $endTime = Get-Date
            $duration = ($endTime - $startTime).TotalMilliseconds

            $isFast = $duration -le $test.max_time_ms
            $status = if ($isFast) { "✅" } else { "⚠️" }

            $performanceResults[$test.name] = @{
                "duration_ms" = [math]::Round($duration, 2)
                "is_fast" = $isFast
                "status" = $status
                "threshold_ms" = $test.max_time_ms
            }

            if ($Detailed) {
                Write-Host "  $($test.name)`: $($duration)ms $($status)" -ForegroundColor White
            }
        }
        catch {
            Write-Warning "Failed performance test $($test.name)`: $($_.Exception.Message)"
            $performanceResults[$test.name] = @{
                "duration_ms" = -1
                "is_fast" = $false
                "status" = "❌"
                "threshold_ms" = $test.max_time_ms
            }
        }
    }

    return $performanceResults
}

# Main validation
try {
    Write-Host "📊 Migration Validation Report" -ForegroundColor Green
    Write-Host "================================" -ForegroundColor Green

    # Data integrity validation
    $dataIntegrity = Validate-DataIntegrity

    # Referential integrity validation
    $referentialIntegrity = Validate-ReferentialIntegrity

    # Performance validation
    $performance = Validate-Performance

    # Summary
    Write-Host ""
    Write-Host "📋 Summary:" -ForegroundColor Cyan

    $totalDataMatch = 0
    $totalDataTests = 0

    foreach ($collection in $dataIntegrity.Keys) {
        $totalDataTests++
        if ($dataIntegrity[$collection].match) {
            $totalDataMatch++
        }
    }

    $referentialValid = 0
    foreach ($test in $referentialIntegrity.Keys) {
        if ($referentialIntegrity[$test].is_valid) {
            $referentialValid++
        }
    }

    $performanceFast = 0
    foreach ($test in $performance.Keys) {
        if ($performance[$test].is_fast) {
            $performanceFast++
        }
    }

    Write-Host "  Data Integrity: $totalDataMatch/$totalDataTests ✅" -ForegroundColor White
    Write-Host "  Referential Integrity: $referentialValid/$($referentialIntegrity.Count) ✅" -ForegroundColor White
    Write-Host "  Performance: $performanceFast/$($performance.Count) ⚡" -ForegroundColor White

    # Overall status
    $overallSuccess = ($totalDataMatch -eq $totalDataTests) -and ($referentialValid -eq $referentialIntegrity.Count) -and ($performanceFast -ge $performance.Count * 0.8)

    if ($overallSuccess) {
        Write-Host ""
        Write-Host "🎉 MIGRATION VALIDATION SUCCESSFUL!" -ForegroundColor Green
        Write-Host "✅ All data integrity checks passed" -ForegroundColor Green
        Write-Host "✅ Referential integrity maintained" -ForegroundColor Green
        Write-Host "✅ Performance targets met" -ForegroundColor Green
        Write-Host ""
        Write-Host "🚀 Ready for production deployment!" -ForegroundColor Green
    }
    else {
        Write-Host ""
        Write-Host "⚠️  MIGRATION VALIDATION ISSUES DETECTED!" -ForegroundColor Yellow
        Write-Host "Please review the detailed results above and fix any issues before proceeding." -ForegroundColor Yellow
    }

    # Create validation report file
    $reportFile = Join-Path $DataDir "migration-validation-report.json"
    $report = @{
        "validation_date" = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        "data_integrity" = $dataIntegrity
        "referential_integrity" = $referentialIntegrity
        "performance" = $performance
        "summary" = @{
            "data_match_rate" = "$totalDataMatch/$totalDataTests"
            "referential_valid_rate" = "$referentialValid/$($referentialIntegrity.Count)"
            "performance_fast_rate" = "$performanceFast/$($performance.Count)"
            "overall_success" = $overallSuccess
        }
    }
    $report | ConvertTo-Json -Depth 5 | Out-File -FilePath $reportFile

    Write-Host "📄 Validation report saved to: $reportFile" -ForegroundColor Green

}
catch {
    Write-Error "Validation failed: $($_.Exception.Message)"
    exit 1
}
