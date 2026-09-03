#!/bin/bash
# setup.sh — 一键配置 ROS2 环境与 DDS
# 用法: source setup.sh
# 首次使用前需要 colcon build

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- ROS2 基础环境 ---
source /opt/ros/foxy/setup.bash

# --- 本项目 workspace ---
if [ -f "$SCRIPT_DIR/install/setup.bash" ]; then
    source "$SCRIPT_DIR/install/setup.bash"
else
    echo "[WARN] install/setup.bash not found. Run 'colcon build --symlink-install' first."
fi

# --- DDS 配置: CycloneDDS (与 Unitree MuJoCo bridge 互通) ---
if [ -d "$HOME/unitree_ros2/cyclonedds_ws/install" ]; then
    source "$HOME/unitree_ros2/cyclonedds_ws/install/setup.bash"
fi
export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp
export ROS_DOMAIN_ID=1

# 覆盖 unitree 默认的 enp3s0 配置，绑定到 lo 网卡
export CYCLONEDDS_URI='<CycloneDDS><Domain><General><NetworkInterfaceAddress>lo</NetworkInterfaceAddress></General></Domain></CycloneDDS>'
unset ROS_LOCALHOST_ONLY  # CycloneDDS 下由 CYCLONEDDS_URI 接管

# --- 日志格式 ---
export RCUTILS_CONSOLE_OUTPUT_FORMAT='[{severity}][{name}]: {message}'

echo "[OK] ROS2 DDS: $RMW_IMPLEMENTATION, domain=$ROS_DOMAIN_ID"
