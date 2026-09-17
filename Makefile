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

.DEFAULT_GOAL := help

.PHONY: help image humble jazzy test build fmt lint check webui webui-dev \
        run server endpoint clean

## help: list available targets
help:
	@echo "ROS Subhealth targets:"
	@echo
	@echo "  make image humble    Build the Ubuntu 22.04 + ROS Humble image"
	@echo "  make image jazzy     Build the Ubuntu 24.04 + ROS Jazzy image"
	@echo "  make test            Run the pure-Rust workspace tests"
	@echo "  make build           Build the pure-Rust workspace"
	@echo "  make fmt             Format the Rust workspace"
	@echo "  make lint            Check formatting and run clippy"
	@echo "  make check           lint + test"
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
	  *" endpoint "*) test -n "$(DEVICE_TYPE)" || { echo "usage: make run endpoint DEVICE_TYPE=<device-type>" >&2; exit 2; }; DEVICE_TYPE=$(DEVICE_TYPE) cargo run -p fake-endpoint-adapter --bin endpoint-runtime ;; \
	  *) echo "usage: make run server | make run endpoint DEVICE_TYPE=<device-type>" >&2; exit 2 ;; \
	esac

server endpoint:
	@:

## clean: remove build artifacts
clean:
	cargo clean
	rm -rf webui/dist dist
	rm -rf ros2_ws/.cargo
