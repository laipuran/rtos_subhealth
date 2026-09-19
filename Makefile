# ROS Subhealth — one entry point for every routine task.
#
# Canonical workflow: WebUI, server/endpoint runtime, and ROS image selection.
#
#   make help

SHELL := /bin/bash

COMPOSE   := docker compose -f docker/dev/compose.yaml
DEV        := $(COMPOSE) run --rm dev
# Make decides at startup whether it is already inside Docker.
IN_CONTAINER := $(if $(wildcard /.dockerenv),1,0)

# Overridable settings.
GATEWAY_HTTP_PORT ?= 5000
ROS_BUILD_ROOT ?= /ws/$(ROS_DISTRO)
ENDPOINT_ARGS ?=
ROS_TASK_MANIFEST := ros2_ws/src/control_plane/ros_task_client/Cargo.toml
ROS_TASK_INTEGRATION := ros2_ws/src/control_plane/ros_task_client/test/run_mock_exec_integration.sh
# Leave empty for an isolated target per task-test invocation. Set this when
# deliberately reusing a known-good generated ROS build directory.
ROS_TASK_CARGO_TARGET ?=

.DEFAULT_GOAL := help

.PHONY: help humble jazzy build ros-test ros-task-test ros-task-integration \
        fmt lint check webui webui-dev run server endpoint clean

## help: list available targets
help:
	@echo "ROS Subhealth targets:"
	@echo
	@echo "  make humble          Build the Ubuntu 22.04 + ROS Humble image and enter it"
	@echo "  make jazzy           Build the Ubuntu 24.04 + ROS Jazzy image and enter it"
	@echo "  make build           Build Rust, plus ROS packages in the container"
	@echo "  make ros-test        Build and run ROS package tests"
	@echo "  make ros-task-test   Run ros_task_client unit tests"
	@echo "  make ros-task-integration  Run the explicit mock Exec integration"
	@echo "  make fmt             Format the Rust workspace"
	@echo "  make lint            Check formatting and run clippy"
	@echo "  make check           Rust lint + build + ROS tests"
	@echo "  make webui           Install deps and build the WebUI"
	@echo "  make webui-dev       Run the WebUI dev server"
	@echo "  make run server      Run the control-plane server"
	@echo "  make run endpoint DEVICE_TYPE=<device-type>"
	@echo "  make clean           Remove build artifacts"
	@echo
	@echo "  endpoint requires DEVICE_TYPE; server and endpoint are mutually exclusive"

## humble: build the Humble image and enter an interactive development container
humble:
	ROS_DISTRO=humble UBUNTU_VERSION=22.04 $(COMPOSE) build --build-arg ROS_DISTRO=humble --build-arg UBUNTU_VERSION=22.04
	ROS_DISTRO=humble UBUNTU_VERSION=22.04 $(COMPOSE) run --rm dev bash

## jazzy: build the Jazzy image and enter an interactive development container
jazzy:
	ROS_DISTRO=jazzy UBUNTU_VERSION=24.04 $(COMPOSE) build --build-arg ROS_DISTRO=jazzy --build-arg UBUNTU_VERSION=24.04
	ROS_DISTRO=jazzy UBUNTU_VERSION=24.04 $(COMPOSE) run --rm dev bash

## build: Rust workspace build, plus ROS packages in the container
build:
ifeq ($(IN_CONTAINER),1)
	source /opt/ros/$(ROS_DISTRO)/setup.bash && colcon --log-base $(ROS_BUILD_ROOT)/log build --merge-install --base-paths ros2_ws/src --build-base $(ROS_BUILD_ROOT)/build/merged-symlink --install-base $(ROS_BUILD_ROOT)/install --symlink-install
	@build_dir="$$(mktemp -d "$${TMPDIR:-/tmp}/ros-subhealth-build.XXXXXX")"; \
		trap 'rm -rf -- "$$build_dir"' EXIT; \
		cd "$$build_dir" && source /opt/ros/$(ROS_DISTRO)/setup.bash && source "$(ROS_BUILD_ROOT)/install/setup.bash" && cargo build --manifest-path "$(CURDIR)/Cargo.toml" --workspace
else
	cargo build --workspace
endif

## ros-test: build and run all ROS package tests
ros-test: build
ifeq ($(IN_CONTAINER),1)
	cd ros2_ws && source /opt/ros/$(ROS_DISTRO)/setup.bash && colcon --log-base $(ROS_BUILD_ROOT)/log test --merge-install --build-base $(ROS_BUILD_ROOT)/build/merged-symlink --install-base $(ROS_BUILD_ROOT)/install
	colcon test-result --test-result-base $(ROS_BUILD_ROOT)/build/merged-symlink --verbose
