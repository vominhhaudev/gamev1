#!/usr/bin/env pwsh
# Service Manager for GameV1 Project
# Manages gateway, worker, and pocketbase services

param(
    [switch]$Status,
    [switch]$Start,
    [switch]$Stop,
    [switch]$Restart,
    [string]$Service = "all" # gateway, worker, pocketbase, or all
)

# Configuration
$GATEWAY_PORT = 8080
$WORKER_PORT = 3000
$POCKETBASE_PORT = 8090

function Get-ServiceStatus {
    param([string]$ServiceName)

    $processes = @()

    switch ($ServiceName) {
        "gateway" {
            $proc = Get-Process -Name "*gateway*" -ErrorAction SilentlyContinue
            if ($proc) {
                $portCheck = netstat -ano | findstr ":$GATEWAY_PORT"
                $status = if ($portCheck -and $portCheck -match "LISTENING") { "Running" } else { "Error" }
                $processes += @{
                    Name = "Gateway"
                    ProcessId = $proc.Id
                    Port = $GATEWAY_PORT
                    Status = $status
                }
            } else {
                $processes += @{
                    Name = "Gateway"
                    ProcessId = "N/A"
                    Port = $GATEWAY_PORT
                    Status = "Stopped"
                }
            }
        }
        "worker" {
            $proc = Get-Process -Name "*worker*" -ErrorAction SilentlyContinue
            if ($proc) {
                $processes += @{
                    Name = "Worker"
                    ProcessId = $proc.Id
                    Port = $WORKER_PORT
                    Status = "Running"
                }
            } else {
                $processes += @{
                    Name = "Worker"
                    ProcessId = "N/A"
                    Port = $WORKER_PORT
                    Status = "Stopped"
                }
            }
        }
        "pocketbase" {
            $proc = Get-Process -Name "*pocketbase*" -ErrorAction SilentlyContinue
            if ($proc) {
                $processes += @{
                    Name = "PocketBase"
                    ProcessId = $proc.Id
                    Port = $POCKETBASE_PORT
                    Status = "Running"
                }
            } else {
                $processes += @{
                    Name = "PocketBase"
                    ProcessId = "N/A"
                    Port = $POCKETBASE_PORT
                    Status = "Stopped"
                }
            }
        }
        "all" {
            $processes += Get-ServiceStatus "gateway"
            $processes += Get-ServiceStatus "worker"
            $processes += Get-ServiceStatus "pocketbase"
        }
    }

    return $processes
}

function Stop-Service {
    param([string]$ServiceName)

    Write-Host "🛑 Stopping $ServiceName..." -ForegroundColor Yellow

    switch ($ServiceName) {
        "gateway" {
            Get-Process -Name "*gateway*" -ErrorAction SilentlyContinue | Stop-Process -Force
        }
        "worker" {
            Get-Process -Name "*worker*" -ErrorAction SilentlyContinue | Stop-Process -Force
        }
        "pocketbase" {
            Get-Process -Name "*pocketbase*" -ErrorAction SilentlyContinue | Stop-Process -Force
        }
        "all" {
            Stop-Service "gateway"
            Stop-Service "worker"
            Stop-Service "pocketbase"
        }
    }

    Start-Sleep -Seconds 2
    Write-Host "✅ $ServiceName stopped" -ForegroundColor Green
}

function Start-Service {
    param([string]$ServiceName)

    Write-Host "🚀 Starting $ServiceName..." -ForegroundColor Yellow

    switch ($ServiceName) {
        "gateway" {
            # Check if port is available
            $portInUse = netstat -ano | findstr ":$GATEWAY_PORT.*LISTENING"
            if ($portInUse) {
                Write-Host "❌ Port $GATEWAY_PORT is already in use" -ForegroundColor Red
                return $false
            }

            Start-Process -NoNewWindow -FilePath "cargo" -ArgumentList "run --package gateway" -PassThru
        }
        "worker" {
            Start-Process -NoNewWindow -FilePath "cargo" -ArgumentList "run --package worker" -PassThru
        }
        "pocketbase" {
            # Check if PocketBase is already running
            $pbProcess = Get-Process -Name "*pocketbase*" -ErrorAction SilentlyContinue
            if ($pbProcess) {
                Write-Host "ℹ️ PocketBase is already running" -ForegroundColor Blue
                return $true
            }

            # Start PocketBase
            $pbPath = Join-Path $PSScriptRoot "..\pocketbase\pocketbase.exe"
            if (Test-Path $pbPath) {
                Start-Process -NoNewWindow -FilePath $pbPath -ArgumentList "serve --http=127.0.0.1:$POCKETBASE_PORT" -PassThru
            } else {
                Write-Host "❌ PocketBase executable not found at $pbPath" -ForegroundColor Red
                return $false
            }
        }
        "all" {
            $success = $true
            $success = $success -and (Start-Service "pocketbase")
            Start-Sleep -Seconds 2
            $success = $success -and (Start-Service "worker")
            Start-Sleep -Seconds 2
            $success = $success -and (Start-Service "gateway")
            return $success
        }
        default {
            Write-Host "❌ Unknown service: $ServiceName" -ForegroundColor Red
            return $false
        }
    }

    Start-Sleep -Seconds 3
    return $true
}

