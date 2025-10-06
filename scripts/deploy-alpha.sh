#!/bin/bash

# GameV1 Alpha Release Native Deployment Script
# Supports 1000+ concurrent players with advanced performance optimizations
# NO DOCKER DEPENDENCIES - Pure native deployment

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly DEPLOY_ENV="${DEPLOY_ENV:-production}"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Prerequisites check - NO DOCKER
check_prerequisites() {
    log_info "Checking prerequisites for native deployment..."

    # Check Rust
    if ! command -v rustc &> /dev/null; then
        log_error "Rust is not installed. Please install Rust 1.70+"
        log_info "Visit: https://rustup.rs/"
        exit 1
    fi

    # Check Node.js
    if ! command -v node &> /dev/null; then
        log_error "Node.js is not installed. Please install Node.js 18+"
        exit 1
    fi

    # Check PostgreSQL
    if ! command -v psql &> /dev/null; then
        log_error "PostgreSQL client is not installed. Please install PostgreSQL"
        exit 1
    fi

    # Check Redis
    if ! command -v redis-cli &> /dev/null; then
        log_error "Redis client is not installed. Please install Redis"
        exit 1
    fi

    # Check Nginx (for load balancing)
    if ! command -v nginx &> /dev/null; then
        log_warning "Nginx not installed - will use simple reverse proxy instead"
    fi

    log_success "Prerequisites check completed"
}

# Install system dependencies
install_dependencies() {
    log_info "Installing system dependencies..."

    # Update package manager
    if command -v apt-get &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y build-essential pkg-config libssl-dev redis-server postgresql postgresql-contrib nginx prometheus-node-exporter
    elif command -v yum &> /dev/null; then
        sudo yum groupinstall -y "Development Tools"
        sudo yum install -y redis postgresql postgresql-server postgresql-contrib nginx
    elif command -v pacman &> /dev/null; then
        sudo pacman -Syu base-devel redis postgresql nginx
    else
        log_warning "Unknown package manager - please install dependencies manually"
    fi

    log_success "System dependencies installed"
}

# Environment setup
setup_environment() {
    log_info "Setting up environment for ${DEPLOY_ENV}..."

    # Create necessary directories
    mkdir -p "${PROJECT_ROOT}/data/redis"
    mkdir -p "${PROJECT_ROOT}/data/postgres"
    mkdir -p "${PROJECT_ROOT}/data/grafana"
    mkdir -p "${PROJECT_ROOT}/logs"
    mkdir -p "${PROJECT_ROOT}/certs"

    # Set proper permissions
    chmod -R 755 "${PROJECT_ROOT}/data"
    chmod -R 755 "${PROJECT_ROOT}/logs"
    chmod -R 755 "${PROJECT_ROOT}/certs"

    # Create environment file if it doesn't exist
    if [[ ! -f "${PROJECT_ROOT}/.env.${DEPLOY_ENV}" ]]; then
        log_warning "Environment file not found: ${PROJECT_ROOT}/.env.${DEPLOY_ENV}"
        log_info "Creating default environment configuration..."

        cat > "${PROJECT_ROOT}/.env.${DEPLOY_ENV}" << EOF
# Database Configuration
DATABASE_URL=postgresql://gamev1_user:gamev1_password@localhost:5432/gamev1
REDIS_URL=redis://localhost:6379/0

# Server Configuration
GATEWAY_BIND_ADDR=0.0.0.0:3000
WORKER_ENDPOINT=http://localhost:50051
ROOM_MANAGER_ENDPOINT=http://localhost:8080

# Performance Settings for 1000+ players
MAX_CONNECTIONS=1000
MAX_CONNECTIONS_PER_ROOM=100
CONNECTION_TIMEOUT=30
MESSAGE_QUEUE_SIZE=1000
HEARTBEAT_INTERVAL=30
MAX_IDLE_TIME=300

# Matchmaking Configuration
MAX_WAIT_TIME=300
MAX_SKILL_DIFF=200.0
MIN_PLAYERS_PER_MATCH=2
MAX_PLAYERS_PER_MATCH=8
STRICT_SKILL_MATCHING=false
REGION_BASED_MATCHING=true
PRIORITY_QUEUE=true

# Auto-scaling Configuration
MIN_SERVERS=2
MAX_SERVERS=10
SCALE_UP_THRESHOLD=75.0
SCALE_DOWN_THRESHOLD=25.0
SCALE_UP_COOLDOWN=300
SCALE_DOWN_COOLDOWN=600

# Redis Cache Configuration
REDIS_POOL_SIZE=20
REDIS_CONNECTION_TIMEOUT=5
REDIS_COMMAND_TIMEOUT=2
REDIS_DEFAULT_TTL=300

# Database Pool Configuration
DB_POOL_SIZE=50
DB_MIN_IDLE=5
DB_CONNECTION_TIMEOUT=30
DB_QUERY_TIMEOUT=10

# Monitoring Configuration
ENABLE_METRICS=true
PROMETHEUS_GATEWAY=http://localhost:9090
GRAFANA_URL=http://localhost:3000

# Security Configuration
JWT_SECRET_KEY=$(openssl rand -hex 32)
CORS_ORIGINS=*

# Logging Configuration
LOG_LEVEL=info
LOG_FORMAT=json
EOF

        log_success "Created default environment file: ${PROJECT_ROOT}/.env.${DEPLOY_ENV}"
    fi

    # Load environment variables
    if [[ -f "${PROJECT_ROOT}/.env.${DEPLOY_ENV}" ]]; then
        source "${PROJECT_ROOT}/.env.${DEPLOY_ENV}"
    fi

    log_success "Environment setup completed"
}

