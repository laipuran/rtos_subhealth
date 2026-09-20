# Makefile maintenance report

## Changes

- Removed the obsolete `ros-test`, `ros-task-test`, and
  `ros-task-integration` targets, variables, help entries, and `check`
  dependencies.
- Kept `check` as the non-test `lint` plus `build` validation target.
- Removed the active guide's instructions and configuration details for the
  retired task-test targets. Historical `docs/superpowers/` plans were not
  modified.
- Changed host-side `make run server` to invoke `make run server` through the
  existing `$(DEV)` container while passing `GATEWAY_HTTP_PORT`; the
  container-side path now checks for the generated ROS install setup, sources
  both ROS environment setup files, and then runs Cargo directly. Quoted the
  port assignment in both paths so the environment value is passed safely.

## Verification

- `make -n run server`: passed; showed
  `docker compose ... run --rm dev make run server GATEWAY_HTTP_PORT="5000"`.
- `make -n check`: passed; showed only formatting, clippy, and workspace
  build commands, with no test target.
- `make IN_CONTAINER=1 ROS_DISTRO=humble
  ROS_BUILD_ROOT=/tmp/ros-subhealth-missing run server`: exited 2 as expected,
  emitting the clear build-first error before attempting Cargo.
- `make -n IN_CONTAINER=1 ROS_DISTRO=humble ROS_BUILD_ROOT=/ws/humble run
  server`: passed; showed the distro setup and install setup being sourced
  before the quoted `GATEWAY_HTTP_PORT` Cargo invocation.
- Grep verification found no retired target names or `ROS_TASK_` variables in
  `Makefile` or `docs/guide/ros-mocks.md`.

## Concerns

The real invocation requires `make build` inside the ROS container first so
the generated `task_interfaces` module and install setup are available.
