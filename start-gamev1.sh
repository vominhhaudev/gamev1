#!/bin/bash

# GameV1 Complete Startup Script
# Starts all services in the correct order with proper configuration
# NO DOCKER DEPENDENCIES - Native deployment only

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}" && pwd)"

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

# Check if running as root (needed for systemctl)
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        log_info "Example: sudo $0"
        exit 1
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check if PocketBase is set up
    if [[ ! -f "${PROJECT_ROOT}/pocketbase/pocketbase.exe" ]]; then
        log_error "PocketBase not found. Please run setup script first:"
        log_info "  ./scripts/setup-pocketbase.sh"
        exit 1
    fi

    # Check if services are built
    if [[ ! -f "${PROJECT_ROOT}/target/release/gateway" ]]; then
        log_warning "Services not built. Building now..."
        cd "${PROJECT_ROOT}"
        cargo build --release
    fi

    # Check if client is built
    if [[ ! -d "${PROJECT_ROOT}/client/dist" ]]; then
        log_warning "Client not built. Building now..."
        cd "${PROJECT_ROOT}/client"
        npm install
        npm run build
    fi

    log_success "Prerequisites check completed"
}

# Setup PocketBase if needed
setup_pocketbase() {
    log_info "Setting up PocketBase..."

    # Start PocketBase service
    if ! systemctl is-active --quiet pocketbase; then
        log_info "Starting PocketBase service..."
        systemctl start pocketbase

        # Wait for PocketBase to be ready
        log_info "Waiting for PocketBase to be ready..."
        timeout=60
        while ! curl -f http://localhost:8090/api/health &> /dev/null; do
            sleep 2
            timeout=$((timeout - 2))
            if [[ $timeout -le 0 ]]; then
                log_warning "PocketBase startup timeout - continuing anyway"
                break
            fi
        done
    else
        log_info "PocketBase already running"
    fi

    log_success "PocketBase setup completed"
}

# Start core services
start_core_services() {
    log_info "Starting core GameV1 services..."

    # Start services in correct order
    local services=("gamev1-gateway" "gamev1-worker" "gamev1-room-manager")

    for service in "${services[@]}"; do
        log_info "Starting ${service}..."

        if systemctl is-active --quiet "${service}"; then
            log_info "${service} already running"
        else
            if systemctl start "${service}"; then
                log_success "Started ${service}"
            else
                log_error "Failed to start ${service}"
                log_info "Check logs: journalctl -u ${service} -n 50"
            fi
        fi
    done

    log_success "Core services startup completed"
}

# Start load balancer
start_load_balancer() {
    log_info "Starting load balancer..."

    # Check if Nginx is available
    if command -v nginx &> /dev/null; then
        if systemctl is-active --quiet nginx; then
            log_info "Nginx already running"
        else
            if systemctl start nginx; then
                log_success "Started Nginx load balancer"
            else
                log_error "Failed to start Nginx"
            fi
        fi
    else
        log_warning "Nginx not installed - skipping load balancer"
    fi
}

# Start monitoring
start_monitoring() {
    log_info "Starting monitoring services..."

    # Start Prometheus if available
    if command -v prometheus &> /dev/null; then
        if systemctl is-active --quiet prometheus; then
            log_info "Prometheus already running"
        else
            systemctl start prometheus || log_warning "Failed to start Prometheus"
        fi
    fi

    # Start Grafana if available
    if command -v grafana-server &> /dev/null; then
        if systemctl is-active --quiet grafana-server; then
            log_info "Grafana already running"
        else
            systemctl start grafana-server || log_warning "Failed to start Grafana"
        fi
    fi

    log_success "Monitoring startup completed"
}

# Health check all services
health_check() {
    log_info "Performing health checks..."

    local all_healthy=true

    # Check PocketBase
    if curl -f --max-time 5 http://localhost:8090/api/health &> /dev/null; then
        log_success "PocketBase is healthy"
    else
        log_error "PocketBase health check failed"
        all_healthy=false
    fi

    # Check Gateway
    if curl -f --max-time 5 http://localhost:3000/healthz &> /dev/null; then
        log_success "Gateway is healthy"
    else
        log_error "Gateway health check failed"
        all_healthy=false
    fi

    # Check Worker
    if curl -f --max-time 5 http://localhost:50051/health &> /dev/null; then
        log_success "Worker is healthy"
    else
        log_warning "Worker health check failed"
    fi

    if [[ "${all_healthy}" == "true" ]]; then
        log_success "All health checks passed"
        return 0
    else
        log_error "Some health checks failed"
        return 1
    fi
}