# PostgreSQL setup
setup_postgresql() {
    log_info "Setting up PostgreSQL database..."

    # Initialize PostgreSQL if needed
    if [[ ! -d "/var/lib/postgresql/data" ]]; then
        sudo -u postgres initdb -D /var/lib/postgresql/data
    fi

    # Start PostgreSQL service
    sudo systemctl start postgresql || sudo service postgresql start

    # Wait for PostgreSQL to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    timeout=60
    while ! pg_isready -h localhost -p 5432; do
        sleep 1
        timeout=$((timeout - 1))
        if [[ $timeout -le 0 ]]; then
            log_error "PostgreSQL startup timeout"
            exit 1
        fi
    done

    # Create database and user
    sudo -u postgres psql -c "CREATE USER gamev1_user WITH PASSWORD 'gamev1_password';" || true
    sudo -u postgres psql -c "CREATE DATABASE gamev1 OWNER gamev1_user;" || true
    sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE gamev1 TO gamev1_user;" || true

    # Run database migrations
    log_info "Running database migrations..."
    psql -h localhost -U gamev1_user -d gamev1 -f "${PROJECT_ROOT}/scripts/database-schema.sql" || {
        log_warning "Database schema migration failed or already exists"
    }

    log_success "PostgreSQL setup completed"
}

# Redis setup
setup_redis() {
    log_info "Setting up Redis cache..."

    # Configure Redis for optimal performance
    sudo tee /etc/redis/redis.conf > /dev/null << EOF
# GameV1 Optimized Redis Configuration
maxmemory 2gb
maxmemory-policy allkeys-lru
tcp-keepalive 300
timeout 300
save 900 1
save 300 10
save 60 10000
appendonly yes
appendfsync everysec
no-appendfsync-on-rewrite no
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb
lua-time-limit 5000
slowlog-log-slower-than 10000
slowlog-max-len 128
hash-max-ziplist-entries 512
hash-max-ziplist-value 64
list-max-ziplist-size -2
list-compress-depth 0
set-max-intset-entries 512
zset-max-ziplist-entries 128
zset-max-ziplist-value 64
hll-sparse-max-bytes 3000
activerehashing yes
hz 10
aof-rewrite-incremental-fsync yes
EOF

    # Start Redis service
    sudo systemctl start redis-server || sudo service redis-server start

    # Wait for Redis to be ready
    log_info "Waiting for Redis to be ready..."
    timeout=30
    while ! redis-cli ping &> /dev/null; do
        sleep 1
        timeout=$((timeout - 1))
        if [[ $timeout -le 0 ]]; then
            log_error "Redis startup timeout"
            exit 1
        fi
    done

    # Configure Redis for high-concurrency
    redis-cli config set maxmemory 2gb || true
    redis-cli config set maxmemory-policy allkeys-lru || true
    redis-cli config set tcp-keepalive 300 || true

    log_success "Redis setup completed"
}