function Test-ServiceHealth {
    param([string]$ServiceName)

    switch ($ServiceName) {
        "gateway" {
            try {
                $response = Invoke-WebRequest -Uri "http://localhost:$GATEWAY_PORT/healthz" -TimeoutSec 3 -ErrorAction Stop
                return $response.StatusCode -eq 200
            } catch {
                return $false
            }
        }
        "pocketbase" {
            try {
                $response = Invoke-WebRequest -Uri "http://localhost:$POCKETBASE_PORT/_/" -TimeoutSec 3 -ErrorAction Stop
                return $response.StatusCode -eq 200
            } catch {
                return $false
            }
        }
        default {
            return $false
        }
    }
}

# Main logic
if ($Status) {
    Write-Host "📊 Service Status:" -ForegroundColor Cyan
    Write-Host "=================" -ForegroundColor Cyan

    $services = Get-ServiceStatus "all"
    foreach ($service in $services) {
        $statusColor = switch ($service.Status) {
            "Running" { "Green" }
            "Error" { "Red" }
            "Stopped" { "Yellow" }
            default { "White" }
        }

        Write-Host "$($service.Name): " -NoNewline
        Write-Host "$($service.Status)" -ForegroundColor $statusColor

        if ($service.Status -eq "Running") {
            $healthy = Test-ServiceHealth $service.Name.ToLower()
            $healthColor = if ($healthy) { "Green" } else { "Red" }
            Write-Host "   Health: " -NoNewline
            if ($healthy) {
                Write-Host "OK" -ForegroundColor $healthColor
            } else {
                Write-Host "FAIL" -ForegroundColor $healthColor
            }
        }
    }
}

if ($Stop) {
    $services = if ($Service -eq "all") { "all" } else { $Service }
    Stop-Service $services
}

if ($Start) {
    $services = if ($Service -eq "all") { "all" } else { $Service }
    $success = Start-Service $services

    if ($success) {
        Write-Host "🎉 Services started successfully!" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed to start some services" -ForegroundColor Red
    }
}

if ($Restart) {
    Write-Host "🔄 Restarting services..." -ForegroundColor Yellow
    Stop-Service $Service
    Start-Sleep -Seconds 2
    $success = Start-Service $Service

    if ($success) {
        Write-Host "🎉 Services restarted successfully!" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed to restart services" -ForegroundColor Red
    }
}

# Default action if no flags provided
if (-not ($Status -or $Start -or $Stop -or $Restart)) {
    Write-Host "🎮 GameV1 Service Manager" -ForegroundColor Cyan
    Write-Host "=========================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\service-manager.ps1 [OPTIONS]" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Options:" -ForegroundColor Yellow
    Write-Host "  -Status       Show status of all services"
    Write-Host "  -Start        Start specified service (default: all)"
    Write-Host "  -Stop         Stop specified service (default: all)"
    Write-Host "  -Restart      Restart specified service (default: all)"
    Write-Host "  -Service <s>  Specify service: gateway, worker, pocketbase, or all (default: all)"
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\service-manager.ps1 -Status"
    Write-Host "  .\service-manager.ps1 -Start -Service gateway"
    Write-Host "  .\service-manager.ps1 -Restart"
    Write-Host ""
    Write-Host "Current Status:" -ForegroundColor Cyan
    $services = Get-ServiceStatus "all"
    foreach ($service in $services) {
        Write-Host "  $($service.Name): $($service.Status)"
    }
}
