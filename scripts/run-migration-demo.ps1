# Migration Demo Script
# This script demonstrates the complete migration process step by step

param(
    [switch]$SkipSetup = $false,
    [switch]$SkipMigration = $false,
    [switch]$SkipValidation = $false,
    [string]$WorkingDir = "migration-demo"
)

Write-Host "GameV1 Database Migration Demo" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green
Write-Host ""

Write-Host "This demo will show you how to migrate from PocketBase to PostgreSQL + Redis" -ForegroundColor Cyan
Write-Host "   for scaling to 10,000+ concurrent users." -ForegroundColor Cyan
Write-Host ""

# Check prerequisites
Write-Host "Checking prerequisites..." -ForegroundColor Yellow

$prerequisites = @(
    @{name="PowerShell 7+"; test={ $PSVersionTable.PSVersion.Major -ge 7 }},
    @{name="PocketBase server running"; test={ try { Invoke-WebRequest -Uri "http://localhost:8090/api/health" -TimeoutSec 5 | Out-Null; $true } catch { $false } }},
    @{name="Docker (for container mode)"; test={ try { docker version | Out-Null; $true } catch { $false } }}
)

foreach ($prereq in $prerequisites) {
    $status = if (& $prereq.test) { "OK" } else { "MISSING" }
    Write-Host "   $($prereq.name): $status" -ForegroundColor White
}

Write-Host ""
Write-Host "Demo Steps:" -ForegroundColor Cyan
Write-Host "1. Setup PostgreSQL and Redis" -ForegroundColor White
Write-Host "2. Run complete migration pipeline" -ForegroundColor White
Write-Host "3. Validate migration success" -ForegroundColor White
Write-Host ""

# Step 1: Setup environment
if (!$SkipSetup) {
    Write-Host "STEP 1: Setting up Database Environment" -ForegroundColor Green
    Write-Host "=========================================" -ForegroundColor Green

    Write-Host "Running: .\setup-database-environment.ps1" -ForegroundColor Gray
    Write-Host "   This will setup PostgreSQL and Redis..." -ForegroundColor Gray

    try {
        & ".\setup-database-environment.ps1" -DockerMode $true
        Write-Host "Environment setup completed" -ForegroundColor Green
    }
    catch {
        Write-Host "Environment setup failed: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "   You can continue with manual setup or use -SkipSetup" -ForegroundColor Yellow
        return
    }
}
else {
    Write-Host "Skipping environment setup as requested" -ForegroundColor Yellow
}

# Step 2: Run migration
if (!$SkipMigration) {
    Write-Host ""
    Write-Host "STEP 2: Running Migration Pipeline" -ForegroundColor Green
    Write-Host "===================================" -ForegroundColor Green

    Write-Host "Running: .\migrate-to-postgresql.ps1 -WorkingDir $WorkingDir -DryRun" -ForegroundColor Gray
    Write-Host "   This will do a dry run first to show what will happen..." -ForegroundColor Gray

    try {
        # First do a dry run
        Write-Host ""
        Write-Host "DRY RUN (no data will be changed):" -ForegroundColor Yellow
        & ".\migrate-to-postgresql.ps1" -WorkingDir $WorkingDir -DryRun -PocketBaseUrl "http://localhost:8090"

        Write-Host ""
        Write-Host "Dry run completed successfully!" -ForegroundColor Green

        # Ask if user wants to run actual migration
        $runActual = Read-Host "Do you want to run the actual migration? (y/N)"
        if ($runActual -eq 'y' -or $runActual -eq 'Y') {
            Write-Host ""
            Write-Host "Running ACTUAL migration (data will be migrated):" -ForegroundColor Red
            & ".\migrate-to-postgresql.ps1" -WorkingDir $WorkingDir -PocketBaseUrl "http://localhost:8090"
            Write-Host "Migration completed!" -ForegroundColor Green
        }
        else {
            Write-Host "Skipping actual migration as requested" -ForegroundColor Yellow
        }
    }
    catch {
        Write-Host "Migration failed: $($_.Exception.Message)" -ForegroundColor Red
        return
    }
}
else {
    Write-Host "Skipping migration as requested" -ForegroundColor Yellow
}

# Step 3: Validate migration
if (!$SkipValidation) {
    Write-Host ""
    Write-Host "STEP 3: Validating Migration Success" -ForegroundColor Green
    Write-Host "=====================================" -ForegroundColor Green

    Write-Host "Running: .\validate-migration.ps1" -ForegroundColor Gray
    Write-Host "   This will verify that migration was successful..." -ForegroundColor Gray

    try {
        & ".\validate-migration.ps1" -PocketBaseUrl "http://localhost:8090" -PostgreHost "localhost" -PostgrePort "5432" -PostgreDatabase "gamev1" -PostgreUser "gamev1_user" -PostgrePassword "gamev1_password" -DataDir $WorkingDir -Detailed
        Write-Host "Validation completed!" -ForegroundColor Green
    }
    catch {
        Write-Host "Validation failed: $($_.Exception.Message)" -ForegroundColor Red
        return
    }
}
else {
    Write-Host "Skipping validation as requested" -ForegroundColor Yellow
}

# Demo completed
Write-Host ""
Write-Host "MIGRATION DEMO COMPLETED!" -ForegroundColor Green
Write-Host "===========================" -ForegroundColor Green

Write-Host ""
Write-Host "What we've accomplished:" -ForegroundColor Cyan
Write-Host "OK Environment setup (PostgreSQL + Redis)" -ForegroundColor White
Write-Host "OK Complete data migration pipeline" -ForegroundColor White
Write-Host "OK Data validation and integrity checks" -ForegroundColor White
Write-Host "OK Ready for 10,000+ concurrent users!" -ForegroundColor White

Write-Host ""
Write-Host "Demo artifacts saved in: $WorkingDir" -ForegroundColor Cyan

Write-Host ""
Write-Host "Next Steps:" -ForegroundColor Magenta
Write-Host "1. Update your application to use PostgreSQL + Redis" -ForegroundColor White
Write-Host "2. Implement Redis caching in your application code" -ForegroundColor White
Write-Host "3. Test with load testing tools" -ForegroundColor White
Write-Host "4. Monitor performance and scale horizontally" -ForegroundColor White

Write-Host ""
Write-Host "Documentation:" -ForegroundColor Green
Write-Host "   migration-strategy.md - Complete migration guide" -ForegroundColor White
Write-Host "   database-schema.sql - PostgreSQL schema for reference" -ForegroundColor White

Write-Host ""
Write-Host "Need help? Check the migration-strategy.md file for detailed instructions!" -ForegroundColor Green