else
	$(DEV) make ros-test
endif

## ros-task-test: run the typed Rust task-client unit tests
ros-task-test: build
ifeq ($(IN_CONTAINER),1)
	@target_dir="$(if $(strip $(ROS_TASK_CARGO_TARGET)),$(ROS_TASK_CARGO_TARGET),$$(mktemp -d "$${TMPDIR:-/tmp}/ros-task-client-test.XXXXXX"))"; \
	 cleanup_target=0; \
	 if [ -z "$(strip $(ROS_TASK_CARGO_TARGET))" ]; then cleanup_target=1; fi; \
	 trap 'if [ "$$cleanup_target" -eq 1 ]; then rm -rf -- "$$target_dir"; fi' EXIT; \
	source /opt/ros/$(ROS_DISTRO)/setup.bash && source "$(ROS_BUILD_ROOT)/install/setup.bash" && CARGO_TARGET_DIR="$$target_dir" cargo test --manifest-path $(ROS_TASK_MANIFEST)
else
	$(DEV) make ros-task-test
endif

## ros-task-integration: run the real ROS mock Exec integration explicitly
ros-task-integration: build
ifeq ($(IN_CONTAINER),1)
	ROS_BUILD_ROOT="$(ROS_BUILD_ROOT)" $(if $(strip $(ROS_TASK_CARGO_TARGET)),CARGO_TARGET_DIR="$(ROS_TASK_CARGO_TARGET)") "$(ROS_TASK_INTEGRATION)"
else
	$(DEV) make ros-task-integration
endif

## fmt: format the Rust workspace
fmt:
	cargo fmt --all

## lint: formatting + clippy
lint:
	cargo fmt --all --check
	@lint_dir="$$(mktemp -d "$${TMPDIR:-/tmp}/ros-subhealth-lint.XXXXXX")"; \
		trap 'rm -rf -- "$$lint_dir"' EXIT; \
		cd "$$lint_dir" && cargo clippy --manifest-path "$(CURDIR)/Cargo.toml" --all-targets -- -D warnings

## check: lint + build + ROS tests
check: ros-test ros-task-test lint

## webui: install deps and build the WebUI
webui:
ifeq ($(IN_CONTAINER),1)
	cd webui && pnpm install --frozen-lockfile && pnpm build
else
	$(DEV) make webui
endif

## webui-dev: run the WebUI dev server
webui-dev:
ifeq ($(IN_CONTAINER),1)
	cd webui && pnpm install --frozen-lockfile && pnpm dev
else
	$(DEV) make webui-dev
endif

## run: dispatch to exactly one runtime mode
run:
	@case " $(MAKECMDGOALS) " in \
	  *" server "*) GATEWAY_HTTP_PORT=$(GATEWAY_HTTP_PORT) cargo run -p gateway ;; \
	  *" endpoint "*) \
	    case "$(DEVICE_TYPE)" in \
	      mock-exec) ros_package=mock_exec_layer; ros_executable=mock_exec_layer_node ;; \
	      mock-sensor) ros_package=physio_mock_publisher; ros_executable=physio_mock_publisher_node ;; \
	      "") echo "usage: make run endpoint DEVICE_TYPE=<device-type>" >&2; echo "supported DEVICE_TYPE values: mock-exec, mock-sensor" >&2; exit 2 ;; \
	      *) echo "unsupported DEVICE_TYPE=$(DEVICE_TYPE); supported DEVICE_TYPE values: mock-exec, mock-sensor" >&2; exit 2 ;; \
	    esac; \
	    test -f "$(ROS_BUILD_ROOT)/install/setup.bash" || { echo "ROS install setup not found at $(ROS_BUILD_ROOT)/install/setup.bash; run 'make build' inside the ROS container first" >&2; exit 2; }; \
	    source /opt/ros/$(ROS_DISTRO)/setup.bash && \
	    source "$(ROS_BUILD_ROOT)/install/setup.bash" && \
	    ros2 run "$$ros_package" "$$ros_executable" $(if $(strip $(ENDPOINT_ARGS)),--ros-args $(ENDPOINT_ARGS),) ;; \
	  *) echo "usage: make run server | make run endpoint DEVICE_TYPE=<device-type>" >&2; exit 2 ;; \
	esac

server endpoint:
	@:

## clean: remove build artifacts
clean:
	cargo clean
	rm -rf webui/dist dist
