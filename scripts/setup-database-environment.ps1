# Setup Database Environment for Migration
# This script sets up PostgreSQL and Redis for the migration process

param(
    [switch]$SetupPostgreSQL = $true,
    [switch]$SetupRedis = $true,
    [string]$PostgreVersion = "15",
    [string]$RedisVersion = "7.2",
    [string]$InstallPath = "C:\database-setup",
    [switch]$DockerMode = $false
)

Write-Host "🛠️  Database Environment Setup" -ForegroundColor Green
Write-Host "============================" -ForegroundColor Green

# Create installation directory
if (!(Test-Path $InstallPath)) {
    New-Item -ItemType Directory -Path $InstallPath | Out-Null
    Write-Host "📁 Created installation directory: $InstallPath" -ForegroundColor Green
}

# Function to check if a service is running
function Test-ServiceRunning {
    param([string]$ServiceName)
    try {
        $service = Get-Service -Name $ServiceName -ErrorAction Stop
        return $service.Status -eq "Running"
    }
    catch {
        return $false
    }
}

# Function to wait for service to be ready
function Wait-ForService {
    param(
        [string]$ServiceName,
        [int]$TimeoutSeconds = 30
    )

    $startTime = Get-Date
    while ((Get-Date) - $startTime -lt (New-TimeSpan -Seconds $TimeoutSeconds)) {
        if (Test-ServiceRunning -ServiceName $ServiceName) {
            Write-Host "✅ $ServiceName is ready!" -ForegroundColor Green
            return $true
        }
        Write-Host "⏳ Waiting for $ServiceName..." -ForegroundColor Yellow
        Start-Sleep -Seconds 2
    }
    Write-Warning "❌ Timeout waiting for $ServiceName"
    return $false
}

# Setup PostgreSQL
if ($SetupPostgreSQL) {
    Write-Host ""
    Write-Host "🐘 Setting up PostgreSQL $PostgreVersion" -ForegroundColor Cyan

    if ($DockerMode) {
        Write-Host "🐳 Using Docker for PostgreSQL..." -ForegroundColor Yellow

        # Check if Docker is running
        $dockerRunning = Test-ServiceRunning -ServiceName "Docker Desktop Service"
        if (!$dockerRunning) {
            Write-Error "Docker Desktop Service is not running. Please start Docker Desktop first."
            exit 1
        }

        # Create PostgreSQL container
        $containerName = "gamev1-postgresql"
        $postgresPassword = "gamev1_password"

        Write-Host "📦 Creating PostgreSQL container..." -ForegroundColor White
        docker run --name $containerName -e POSTGRES_PASSWORD=$postgresPassword -e POSTGRES_DB=gamev1 -p 5432:5432 -d postgres:$PostgreVersion

        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ PostgreSQL container created successfully" -ForegroundColor Green
            Write-Host "📡 PostgreSQL is accessible at: localhost:5432" -ForegroundColor Green
            Write-Host "🔑 Password: $postgresPassword" -ForegroundColor Green
        }
        else {
            Write-Error "Failed to create PostgreSQL container"
        }
    }
    else {
        Write-Host "💻 Using native PostgreSQL installation..." -ForegroundColor Yellow

        # Check if PostgreSQL is already installed
        $postgresInstalled = Test-ServiceRunning -ServiceName "postgresql*"

        if ($postgresInstalled) {
            Write-Host "✅ PostgreSQL is already installed and running" -ForegroundColor Green
        }
        else {
            Write-Host "📥 PostgreSQL not found. Please install PostgreSQL $PostgreVersion manually." -ForegroundColor Yellow
            Write-Host "   Download from: https://www.postgresql.org/download/windows/" -ForegroundColor Gray
            Write-Host "   Or use: choco install postgresql$PostgreVersion" -ForegroundColor Gray

            # Try to install with Chocolatey if available
            try {
                $chocoInstalled = Get-Command choco -ErrorAction SilentlyContinue
                if ($chocoInstalled) {
                    Write-Host "🍫 Installing PostgreSQL with Chocolatey..." -ForegroundColor White
                    choco install postgresql$PostgreVersion -y
                }
            }
            catch {
                Write-Host "   Chocolatey not found or installation failed" -ForegroundColor Gray
            }
        }
    }
}

