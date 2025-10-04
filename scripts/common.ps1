# Common helper functions for PowerShell scripts

function Test-PocketBaseConnection {
    param([string]$Url = "http://localhost:8090")

    try {
        $response = Invoke-RestMethod -Uri "$Url/api/health" -Method Get -TimeoutSec 5
        return $true
    }
    catch {
        return $false
    }
}

function Wait-ForPocketBase {
    param(
        [string]$Url = "http://localhost:8090",
        [int]$MaxAttempts = 30,
        [int]$DelaySeconds = 2
    )

    Write-Host "⏳ Waiting for PocketBase to be ready..." -ForegroundColor Yellow

    for ($i = 1; $i -le $MaxAttempts; $i++) {
        if (Test-PocketBaseConnection -Url $Url) {
            Write-Host "✅ PocketBase is ready!" -ForegroundColor Green
            return $true
        }

        Write-Host "  Attempt $i/$MaxAttempts - PocketBase not ready yet..." -ForegroundColor Gray
        Start-Sleep -Seconds $DelaySeconds
    }

    Write-Host "❌ PocketBase failed to start within $MaxAttempts attempts" -ForegroundColor Red
    return $false
}

function Get-PocketBaseAdminToken {
    param(
        [string]$Url = "http://localhost:8090",
        [string]$Email = "admin@pocketbase.local",
        [string]$Password = "123456789"
    )

    try {
        $response = Invoke-RestMethod -Uri "$Url/api/admins/auth-with-password" `
            -Method Post `
            -ContentType "application/json" `
            -Body (@{
                identity = $Email
                password = $Password
            } | ConvertTo-Json)

        return $response.token
    }
    catch {
        Write-Host "Failed to authenticate with PocketBase: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

Write-Host "Common functions loaded" -ForegroundColor Cyan
