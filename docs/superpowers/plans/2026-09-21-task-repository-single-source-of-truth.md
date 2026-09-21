# Task Repository Single Source of Truth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make one shared `TaskRepository` the sole owner of every accepted task, including active and terminal tasks, while removing task state storage from Gateway and Orchestrator.

**Architecture:** `platform` defines the task record and repository contract. A separate repository service owns the storage implementation and is injected as one shared instance into both Orchestration and Gateway. Orchestrator owns only execution coordination and delegates every task read/write/state transition to the repository; Gateway only calls the same repository for HTTP queries and publishes events after repository transitions.

**Tech Stack:** Rust workspace, Tokio/Axum application composition, `std::sync::RwLock` inside the in-memory repository, existing Execution/ROS session flow.

**Spec:** User-approved direction in the current conversation: “gateway不再存储taskview，orch也不维护内存数据，全部走repo”.

## Global Constraints

- `TaskRepository` is the only source of truth for accepted task records and their state/progress/phase.
- Gateway must not contain `tasks: HashMap`, `TaskView` storage, or a task ID sequence.
- Orchestrator must not contain `active`, `device_tasks`, or any task state cache.
- Do not add cancellation, capabilities, parameters, new task fields, adapters, wrappers, or compatibility paths.
- Preserve the canonical five task fields: `id`, `device_id`, `primitive`, `target`, `deadline_ms`.
- Keep `Primitive::GoToTag` as the only primitive.
- Completed and failed tasks remain queryable in the repository; terminal transitions release execution occupancy through repository state.
- Do not add tests or test scaffolding. Use existing non-test build/check commands and manual verification only.
- Use only `pub` or private Rust visibility; never use restricted `pub(...)` forms.

---

### Task 1: Define the repository contract and record type

**Files:**
- Modify: `ros2_ws/src/services/platform/src/task.rs`
- Modify: `ros2_ws/src/services/platform/src/lib.rs`
- Create: `ros2_ws/src/services/platform/src/repository.rs`
- Modify: `ros2_ws/src/services/platform/Cargo.toml` only if the contract requires a currently absent dependency
- Modify: `docs/contracts/task.md`
- Modify: `docs/contracts/domain.md`

**Interfaces:**
- Add a platform-owned record representing the complete persisted task state:

```rust
pub struct TaskRecord {
    pub task: Task,
    pub state: TaskState,
    pub progress: f32,
    pub phase: String,
}
```

- Define a repository contract consumed by both Gateway and Orchestration. The contract must support only the operations required by the current flow:

```text
create/accept task with generated ID
get task by ID
list all tasks
apply execution feedback
apply terminal execution result
```

- Repository errors must distinguish only real consumers: duplicate task, busy device, unknown task, invalid target, and storage failure if the implementation can fail.

- [ ] Move `TaskView`'s domain meaning into `TaskRecord`; Gateway must no longer own the record type.
- [ ] Define the repository operations so state transitions are atomic from the caller's perspective.
- [ ] Make repository creation generate task IDs, removing ID generation from Gateway.
- [ ] Define active occupancy from repository task state; terminal records remain stored but no longer occupy their device.
- [ ] Remove obsolete contract wording that describes Gateway-owned task storage.

**Verification:** Run the focused platform check and search that no repository contract contains cancellation, capabilities, parameters, or extra task fields.

---

### Task 2: Add the single repository implementation

**Files:**
- Create: `ros2_ws/src/services/task_repository/Cargo.toml`
- Create: `ros2_ws/src/services/task_repository/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `ros2_ws/src/services/task_repository/package.xml` if required by the existing ROS workspace build conventions

**Interfaces:**
- Produce `InMemoryTaskRepository` implementing the platform repository contract.
- Store all records in one internal `RwLock<HashMap<TaskId, TaskRecord>>` and keep the task ID sequence inside this repository.
- Expose one cloneable shared handle suitable for injection into Gateway and Orchestrator without copying records.

- [ ] Implement task creation with `task-{n}` IDs and initial `Accepted` state.
- [ ] Implement duplicate and busy-device checks while holding the repository write lock.
- [ ] Implement feedback updates as `Accepted/Running` record transitions with clamped progress.
- [ ] Implement terminal result updates as `Succeeded` or `Failed`, preserving the record and releasing device occupancy by state.
- [ ] Implement `get` and `list` by cloning repository records; no references may escape the lock.
- [ ] Keep repository storage concerns out of Gateway and Orchestration.

**Verification:** Run focused non-test checks for `platform` and `task_repository`; inspect that no task map or sequence exists outside the repository implementation.

---

### Task 3: Remove task state from Orchestration

**Files:**
- Modify: `ros2_ws/src/services/orchestration/src/lib.rs`
- Modify: `ros2_ws/src/services/orchestration/src/error.rs`
- Modify: `ros2_ws/src/services/orchestration/Cargo.toml`
- Delete: `ros2_ws/src/services/orchestration/src/device.rs` if it becomes unused after repository-owned device occupancy

**Interfaces:**
- `Orchestrator` contains only execution dependency and shared `TaskRepository` handle.
- `Orchestrator` has no `HashMap`, `active`, `device_tasks`, or task record cache.
- `submit` delegates creation/acceptance and all conflict checks to the repository, then calls Execution with the created canonical `Task`.
- `feedback` delegates to the repository and returns the updated `TaskRecord`.
- `complete` delegates to the repository and returns the updated terminal `TaskRecord`.

- [ ] Replace `ActiveTask` return values with `TaskRecord`.
- [ ] Remove `HashMap` imports and all Orchestrator-owned task/device state.
- [ ] Ensure a failed Execution submission transitions the repository record to `Failed` rather than leaving an accepted record indefinitely.
- [ ] Ensure late feedback/result for a terminal task returns the repository's unknown/terminal error without mutating the record.
- [ ] Remove `Orchestrator::task` and any list method; Gateway will query the repository directly.
- [ ] Remove registry/device-selection code only if the repository's concrete `device_id` and execution occupancy fully replace it.

**Verification:** Run focused Orchestration checks and confirm its source contains no `HashMap`, `active`, `device_tasks`, `TaskView`, or repository implementation.

#### Task 3 review-fix report

- Added the repository-owned `TaskRepositoryError::TerminalTask` error and made both in-memory `apply_feedback` and `apply_result` reject terminal records while holding the repository write lock.
- Removed Orchestrator's separate `get_task`/`reject_if_terminal` pre-checks; `feedback` and `complete` now delegate directly to the atomic repository operations.
- Updated `OrchestrationError` mapping for the repository terminal error.
- Updated `submit` so a failed Execution transition returns the repository error when that transition fails; otherwise it preserves the original Execution error.
- Verification: `cargo fmt --all -- --check`, isolated focused `cargo check --workspace`, isolated `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed. The repository-root `cargo check --workspace` remains unavailable in this host because `.cargo/config.toml` references the missing `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`.

