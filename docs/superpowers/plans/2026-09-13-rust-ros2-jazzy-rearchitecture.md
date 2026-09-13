# Rust-first / ROS 2 Jazzy Industrial Rearchitecture — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python/Flask/Foxy codebase with a Rust-first, ROS 2 Jazzy, industrially deployable system without losing GO2 compatibility.

**Architecture:** A single Cargo workspace hosts the Rust application services (`gateway`, `diagnosis`) and the ROS nodes (`robot_driver`, `control`, `perception`, `sim`). ROS interfaces stay rosidl and are code-generated into Rust via `rclrs`/`rosidl_rust`. The robot hardware boundary is isolated behind a `RobotBackend` trait so the 500 Hz Unitree path can fall back to C++ if the Phase 0 spike fails. HTTP/WS contract is preserved so the WebUI is untouched. Deployment is per-package `.deb` → signed apt → systemd, with RAUC A/B OTA.

**Tech Stack:** Rust 1.85+ (pinned), `rclrs` 0.7, ROS 2 Jazzy / Ubuntu 24.04, `axum` 0.8, `tokio`, `tokio-tungstenite`, `rusqlite`, `async-openai`, `opencv`, `nokhwa`, `mujoco-rs`, React/Vite (existing), Docker, colcon-cargo, bloom, aptly, systemd, RAUC.

**Spec:** `docs/superpowers/specs/2026-09-13-rust-ros2-jazzy-rearchitecture-design.md`

## Global Constraints

- Target runtime: ROS 2 **Jazzy** on **Ubuntu 24.04**, `rmw_cyclonedds_cpp`, `ROS_DOMAIN_ID=1`.
- Rust toolchain pinned in `rust-toolchain.toml`; `edition = "2021"`, MSRV 1.85.
- HTTP/WS contract (RFC-005/006/009) must not change: paths, `{error:{code,message,details}}`, pagination `offset`/`limit` (limit ≤ 200), ETag on `/api/v1/map`, `X-Trace-Id`, `X-API-Key` auth.
- Interface packages are the single source of contract truth.
- No new Python production code. Existing Python is retired package-by-package.
- Every Rust crate compiles and tests with plain `cargo test` unless it is explicitly `--features ros`.
- No secrets in the repo or in built artifacts; runtime secrets via systemd credentials.
- Phase 0 gate: the Rust `UnitreeBackend` must be validated end-to-end against GO2/MuJoCo before Phase 3 hardware code merges.

---

## File Structure

```
Cargo.toml                                  workspace root
rust-toolchain.toml                         pinned toolchain
services/gateway/                           Rust/axum HTTP+WS+SQLite+static
  Cargo.toml
  src/main.rs, lib.rs, config.rs, error.rs
  src/model/{mod,task,goal,diagnosis}.rs
  src/store/{mod,task_store,diagnosis_store}.rs
  src/api/{mod,http,ws,auth,trace,map}.rs
  src/bridge/{mod,ros,mock}.rs
  tests/{http_contract,store,ws}.rs
services/diagnosis/                         Rust aggregation+RAG+LLM
  Cargo.toml
  src/lib.rs, aggregator.rs, anomaly.rs, rag.rs, llm.rs, schema.rs
  tests/*.rs
ros2_ws/src/robot/interfaces/task_interfaces/        rosidl
ros2_ws/src/robot/interfaces/perception_interfaces/  rosidl
ros2_ws/src/robot/interfaces/diagnosis_interfaces/      rosidl
ros2_ws/src/robot/robot_driver/                   rclrs crate + trait
ros2_ws/src/robot/control/                        rclrs crate
ros2_ws/src/robot/perception/                     rclrs crate
ros2_ws/src/robot/sim/                            MuJoCo + mocks
deploy/{debian,systemd,apt,rauc,config}/
docker/{dev,ci}/Dockerfile
.github/workflows/ci.yml
```

---

## Phase 1 — Foundation

### Task 1: Resolve the documentation merge conflict

**Files:**
- Modify: `docs/guide/getting-started.md`

**Interfaces:** none.

- [ ] **Step 1: Inspect the conflict**

Run: `grep -n '^<<<<<<<\|^=======$\|^>>>>>>>' docs/guide/getting-started.md`
Expected: markers at lines 1, 366, 778.

- [ ] **Step 2: Reconstruct the file**

