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
  container-side path still runs Cargo directly. Quoted the port assignment in
  both paths so the environment value is passed safely.

## Verification

- `make -n run server`: passed; showed
  `docker compose ... run --rm dev make run server GATEWAY_HTTP_PORT="5000"`.
- `make -n check`: passed; showed only formatting, clippy, and workspace
  build commands, with no test target.
- `make run server` under a 15-second timeout: reached and created the dev
  container, then entered the container-side Cargo command. It exited with
  the existing generated-interface error (`could not find task_interfaces in
  ros_env`) before starting a server. The `--rm` container was removed; no
  server container remains running.
- Grep verification found no retired target names or `ROS_TASK_` variables in
  `Makefile` or `docs/guide/ros-mocks.md`.

## Concerns

The real invocation still cannot compile the current ROS workspace because
the container's generated `task_interfaces` module is unavailable. This is
outside the requested Makefile composition fix and was not changed.
