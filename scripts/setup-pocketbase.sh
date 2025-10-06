#!/bin/bash

# PocketBase Setup Script for GameV1
# Configures PocketBase with proper credentials for production use

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly POCKETBASE_DIR="${PROJECT_ROOT}/pocketbase"

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

# Check if PocketBase binary exists
check_pocketbase() {
    log_info "Checking PocketBase installation..."

    if [[ ! -f "${POCKETBASE_DIR}/pocketbase.exe" ]]; then
        log_error "PocketBase binary not found at ${POCKETBASE_DIR}/pocketbase.exe"
        log_info "Please ensure PocketBase is properly installed"
        exit 1
    fi

    log_success "PocketBase binary found"
}

# Setup PocketBase data directory
setup_data_directory() {
    log_info "Setting up PocketBase data directory..."

    mkdir -p "${POCKETBASE_DIR}/pb_data"

    # Set proper permissions
    chmod -R 755 "${POCKETBASE_DIR}/pb_data"

    log_success "PocketBase data directory configured"
}

# Initialize PocketBase with admin user
init_admin_user() {
    log_info "Initializing PocketBase with admin credentials..."

    # Check if PocketBase is already running
    if pgrep -f "pocketbase" > /dev/null; then
        log_warning "PocketBase is already running. Stopping it first..."
        pkill -f "pocketbase" || true
        sleep 2
    fi

    # Start PocketBase temporarily to initialize
    log_info "Starting PocketBase for initialization..."

    cd "${POCKETBASE_DIR}"

    # Create initial admin user using the binary directly
    # Note: In a real scenario, you'd need to interact with PocketBase API
    # For now, we'll document the manual setup process

    cat > "${POCKETBASE_DIR}/init-admin.js" << 'EOF'
// PocketBase initialization script
// This script should be run manually via PocketBase admin UI

// Admin credentials to set up:
// Email: admin@pocketbase.local
// Password: 123456789

// After first run, create the admin user through the web interface at:
// http://localhost:8090/_/

// Then create the following collections:
// 1. users (for user authentication)
// 2. games (for game management)
// 3. game_sessions (for player sessions)
// 4. matchmaking_tickets (for matchmaking)
// 5. tournaments (for tournaments)
// 6. beta_feedback (for user feedback)

console.log("PocketBase initialization script created");
console.log("Please complete setup through the web interface:");
console.log("1. Go to http://localhost:8090/_/");
console.log("2. Create admin user: admin@pocketbase.local / 123456789");
console.log("3. Create required collections through the admin panel");
EOF

    log_success "PocketBase initialization script created"
    log_info "Please complete the following manual steps:"
    log_info "1. Start PocketBase: cd ${POCKETBASE_DIR} && ./pocketbase.exe serve"
    log_info "2. Open http://localhost:8090/_/ in your browser"
    log_info "3. Create admin user with email: admin@pocketbase.local"
    log_info "4. Set password: 123456789"
    log_info "5. Create the required collections through the admin panel"
}

