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
ROS_BUILD_ROOT ?= $(if $(ROS_RUST_WS),$(ROS_RUST_WS),/ws)
ENDPOINT_ARGS ?=

.DEFAULT_GOAL := help

.PHONY: help image humble jazzy build fmt lint check webui webui-dev \
        run server endpoint clean

## help: list available targets
help:
	@echo "ROS Subhealth targets:"
	@echo
	@echo "  make image humble    Build the Ubuntu 22.04 + ROS Humble image"
	@echo "  make image jazzy     Build the Ubuntu 24.04 + ROS Jazzy image"
	@echo "  make build           Build Rust, plus ROS packages in the container"
	@echo "  make fmt             Format the Rust workspace"
	@echo "  make lint            Check formatting and run clippy"
	@echo "  make check           format + lint + build"
	@echo "  make webui           Install deps and build the WebUI"
	@echo "  make webui-dev       Run the WebUI dev server"
	@echo "  make run server      Run the control-plane server"
	@echo "  make run endpoint DEVICE_TYPE=<device-type>"
	@echo "  make clean           Remove build artifacts"
	@echo
	@echo "  endpoint requires DEVICE_TYPE; server and endpoint are mutually exclusive"

## image: build the selected ROS image profile
image:
	@case " $(MAKECMDGOALS) " in \
	  *" humble "*) $(COMPOSE) build --build-arg ROS_DISTRO=humble --build-arg UBUNTU_VERSION=22.04 ;; \
	  *" jazzy "*) $(COMPOSE) build --build-arg ROS_DISTRO=jazzy --build-arg UBUNTU_VERSION=24.04 ;; \
	  *) echo "usage: make image humble|jazzy" >&2; exit 2 ;; \
	esac

humble jazzy:
	@:

## build: Rust workspace build, plus ROS packages in the container
build:
	cargo build --workspace
ifeq ($(IN_CONTAINER),1)
	source /opt/ros/$(ROS_DISTRO)/setup.bash && colcon --log-base $(ROS_BUILD_ROOT)/log build --merge-install --base-paths ros2_ws/src --build-base $(ROS_BUILD_ROOT)/build/merged-symlink --install-base $(ROS_BUILD_ROOT)/install --symlink-install
endif

## fmt: format the Rust workspace
fmt:
	cargo fmt --all

## lint: formatting + clippy
lint:
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings

## check: lint + build
check: lint build

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