# Build services
build_services() {
    log_info "Building GameV1 services..."

    # Build Rust services
    cd "${PROJECT_ROOT}"
    cargo build --release

    # Build client
    cd "${PROJECT_ROOT}/client"
    npm install
    npm run build

    log_success "Services built successfully"
}

# Deploy services
deploy_services() {
    log_info "Deploying GameV1 services..."

    # Create systemd service files
    create_systemd_services

    # Start services
    sudo systemctl daemon-reload

    # Start core services
    sudo systemctl start gamev1-gateway
    sudo systemctl start gamev1-worker
    sudo systemctl start gamev1-room-manager

    # Wait for services to be healthy
    log_info "Waiting for services to become healthy..."

    # Check gateway health
    timeout=120
    while ! curl -f http://localhost:3000/healthz &> /dev/null; do
        sleep 2
        timeout=$((timeout - 2))
        if [[ $timeout -le 0 ]]; then
            log_error "Gateway health check timeout"
            exit 1
        fi
    done

    # Check worker health
    timeout=60
    while ! curl -f http://localhost:50051/health &> /dev/null; do
        sleep 2
        timeout=$((timeout - 2))
        if [[ $timeout -le 0 ]]; then
            log_warning "Worker health check timeout - continuing anyway"
            break
        fi
    done

    log_success "Service deployment completed"
}

# Create systemd service files
create_systemd_services() {
    log_info "Creating systemd service files..."

    # Gateway service
    sudo tee /etc/systemd/system/gamev1-gateway.service > /dev/null << EOF
[Unit]
Description=GameV1 Gateway Server
After=network.target redis.service postgresql.service

[Service]
Type=simple
User=gamev1
Group=gamev1
WorkingDirectory=${PROJECT_ROOT}
ExecStart=${PROJECT_ROOT}/target/release/gateway
EnvironmentFile=${PROJECT_ROOT}/.env.${DEPLOY_ENV}
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${PROJECT_ROOT}/logs

# Resource limits
LimitNOFILE=65536
MemoryLimit=1G

[Install]
WantedBy=multi-user.target
EOF

    # Worker service
    sudo tee /etc/systemd/system/gamev1-worker.service > /dev/null << EOF
[Unit]
Description=GameV1 Worker Server
After=network.target redis.service postgresql.service

[Service]
Type=simple
User=gamev1
Group=gamev1
WorkingDirectory=${PROJECT_ROOT}
ExecStart=${PROJECT_ROOT}/target/release/worker
EnvironmentFile=${PROJECT_ROOT}/.env.${DEPLOY_ENV}
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${PROJECT_ROOT}/logs

# Resource limits
LimitNOFILE=65536
MemoryLimit=2G

[Install]
WantedBy=multi-user.target
EOF

    # Room Manager service
    sudo tee /etc/systemd/system/gamev1-room-manager.service > /dev/null << EOF
[Unit]
Description=GameV1 Room Manager
After=network.target redis.service postgresql.service

[Service]
Type=simple
User=gamev1
Group=gamev1
WorkingDirectory=${PROJECT_ROOT}
ExecStart=${PROJECT_ROOT}/target/release/room-manager
EnvironmentFile=${PROJECT_ROOT}/.env.${DEPLOY_ENV}
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${PROJECT_ROOT}/logs

# Resource limits
LimitNOFILE=65536
MemoryLimit=512M

[Install]
WantedBy=multi-user.target
EOF

    # Create gamev1 user if it doesn't exist
    if ! id -u gamev1 &> /dev/null; then
        sudo useradd --system --shell /bin/false --home "${PROJECT_ROOT}" gamev1
    fi

    # Set ownership
    sudo chown -R gamev1:gamev1 "${PROJECT_ROOT}/logs"
    sudo chown -R gamev1:gamev1 "${PROJECT_ROOT}/data"

    log_success "Systemd service files created"
}

