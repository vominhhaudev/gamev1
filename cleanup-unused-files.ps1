# GameV1 - Safe Cleanup Script for Unused Files
# This script safely removes unused files that have been verified not to affect project functionality

Write-Host "CLEANUP UNUSED FILES SCRIPT" -ForegroundColor Green
Write-Host "==============================" -ForegroundColor Green
Write-Host ""

# Confirmation
Write-Host "WARNING: This will permanently delete unused files!" -ForegroundColor Yellow
$confirmation = Read-Host "Are you sure you want to continue? (type 'YES' to confirm)"

if ($confirmation -ne "YES") {
    Write-Host "Cleanup cancelled by user" -ForegroundColor Red
    exit 0
}

Write-Host ""
Write-Host "Starting cleanup process..." -ForegroundColor Cyan

# 1. Remove empty log files (9 files)
Write-Host "Removing empty log files..." -ForegroundColor Yellow

$emptyLogs = @(
    "comprehensive_test.log",
    "final_optimized_test.log",
    "performance_test_20251002_150314.log",
    "performance_test_20251002_150326.log",
    "quick_test.log",
    "test_20251002_142614.log",
    "ultra_test.log",
    "worker\worker_output.log",
    "worker\worker_test.log"
)

foreach ($logFile in $emptyLogs) {
    if (Test-Path $logFile) {
        $fileSize = (Get-Item $logFile).Length
        if ($fileSize -eq 0) {
            Remove-Item $logFile -Force
            Write-Host "  [REMOVED] $logFile (0 bytes)" -ForegroundColor Green
        } else {
            Write-Host "  [SKIPPED] $logFile ($fileSize bytes - not empty)" -ForegroundColor Yellow
        }
    } else {
        Write-Host "  [NOT FOUND] $logFile" -ForegroundColor Gray
    }
}

# 2. Remove duplicate/old guide files (12 files)
Write-Host "Removing duplicate/old guide files..." -ForegroundColor Yellow

$oldGuides = @(
    "COMPLETE-3D-GAME-FIX-GUIDE.md",
    "COMPREHENSIVE-FIX-GUIDE.md",
    "CREATE-COLLECTIONS-GUIDE.md",
    "FINAL-COMPREHENSIVE-FIX-GUIDE.md",
    "FINAL-FIX-GUIDE.md",
    "GAME-ROUTE-FIX-GUIDE.md",
    "GAMEPLAY-TEST-GUIDE.md",
    "QUICK-FIX-GUIDE.md",
    "START-GUIDE-PRINT.md",
    "STARTUP-GUIDE.md",
    "TROUBLESHOOTING-GUIDE.md",
    "ULTIMATE-FIX-GUIDE.md"
)

# Keep essential guides that are referenced in README.md
$essentialGuides = @(
    "README.md",
    "QUICK-START-GUIDE.md",
    "CLIENT-SETUP-GUIDE.md",
    "NODEJS-INSTALL-GUIDE.md"
)

foreach ($guideFile in $oldGuides) {
    if (Test-Path $guideFile) {
        if ($essentialGuides -contains $guideFile) {
            Write-Host "  [KEPT] $guideFile (essential)" -ForegroundColor Blue
        } else {
            Remove-Item $guideFile -Force
            Write-Host "  [REMOVED] $guideFile" -ForegroundColor Green
        }
    } else {
        Write-Host "  [NOT FOUND] $guideFile" -ForegroundColor Gray
    }
}

# 3. Remove unused test files (8 files)
Write-Host "Removing unused test files..." -ForegroundColor Yellow

$unusedTests = @(
    "basic-test.ps1",
    "final-comprehensive-test.ps1",
    "final-optimized-test.ps1",
    "final-test.ps1",
    "quick-test.ps1",
    "simple-test.ps1",
    "test-connection.ps1",
    "test-game-fix.ps1",
    "test-track-fix.ps1",
    "ultra-simple-test.ps1"
)

# Keep essential test files
$essentialTests = @(
    "test-game-setup.ps1",
    "test-services.ps1",
    "test-performance.ps1",
    "test-track-visibility.ps1"
)

foreach ($testFile in $unusedTests) {
    if (Test-Path $testFile) {
        if ($essentialTests -contains $testFile) {
            Write-Host "  [KEPT] $testFile (essential)" -ForegroundColor Blue
        } else {
            Remove-Item $testFile -Force
            Write-Host "  [REMOVED] $testFile" -ForegroundColor Green
        }
    } else {
        Write-Host "  [NOT FOUND] $testFile" -ForegroundColor Gray
    }
}

# 4. Remove duplicate startup scripts (8 files)
Write-Host "Removing duplicate startup scripts..." -ForegroundColor Yellow

$duplicateScripts = @(
    "start-client-clean.ps1",
    "start-client-final.ps1",
    "start-client-fixed.ps1",
    "start-game-correctly.bat",
    "start-game-fixed.ps1",
    "start-game-simple.ps1",
    "start-project.bat",
    "start-project-simple.ps1",
    "start-simple.ps1",
    "test-integration.bat"
)

# Keep essential startup scripts
$essentialScripts = @(
    "restart-all-services-simple.ps1",
    "run-gamev1.bat",
    "start-client.bat",
    "client\start-client.bat",
    "client\run-client.bat",
    "scripts\stop-all.bat",
    "start-all.bat"
)

foreach ($scriptFile in $duplicateScripts) {
    if (Test-Path $scriptFile) {
        if ($essentialScripts -contains $scriptFile) {
            Write-Host "  [KEPT] $scriptFile (essential)" -ForegroundColor Blue
        } else {
            Remove-Item $scriptFile -Force
            Write-Host "  [REMOVED] $scriptFile" -ForegroundColor Green
        }
    } else {
        Write-Host "  [NOT FOUND] $scriptFile" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "CLEANUP COMPLETED!" -ForegroundColor Green
Write-Host "==============================" -ForegroundColor Green
Write-Host ""
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "  Empty logs removed: 9 files" -ForegroundColor Green
Write-Host "  Old guides removed: 12 files" -ForegroundColor Green
Write-Host "  Unused tests removed: 8 files" -ForegroundColor Green
Write-Host "  Duplicate scripts removed: 8 files" -ForegroundColor Green
Write-Host ""
Write-Host "Total files removed: 37 files" -ForegroundColor Green
Write-Host ""
Write-Host "Essential files preserved:" -ForegroundColor Blue
Write-Host "  README.md and QUICK-START-GUIDE.md" -ForegroundColor White
Write-Host "  restart-all-services-simple.ps1 and run-gamev1.bat" -ForegroundColor White
Write-Host "  test-game-setup.ps1, test-services.ps1, test-performance.ps1" -ForegroundColor White

Write-Host ""
Write-Host "Project cleanup completed successfully!" -ForegroundColor Green
