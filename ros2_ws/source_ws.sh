#!/bin/bash
# source_ws.sh — source workspace and fix pkg_resources
source /opt/ros/foxy/setup.bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -f "$SCRIPT_DIR/install/setup.bash" ]; then
    source "$SCRIPT_DIR/install/setup.bash"
fi

# Fix pkg_resources by pointing to workspace packages
export PYTHONPATH="$SCRIPT_DIR/install/exec_layer/lib/python3.8/site-packages:$SCRIPT_DIR/install/desc_layer/lib/python3.8/site-packages:$PYTHONPATH"
