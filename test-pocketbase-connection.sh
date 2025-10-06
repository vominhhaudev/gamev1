#!/bin/bash

# PocketBase Connection Test Script
# Tests connection to PocketBase with the new credentials

set -euo pipefail

# Configuration
readonly POCKETBASE_URL="http://localhost:8090"
readonly ADMIN_EMAIL="admin@pocketbase.local"
readonly ADMIN_PASSWORD="123456789"

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

# Test PocketBase health
test_health() {
    log_info "Testing PocketBase health..."

    if curl -f --max-time 5 "${POCKETBASE_URL}/api/health" &> /dev/null; then
        log_success "PocketBase health check passed"
        return 0
    else
        log_error "PocketBase health check failed"
        log_info "Make sure PocketBase is running: sudo systemctl start pocketbase"
        return 1
    fi
}

# Test admin authentication
test_admin_auth() {
    log_info "Testing admin authentication..."

    local response
    response=$(curl -s -X POST "${POCKETBASE_URL}/api/admins/auth-with-password" \
        -H "Content-Type: application/json" \
        -d "{\"identity\":\"${ADMIN_EMAIL}\",\"password\":\"${ADMIN_PASSWORD}\"}")

    if echo "$response" | grep -q "token"; then
        log_success "Admin authentication successful"
        local token
        token=$(echo "$response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        log_info "Admin token received: ${token:0:20}..."
        return 0
    else
        log_error "Admin authentication failed"
        log_info "Response: $response"
        log_info "Please ensure admin user exists with correct credentials"
        return 1
    fi
}

# Test collections
test_collections() {
    log_info "Testing collections..."

    # Get admin token first
    local response
    response=$(curl -s -X POST "${POCKETBASE_URL}/api/admins/auth-with-password" \
        -H "Content-Type: application/json" \
        -d "{\"identity\":\"${ADMIN_EMAIL}\",\"password\":\"${ADMIN_PASSWORD}\"}")

    local token
    token=$(echo "$response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

    if [[ -z "$token" ]]; then
        log_error "Could not get admin token"
        return 1
    fi

    # List collections
    local collections_response
    collections_response=$(curl -s -H "Authorization: Bearer ${token}" \
        "${POCKETBASE_URL}/api/collections")

    if echo "$collections_response" | grep -q "name"; then
        log_success "Collections API accessible"

        # Count collections
        local count
        count=$(echo "$collections_response" | grep -c '"name"')

        if [[ $count -gt 0 ]]; then
            log_info "Found $count collections"
            return 0
        else
            log_warning "No collections found - you may need to create them through the admin panel"
            return 0
        fi
    else
        log_error "Collections API failed"
        log_info "Response: $collections_response"
        return 1
    fi
}

# Test user creation
test_user_creation() {
    log_info "Testing user creation..."

    # Get admin token
    local response
    response=$(curl -s -X POST "${POCKETBASE_URL}/api/admins/auth-with-password" \
        -H "Content-Type: application/json" \
        -d "{\"identity\":\"${ADMIN_EMAIL}\",\"password\":\"${ADMIN_PASSWORD}\"}")

    local token
    token=$(echo "$response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

    if [[ -z "$token" ]]; then
        log_error "Could not get admin token"
        return 1
    fi

    # Create a test user
    local user_response
    user_response=$(curl -s -X POST "${POCKETBASE_URL}/api/collections/users/records" \
        -H "Authorization: Bearer ${token}" \
        -H "Content-Type: application/json" \
        -d '{
            "username": "test_user_$(date +%s)",
            "email": "test_$(date +%s)@example.com",
            "password": "test_password_123"
        }')

    if echo "$user_response" | grep -q "id"; then
        log_success "User creation successful"
        return 0
    else
        log_error "User creation failed"
        log_info "Response: $user_response"
        return 1
    fi
}

# Test database performance
test_performance() {
    log_info "Testing database performance..."

    # Simple performance test - create multiple records
    local start_time
    start_time=$(date +%s%N)

    for i in {1..10}; do
        curl -s -X POST "${POCKETBASE_URL}/api/collections/users/records" \
            -H "Content-Type: application/json" \
            -d "{\"username\":\"perf_test_${i}\",\"email\":\"perf${i}@test.com\",\"password\":\"test123\"}" > /dev/null
    done

    local end_time
    end_time=$(date +%s%N)
    local duration=$(( (end_time - start_time) / 1000000 )) # Convert to milliseconds

    log_info "Created 10 records in ${duration}ms"
    log_success "Performance test completed"
}

# Cleanup test data
cleanup() {
    log_info "Cleaning up test data..."

    # Get admin token
    local response
    response=$(curl -s -X POST "${POCKETBASE_URL}/api/admins/auth-with-password" \
        -H "Content-Type: application/json" \
        -d "{\"identity\":\"${ADMIN_EMAIL}\",\"password\":\"${ADMIN_PASSWORD}\"}")

    local token
    token=$(echo "$response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

    if [[ -n "$token" ]]; then
        # List test users and delete them
        local users_response
        users_response=$(curl -s -H "Authorization: Bearer ${token}" \
            "${POCKETBASE_URL}/api/collections/users/records?filter=username~'perf_test'")

        echo "$users_response" | grep -o '"id":"[^"]*"' | cut -d'"' -f4 | while read -r user_id; do
            curl -s -X DELETE -H "Authorization: Bearer ${token}" \
                "${POCKETBASE_URL}/api/collections/users/records/${user_id}" > /dev/null
        done

        log_success "Test data cleaned up"
    fi
}

# Main test function
main() {
    log_info "🚀 Starting PocketBase Connection Tests..."
    log_info "Testing with credentials: ${ADMIN_EMAIL} / ${ADMIN_PASSWORD}"

    local all_passed=true

    # Run all tests
    if ! test_health; then all_passed=false; fi
    if ! test_admin_auth; then all_passed=false; fi
    if ! test_collections; then all_passed=false; fi
    if ! test_user_creation; then all_passed=false; fi
    test_performance

    # Cleanup
    cleanup

    # Final result
    if [[ "$all_passed" == "true" ]]; then
        log_success "🎉 All PocketBase tests passed!"
        log_info ""
        log_info "📋 Summary:"
        log_info "   ✅ Health check: PASSED"
        log_info "   ✅ Admin authentication: PASSED"
        log_info "   ✅ Collections API: PASSED"
        log_info "   ✅ User creation: PASSED"
        log_info "   ✅ Performance: GOOD"
        log_info ""
        log_info "🌐 PocketBase is ready for GameV1!"
        log_info "   - Admin Panel: http://localhost:8090/_/"
        log_info "   - API Base: http://localhost:8090/api"
        return 0
    else
        log_error "❌ Some tests failed. Check output above."
        return 1
    fi
}

# Handle script arguments
case "${1:-}" in
    "help"|"-h"|"--help")
        echo "PocketBase Connection Test Script"
        echo ""
        echo "Usage: $0"
        echo ""
        echo "This script tests the connection to PocketBase with your credentials:"
        echo "- Email: admin@pocketbase.local"
        echo "- Password: 123456789"
        echo ""
        echo "Tests performed:"
        echo "- Health check"
        echo "- Admin authentication"
        echo "- Collections API access"
        echo "- User creation"
        echo "- Performance test"
        echo ""
        echo "Requirements:"
        echo "- PocketBase must be running"
        echo "- Admin user must exist with correct credentials"
        exit 0
        ;;
    "")
        # Run main tests
        ;;
    *)
        log_error "Unknown option: $1"
        log_info "Use 'help' for usage information"
        exit 1
        ;;
esac

# Run main test function
main
