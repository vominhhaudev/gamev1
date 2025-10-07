# Import Transformed Data into PostgreSQL
# This script imports transformed data into PostgreSQL database

param(
    [string]$InputDir = "transformed-data",
    [string]$PostgreHost = "localhost",
    [string]$PostgrePort = "5432",
    [string]$PostgreDatabase = "gamev1",
    [string]$PostgreUser = "gamev1_user",
    [string]$PostgrePassword = "gamev1_password",
    [string]$ConnectionString = "",
    [switch]$DryRun = $false,
    [switch]$ValidateOnly = $false
)

Write-Host "📥 Starting PostgreSQL Data Import..." -ForegroundColor Green

# Determine connection method
if ($ConnectionString) {
    Write-Host "Using connection string..." -ForegroundColor Cyan
    $connectionInfo = $ConnectionString
}
else {
    Write-Host "Using individual connection parameters..." -ForegroundColor Cyan
    $connectionInfo = "Host=$PostgreHost;Port=$PostgrePort;Database=$PostgreDatabase;Username=$PostgreUser;Password=$PostgrePassword"
}

# Tables to import (in order to handle foreign key dependencies)
$tableOrder = @(
    "players",
    "games",
    "game_sessions",
    "player_stats"
)

# Function to execute SQL query
function Execute-SQL {
    param(
        [string]$Query,
        [switch]$IgnoreErrors = $false
    )

    try {
        if ($DryRun) {
            Write-Host "[DRY RUN] Would execute: $Query" -ForegroundColor Yellow
            return $true
        }

        $result = Invoke-Sqlcmd -ConnectionString $connectionInfo -Query $Query -ErrorAction Stop
        return $true
    }
    catch {
        if (!$IgnoreErrors) {
            Write-Error "SQL execution failed: $($_.Exception.Message)"
            Write-Host "Query: $Query" -ForegroundColor Red
            return $false
        }
        else {
            Write-Warning "SQL execution failed (ignored): $($_.Exception.Message)"
            return $false
        }
    }
}

# Function to import data for a table
function Import-Table {
    param(
        [string]$TableName
    )

    $inputFile = Join-Path $InputDir "$TableName.json"
    if (!(Test-Path $inputFile)) {
        Write-Warning "Input file not found: $inputFile"
        return 0, 0
    }

    Write-Host "📥 Importing table: $TableName" -ForegroundColor Yellow

    # Read and parse JSON
    $content = Get-Content $inputFile -Raw
    $records = $content | ConvertFrom-Json

    if (!$records -or $records.Count -eq 0) {
        Write-Warning "No records found in $inputFile"
        return 0, 0
    }

    $successCount = 0
    $errorCount = 0

    foreach ($record in $records) {
        try {
            # Build INSERT query
            $columns = $record.PSObject.Properties.Name -join ", "
            $values = $record.PSObject.Properties | ForEach-Object {
                $value = $_.Value

                # Handle different data types
                if ($value -eq $null) {
                    "NULL"
                }
                elseif ($value -is [string]) {
                    "'$($value.Replace("'", "''"))'"
                }
                elseif ($value -is [boolean]) {
                    if ($value) { "true" } else { "false" }
                }
                else {
                    $value.ToString()
                }
            }

            $values = $values -join ", "
            $insertQuery = "INSERT INTO $TableName ($columns) VALUES ($values);"

            if (Execute-SQL -Query $insertQuery) {
                $successCount++
            }
            else {
                $errorCount++
            }
        }
        catch {
            Write-Warning "Failed to import record $($record.id): $($_.Exception.Message)"
            $errorCount++
        }
    }

    Write-Host "  ✅ Imported $successCount records, $errorCount errors" -ForegroundColor Green
    return $successCount, $errorCount
}

# Function to validate import
function Validate-Import {
    Write-Host "🔍 Validating import..." -ForegroundColor Cyan

    $validationResults = @{}

    foreach ($table in $tableOrder) {
        $countQuery = "SELECT COUNT(*) as count FROM $table;"
        try {
            $result = Invoke-Sqlcmd -ConnectionString $connectionInfo -Query $countQuery
            $validationResults[$table] = $result.count
            Write-Host "  $table`: $($result.count) records" -ForegroundColor Green
        }
        catch {
            Write-Warning "Failed to validate $table`: $($_.Exception.Message)"
            $validationResults[$table] = -1
        }
    }

    return $validationResults
}

# Main execution
try {
    # Test connection
    Write-Host "🔗 Testing database connection..." -ForegroundColor Cyan
    $testQuery = "SELECT version();"
    if (!(Execute-SQL -Query $testQuery -IgnoreErrors $true)) {
        throw "Cannot connect to PostgreSQL database"
    }

    if ($ValidateOnly) {
        $validationResults = Validate-Import
        Write-Host "✅ Validation completed" -ForegroundColor Green
        return
    }

    # Begin transaction for data consistency
    if (!$DryRun) {
        Write-Host "🔒 Starting transaction..." -ForegroundColor Cyan
        Execute-SQL -Query "BEGIN TRANSACTION;"
    }

    $totalImported = 0
    $totalErrors = 0
    $importSummary = @{}

    try {
        # Import each table in order
        foreach ($table in $tableOrder) {
            $imported, $errors = Import-Table -TableName $table
            $importSummary[$table] = @{
                "imported" = $imported
                "errors" = $errors
            }
            $totalImported += $imported
            $totalErrors += $errors
        }

        if (!$DryRun) {
            # Commit transaction
            Write-Host "💾 Committing transaction..." -ForegroundColor Cyan
            Execute-SQL -Query "COMMIT;"
        }

        # Validate final state
        $validationResults = Validate-Import

        # Create import summary
        $summaryFile = Join-Path $InputDir "import-summary.json"
        $summary = @{
            "import_date" = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            "dry_run" = $DryRun
            "total_imported" = $totalImported
            "total_errors" = $totalErrors
            "tables" = $importSummary
            "validation" = $validationResults
        }
        $summary | ConvertTo-Json -Depth 3 | Out-File -FilePath $summaryFile

        Write-Host ""
        Write-Host "🎉 Import completed successfully!" -ForegroundColor Green
        Write-Host "📊 Summary:" -ForegroundColor Cyan
        foreach ($table in $tableOrder) {
            $stats = $importSummary[$table]
            Write-Host "  $table`: $($stats.imported) imported, $($stats.errors) errors" -ForegroundColor White
        }
        Write-Host "  Total: $totalImported records imported, $totalErrors errors" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "🔍 Final validation:" -ForegroundColor Cyan
        foreach ($table in $tableOrder) {
            $count = $validationResults[$table]
            Write-Host "  $table`: $count records" -ForegroundColor White
        }

        if ($DryRun) {
            Write-Host "🧪 This was a dry run - no data was actually imported" -ForegroundColor Yellow
        }

    }
    catch {
        if (!$DryRun) {
            Write-Host "❌ Rolling back transaction..." -ForegroundColor Red
            Execute-SQL -Query "ROLLBACK;" -IgnoreErrors $true
        }
        throw $_.Exception.Message
    }

}
catch {
    Write-Error "Import failed: $($_.Exception.Message)"
    exit 1
}
