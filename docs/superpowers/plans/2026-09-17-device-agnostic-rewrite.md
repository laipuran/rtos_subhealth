# Device-Agnostic Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the repository around device-neutral contracts, shared Sensor access, isolated Execution endpoints, and explicit server/endpoint Make entry points.

**Architecture:** Domain contracts are pure Rust and have no transport or device dependencies. Gateway, Orchestration, Execution, and Sensor services depend only on those contracts; HTTP, WebSocket, ROS, endpoint runtimes, and vendor SDKs are adapters at the edge. The old services and ROS nodes are removed after the new fake-endpoint path is verified.

**Tech Stack:** Rust workspace, Tokio, Serde, Axum, React/Vite, ROS 2 interface packages, Docker Compose, GNU Make.

**Spec:** `docs/superpowers/specs/2026-09-17-device-agnostic-rewrite-design.md`

## Global Constraints

- Control-plane crates must not depend on a concrete machine, vendor SDK, device OS, or ROS distribution.
- Sensor must be consumable by both Orchestration and Execution.
- Device selection belongs only to endpoint configuration, endpoint adapters, and backend SDKs.
- Generated ROS types may appear only in explicit transport mappers.
- `make run server` and `make run endpoint DEVICE_TYPE=<device-type>` are mutually exclusive modes.
- `make image humble` and `make image jazzy` select the ROS image configuration.
- All core tests use fake sensors and fake endpoints; real hardware is never a test prerequisite.
- Do not retain compatibility branches from the old implementation.

---

### Task 1: Replace the workspace foundation with pure contracts

**Files:**
- Create: `contracts/domain-contract/Cargo.toml`, `contracts/domain-contract/src/lib.rs`
- Create: `contracts/task-contract/Cargo.toml`, `contracts/task-contract/src/lib.rs`
- Create: `contracts/execution-contract/Cargo.toml`, `contracts/execution-contract/src/lib.rs`
- Create: `contracts/sensor-contract/Cargo.toml`, `contracts/sensor-contract/src/lib.rs`
- Create: `contracts/event-contract/Cargo.toml`, `contracts/event-contract/src/lib.rs`
- Modify: `Cargo.toml`
- Test: contract crate unit tests in each `src/lib.rs`

**Interfaces:**
- `domain-contract`: `TaskId`, `DeviceId`, `SensorId`, `DeviceDescriptor`, `DeviceState`, `DeviceCapability`.
- `task-contract`: `Task`, `TaskTarget`, `Primitive`, `TaskState`, `TaskOutcome`.
- `execution-contract`: `ExecutionCommand`, `ExecutionFeedback`, `ExecutionResult`, `Executor`.
- `sensor-contract`: `SensorDescriptor`, `SensorSample`, `SensorFilter`, `SensorProvider`.
- `event-contract`: `SystemEvent` and monotonic `EventSequence`.

- [ ] Write serialization and capability tests before implementation.
- [ ] Run `cargo test -p domain-contract -p task-contract -p execution-contract -p sensor-contract -p event-contract` and verify the new crates fail to compile.
- [ ] Implement the types with `serde`, `thiserror`, and no transport dependencies.
- [ ] Add `FakeSensorProvider` and `FakeExecutor` only in test modules.
- [ ] Run the contract tests and `cargo tree -p execution-contract`; verify no ROS or device crate appears.
- [ ] Commit `feat: add device-neutral domain contracts`.

### Task 2: Implement Sensor service and execution runtime

