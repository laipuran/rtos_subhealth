# Diagnosis Event Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist ROS diagnosis results and publish diagnosis and vital-monitor events through the Gateway WebSocket.

**Architecture:** `gateway_bridge` owns ROS subscriptions and translates generated ROS messages into Gateway-owned JSON and `DiagnosisRecord` values. Diagnosis results are persisted before broadcast; monitor samples are best-effort WebSocket telemetry. The root Cargo workspace remains independent of ROS message crates.

**Tech Stack:** Rust 1.85, ROS 2 Jazzy, rclrs 0.7, Axum, rusqlite, serde_json.

**Spec:** `docs/superpowers/specs/2026-09-13-industrial-control-plane-design.md`

## Global Constraints

- Keep generated ROS message dependencies outside the root Cargo workspace.
- Persist a diagnosis result before broadcasting its `diagnosis` event.
- Keep monitor/vitals events best-effort and do not add a persistent vitals store in this phase.
- Preserve the WebUI's existing diagnosis and vitals JSON field names.
- Validate the bridge only through the pinned Jazzy container using `make ros`.

---

### Task 1: Subscribe and Project Diagnosis Streams

**Files:**
- Modify: `ros2_ws/src/robot/gateway_bridge/Cargo.toml`
- Modify: `ros2_ws/src/robot/gateway_bridge/package.xml`
- Modify: `ros2_ws/src/robot/gateway_bridge/src/main.rs`

**Interfaces:**
- Consumes: `/diagnosis/results` as `diagnosis_interfaces::msg::DiagnosisResult`.
- Consumes: `/diagnosis/monitor` as `diagnosis_interfaces::msg::VitalsStream`.
- Produces: `DiagnosisStore::add(&DiagnosisRecord)` followed by `EventHub::broadcast(Event::diagnosis(...))`.
- Produces: `EventHub::broadcast(Event::vitals(...))`.

- [ ] **Step 1: Add the generated interface dependency**

Add `diagnosis_interfaces = { version = "0.1" }` beside the existing ROS interface dependencies in the bridge Cargo manifest. Add `<depend>diagnosis_interfaces</depend>` in `package.xml`.

- [ ] **Step 2: Extract pure record and monitor payload mapping helpers**

In `gateway_bridge/src/main.rs`, add helpers accepting `DiagnosisResult` and `VitalsStream`. Map each `DiagnosisMetric` to:

```rust
json!({
    "data_src": metric.data_src,
    "data_type": metric.data_type,
    "latest": metric.latest,
    "mean": metric.mean,
    "min": metric.min,
    "max": metric.max,
    "trend": metric.trend,
    "valid": metric.valid,
})
```

Construct `DiagnosisRecord` from every diagnosis message field. Convert `confidence` to `f64`. Use gateway receipt time for `created_at` when ROS timestamps are zero; otherwise convert ROS `sec` and `nanosec` to seconds. Build vitals payloads with `timestamp` and `metrics`.

- [ ] **Step 3: Create retained ROS subscriptions in `ros_main`**

Pass `Arc<DiagnosisStore>` from `main` into `ros_main`. Retain both subscriptions in local bindings until `executor.spin`:

```rust
let _diagnosis_subscription = node.create_subscription::<DiagnosisResult, _>(
    "/diagnosis/results",
    move |message: DiagnosisResult| {
        let record = diagnosis_record_from_message(message);
        if diagnoses.add(&record).is_ok() {
            hub.broadcast(gateway::api::ws::Event::diagnosis(record.to_wire_dict()));
        }
    },
)?;
```

Create the monitor subscription with the same pattern, calling `Event::vitals(vitals_payload_from_message(message))` without persistence.

- [ ] **Step 4: Build the actual Jazzy bridge**

Run: `make ros`

Expected: the generated diagnosis interfaces and `gateway_bridge` compile in the Jazzy container.

- [ ] **Step 5: Commit the bridge event flow**

```bash
git add ros2_ws/src/robot/gateway_bridge/Cargo.toml ros2_ws/src/robot/gateway_bridge/package.xml ros2_ws/src/robot/gateway_bridge/src/main.rs
```

### Task 2: Verify Delivery Contract

**Files:**
- Modify: `ros2_ws/src/robot/diagnosis_node/src/main.rs`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: non-zero ROS timestamps on published diagnosis and monitor messages.
- Produces: CI evidence that the Jazzy bridge still builds.

- [ ] **Step 1: Set timestamps at the diagnosis producer**

Before publishing `DiagnosisResult` and `VitalsStream`, use the ROS node clock to populate `timestamp.sec` and `timestamp.nanosec`. The bridge then carries producer time instead of falling back to receipt time.

- [ ] **Step 2: Add a Jazzy bridge build verification in CI**

Keep the existing ROS CI `bash docker/dev/build_ros_rust.sh` command, which rebuilds `diagnosis_node`, generated interfaces, and `gateway_bridge` together. Do not duplicate the command.

- [ ] **Step 3: Run host and container verification**

Run: `cargo test --workspace`

Expected: PASS; no root crate imports ROS messages.

Run: `make ros`

Expected: PASS; both ROS publishers and subscriptions compile.

- [ ] **Step 4: Commit producer timestamp support**

```bash
git add ros2_ws/src/robot/diagnosis_node/src/main.rs
```