# Display status
display_status() {
    log_info "GameV1 System Status:"
    log_info ""

    # Show service status
    log_info "📊 Services Status:"
    systemctl status gamev1-gateway --no-pager -l || true
    systemctl status gamev1-worker --no-pager -l || true
    systemctl status gamev1-room-manager --no-pager -l || true
    systemctl status pocketbase --no-pager -l || true

    log_info ""
    log_info "🌐 Access URLs:"
    log_info "   - Game Client: http://localhost:5173"
    log_info "   - Admin Panel: http://localhost:3000/admin"
    log_info "   - PocketBase Admin: http://localhost:8090/_/"
    log_info "   - API Gateway: http://localhost:3000"
    log_info "   - Health Check: http://localhost:3000/healthz"
    log_info "   - Metrics: http://localhost:9090/metrics"

    log_info ""
    log_info "🔑 Login Credentials:"
    log_info "   - PocketBase Admin: admin@pocketbase.local / 123456789"

    log_info ""
    log_info "🔧 Useful Commands:"
    log_info "   - View logs: journalctl -u gamev1-gateway -f"
    log_info "   - Restart all: sudo systemctl restart gamev1-gateway gamev1-worker gamev1-room-manager"
    log_info "   - Stop all: sudo systemctl stop gamev1-gateway gamev1-worker gamev1-room-manager"
    log_info "   - Check connections: curl http://localhost:9090/metrics | grep connections"
}

# Main startup function
main() {
    log_info "🚀 Starting GameV1 Complete System..."
    log_info "Using PocketBase with credentials: admin@pocketbase.local / 123456789"
    log_info "NO DOCKER DEPENDENCIES - Native deployment"

    # Pre-startup checks
    check_root
    check_prerequisites

    # Core setup and startup
    setup_pocketbase
    start_core_services
    start_load_balancer
    start_monitoring

    # Final verification
    if health_check; then
        log_success "🎉 GameV1 startup completed successfully!"
        log_info ""
        display_status
    else
        log_error "❌ Startup completed with errors. Check logs above."
        log_info "Try running individual services manually to debug:"
        log_info "  sudo systemctl start pocketbase"
        log_info "  sudo systemctl start gamev1-gateway"
        log_info "  sudo systemctl start gamev1-worker"
        exit 1
    fi
}

# Handle script arguments
case "${1:-}" in
    "help"|"-h"|"--help")
        echo "GameV1 Complete Startup Script"
        echo ""
        echo "Usage: sudo $0"
        echo ""
        echo "This script starts all GameV1 services in the correct order:"
        echo "1. PocketBase database (with your credentials)"
        echo "2. Core services (Gateway, Worker, Room Manager)"
        echo "3. Load balancer (Nginx)"
        echo "4. Monitoring services"
        echo ""
        echo "Requirements:"
        echo "- Must be run as root (sudo)"
        echo "- PocketBase must be set up first"
        echo "- All services must be properly configured"
        echo ""
        echo "After startup, access your game at:"
        echo "- Game Client: http://localhost:5173"
        echo "- Admin Panel: http://localhost:3000/admin"
        echo ""
        echo "Login: admin@pocketbase.local / 123456789"
        exit 0
        ;;
    "status")
        check_root
        display_status
        exit 0
        ;;
    "stop")
        check_root
        log_info "Stopping all GameV1 services..."
        sudo systemctl stop gamev1-gateway gamev1-worker gamev1-room-manager pocketbase || true
        log_success "All services stopped"
        exit 0
        ;;
    "restart")
        check_root
        log_info "Restarting all GameV1 services..."
        sudo systemctl restart gamev1-gateway gamev1-worker gamev1-room-manager pocketbase || true
        sleep 3
        health_check && log_success "All services restarted successfully" || log_error "Some services failed to restart"
        exit 0
        ;;
    "")
        # Run main startup
        ;;
    *)
        log_error "Unknown option: $1"
        log_info "Use 'help' for usage information"
        exit 1
        ;;
esac

# Run main startup function
main