Take the `HEAD` side (lines 2–365) and append the useful `Arch + Docker` appendix from the `origin/master` side (lines 734–778). Remove all three marker lines. Use `git checkout --ours` semantics mentally; do not keep duplicate sections.

- [ ] **Step 3: Verify no markers remain**

Run: `grep -c '^<<<<<<<\|^>>>>>>>' docs/guide/getting-started.md`
Expected: `0`

- [ ] **Step 4: Commit**

```bash
git add docs/guide/getting-started.md
git commit -m "docs: resolve getting-started merge conflict"
```

### Task 2: Stop tracking runtime artifacts

**Files:**
- Modify: `.gitignore`
- Delete from index: `ros2_ws/config/tasks.db`

**Interfaces:** none.

- [ ] **Step 1: Confirm what is tracked**

Run: `git ls-files | grep -E '\.(db|sqlite)$'`
Expected: `ros2_ws/config/tasks.db`

- [ ] **Step 2: Remove from the index and extend ignore rules**

```bash
git rm --cached ros2_ws/config/tasks.db
printf '\n# Runtime state\n**/*.db\n**/*.db-wal\n**/*.db-shm\nros2_ws/config/maps/*.local.json\n' >> .gitignore
```

- [ ] **Step 3: Verify**

Run: `git ls-files | grep -c '\.db$'`
Expected: `0`

- [ ] **Step 4: Commit**

```bash
git add .gitignore
git commit -m "chore: untrack runtime sqlite databases"
```

### Task 3: Create the Cargo workspace skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `services/gateway/Cargo.toml`, `services/gateway/src/lib.rs`
- Create: `services/diagnosis/Cargo.toml`, `services/diagnosis/src/lib.rs`

**Interfaces:**
- Produces: workspace members `services/gateway`, `services/diagnosis`.

- [ ] **Step 1: Write the workspace manifest**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = ["services/gateway", "services/diagnosis"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
license = "Apache-2.0"

[workspace.dependencies]
anyhow = "1"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "signal", "sync", "time"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[workspace.lints.rust]
unsafe_code = "forbid"
[workspace.lints.clippy]
all = "warn"
```

- [ ] **Step 2: Write the toolchain pin**

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.85"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

- [ ] **Step 3: Create empty crate manifests**

`services/gateway/Cargo.toml`:

```toml
[package]
name = "gateway"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true

[lints]
workspace = true
```

`services/diagnosis/Cargo.toml` is identical with `name = "diagnosis"`.

`services/gateway/src/lib.rs`:

```rust
pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }
```

`services/diagnosis/src/lib.rs`:

```rust
pub mod aggregator;
pub mod anomaly;
pub mod rag;
pub mod llm;
pub mod schema;
```

For Step 3 create only `lib.rs` placeholders; the modules are added in Phase 2b.

- [ ] **Step 4: Verify the workspace builds**

Run: `cargo build`
Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml services
git commit -m "build: add rust workspace skeleton"
```

### Task 4: CI pipeline

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:** none.

- [ ] **Step 1: Write the workflow**

```yaml
name: ci
on:
  push: { branches: [master, 'feat.*'] }
  pull_request:
jobs:
  rust:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.85.0
        with: { components: 'rustfmt, clippy' }
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --workspace
  ros:
    runs-on: ubuntu-24.04
    container: ros:jazzy-ros-base
    steps:
      - uses: actions/checkout@v4
      - run: apt-get update && apt-get install -y python3-vcstool cmake build-essential
      - run: rosdep update && rosdep install --from-paths ros2_ws/src --ignore-src -y || true
      - run: . /opt/ros/jazzy/setup.sh && colcon build --symlink-install
  webui:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - run: cd webui && pnpm install --frozen-lockfile && pnpm build
```

- [ ] **Step 2: Validate YAML**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Expected: no error.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add rust, ros and webui jobs"
```

### Task 5: Jazzy dev container

**Files:**
- Create: `docker/dev/Dockerfile`
- Create: `docker/dev/compose.yaml`
- Modify: delete `ros2_ws/docker/Dockerfile`, `ros2_ws/docker/docker-compose.yml`, `ros2_ws/docker/dev.sh`, `ros2_ws/docker/entrypoint.sh`

**Interfaces:** provides `make -C docker/dev up` shell with ROS + Rust.

- [ ] **Step 1: Write the Dockerfile**

```dockerfile
FROM ros:jazzy-ros-base
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake git curl pkg-config libclang-dev \
      python3-pip python3-vcstool \
      ros-jazzy-rmw-cyclonedds-cpp ros-jazzy-cv-bridge ros-jazzy-sensor-msgs \
 && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install colcon-cargo colcon-ros-cargo --locked || true
