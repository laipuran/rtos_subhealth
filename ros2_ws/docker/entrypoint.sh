#!/bin/bash
# entrypoint.sh — 容器内统一入口：加载 ROS 环境后执行命令（默认进入 bash）
# 容器内操作请使用 make（见 /workspace/Makefile）。

# ROS2 基础环境（让 colcon / ros2 / ament 在后续 bash/make 中可用）
source /opt/ros/foxy/setup.bash

# 兼容宿主机无 unitree 相关目录（mock 不依赖，存在才 source）
for f in \
    "$HOME/unitree_ros2/cyclonedds_ws/install/setup.bash"; do
    if [ -f "$f" ]; then
        source "$f"
        echo "[entrypoint] sourced $f"
    fi
done

CMD="${1:-bash}"
shift || true

exec "$CMD" "$@"
