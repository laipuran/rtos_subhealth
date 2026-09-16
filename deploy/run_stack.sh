#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-server}"
case "$MODE" in
  server)
    exec cargo run -p gateway
    ;;
  endpoint)
    : "${DEVICE_TYPE:?usage: deploy/run_stack.sh endpoint DEVICE_TYPE=<device-type>}"
    echo "endpoint runtime is selected by DEVICE_TYPE=${DEVICE_TYPE}"
    ;;
  *)
    echo "usage: deploy/run_stack.sh server|endpoint" >&2
    exit 2
    ;;
esac