# Setup Redis
if ($SetupRedis) {
    Write-Host ""
    Write-Host "🔴 Setting up Redis $RedisVersion" -ForegroundColor Cyan

    if ($DockerMode) {
        Write-Host "🐳 Using Docker for Redis..." -ForegroundColor Yellow

        # Create Redis container
        $containerName = "gamev1-redis"
        Write-Host "📦 Creating Redis container..." -ForegroundColor White
        docker run --name $containerName -p 6379:6379 -d redis:$RedisVersion

        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Redis container created successfully" -ForegroundColor Green
            Write-Host "📡 Redis is accessible at: localhost:6379" -ForegroundColor Green
        }
        else {
            Write-Error "Failed to create Redis container"
        }
    }
    else {
        Write-Host "💻 Using native Redis installation..." -ForegroundColor Yellow

        # Check if Redis is already installed
        $redisInstalled = Test-ServiceRunning -ServiceName "Redis"

        if ($redisInstalled) {
            Write-Host "✅ Redis is already installed and running" -ForegroundColor Green
        }
        else {
            Write-Host "📥 Redis not found. Please install Redis $RedisVersion manually." -ForegroundColor Yellow
            Write-Host "   Download from: https://redis.io/download" -ForegroundColor Gray
            Write-Host "   Or use: choco install redis-64" -ForegroundColor Gray

            # Try to install with Chocolatey if available
            try {
                $chocoInstalled = Get-Command choco -ErrorAction SilentlyContinue
                if ($chocoInstalled) {
                    Write-Host "🍫 Installing Redis with Chocolatey..." -ForegroundColor White
                    choco install redis-64 -y
                }
            }
            catch {
                Write-Host "   Chocolatey not found or installation failed" -ForegroundColor Gray
            }
        }
    }
}

# Wait for services to be ready
Write-Host ""
Write-Host "⏳ Waiting for services to be ready..." -ForegroundColor Cyan

if ($SetupPostgreSQL -and !$DockerMode) {
    if (Wait-ForService -ServiceName "postgresql*") {
        # Test PostgreSQL connection
        try {
            $result = Invoke-Sqlcmd -ServerInstance "localhost" -Database "postgres" -Query "SELECT version();" -ErrorAction Stop
            Write-Host "✅ PostgreSQL connection test successful" -ForegroundColor Green
        }
        catch {
            Write-Warning "❌ PostgreSQL connection test failed: $($_.Exception.Message)"
        }
    }
}

if ($SetupRedis -and !$DockerMode) {
    if (Wait-ForService -ServiceName "Redis") {
        # Test Redis connection
        try {
            $redisCli = Get-Command "redis-cli" -ErrorAction SilentlyContinue
            if ($redisCli) {
                $result = & redis-cli ping
                if ($result -eq "PONG") {
                    Write-Host "✅ Redis connection test successful" -ForegroundColor Green
                }
                else {
                    Write-Warning "❌ Redis connection test failed"
                }
            }
        }
        catch {
            Write-Warning "❌ Redis connection test failed: $($_.Exception.Message)"
        }
    }
}

# Create database and user for the game
Write-Host ""
Write-Host "🏗️  Setting up game database..." -ForegroundColor Cyan

try {
    # Create database and user
    $createDbQuery = @"
    -- Create user if not exists
    DO $$
    BEGIN
       IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'gamev1_user') THEN
          CREATE ROLE gamev1_user LOGIN PASSWORD 'gamev1_password';
       END IF;
    END
    $$;

    -- Create database if not exists
    SELECT 'CREATE DATABASE gamev1 OWNER gamev1_user' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'gamev1')\gexec

    -- Grant privileges
    GRANT ALL PRIVILEGES ON DATABASE gamev1 TO gamev1_user;
"@

    $createDbQuery | Invoke-Sqlcmd -ServerInstance "localhost" -Database "postgres" -ErrorAction Stop
    Write-Host "✅ Database and user created successfully" -ForegroundColor Green

    # Run the schema script
    Write-Host "📋 Creating database schema..." -ForegroundColor White
    & ".\database-schema.sql"  # This should run the schema creation

}
catch {
    Write-Warning "❌ Database setup failed: $($_.Exception.Message)"
    Write-Host "   You may need to run the schema manually or check permissions" -ForegroundColor Gray
}

# Final status
Write-Host ""
Write-Host "🎉 Database Environment Setup Complete!" -ForegroundColor Green
Write-Host "======================================" -ForegroundColor Green

if ($DockerMode) {
    Write-Host "🐳 Docker containers created:" -ForegroundColor Cyan
    Write-Host "   - gamev1-postgresql (port 5432)" -ForegroundColor White
    Write-Host "   - gamev1-redis (port 6379)" -ForegroundColor White
}
else {
    Write-Host "💻 Services status:" -ForegroundColor Cyan
    Write-Host "   - PostgreSQL: $(if (Test-ServiceRunning -ServiceName 'postgresql*') { '✅ Running' } else { '❌ Stopped' })" -ForegroundColor White
    Write-Host "   - Redis: $(if (Test-ServiceRunning -ServiceName 'Redis') { '✅ Running' } else { '❌ Stopped' })" -ForegroundColor White
}

Write-Host ""
Write-Host "📡 Connection Details:" -ForegroundColor Cyan
Write-Host "   PostgreSQL: localhost:5432/gamev1 (user: gamev1_user, pass: gamev1_password)" -ForegroundColor White
Write-Host "   Redis: localhost:6379" -ForegroundColor White

Write-Host ""
Write-Host "🚀 Next Steps:" -ForegroundColor Magenta
Write-Host "1. Run migration: .\migrate-to-postgresql.ps1" -ForegroundColor White
Write-Host "2. Update application configuration" -ForegroundColor White
Write-Host "3. Test the application with new database" -ForegroundColor White

Write-Host ""
Write-Host "📚 For troubleshooting, see: migration-strategy.md" -ForegroundColor Green
