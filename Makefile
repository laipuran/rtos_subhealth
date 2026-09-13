# ROS Subhealth — one entry point for every routine task.
#
# Most targets run anywhere. The ROS targets (`ros`, `ros-deb`, `run-stack`,
# `deb`) automatically enter the pinned Jazzy dev container when invoked from
# the host, and run directly when invoked inside it.
#
#   make help

SHELL := /bin/bash

COMPOSE   := docker compose -f docker/dev/compose.yaml
DEV        := $(COMPOSE) run --rm dev
# Make decides at startup whether it is already inside Docker.
IN_CONTAINER := $(if $(wildcard /.dockerenv),1,0)

# Overridable settings.
ROS_RUST_WS      ?= /ws
SERVICE_DEBS     ?= gateway
GATEWAY_DB_DIR   ?= /tmp/ros
GATEWAY_MAPS_DIR ?= ros2_ws/config/maps
GATEWAY_HTTP_PORT ?= 5000

STACK_CMD = source /opt/ros/jazzy/setup.bash && \
            source $$ROS_RUST_WS/install/setup.bash && \
            source $$ROS_RUST_WS/install_nodes/setup.bash && \
            bash deploy/run_stack.sh

.DEFAULT_GOAL := help

.PHONY: help image shell test build fmt lint check gateway webui webui-dev \
        ros ros-deb run-stack deb publish clean

## help: list available targets
help:
	@echo "ROS Subhealth targets:"
	@echo
	@echo "  make image           Build the Jazzy + Rust dev image"
	@echo "  make shell           Open a shell in the dev container"
	@echo "  make test            Run the pure-Rust workspace tests"
	@echo "  make build           Build the pure-Rust workspace"
	@echo "  make fmt             Format the Rust workspace"
	@echo "  make lint            Check formatting and run clippy"
	@echo "  make check           lint + test"
	@echo "  make gateway         Run the HTTP/WS gateway (host, no ROS)"
	@echo "  make webui           Install deps and build the WebUI"
	@echo "  make webui-dev       Run the WebUI dev server"
	@echo "  make ros             Build ROS interfaces + nodes (dev container)"
	@echo "  make ros-deb         Package the built ROS nodes into a .deb"
	@echo "  make run-stack       Run orchestrator + adapter (dev container)"
	@echo "  make deb             Build service .deb(s) with cargo-deb"
	@echo "  make publish         Publish .deb artifacts to the apt repo"
	@echo "  make clean           Remove build artifacts"
	@echo
	@echo "Variables: SERVICE_DEBS='gateway' ROS_RUST_WS=/ws DEBS='dist/*.deb'"

## image: build the dev image
image:
	$(COMPOSE) build

## shell: interactive shell in the dev container
shell:
ifeq ($(IN_CONTAINER),1)
	bash
else
	$(DEV) bash
endif

## test: pure-Rust workspace tests
test:
	cargo test --workspace

## build: pure-Rust workspace build
build:
	cargo build --workspace

## fmt: format the Rust workspace
fmt:
	cargo fmt --all

## lint: formatting + clippy
lint:
	cargo fmt --all --check
	cargo clippy --all-targets -- -D warnings

## check: lint + test
check: lint test

## gateway: run the gateway (no ROS required)
gateway:
	GATEWAY_DB_DIR=$(GATEWAY_DB_DIR) \
	GATEWAY_MAPS_DIR=$(GATEWAY_MAPS_DIR) \
	GATEWAY_HTTP_PORT=$(GATEWAY_HTTP_PORT) \
	cargo run -p gateway

## webui: install deps and build the WebUI (host only)
webui:
	@command -v pnpm >/dev/null || { echo "pnpm not found: run 'make webui' on the host"; exit 1; }
	cd webui && pnpm install --frozen-lockfile && pnpm build

## webui-dev: run the WebUI dev server (host only)
webui-dev:
	@command -v pnpm >/dev/null || { echo "pnpm not found: run 'make webui-dev' on the host"; exit 1; }
	cd webui && pnpm install --frozen-lockfile && pnpm dev

## ros: build ROS interfaces + nodes
ros:
ifeq ($(IN_CONTAINER),1)
	bash docker/dev/build_ros_rust.sh
else
	$(DEV) bash docker/dev/build_ros_rust.sh
endif

## ros-deb: package the built ROS nodes into a .deb
ros-deb:
ifeq ($(IN_CONTAINER),1)
	deploy/deb/build_ros_debs.sh "$$ROS_RUST_WS/install_nodes" "$$ROS_RUST_WS/install" dist
else
	$(DEV) bash -lc 'deploy/deb/build_ros_debs.sh "$$ROS_RUST_WS/install_nodes" "$$ROS_RUST_WS/install" dist'
endif

## run-stack: run orchestrator + one adapter
run-stack:
ifeq ($(IN_CONTAINER),1)
	bash -lc '$(STACK_CMD)'
else
	$(DEV) bash -lc '$(STACK_CMD)'
endif

## deb: build service .deb(s)
deb:
ifeq ($(IN_CONTAINER),1)
	cargo deb $(addprefix -p ,$(SERVICE_DEBS))
else
	$(DEV) cargo deb $(addprefix -p ,$(SERVICE_DEBS))
endif

## publish: publish .deb artifacts to the apt repo
publish:
	@test -n "$(DEBS)" || { echo "usage: make publish DEBS='dist/*.deb' [APTLY_REPO=ros APTLY_DIST=jazzy GPG_KEY=<id>]"; exit 1; }
	deploy/apt/publish.sh $(DEBS)

## clean: remove build artifacts
clean:
	cargo clean
	rm -rf webui/dist dist
	rm -rf ros2_ws/.cargo
