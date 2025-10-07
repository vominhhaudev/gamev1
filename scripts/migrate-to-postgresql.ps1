# Complete Database Migration Pipeline: PocketBase → PostgreSQL + Redis
# This is the master script that runs the entire migration process

param(
    [string]$PocketBaseUrl = "http://localhost:8090",
    [string]$PostgreHost = "localhost",
    [string]$PostgrePort = "5432",
    [string]$PostgreDatabase = "gamev1",
    [string]$PostgreUser = "gamev1_user",
    [string]$PostgrePassword = "gamev1_password",
    [string]$WorkingDir = "migration-workspace",
    [switch]$SkipValidation = $false,
    [switch]$DryRun = $false,
    [switch]$Force = $false
)

Write-Host "🚀 Complete Database Migration Pipeline" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green
Write-Host ""

# Configuration
$migrationConfig = @{
    "pocketbase_url" = $PocketBaseUrl
    "postgresql_host" = $PostgreHost
    "postgresql_port" = $PostgrePort
    "postgresql_database" = $PostgreDatabase
    "postgresql_user" = $PostgreUser
    "postgresql_password" = $PostgrePassword
    "working_directory" = $WorkingDir
    "dry_run" = $DryRun
    "start_time" = Get-Date
}

Write-Host "📋 Migration Configuration:" -ForegroundColor Cyan
$migrationConfig.GetEnumerator() | ForEach-Object {
    $key = $_.Key
    $value = if ($key -eq "postgresql_password") { "********" } else { $_.Value }
    Write-Host "  $key`: $value" -ForegroundColor White
}

# Create working directory
if (!(Test-Path $WorkingDir)) {
    New-Item -ItemType Directory -Path $WorkingDir | Out-Null
    Write-Host "📁 Created working directory: $WorkingDir" -ForegroundColor Green
}

# Save configuration for other scripts
$configFile = Join-Path $WorkingDir "migration-config.json"
$migrationConfig | ConvertTo-Json -Depth 3 | Out-File -FilePath $configFile

Write-Host ""
Write-Host "🔄 Migration Pipeline Steps:" -ForegroundColor Cyan
Write-Host "1. Export PocketBase data" -ForegroundColor White
Write-Host "2. Transform data for PostgreSQL" -ForegroundColor White
Write-Host "3. Import data into PostgreSQL" -ForegroundColor White
Write-Host "4. Validate migration success" -ForegroundColor White
Write-Host ""