---

### Task 4: Make Gateway a repository consumer only

**Files:**
- Modify: `ros2_ws/src/services/gateway/src/dto.rs`
- Modify: `ros2_ws/src/services/gateway/src/state.rs`
- Modify: `ros2_ws/src/services/gateway/src/handlers.rs`
- Modify: `ros2_ws/src/services/gateway/src/lib.rs`
- Modify: `ros2_ws/src/services/gateway/src/main.rs`
- Modify: `ros2_ws/src/services/gateway/Cargo.toml`

**Interfaces:**
- `AppStateInner` contains only the shared repository, event sequence, event sender, and Orchestrator execution handle.
- It must not contain `tasks`, `TaskView`, or `task_sequence`.
- `list_tasks` and `get_task` read from the repository.
- `create_task` submits through Orchestration; the returned `TaskRecord` is already the repository record and is returned directly.

- [ ] Delete Gateway `TaskView` and all `RwLock<HashMap<TaskId, TaskView>>` storage.
- [ ] Remove Gateway task ID generation and let the repository allocate IDs.
- [ ] Update DTO conversion so Gateway does not construct a second task record.
- [ ] Change feedback/result consumers to call Orchestrator, which updates the repository, then publish events from the returned `TaskRecord`.
- [ ] Keep event broadcasting as notification only; it must never be used as state storage.
- [ ] Ensure HTTP list/get always reflect the same repository used by Orchestration.

**Verification:** Run Gateway and workspace non-test checks; manually verify create/list/get and terminal task state use the same repository record.

#### Task 4 review-fix report

- Completed the `OrchestrationError` HTTP mapping for the upstream `TerminalTask` and `Repository` variants: terminal-task conflicts return `409 CONFLICT`, and repository failures return `500 INTERNAL_SERVER_ERROR`, matching the existing handler categories.
- Verification: `cargo fmt --all -- --check` and `git diff --check` passed. The focused `cargo check -p gateway` is blocked by the host's missing `/opt/ros/jazzy/share/action_msgs/rust/Cargo.toml`; an isolated workspace check without the repository Cargo config reaches ROS build dependencies but is blocked because `AMENT_PREFIX_PATH` is not set.

---

### Task 5: Recompose the runtime around one repository instance

**Files:**
- Modify: `ros2_ws/src/services/gateway/src/main.rs`
- Modify: `ros2_ws/src/services/execution/src/lib.rs` only if constructor signatures require the repository-free execution path to be explicit
- Modify: workspace/package metadata only where required by the new repository crate
- Modify: `docs/architecture/layers.md`
- Modify: `docs/architecture/overview.md`

**Interfaces:**
- The composition root constructs exactly one repository instance and injects the same handle into Orchestrator and Gateway.
- Execution remains independent of task persistence and does not receive the repository.

- [ ] Construct repository once before Orchestrator and AppState.
- [ ] Pass the same repository handle to both components.
- [ ] Keep ROS/Execution dependencies below Orchestration.
- [ ] Document that Repository is the task-state source of truth, while events are transient notifications.
- [ ] Remove architecture wording that implies Gateway-owned task state.

**Verification:** Run the full ROS-container workspace check and Clippy; inspect the composition root to confirm only one repository allocation exists.

---

### Task 6: Remove stale task projection APIs and verify the invariant

**Files:**
- Modify or delete stale references in `ros2_ws/src`, `docs/contracts`, and `docs/architecture`.
- Do not modify `webui` in this task.

**Interfaces:**
- There is one task record type, one repository implementation, and one injected repository instance.

- [ ] Search production Rust for `TaskView`, Gateway-owned task maps, Orchestrator `active`, `device_tasks`, and task sequence fields outside the repository.
- [ ] Remove stale imports and comments from modified modules.
- [ ] Verify completed and failed records remain queryable after terminal transition.
- [ ] Run `cargo fmt --all -- --check`, the ROS-container workspace check, Clippy, and existing non-test build commands.
- [ ] Do not add tests or test scaffolding.

**Verification:** Repository-wide source search confirms all task reads/writes pass through `TaskRepository`, with no second task-state cache.

---

## Decisions to confirm before implementation

- The repository implementation is a separate `services/task_repository` crate, while the contract and `TaskRecord` live in `platform`.
- The repository uses in-memory storage for this iteration; persistence technology is not added without a separate requirement.
- The repository owns task ID allocation.
- `TaskRecord` is the replacement for Gateway `TaskView` and is visible to both Gateway and Orchestration.