WORKDIR /workspace
```

- [ ] **Step 2: Write compose**

```yaml
services:
  dev:
    build: .
    image: ros-dev:jazzy
    network_mode: host
    environment:
      RMW_IMPLEMENTATION: rmw_cyclonedds_cpp
      ROS_DOMAIN_ID: "1"
    volumes:
      - ../..:/workspace
    command: bash
```

- [ ] **Step 3: Build the image**

Run: `docker build -t ros-dev:jazzy docker/dev`
Expected: image builds; `ros2 --version` runs inside.

- [ ] **Step 4: Commit**

```bash
git add docker ros2_ws/docker
git commit -m "build: replace foxy dev container with jazzy+rust"
```

---

## Phase 0 — GO2 Compatibility Gate

### Task 6: Unitree message bridge spike

**Files:**
- Create: `ros2_ws/src/robot_driver/` crate with `--features ros`
- Create: `deploy/spikes/go2_lowcmd_probe.md` (runbook)

**Interfaces:**
- Produces: validation that a Rust node can publish `rt/lowcmd` and receive `rt/sportmodestate` on Jazzy.

- [ ] **Step 1: Vendor Unitree interfaces**

```bash
mkdir -p ros2_ws/src/unitree && cd ros2_ws/src/unitree
git clone https://github.com/unitreerobotics/unitree_ros2.git
cd unitree_ros2 && git checkout v0.3.0
```

Keep only `unitree_go` / `unitree_api` message packages; drop C++ examples. Record the pinned commit in the runbook.

- [ ] **Step 2: Build on Jazzy inside the dev container**

Run: `docker run --rm -v "$PWD":/workspace -w /workspace ros-dev:jazzy bash -lc 'source /opt/ros/jazzy/setup.bash && colcon build --packages-select unitree_go unitree_api'`
Expected: packages build.

- [ ] **Step 3: Run the probe against MuJoCo**

Follow `deploy/spikes/go2_lowcmd_probe.md`: start the Unitree MuJoCo bridge, run the Rust probe, assert a `rt/lowcmd` write is acknowledged by a `rt/sportmodestate` change.

- [ ] **Step 4: Record the gate result**

- PASS → proceed with Phase 3 Rust `UnitreeBackend`.
- FAIL → open a tracked task to implement the C++ `unitree_sdk2` shim behind `RobotBackend`; do not block Phases 1/2/4.

- [ ] **Step 5: Commit**

```bash
git add ros2_ws/src/unitree deploy/spikes
git commit -m "spike(go2): jazzy lowcmd/ sportmodestate probe"
```

---

## Phase 2 — Rust Gateway (replaces Flask desc_layer)

### Task 7: Gateway crate: config and error types

**Files:**
- Modify: `services/gateway/Cargo.toml`
- Create: `services/gateway/src/config.rs`, `src/error.rs`
- Test: `services/gateway/tests/error.rs`

**Interfaces:**
- Produces: `Config { http_port, exec_action_name, maps_dir, db_dir, api_token, webui_dir }`, `ApiError { code: ErrorCode, message, details: Option<Value> }`, `ApiError::status() -> StatusCode`.

- [ ] **Step 1: Write the failing test**

```rust
// tests/error.rs
use gateway::error::{ApiError, ErrorCode};
#[test]
fn error_codes_map_to_http_status() {
    assert_eq!(ApiError::new(ErrorCode::InvalidJson, "x").status(), 400);
    assert_eq!(ApiError::new(ErrorCode::NotFound, "x").status(), 404);
    assert_eq!(ApiError::new(ErrorCode::Conflict, "x").status(), 409);
    assert_eq!(ApiError::new(ErrorCode::Unauthorized, "x").status(), 401);
}
```

- [ ] **Step 2: Run test, expect failure**

Run: `cargo test -p gateway --test error`
Expected: FAIL (`gateway::error` not found).

- [ ] **Step 3: Implement**

```rust
// src/error.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode { InvalidJson, InvalidGoal, InvalidParam, InvalidState, NotFound, Conflict, Unauthorized, Internal }
impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self { Self::InvalidJson => "INVALID_JSON", Self::InvalidGoal => "INVALID_GOAL",
            Self::InvalidParam => "INVALID_PARAM", Self::InvalidState => "INVALID_STATE",
            Self::NotFound => "NOT_FOUND", Self::Conflict => "CONFLICT",
            Self::Unauthorized => "UNAUTHORIZED", Self::Internal => "INTERNAL" }
    }
    pub fn status(self) -> u16 {
        match self { Self::InvalidJson | Self::InvalidGoal | Self::InvalidParam | Self::InvalidState => 400,
            Self::Unauthorized => 401, Self::NotFound => 404, Self::Conflict => 409, Self::Internal => 500 }
    }
}
#[derive(Debug, Clone)]
pub struct ApiError { pub code: ErrorCode, pub message: String, pub details: Option<serde_json::Value> }
impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self { Self { code, message: message.into(), details: None } }
    pub fn with_details(mut self, d: serde_json::Value) -> Self { self.details = Some(d); self }
    pub fn status(&self) -> u16 { self.code.status() }
}
```

- [ ] **Step 4: Run test, expect pass**

Run: `cargo test -p gateway --test error`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add services/gateway
git commit -m "feat(gateway): error model with rfc005 status mapping"
```

