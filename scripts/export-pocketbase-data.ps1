# Export PocketBase Data for Migration to PostgreSQL
# This script exports all data from PocketBase collections for migration

param(
    [string]$PocketBaseUrl = "http://localhost:8090",
    [string]$AdminEmail = "admin@pocketbase.local",
    [string]$AdminPassword = "123456789",
    [string]$OutputDir = "migration-data",
    [int]$BatchSize = 1000
)

Write-Host "🚀 Starting PocketBase Data Export..." -ForegroundColor Green
Write-Host "Target: $PocketBaseUrl" -ForegroundColor Cyan

# Create output directory
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Collections to export (from collections.json)
$collections = @(
    "users",
    "matches",
    "participants",
    "leaderboard",
    "inventory",
    "achievements",
    "user_stats"
)

# Function to authenticate and get token
function Get-AuthToken {
    $authBody = @{
        identity = $AdminEmail
        password = $AdminPassword
    } | ConvertTo-Json

    try {
        $response = Invoke-RestMethod -Uri "$PocketBaseUrl/api/admins/auth-with-password" -Method Post -Body $authBody -ContentType "application/json"
        return $response.token
    }
    catch {
        Write-Error "Failed to authenticate with PocketBase: $($_.Exception.Message)"
        exit 1
    }
}

# Function to export collection data
function Export-Collection {
    param(
        [string]$CollectionName,
        [string]$Token
    )

    Write-Host "📦 Exporting collection: $CollectionName" -ForegroundColor Yellow

    $allRecords = @()
    $page = 1

    do {
        $headers = @{
            "Authorization" = "Bearer $Token"
        }

        $url = "$PocketBaseUrl/api/collections/$CollectionName/records?page=$page&perPage=$BatchSize"

        try {
            $response = Invoke-RestMethod -Uri $url -Method Get -Headers $headers

            if ($response.items.Count -gt 0) {
                $allRecords += $response.items
                Write-Host "  Page $page`: $($response.items.Count) records" -ForegroundColor Gray
            }

            $page++
            $totalPages = [math]::Ceiling($response.totalItems / $BatchSize)
        }
        catch {
            Write-Warning "Failed to fetch page $page for $CollectionName`: $($_.Exception.Message)"
            break
        }

    } while ($page -le $totalPages -and $response.items.Count -gt 0)

    # Save to file
    $outputFile = Join-Path $OutputDir "$CollectionName.json"
    $allRecords | ConvertTo-Json -Depth 10 | Out-File -FilePath $outputFile -Encoding UTF8

    Write-Host "  ✅ Exported $($allRecords.Count) records to $outputFile" -ForegroundColor Green

    return $allRecords.Count
}

# Main execution
try {
    # Authenticate
    Write-Host "🔐 Authenticating with PocketBase..." -ForegroundColor Cyan
    $token = Get-AuthToken

    $totalExported = 0
    $exportSummary = @{}

    # Export each collection
    foreach ($collection in $collections) {
        $count = Export-Collection -CollectionName $collection -Token $token
        $exportSummary[$collection] = $count
        $totalExported += $count

        # Small delay between collections to avoid rate limiting
        Start-Sleep -Milliseconds 500
    }

    # Create export summary
    $summaryFile = Join-Path $OutputDir "export-summary.json"
    $exportSummary | ConvertTo-Json | Out-File -FilePath $summaryFile

    Write-Host ""
    Write-Host "🎉 Export completed successfully!" -ForegroundColor Green
    Write-Host "📊 Summary:" -ForegroundColor Cyan
    foreach ($collection in $collections) {
        Write-Host "  $collection`: $($exportSummary[$collection]) records" -ForegroundColor White
    }
    Write-Host "  Total: $totalExported records exported" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "📁 Output directory: $OutputDir" -ForegroundColor Green
    Write-Host "📋 Next step: Run 'transform-data.ps1' to transform data for PostgreSQL" -ForegroundColor Magenta

}
catch {
    Write-Error "Export failed: $($_.Exception.Message)"
    exit 1
}
