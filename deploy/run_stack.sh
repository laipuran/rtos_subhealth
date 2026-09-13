#!/usr/bin/env bash
# Run the orchestrator plus one device adapter.
#
# Prereqs: source ROS and the built workspaces first, e.g.
#   source /opt/ros/jazzy/setup.bash
#   source "$ROS_RUST_WS/install/setup.bash"
#   source "$ROS_RUST_WS/install_nodes/setup.bash"
#
# Environment:
#   DEVICE_ID    (default: mock)
#   DEVICE_TYPE  mock | diff_drive | tonypi (default: mock)
#   TONYPI_RPC_URL (only for DEVICE_TYPE=tonypi)
set -eo pipefail

DEVICE_ID="${DEVICE_ID:-mock}"
DEVICE_TYPE="${DEVICE_TYPE:-mock}"
export DEVICE_ID DEVICE_TYPE

ros2 run adapter adapter &
ADAPTER_PID=$!
ros2 run orchestrator orchestrator &
ORCH_PID=$!

cleanup() { kill "$ADAPTER_PID" "$ORCH_PID" 2>/dev/null || true; }
trap cleanup INT TERM EXIT

echo "[run] adapter DEVICE_ID=$DEVICE_ID DEVICE_TYPE=$DEVICE_TYPE, orchestrator"
wait