### Task 8: Gateway: domain model + SQLite task store

**Files:**
- Create: `services/gateway/src/model/mod.rs`, `src/model/task.rs`, `src/store/mod.rs`, `src/store/task_store.rs`
- Test: `services/gateway/tests/store.rs`

**Interfaces:**
- Produces: `Goal { type_, priority, route_id, target_tags, constraints, deadline_ms }`, `TaskRecord { goal_id, goal, state, ... }`, `TaskStore::open(dir)`, `add/get/update/list_all/list_active/exists`.

- [ ] **Step 1: Write the failing test**

```rust
// tests/store.rs
use gateway::model::task::{Goal, TaskRecord};
use gateway::store::task_store::TaskStore;
#[test]
fn round_trip_task() {
    let dir = tempfile::tempdir().unwrap();
    let store = TaskStore::open(dir.path()).unwrap();
    let rec = TaskRecord::new("T-1", Goal::new("go_to_tag").with_tags(vec![42]));
    store.add(&rec).unwrap();
    let got = store.get("T-1").unwrap().unwrap();
    assert_eq!(got.goal.type_, "go_to_tag");
    assert_eq!(got.goal.target_tags, vec![42]);
    store.update_state("T-1", "running").unwrap();
    assert_eq!(store.get("T-1").unwrap().unwrap().state, "running");
}
```

- [ ] **Step 2: Run test, expect failure**

Run: `cargo test -p gateway --test store`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement model + store**

Serialize `Goal` to `goal_json` exactly as the Python `TaskRecord._to_row` (fields `type`, `priority`, `route_id`, `target_tags`, `constraints{max_speed_mps,min_clearance_m,avoid_tags}`, `deadline_ms`) so existing `tasks.db` rows remain readable. Schema is the same `tasks` table; keep `_ensure_column` migration semantics for `route_json`/`finished_stages`.

Use `serde` with `#[serde(rename = "type")]` and `rusqlite` with `Mutex<Connection>`.

- [ ] **Step 4: Run test, expect pass**

