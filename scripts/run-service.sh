#!/usr/bin/env bash
set -euo pipefail

SERVICE=${1:?usage: $0 <gateway|worker|room-manager>}
GATEWAY_BIND=${GATEWAY_BIND_ADDR:-127.0.0.1:8080}
WORKER_RPC=${WORKER_RPC_ADDR:-127.0.0.1:50051}
WORKER_METRICS=${WORKER_METRICS_ADDR:-127.0.0.1:3100}
ROOMMGR_METRICS=${ROOM_MANAGER_METRICS_ADDR:-127.0.0.1:3200}

case "$SERVICE" in
  gateway)
    export GATEWAY_BIND_ADDR="$GATEWAY_BIND"
    export WORKER_ENDPOINT="http://$WORKER_RPC"
    exec cargo run -p gateway
    ;;
  worker)
    export WORKER_RPC_ADDR="$WORKER_RPC"
    export WORKER_METRICS_ADDR="$WORKER_METRICS"
    exec cargo run -p worker
    ;;
  room-manager)
    export ROOM_MANAGER_METRICS_ADDR="$ROOMMGR_METRICS"
    exec cargo run -p room-manager
    ;;
  *)
    echo "unknown service: $SERVICE" >&2; exit 1;
    ;;
esac

