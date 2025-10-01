#!/usr/bin/env bash
set -euo pipefail

POCKETBASE_ADDR=${POCKETBASE_ADDR:-127.0.0.1:8090}
WORKER_ADDR=${WORKER_ADDR:-127.0.0.1:50051}
GATEWAY_ADDR=${GATEWAY_ADDR:-0.0.0.0:8080}

echo "Starting PocketBase (Admin UI at $POCKETBASE_ADDR)"
POCKETBASE_PATH="pocketbase/pocketbase"
if [ -f "$POCKETBASE_PATH" ]; then
    ./$POCKETBASE_PATH serve &
    echo "  PocketBase started successfully"
else
    echo "  WARNING: PocketBase binary not found at $POCKETBASE_PATH"
    echo "  Run: ./scripts/setup-pocketbase.sh"
fi
sleep 2

echo "Starting worker (gRPC at $WORKER_ADDR)"
export WORKER_RPC_ADDR="$WORKER_ADDR"
cargo run -p worker &
sleep 1

echo "Starting gateway (HTTP at $GATEWAY_ADDR)"
export WORKER_GRPC_URI="http://$WORKER_ADDR"
cargo run -p gateway &
sleep 1

echo "All services launched. Access points:"
echo "  PocketBase Admin: http://$POCKETBASE_ADDR/_/"
echo "  Gateway API: http://$GATEWAY_ADDR"
echo "  Health check: http://$GATEWAY_ADDR/healthz"
echo "  Metrics: http://$GATEWAY_ADDR/metrics"
echo "  WebSocket: ws://$GATEWAY_ADDR/ws"
echo ""
echo "Press Ctrl+C to stop all services"

# Wait for all background processes
wait
