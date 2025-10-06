# Setup PocketBase collections for game services
# This script creates all necessary collections for the game server

param(
    [string]$PocketBaseUrl = "http://localhost:8090",
    [string]$AdminEmail = "admin@pocketbase.local",
    [string]$AdminPassword = "123456789"
)

Write-Host "Setting up PocketBase collections for game server..." -ForegroundColor Green

# Import collections JSON
$collectionsJson = Get-Content "..\services\collections.json" -Raw | ConvertFrom-Json

# Authenticate as admin (this is a simplified version)
# In a real deployment, you would:
# 1. Create admin user if it doesn't exist
# 2. Authenticate to get token
# 3. Use token for API calls

Write-Host "PocketBase collections schema:" -ForegroundColor Yellow
Write-Host "Collections to create:" -ForegroundColor Cyan

foreach ($collection in $collectionsJson) {
    Write-Host "  - $($collection.name)" -ForegroundColor White

    # In real implementation, you would make API calls to PocketBase admin API
    # For now, just display what would be created
    Write-Host "    Fields:" -ForegroundColor Gray
    foreach ($field in $collection.schema) {
        Write-Host "      - $($field.name) ($($field.type))" -ForegroundColor DarkGray
    }
}

Write-Host ""
Write-Host "To complete setup:" -ForegroundColor Yellow
Write-Host "1. Start PocketBase server" -ForegroundColor White
Write-Host "2. Create admin user if needed" -ForegroundColor White
Write-Host "3. Use PocketBase admin UI or API to create collections" -ForegroundColor White
Write-Host "4. Run: pb collections import collections.json" -ForegroundColor White

Write-Host ""
Write-Host "Collection JSON structure:" -ForegroundColor Cyan
Write-Host (ConvertTo-Json $collectionsJson -Depth 3) -ForegroundColor Gray