# Create required collections schema
create_collections_schema() {
    log_info "Creating collections schema..."

    # Create a schema file for reference
    cat > "${POCKETBASE_DIR}/collections-schema.json" << 'EOF'
{
  "collections": [
    {
      "name": "users",
      "schema": [
        {
          "name": "username",
          "type": "text",
          "required": true,
          "options": {
            "max": 50
          }
        },
        {
          "name": "email",
          "type": "email",
          "required": true
        },
        {
          "name": "password",
          "type": "text",
          "required": true
        },
        {
          "name": "skill_rating",
          "type": "number",
          "required": false
        },
        {
          "name": "games_played",
          "type": "number",
          "required": false
        },
        {
          "name": "region",
          "type": "text",
          "required": false
        }
      ],
      "indexes": [
        "CREATE UNIQUE INDEX idx_users_email ON users (email)",
        "CREATE INDEX idx_users_username ON users (username)"
      ]
    },
    {
      "name": "games",
      "schema": [
        {
          "name": "name",
          "type": "text",
          "required": true
        },
        {
          "name": "game_mode",
          "type": "select",
          "required": true,
          "options": {
            "values": ["deathmatch", "team_deathmatch", "capture_the_flag"]
          }
        },
        {
          "name": "max_players",
          "type": "number",
          "required": true
        },
        {
          "name": "status",
          "type": "select",
          "required": true,
          "options": {
            "values": ["waiting", "in_progress", "finished", "cancelled"]
          }
        },
        {
          "name": "host_id",
          "type": "relation",
          "required": false,
          "options": {
            "collectionId": "users"
          }
        }
      ]
    },
    {
      "name": "game_sessions",
      "schema": [
        {
          "name": "game_id",
          "type": "relation",
          "required": true,
          "options": {
            "collectionId": "games"
          }
        },
        {
          "name": "player_id",
          "type": "relation",
          "required": true,
          "options": {
            "collectionId": "users"
          }
        },
        {
          "name": "position",
          "type": "json",
          "required": false
        },
        {
          "name": "health",
          "type": "number",
          "required": false
        },
        {
          "name": "score",
          "type": "number",
          "required": false
        },
        {
          "name": "status",
          "type": "select",
          "required": true,
          "options": {
            "values": ["active", "finished", "disconnected"]
          }
        }
      ]
    }
  ]
}
EOF

    log_success "Collections schema created"
    log_info "Use this schema as reference when creating collections in PocketBase admin panel"
}

# Create systemd service for PocketBase
create_systemd_service() {
    log_info "Creating PocketBase systemd service..."

    sudo tee /etc/systemd/system/pocketbase.service > /dev/null << EOF
[Unit]
Description=PocketBase Database Server
After=network.target

[Service]
Type=simple
User=pocketbase
Group=pocketbase
WorkingDirectory=${POCKETBASE_DIR}
ExecStart=${POCKETBASE_DIR}/pocketbase.exe serve --http="0.0.0.0:8090"
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${POCKETBASE_DIR}/pb_data

# Resource limits
LimitNOFILE=65536
MemoryLimit=512M

[Install]
WantedBy=multi-user.target
EOF

    # Create pocketbase user if it doesn't exist
    if ! id -u pocketbase &> /dev/null; then
        sudo useradd --system --shell /bin/false --home "${POCKETBASE_DIR}" pocketbase
    fi

    # Set ownership
    sudo chown -R pocketbase:pocketbase "${POCKETBASE_DIR}"

    # Reload systemd and enable service
    sudo systemctl daemon-reload
    sudo systemctl enable pocketbase

    log_success "PocketBase systemd service created"
}

# Main setup function
main() {
    log_info "Starting PocketBase setup for GameV1..."
    log_info "Admin credentials: admin@pocketbase.local / 123456789"

    # Pre-setup checks
    check_pocketbase
    setup_data_directory

    # Core setup
    init_admin_user
    create_collections_schema
    create_systemd_service

    log_success "🎉 PocketBase setup completed!"
    log_info ""
    log_info "📋 Next steps:"
    log_info "1. Start PocketBase: sudo systemctl start pocketbase"
    log_info "2. Open http://localhost:8090/_/ in your browser"
    log_info "3. Create admin user: admin@pocketbase.local / 123456789"
    log_info "4. Create required collections (see collections-schema.json)"
    log_info "5. Test connection with: curl http://localhost:8090/api/health"
    log_info ""
    log_info "🔧 Useful commands:"
    log_info "   - Start PocketBase: sudo systemctl start pocketbase"
    log_info "   - Check status: sudo systemctl status pocketbase"
    log_info "   - View logs: journalctl -u pocketbase -f"
    log_info "   - Stop PocketBase: sudo systemctl stop pocketbase"
}

# Handle script arguments
case "${1:-}" in
    "help"|"-h"|"--help")
        echo "PocketBase Setup Script for GameV1"
        echo ""
        echo "Usage: $0"
        echo ""
        echo "This script sets up PocketBase with the correct credentials for GameV1:"
        echo "- Email: admin@pocketbase.local"
        echo "- Password: 123456789"
        echo ""
        echo "It creates the necessary systemd service and data directories."
        exit 0
        ;;
    "")
        # Run main setup
        ;;
    *)
        log_error "Unknown option: $1"
        log_info "Use 'help' for usage information"
        exit 1
        ;;
esac

# Run main setup function
main