Run: `cargo test -p gateway --test store`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add services/gateway
git commit -m "feat(gateway): task domain model and sqlite store"
```

### Task 9: Gateway: HTTP contract

**Files:**
- Create: `services/gateway/src/api/{mod,http,auth,trace,map}.rs`, `src/lib.rs` router builder
- Test: `services/gateway/tests/http_contract.rs`

**Interfaces:**
- Consumes: `TaskStore`, `ApiError`.
- Produces: `build_router(state) -> axum::Router` implementing every RFC-005 endpoint.

- [ ] **Step 1: Write failing contract tests**

Cover: `POST /api/v1/tasks` → 201 `{task_id,status,type}`; invalid JSON → 400 `INVALID_JSON`; bad type → 400 `INVALID_GOAL`; `GET /api/v1/tasks?offset=0&limit=50` → pagination with `total/offset/limit`; `GET /api/v1/tasks/{unknown}` → 404; cancel final-state → 400 `INVALID_STATE`; `GET /api/v1/map` ETag + `If-None-Match` → 304; missing `X-API-Key` when token set → 401; every response has `X-Trace-Id` and body `trace_id`.

Use `axum::body::to_bytes` + `tower::ServiceExt::oneshot`.

- [ ] **Step 2: Run tests, expect failure**

Run: `cargo test -p gateway --test http_contract`
Expected: FAIL.

- [ ] **Step 3: Implement handlers**

Add `axum = "0.8"`, `tower = "0.5"`, `tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }`, `uuid`, `sha2`/`md5`, `rusqlite`. Use a `trace` middleware that reads `X-Trace-Id` or generates a 16-hex id and injects it into the response header and JSON body.

- [ ] **Step 4: Run tests, expect pass**

Run: `cargo test -p gateway --test http_contract`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add services/gateway
git commit -m "feat(gateway): rfc005 http contract"
```

### Task 10: Gateway: WebSocket event hub

**Files:**
- Create: `services/gateway/src/api/ws.rs`
- Test: `services/gateway/tests/ws.rs`

**Interfaces:**
- Produces: `EventHub::broadcast(&self, event: Event)`; single `/api/v1/events` socket; events `feedback`, `result`, `diagnosis`, `vitals`; optional first-message token auth.

- [ ] **Step 1: Write failing test**

Connect two `tokio_tungstenite` clients, broadcast a `feedback` event, assert both receive identical JSON.

- [ ] **Step 2: Run, expect failure**

Run: `cargo test -p gateway --test ws`
Expected: FAIL.

- [ ] **Step 3: Implement with `tokio::sync::broadcast`**

Each client gets a receiver; lagging clients resubscribe. Preserve JSON shape from `http_server.py::broadcast`.

- [ ] **Step 4: Run, expect pass**

Run: `cargo test -p gateway --test ws`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add services/gateway
git commit -m "feat(gateway): websocket event hub"
```

### Task 11: Gateway: ROS bridge trait + mock + rclrs adapter

**Files:**
- Create: `services/gateway/src/bridge/{mod,ros,mock}.rs`
- Test: `services/gateway/tests/bridge.rs`

**Interfaces:**
- Produces: `trait RosBridge { async fn send_goal(&self, id, goal); async fn cancel(&self, id); async fn trigger_diagnosis(&self, id); }`; `MockBridge` (tests), `RclrsBridge` behind `--features ros`.

- [ ] **Step 1: Write failing test** using `MockBridge`: a sent goal is recorded and a canned feedback event is emitted to the hub.

- [ ] **Step 2: Implement bridge; keep `rclrs` import gated** so `cargo test` on the host does not need ROS.

- [ ] **Step 3: Run tests** `cargo test -p gateway`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add services/gateway
git commit -m "feat(gateway): ros bridge seam with mock and rclrs adapter"
```

### Task 12: Gateway: serve WebUI + mock compose demo

**Files:**
- Modify: `services/gateway/src/lib.rs`
- Create: `docker/dev/compose.demo.yaml`, `services/gateway/src/main.rs`

**Interfaces:**
- Consumes: `build_router`, `Config`.
- Produces: `GET /` serving `webui/dist/index.html`; SPA fallback.

- [ ] **Step 1: Add static serving** via `tower_http::services::{ServeDir, ServeFile}` with fallback to `index.html`.

- [ ] **Step 2: Build and run the demo**

Run: `cd webui && pnpm build` then `cargo run -p gateway -- --db-dir /tmp/ros --webui-dir webui/dist`
Expected: `curl localhost:5000/api/v1/tasks` returns `{tasks:[],total:0,...}` and `/` serves the app.

- [ ] **Step 3: Commit**

```bash
git add services/gateway docker
git commit -m "feat(gateway): serve webui and add mock demo compose"
```

---

## Phase 2b — Rust Diagnosis

### Task 13: Aggregator + anomaly rules

**Files:** `services/diagnosis/src/{aggregator,anomaly}.rs`; Test: `tests/aggregator.rs`, `tests/anomaly.rs`