# Load balancer setup
setup_load_balancer() {
    log_info "Setting up load balancer..."

    # Generate SSL certificate if needed
    if [[ ! -f "${PROJECT_ROOT}/certs/gamev1.crt" ]]; then
        log_info "Generating self-signed SSL certificate..."
        openssl req -x509 -newkey rsa:4096 -keyout "${PROJECT_ROOT}/certs/gamev1.key" \
            -out "${PROJECT_ROOT}/certs/gamev1.crt" -days 365 -nodes \
            -subj "/C=US/ST=State/L=City/O=Organization/CN=gamev1.local"
        log_success "Generated SSL certificate"
    fi

    # Configure Nginx if available
    if command -v nginx &> /dev/null; then
        setup_nginx_load_balancer
    else
        setup_simple_load_balancer
    fi
}

# Setup Nginx load balancer
setup_nginx_load_balancer() {
    log_info "Setting up Nginx load balancer..."

    sudo tee /etc/nginx/sites-available/gamev1 > /dev/null << EOF
upstream gamev1_gateway {
    least_conn;
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    server 127.0.0.1:3002;
    keepalive 32;
}

server {
    listen 80;
    listen 443 ssl http2;
    server_name localhost;

    ssl_certificate ${PROJECT_ROOT}/certs/gamev1.crt;
    ssl_certificate_key ${PROJECT_ROOT}/certs/gamev1.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload";

    # Proxy settings for high concurrency
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;
    proxy_request_buffering off;
    proxy_buffering off;
    proxy_http_version 1.1;
    proxy_set_header Connection "";

    location / {
        proxy_pass http://gamev1_gateway;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # WebSocket support
    location /ws {
        proxy_pass http://gamev1_gateway;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_read_timeout 86400;
    }

    # Health check endpoint
    location /nginx-health {
        access_log off;
        return 200 "healthy\n";
        add_header Content-Type text/plain;
    }
}

# Monitoring server block
server {
    listen 9090;
    server_name localhost;

    location /stub_status {
        stub_status on;
        access_log off;
        allow 127.0.0.1;
        deny all;
    }
}
EOF

    sudo ln -sf /etc/nginx/sites-available/gamev1 /etc/nginx/sites-enabled/
    sudo rm -f /etc/nginx/sites-enabled/default

    # Test Nginx configuration
    sudo nginx -t

    # Start/Restart Nginx
    sudo systemctl restart nginx

    log_success "Nginx load balancer configured and started"
}

# Setup simple load balancer (fallback)
setup_simple_load_balancer() {
    log_info "Setting up simple reverse proxy..."

    # Create simple Node.js reverse proxy
    cat > "${PROJECT_ROOT}/scripts/simple-proxy.js" << EOF
const http = require('http');
const httpProxy = require('http-proxy');

const proxy = httpProxy.createProxyServer({});
const servers = [
    'http://127.0.0.1:3000',
    'http://127.0.0.1:3001',
    'http://127.0.0.1:3002'
];

let currentServer = 0;

const server = http.createServer((req, res) => {
    const target = servers[currentServer];

    console.log(\`Proxying request to: \${target}\`);

    proxy.web(req, res, { target });

    currentServer = (currentServer + 1) % servers.length;
});

server.listen(80, () => {
    console.log('Simple reverse proxy listening on port 80');
});
EOF

    log_success "Simple load balancer setup completed (requires manual start)"
}

# Monitoring setup
setup_monitoring() {
    log_info "Setting up monitoring stack..."

    # Create Prometheus configuration
    if [[ ! -f "${PROJECT_ROOT}/prometheus.yml" ]]; then
        log_info "Creating Prometheus configuration..."

        cat > "${PROJECT_ROOT}/prometheus.yml" << EOF
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "alerts.yml"

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']

  - job_name: 'gamev1_gateway'
    static_configs:
      - targets: ['localhost:3000']
    scrape_interval: 10s
    metrics_path: '/metrics'

  - job_name: 'gamev1_worker'
    static_configs:
      - targets: ['localhost:50051']
    scrape_interval: 10s
    metrics_path: '/metrics'

  - job_name: 'redis'
    static_configs:
      - targets: ['localhost:9121']
    scrape_interval: 30s

  - job_name: 'postgres'
    static_configs:
      - targets: ['localhost:9187']
    scrape_interval: 30s
EOF

        log_success "Created Prometheus configuration"
    fi

    # Create alert rules
    if [[ ! -f "${PROJECT_ROOT}/alerts.yml" ]]; then
        log_info "Creating alert rules..."

        cat > "${PROJECT_ROOT}/alerts.yml" << EOF
groups:
- name: gamev1.alerts
  rules:
  - alert: HighConnectionCount
    expr: gamev1_connections_active > 800
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High connection count detected"

  - alert: DatabaseConnectionPoolExhausted
    expr: increase(gamev1_db_connection_errors_total[5m]) > 10
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "Database connection pool exhausted"

  - alert: ServiceDown
    expr: up{job=~"gamev1_gateway|gamev1_worker"} == 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "Service is down"
EOF

        log_success "Created alert rules"
    fi

    # Install and start Prometheus if not already running
    if ! systemctl is-active --quiet prometheus; then
        log_info "Installing and starting Prometheus..."
        # Note: In production, use official Prometheus installation
        log_warning "Prometheus not installed - install manually for full monitoring"
    fi

    log_success "Monitoring setup completed"
}

# Health check
health_check() {
    log_info "Performing health checks..."

    # Check main services
    local services=("gateway" "worker" "redis" "postgres")
    local all_healthy=true

    for service in "${services[@]}"; do
        case "${service}" in
            "gateway")
                if curl -f --max-time 5 http://localhost:3000/healthz &> /dev/null; then
                    log_success "Gateway is healthy"
                else
                    log_error "Gateway health check failed"
                    all_healthy=false
                fi
                ;;
            "worker")
                if curl -f --max-time 5 http://localhost:50051/health &> /dev/null; then
                    log_success "Worker is healthy"
                else
                    log_warning "Worker health check failed"
                fi
                ;;
            "redis")
                if redis-cli ping | grep -q PONG; then
                    log_success "Redis is healthy"
                else
                    log_error "Redis health check failed"
                    all_healthy=false
                fi
                ;;
            "postgres")
                if pg_isready -h localhost -p 5432; then
                    log_success "PostgreSQL is healthy"
                else
                    log_error "PostgreSQL health check failed"
                    all_healthy=false
                fi
                ;;
        esac
    done

    if [[ "${all_healthy}" == "true" ]]; then
        log_success "All health checks passed"
        return 0
    else
        log_error "Some health checks failed"
        return 1
    fi
}

# Performance optimization
optimize_performance() {
    log_info "Applying performance optimizations..."

    # Optimize system limits
    if [[ -f /etc/security/limits.conf ]]; then
        log_info "Updating system limits for high concurrency..."
        echo '* soft nofile 65536' | sudo tee -a /etc/security/limits.conf
        echo '* hard nofile 65536' | sudo tee -a /etc/security/limits.conf
        echo '* soft nproc 65536' | sudo tee -a /etc/security/limits.conf
        echo '* hard nproc 65536' | sudo tee -a /etc/security/limits.conf
    fi

    # Optimize kernel parameters for high-concurrency networking
    if [[ -f /etc/sysctl.conf ]]; then
        log_info "Optimizing kernel parameters..."
        echo 'net.core.somaxconn = 65536' | sudo tee -a /etc/sysctl.conf
        echo 'net.ipv4.tcp_max_syn_backlog = 65536' | sudo tee -a /etc/sysctl.conf
        echo 'net.core.netdev_max_backlog = 65536' | sudo tee -a /etc/sysctl.conf
        echo 'net.ipv4.tcp_fin_timeout = 30' | sudo tee -a /etc/sysctl.conf
        echo 'net.ipv4.tcp_keepalive_time = 300' | sudo tee -a /etc/sysctl.conf
        echo 'net.ipv4.tcp_keepalive_intvl = 60' | sudo tee -a /etc/sysctl.conf
        echo 'net.ipv4.tcp_keepalive_probes = 3' | sudo tee -a /etc/sysctl.conf
        sudo sysctl -p
    fi

    log_success "Performance optimizations applied"
}

# Main deployment function
main() {
    log_info "Starting GameV1 Alpha Release native deployment..."
    log_info "Target: ${DEPLOY_ENV} environment"
    log_info "Supports 1000+ concurrent players"

    # Pre-deployment checks
    check_prerequisites
    install_dependencies
    setup_environment

    # Core services setup
    setup_postgresql
    setup_redis
    build_services
    deploy_services

    # Infrastructure setup
    setup_load_balancer
    setup_monitoring
    optimize_performance

    # Final health check
    if health_check; then
        log_success "🎉 GameV1 Alpha Release deployment completed successfully!"
        log_info ""
        log_info "🌐 Access your game server:"
        log_info "   - Game Client: http://localhost (or your domain)"
        log_info "   - Admin Panel: http://localhost:3000/admin"
        log_info "   - Metrics: http://localhost:9090"
        log_info "   - Load Balancer: http://localhost (port 80/443)"
        log_info ""
        log_info "📊 Performance monitoring:"
        log_info "   - Active connections: curl http://localhost:9090/metrics | grep gamev1_connections_active"
        log_info "   - Matchmaking queues: curl http://localhost:9090/metrics | grep matchmaking"
        log_info "   - Database performance: curl http://localhost:9090/metrics | grep database"
        log_info ""
        log_info "🔧 Useful commands:"
        log_info "   - View logs: journalctl -u gamev1-gateway -f"
        log_info "   - Restart services: sudo systemctl restart gamev1-gateway gamev1-worker"
        log_info "   - Check status: sudo systemctl status gamev1-gateway"
        log_info "   - Update deployment: $0"
        log_info ""
        log_info "📖 For more information, see ALPHA-RELEASE-GUIDE.md"
    else
        log_error "❌ Deployment completed with errors. Check logs above."
        exit 1
    fi
}

# Handle script arguments
case "${1:-}" in
    "help"|"-h"|"--help")
        echo "GameV1 Alpha Release Native Deployment Script"
        echo ""
        echo "Usage: $0 [ENVIRONMENT]"
        echo ""
        echo "Environments:"
        echo "  development  - Development environment (default)"
        echo "  production   - Production environment"
        echo "  staging      - Staging environment"
        echo ""
        echo "Examples:"
        echo "  $0                    # Deploy to development"
        echo "  $0 production         # Deploy to production"
        echo "  DEPLOY_ENV=staging $0 # Deploy to staging"
        echo ""
        echo "For more information, see ALPHA-RELEASE-GUIDE.md"
        exit 0
        ;;
    "development"|"staging"|"production")
        DEPLOY_ENV="$1"
        ;;
    "")
        # Use default environment
        ;;
    *)
        log_error "Unknown environment: $1"
        log_info "Use 'development', 'staging', or 'production'"
        exit 1
        ;;
esac

# Run main deployment function
main