# Makefile — ROS2 Foxy 容器内操作入口
# 宿主侧: 进入容器 `./docker/dev.sh`（挂载 ros2_ws → /workspace）
# 容器内: 使用本文件的目标，如 `make build`、`make run backend=mock`

.DEFAULT_GOAL := help
.PHONY: help build run clean

# 运行后端，默认 mock（可覆盖: make run backend=sim）
backend ?= mock

help: ## 显示可用命令
	@echo "ROS2 Foxy 容器内操作："
	@echo "  make build                           编译 (colcon build --symlink-install)"
	@echo "  make run backend=mock|sim|real       运行后端 (默认 mock)"
	@echo "  make clean                           清理 build/ install/ log/"

build: ## 编译
	colcon build --symlink-install

run: ## 运行后端: make run backend=mock|sim|real
	@case "$(backend)" in \
	  mock) : ;; \
	  sim)  test -d "$$HOME/unitree_mujoco"      || { echo "sim 需挂载 unitree_sdk2_python / unitree_mujoco（暂未容器化），仅 mock 可用"; exit 1; } ;; \
	  real) test -d "$$HOME/unitree_sdk2_python" || { echo "real 需挂载 Unitree SDK（暂未容器化），仅 mock 可用"; exit 1; } ;; \
	  *) echo "backend 必须是 mock|sim|real (你给的是: $(backend))"; exit 1 ;; \
	esac
	./run.sh "$(backend)"

clean: ## 清理编译产物
	rm -rf build install log