**Files:**
- Create: `services/sensor/Cargo.toml`, `services/sensor/src/lib.rs`
- Create: `services/execution/Cargo.toml`, `services/execution/src/lib.rs`
- Create: `services/sensor/tests/provider.rs`
- Create: `services/execution/tests/runtime.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- `SensorRegistry::register(provider)`, `SensorRegistry::latest(id)`, `SensorRegistry::subscribe(filter)`.
- `ExecutionRuntime<S, E>::submit(command)`, `cancel(task_id)`, `state(task_id)`.
- `ExecutionRuntime` consumes `SensorProvider` and `Executor` traits only.

- [ ] Add failing tests for sensor lookup, subscription filtering, stale samples, command submission, cancellation, deadline, and safe stop.
- [ ] Run the focused tests and verify failures describe missing runtime behavior.
- [ ] Implement registry fan-out and runtime lifecycle without ROS or device branches.
- [ ] Verify an execution receives Sensor samples while Orchestration can read the same registry.
- [ ] Run `cargo test -p sensor-service -p execution-service` and clippy.
- [ ] Commit `feat: add shared sensor and execution runtimes`.

### Task 3: Rewrite Orchestration around contracts

**Files:**
- Create: `services/orchestration/Cargo.toml`, `services/orchestration/src/lib.rs`
- Create: `services/orchestration/src/registry.rs`, `services/orchestration/src/lifecycle.rs`
- Create: `services/orchestration/tests/lifecycle.rs`
- Remove: `services/orchestrator-core/`
- Modify: `Cargo.toml`

**Interfaces:**
- `DeviceRegistry::register(DeviceDescriptor)`, `select(requirements)`, `state(device_id)`.
- `Orchestrator::submit(Task)`, `feedback(task_id, ExecutionFeedback)`, `complete(task_id, ExecutionResult)`, `cancel(task_id)`.
- `Orchestrator` depends on `SensorProvider` and an `ExecutionPort` trait, never on ROS or endpoint implementations.

- [ ] Add tests for capability selection, busy-device rejection, cancellation, deadline failure, feedback, and release after terminal result.
- [ ] Verify tests fail without the new orchestration crate.
- [ ] Implement lifecycle transitions and explicit error codes.
- [ ] Add a compile-time dependency check proving orchestration has no endpoint/backend dependency.
- [ ] Run focused tests, workspace tests for contracts/services, and clippy.
- [ ] Commit `feat: rewrite orchestration on neutral contracts`.

### Task 4: Rewrite Gateway and transport boundary

**Files:**
- Create: `adapters/gateway-http/Cargo.toml`, `adapters/gateway-http/src/lib.rs`
- Create: `adapters/gateway-ws/Cargo.toml`, `adapters/gateway-ws/src/lib.rs`
- Create: `services/gateway/src/application.rs`, `services/gateway/src/events.rs`
- Modify: `services/gateway/Cargo.toml`, `services/gateway/src/main.rs`
- Remove: old gateway bridge coupling and duplicated task models
- Modify: `webui/src/api/*`, `webui/src/hooks/*`

**Interfaces:**
- `TaskPort::submit(Task)`, `get(TaskId)`, `cancel(TaskId)`.
- `EventPort::subscribe(EventSequence)` and one multiplexed WebSocket stream.
- HTTP handlers translate payloads into `task-contract` and never call endpoint code.

- [ ] Add HTTP/WS contract tests for submit, list, get, cancel, event ordering, and invalid device-specific fields.
- [ ] Run gateway tests and verify they fail against absent application ports.
- [ ] Implement application services using orchestration ports and persistence interfaces.
- [ ] Update WebUI to use canonical task/event payloads.
- [ ] Run Rust gateway tests and `pnpm build`.
- [ ] Commit `feat: rebuild gateway around application contracts`.

### Task 5: Rebuild ROS interfaces and explicit mappers

**Files:**
- Create: `interfaces/ros/` packages for task, execution, sensor, and event transport.
- Create: `adapters/ros-transport/` with domain-to-ROS and ROS-to-domain mappers.
- Remove: `ros2_ws/src/robot/interfaces/` and old node-specific domain conversions.
- Modify: Docker/colcon build scripts.

**Interfaces:**
- ROS actions/topics are serialization boundaries only.
- Mappers expose `to_ros_*` and `from_ros_*` functions returning contract errors.

- [ ] Add mapper fixtures for task, descriptor, state, sensor sample, feedback, result, and event.
- [ ] Run mapper tests outside ROS and verify missing/invalid fields fail explicitly.
- [ ] Implement the minimal ROS packages and mappers.
- [ ] Build interfaces in both selected container profiles.
- [ ] Run mapper tests and `make image jazzy`/`make image humble` smoke checks.
- [ ] Commit `feat: isolate ROS transport from domain contracts`.

### Task 6: Create generic endpoint runtime and backend registry

**Files:**
- Create: `adapters/endpoint-runtime/`
- Create: `adapters/endpoint-adapters/fake/`
- Create: `adapters/backend-sdks/README.md`
- Remove: `services/device-sdk/`, `ros2_ws/src/robot/adapter/`, `deploy/config/adapter-*.env`
- Modify: endpoint deployment manifests.

**Interfaces:**
- `BackendSdk` is implemented only by endpoint adapters.
- `EndpointRegistry::from_config(config)` selects a backend by configuration.
- Runtime exposes descriptor, state, execution, cancellation, Sensor stream, watchdog, and local stop.

- [ ] Add tests proving fake endpoint executes a task with Sensor support and rejects unsupported primitives.
- [ ] Verify the runtime tests fail before the new endpoint modules exist.
- [ ] Implement registry/config validation with no machine-name match in core runtime.
- [ ] Implement fake backend as the first complete endpoint.
- [ ] Run endpoint tests and grep core crates for concrete device names.
- [ ] Commit `feat: add configurable endpoint runtime`.

### Task 7: Rewrite Make, Docker, deployment, and top-level docs

**Files:**
- Modify: `Makefile`, `docker/dev/compose.yaml`, `docker/dev/Dockerfile`
- Modify: `README.md`, `docs/guide/getting-started.md`, `deploy/README.md`
- Create: `docker/profiles/ubuntu22-ros-humble/`, `docker/profiles/ubuntu24-ros-jazzy/`
- Modify: `deploy/systemd/*`, `deploy/run_stack.sh`

**Interfaces:**
- `make webui`, `make webui-dev`.
- `make run server`.
- `make run endpoint DEVICE_TYPE=<device-type>`.
- `make image humble`, `make image jazzy`.

- [ ] Add shell-level Make tests for accepted commands, missing `DEVICE_TYPE`, unknown image, and mutually exclusive run modes.
- [ ] Implement `.PHONY` targets and image/profile selection.
- [ ] Make server mode start only control-plane services; endpoint mode start only configured endpoint runtime.
- [ ] Update Docker and systemd files to consume endpoint configuration instead of adapter-specific branches.
- [ ] Run all Make smoke tests, `make help`, and documentation link checks.
- [ ] Commit `build: replace runtime entry points with layered modes`.

### Task 8: Remove superseded implementation and establish final verification

**Files:**
- Remove: old `services/*` crates and ROS nodes not referenced by the new graph.
- Modify: all `docs/rfc/*`, `docs/tech/*`, and old plans with superseded/archive markers.
- Create: `docs/architecture/overview.md`, `layers.md`, `contracts.md`, `runtime.md`, `endpoint-adapters.md`, `build-and-run.md`, `migration-analysis.md`.
- Modify: `.gitignore`, CI workflow, root `Cargo.toml`.

**Interfaces:**
- Documentation points to the new architecture spec and contract documents only.
- CI runs contract, service, adapter, transport, WebUI, and Make checks.

- [ ] Search for concrete device names in control-plane source and remove every remaining coupling.
- [ ] Search for old package imports, old ROS paths, stale Make targets, and duplicate interface definitions.
- [ ] Mark historical documents as superseded without leaving contradictory authoritative claims.
- [ ] Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm build`, and all Make smoke tests.
- [ ] Run `git diff --check` and inspect the final dependency graph.
- [ ] Commit `refactor: complete device-agnostic architecture rewrite`.