**Interfaces:**
- Produces: `WindowAggregator` with per-`data_src` rolling windows (default 60 s) computing mean/min/max/trend; `RuleEngine` thresholds (`spo2 < 90`, `systolic > 140`, `valid == false`) with debounce (no repeat until recovery).

Port the semantics from `diagnosis_layer/{aggregator.py}` and the RFC-009 thresholds.

- [ ] Write failing tests for window stats, out-of-order timestamps, threshold trigger/recovery. Commit.

### Task 14: RAG retrieval

**Files:** `services/diagnosis/src/rag.rs`; Test: `tests/rag.rs`

**Interfaces:** `Retriever { fn retrieve(&self, snapshot: &Snapshot, k: usize) -> Vec<Chunk> }`; keyword fallback when embeddings unavailable; cosine scoring.

- [ ] Test keyword retrieval + top-k ordering. Commit.

### Task 15: LLM client + JSON schema validation

**Files:** `services/diagnosis/src/{llm,schema}.rs`; Test: `tests/llm.rs`

**Interfaces:** `trait LlmClient { async fn diagnose(&self, prompt: &str) -> Result<DiagnosisJson> }`; `OpenAiClient` via `async-openai` honoring `base_url`/`api_key`/`model`; retry ≤3; confidence threshold filtering.

- [ ] Test schema accept/reject + confidence filtering with a `MockLlm`. Commit.

### Task 16: Diagnosis node wiring

**Files:** `ros2_ws/src/diagnosis/` rclrs crate (feature `ros`)

- [ ] Subscribe `/physio/{data_src}`, run aggregator/anomaly, publish `/diagnosis/results` and `/diagnosis/monitor`. Verify in Jazzy container. Commit.

---

## Phase 3 — ROS Rust Nodes

### Task 17: Reorganize rosidl interfaces under `robot/interfaces`

- [ ] Create three interface packages mirroring current `.msg/.srv/.action`; delete old packages after downstream crates compile. Verify `colcon build`. Commit.

### Task 18: `control` crate: planner + explicit FSM (pure logic, no ROS)

- [ ] Port `graph.py` Dijkstra and `fsm.py` transitions to Rust enums. Tests: shortest path, no-route, partial; FSM valid/invalid transitions. `cargo test`. Commit.

### Task 19: `control` crate: ExecTask action server (rclrs)

- [ ] Wire FSM + planner + `RobotBackend`. Remove nested spin; use rclrs async. Test in Jazzy. Commit.

### Task 20: `perception` crate: AprilTag + camera (rclrs)

- [ ] `opencv` detection + `nokhwa` capture; publish at ≥10 Hz including empty arrays. Verify with recorded frames. Commit.

### Task 21: `robot_driver` crate: trait + Mock/Sim/Unitree

- [ ] `RobotBackend` trait; `MockBackend` unit-tested; `SimBackend` via `mujoco-rs`; `UnitreeBackend` from Task 6 spike. Commit.

---

## Phase 4 — Industrial Deployment

### Task 22: Per-package Debian packaging

- [ ] `deploy/debian/` with `bloom-generate rosdebian` for ROS/interface packages and `cargo-deb` metadata for services. CI step builds `.deb`s. Commit.

### Task 23: systemd units + config + credentials

- [ ] One unit per node/service under `deploy/systemd/`; config defaults under `deploy/config/`, overrides in `/etc/ros/`; secrets via `LoadCredentialEncrypted=`. Commit.

### Task 24: Signed apt repository

- [ ] `deploy/apt/publish.sh` using aptly snapshots + GPG signing; document install steps. Commit.

### Task 25: RAUC A/B OTA

- [ ] `deploy/rauc/system.conf` + bundle recipe; mutable state on separate partition. Document. Commit.

---

## Self-Review

**Spec coverage:** Platform decision (Tasks 5,6), language decision (Tasks 3,21), repo structure (Tasks 3,17), gateway (7–12), diagnosis (13–16), ROS nodes (17–21), deployment (22–25), CI (4), testing (all), risks (Task 6 gate). Covered.

**Placeholder scan:** No TBD/TODO. Phase 3/4 tasks use checklists rather than full code because they depend on the Phase 0 gate outcome; each names its deliverable and verification command.

**Type consistency:** `Goal`/`TaskRecord` names match the Python wire fields; `ApiError`/`ErrorCode` used consistently; `RosBridge` naming consistent across Tasks 11/12/16.