# Function to run a step with error handling
function Run-Step {
    param(
        [string]$StepName,
        [string]$ScriptPath,
        [hashtable]$Parameters = @{}
    )

    Write-Host "🔧 Running step: $StepName" -ForegroundColor Yellow
    Write-Host "   Script: $ScriptPath" -ForegroundColor Gray

    try {
        # Build parameter string
        $paramString = ""
        foreach ($key in $Parameters.Keys) {
            $value = $Parameters[$key]
            if ($value -is [string]) {
                $paramString += " -$key `"$value`""
            }
            elseif ($value -is [switch] -and $value) {
                $paramString += " -$key"
            }
        }

        $command = "& `"$ScriptPath`" $paramString"
        Write-Host "   Command: $command" -ForegroundColor DarkGray

        # Execute script
        Invoke-Expression $command

        Write-Host "   ✅ Step completed successfully" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "   ❌ Step failed: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Step 1: Export PocketBase data
Write-Host ""
Write-Host "📦 STEP 1: Exporting PocketBase Data" -ForegroundColor Green
Write-Host "====================================" -ForegroundColor Green

$exportParams = @{
    "PocketBaseUrl" = $PocketBaseUrl
    "OutputDir" = Join-Path $WorkingDir "migration-data"
}

if (!(Run-Step -StepName "Export PocketBase Data" -ScriptPath ".\export-pocketbase-data.ps1" -Parameters $exportParams)) {
    if (!$Force) {
        Write-Host "❌ Migration stopped. Use -Force to continue despite errors." -ForegroundColor Red
        exit 1
    }
}

# Step 2: Transform data
Write-Host ""
Write-Host "🔄 STEP 2: Transforming Data for PostgreSQL" -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green

$transformParams = @{
    "InputDir" = Join-Path $WorkingDir "migration-data"
    "OutputDir" = Join-Path $WorkingDir "transformed-data"
}

if (!(Run-Step -StepName "Transform Data" -ScriptPath ".\transform-data.ps1" -Parameters $transformParams)) {
    if (!$Force) {
        Write-Host "❌ Migration stopped. Use -Force to continue despite errors." -ForegroundColor Red
        exit 1
    }
}

# Step 3: Import into PostgreSQL
Write-Host ""
Write-Host "📥 STEP 3: Importing Data into PostgreSQL" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green

$importParams = @{
    "InputDir" = Join-Path $WorkingDir "transformed-data"
    "PostgreHost" = $PostgreHost
    "PostgrePort" = $PostgrePort
    "PostgreDatabase" = $PostgreDatabase
    "PostgreUser" = $PostgreUser
    "PostgrePassword" = $PostgrePassword
    "DryRun" = $DryRun
}

if (!$DryRun) {
    if (!(Run-Step -StepName "Import PostgreSQL Data" -ScriptPath ".\import-postgresql.ps1" -Parameters $importParams)) {
        if (!$Force) {
            Write-Host "❌ Migration stopped. Use -Force to continue despite errors." -ForegroundColor Red
            exit 1
        }
    }
}
else {
    Write-Host "🧪 DRY RUN: Skipping actual import" -ForegroundColor Yellow
}

# Step 4: Validate migration
Write-Host ""
Write-Host "🔍 STEP 4: Validating Migration Success" -ForegroundColor Green
Write-Host "=======================================" -ForegroundColor Green

if (!$SkipValidation) {
    $validateParams = @{
        "PocketBaseUrl" = $PocketBaseUrl
        "PostgreHost" = $PostgreHost
        "PostgrePort" = $PostgrePort
        "PostgreDatabase" = $PostgreDatabase
        "PostgreUser" = $PostgreUser
        "PostgrePassword" = $PostgrePassword
        "DataDir" = $WorkingDir
        "Detailed" = $true
    }

    if (!(Run-Step -StepName "Validate Migration" -ScriptPath ".\validate-migration.ps1" -Parameters $validateParams)) {
        if (!$Force) {
            Write-Host "❌ Migration validation failed. Use -Force to continue despite validation errors." -ForegroundColor Red
            exit 1
        }
    }
}
else {
    Write-Host "⏭️  Skipping validation as requested" -ForegroundColor Yellow
}

# Migration completed
$endTime = Get-Date
$duration = $endTime - $migrationConfig.start_time

Write-Host ""
Write-Host "🎉 MIGRATION PIPELINE COMPLETED!" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green
Write-Host "⏱️  Total duration: $($duration.TotalMinutes) minutes" -ForegroundColor Cyan
Write-Host "📁 Working directory: $WorkingDir" -ForegroundColor Cyan
Write-Host "📄 Configuration saved: $configFile" -ForegroundColor Cyan

if ($DryRun) {
    Write-Host ""
    Write-Host "🧪 DRY RUN COMPLETED" -ForegroundColor Yellow
    Write-Host "Run without -DryRun to perform actual migration" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🚀 Next Steps:" -ForegroundColor Magenta
Write-Host "1. Review migration logs and reports" -ForegroundColor White
Write-Host "2. Update application connection strings" -ForegroundColor White
Write-Host "3. Implement Redis caching layer" -ForegroundColor White
Write-Host "4. Test application with new database" -ForegroundColor White
Write-Host "5. Monitor performance and scale as needed" -ForegroundColor White

Write-Host ""
Write-Host "📚 For more information, see: migration-strategy.md" -ForegroundColor Green
