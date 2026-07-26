#!/bin/bash
# run.sh — 一键启动 ROS2 后端
# 用法: ./run.sh <backend>
#   backend: mock | sim | real
#
# 示例:
#   ./run.sh mock     # 纯软件模拟，测 WebUI 链路
#   ./run.sh sim      # MuJoCo 仿真，看 GO2 走路
#   ./run.sh real     # 真机 GO2 控制

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR" || exit 1

if [ $# -lt 1 ]; then
    echo "Usage: $0 <mock|sim|real>"
    exit 1
fi

BACKEND="$1"
if [[ "$BACKEND" != "mock" && "$BACKEND" != "sim" && "$BACKEND" != "real" ]]; then
    echo "Error: backend must be mock, sim, or real"
    exit 1
fi

# 环境 + DDS
source setup.sh

# 启动 ROS2
ros2 launch desc_layer run.launch.py "backend:=$BACKEND"
