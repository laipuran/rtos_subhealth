# Migrate Services into ROS Workspace Implementation Plan

> **For agentic workers:** Execute task-by-task with subagent-driven development. Steps use checkbox syntax.

**Goal:** Move the five existing Server service crates into `ros2_ws/src/services/` while preserving their package boundaries, code behavior, tests, and short package names.

**Architecture:** Keep `gateway`, `platform`, `orchestration`, `execution`, and `sensor` as separate Cargo packages. Place only these five service packages under `ros2_ws/src/services/`; keep `interfaces`, `mocks`, and `control_plane/ros_task_client` in their existing sibling areas. The first migration does not merge crates or redesign service APIs.

**Tech Stack:** Cargo workspace, ROS 2 Humble/Jazzy, colcon-cargo/ament-cargo, Docker development image.

## Global Constraints

- Only the five service packages move under `ros2_ws/src/services/`.
- Do not move `ros_task_client`, `task_interfaces`, mocks, WebUI, or unrelated files.
- Preserve service source code and tests except for required crate-name/path/import adjustments.
- Keep package names short: `gateway`, `platform`, `orchestration`, `execution`, `sensor`.
- Do not introduce IPC, adapter traits, `ExecutionPort`, or business-layer refactors.
- Gateway remains the executable entry point.
- The migrated tree must build and test inside ROS containers for Humble and Jazzy.

---

### Task 1: Prove the ROS Cargo package layout

- [ ] Create the destination package skeleton under `ros2_ws/src/services/` without moving implementation yet.
- [ ] Determine whether the repository can expose one Cargo workspace root at `ros2_ws/src/services/Cargo.toml` through one `package.xml`, or whether each crate needs its own `package.xml` for colcon discovery.
- [ ] Add only the minimum metadata required by colcon/ament-cargo.
- [ ] Run `colcon list` and a minimal build proof in Jazzy.
- [ ] Commit the package-layout proof separately.

### Task 2: Move the five crates and preserve short package names

- [ ] Move `services/platform` to `ros2_ws/src/services/platform`.
- [ ] Move `services/orchestration` to `ros2_ws/src/services/orchestration` and rename Cargo package to `orchestration`.
- [ ] Move `services/execution` to `ros2_ws/src/services/execution` and rename Cargo package to `execution`.
- [ ] Move `services/sensor` to `ros2_ws/src/services/sensor` and rename Cargo package to `sensor`.
- [ ] Move `services/gateway` to `ros2_ws/src/services/gateway`.
- [ ] Update Rust imports and package path dependencies without changing logic.
- [ ] Delete the old `services/` tree only after all references point to the new location.
- [ ] Commit the mechanical move.

### Task 3: Relocate workspace/build metadata

- [ ] Move the pure Cargo workspace manifest membership to the new services location or adjust the root workspace so the migrated packages are not built twice.
- [ ] Keep `ros_task_client` excluded from the pure Cargo workspace and independently built by colcon.
- [ ] Update Makefile, Docker, CI, and documentation paths from `services/` to `ros2_ws/src/services/`.
- [ ] Ensure `make build` does not discover duplicate old packages.
- [ ] Commit metadata changes.

### Task 4: Restore build, unit tests, and runtime entry point

- [ ] Run package-level Cargo tests for all five crates in the ROS container.
- [ ] Run `colcon build` for `ros2_ws/src` in Jazzy and Humble.
- [ ] Run ROS package tests and verify `gateway` remains the executable entry point.
- [ ] Run the existing mock Exec/Sensor checks without modifying mock behavior.
- [ ] Fix only migration-induced path/package/build errors.
- [ ] Commit verified migration.

### Task 5: Final dual-distro and scope gate

- [ ] Run Jazzy `make check` and the existing Rust ROS task-client integration.
- [ ] Run Humble `make check` and the existing Rust ROS task-client integration.
- [ ] Verify `colcon list` shows the five short service packages exactly once.
- [ ] Verify no old `services/` paths remain in source/build configuration.
- [ ] Verify no files under WebUI or mock implementation changed.
- [ ] Record the migration evidence and commit final documentation if needed.
