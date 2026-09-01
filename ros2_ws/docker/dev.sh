#!/bin/bash
# dev.sh — 宿主侧入口：起一次性容器并进入交互 bash（挂载 ros2_ws → /workspace）
#
# 容器内使用 make（见 /workspace/Makefile）：
#   make build                    编译
#   make run backend=mock|sim|real  运行后端
#   make clean                    清理
#
# 镜像重装（仅当 Dockerfile 依赖变化时）：cd docker && docker compose build

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v docker >/dev/null 2>&1; then
    echo "[dev.sh] 未找到 Docker，请先安装 Docker" >&2
    exit 1
fi

cd "$SCRIPT_DIR"
exec docker compose run --rm app bash "$@"
