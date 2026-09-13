# Gateway Command Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Gateway accept explicit device commands without silently changing their semantics and confine map reads and writes to safe scene files.

**Architecture:** The Gateway treats the existing `Goal` as its persisted command representation. HTTP parsing becomes an explicit adapter: canonical commands are validated directly, while the legacy WebUI envelope is translated only for `go_to_tag` and `hold`; all ambiguous commands are rejected. Map handlers resolve validated scene identifiers and replace map files atomically.

**Tech Stack:** Rust 1.85, Axum, serde_json, rusqlite.

**Spec:** `docs/superpowers/specs/2026-09-13-industrial-control-plane-design.md`

## Global Constraints

- Retain persisted `TaskRecord` SQLite compatibility during the migration.
- Reject ambiguous requests; never default an empty primitive to `hold` or device ID to `mock`.
- Use atomic map replacement within `GATEWAY_MAPS_DIR` only.
- Preserve the existing API error envelope and stable error codes.
- Every behavior change starts with a failing test and ends with targeted plus workspace verification.

---

### Task 1: Parse Canonical Tasks and Explicit Legacy Compatibility

**Files:**
- Modify: `services/gateway/src/model/task.rs`
- Modify: `services/gateway/src/api/http.rs`
- Modify: `services/gateway/tests/http_contract.rs`

**Interfaces:**
- Produces: `Goal::from_canonical_value(value: &Value) -> Result<Goal, ApiError>`.
- Produces: `Goal::from_legacy_value(value: &Value) -> Result<Goal, ApiError>`.
- Consumes: canonical `POST /api/v1/tasks` JSON with `device_id`, `primitive`, optional `target`, `params_json`, `constraints`, and `deadline_ms`.
- Consumes: legacy JSON with `target_device` and `goal.type` only as a compatibility path.

- [ ] **Step 1: Add focused failing canonical request tests**

Add these tests to `services/gateway/tests/http_contract.rs`, using the existing app and mock bridge fixture helpers:

```rust
#[tokio::test]
async fn canonical_navigation_dispatches_tag_target() {
    let body = json!({
        "device_id": "mock",
        "primitive": "move_to_pose",
        "target": {"kind": "tag", "tag_id": 12},
        "params_json": "{}",
        "constraints": {"max_speed_mps": 0.0, "min_clearance_m": 0.0, "avoid_tags": []},
        "deadline_ms": 0
    });
    let response = post_task(&app, body).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let sent = bridge.goals().pop().unwrap();
    assert_eq!(sent.primitive, "move_to_pose");
    assert_eq!(sent.target.unwrap().tag_id, 12);
}

#[tokio::test]
async fn canonical_task_requires_device_and_primitive() {
    assert_invalid_goal(post_task(&app, json!({"primitive": "hold"})).await).await;
    assert_invalid_goal(post_task(&app, json!({"device_id": "mock"})).await).await;
}
```

- [ ] **Step 2: Run the new tests and verify failure**

Run: `cargo test -p gateway --test http_contract canonical_`

Expected: FAIL because `create_task` currently expects the legacy `goal` object or falls through to the unrelated primitive parser shape.

- [ ] **Step 3: Implement canonical parsing without defaults**

In `services/gateway/src/model/task.rs`, add `Goal::from_canonical_value`. It must require non-empty `device_id` and `primitive`, populate all `Goal` device-oriented fields, and reject `move_to_pose` without `target.kind == "tag"` or a non-zero tag ID. Do not use the current empty-device or empty-primitive defaults.

Use this construction shape:

```rust
Ok(Goal {
    type_: primitive.clone(),
    priority: 0,
    route_id: String::new(),
    target_tags: target.iter().map(|item| item.tag_id).filter(|id| *id != 0).collect(),
    constraints,
    deadline_ms,
    device_id,
    primitive,
    target,
    params_json,
})
```

In `services/gateway/src/api/http.rs`, select `from_legacy_value` only when `value.get("goal").is_some()`; otherwise call `from_canonical_value`. Convert either error to the existing `INVALID_GOAL` response before persistence or bridge dispatch.

- [ ] **Step 4: Add failing legacy conversion tests**

```rust
#[tokio::test]
async fn legacy_go_to_tag_becomes_move_to_pose() {
    let response = post_task(&app, json!({
        "target_device": "mock",
        "goal": {"type": "go_to_tag", "target_tags": [7]}
    })).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let sent = bridge.goals().pop().unwrap();
    assert_eq!(sent.primitive, "move_to_pose");
    assert_eq!(sent.target.unwrap().kind, "tag");
}

#[tokio::test]
async fn legacy_navigation_with_multiple_tags_is_rejected() {
    let response = post_task(&app, json!({
        "target_device": "mock",
        "goal": {"type": "go_to_tag", "target_tags": [7, 8]}
    })).await;
    assert_invalid_goal(response).await;
}
```

- [ ] **Step 5: Implement the limited legacy adapter**

`Goal::from_legacy_value` must require non-empty `target_device`. It converts `go_to_tag` with exactly one integer tag to canonical `move_to_pose` and `TaskTarget { kind: "tag", tag_id, ..Default::default() }`; it converts `hold` to canonical `hold`; it rejects `patrol_route` and every unknown type as `INVALID_GOAL` until a route primitive is implemented.

- [ ] **Step 6: Verify and commit the command boundary**

Run: `cargo test -p gateway --test http_contract`

Expected: PASS.

Run: `cargo fmt --all --check`

Expected: PASS.

```bash
git add services/gateway/src/model/task.rs services/gateway/src/api/http.rs services/gateway/tests/http_contract.rs
```

### Task 2: Validate and Atomically Replace Map Scenes

**Files:**
- Modify: `services/gateway/src/api/http.rs`
- Modify: `services/gateway/tests/http_contract.rs`

**Interfaces:**
- Produces: `scene_path(maps_dir: &Path, scene: &str) -> Result<PathBuf, ApiError>`.
- Produces: `atomic_write_json(path: &Path, value: &Value) -> std::io::Result<()>`.
- Consumes: `scene` query values matching `[A-Za-z0-9][A-Za-z0-9_-]{0,63}`.

- [ ] **Step 1: Add failing invalid-scene and valid-scene tests**

```rust
#[tokio::test]
async fn map_scene_rejects_traversal_for_reads_and_writes() {
    assert_invalid_param(get_map(&app, "../tasks").await).await;
    assert_invalid_param(put_map(&app, "../../outside", valid_map()).await).await;
}

#[tokio::test]
async fn map_scene_accepts_a_valid_named_scene() {
    let response = put_map(&app, "ward_2-night", valid_map()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(maps_dir.path().join("ward_2-night.json").exists());
}
```

- [ ] **Step 2: Run the new tests and verify failure**

Run: `cargo test -p gateway --test http_contract map_scene_`

Expected: FAIL because handlers currently interpolate traversal input into `maps_dir.join`.

- [ ] **Step 3: Implement scene validation and atomic replacement**

Add a local `scene_path` helper in `services/gateway/src/api/http.rs`. Validate byte-by-byte: the first byte is ASCII alphanumeric; every later byte is ASCII alphanumeric, `_`, or `-`; total length is at most 64. Return `ErrorCode::InvalidParam` for any violation.

Add `atomic_write_json`: serialize `Value` before creating a temporary file in `path.parent()`, write all bytes, `sync_all`, rename the temporary file to `path`, then best-effort sync the parent directory. Use a UUID suffix for the temporary filename. Replace direct `std::fs::write` in `put_map`; both handlers must use `scene_path`.

- [ ] **Step 4: Verify and commit map safety**

Run: `cargo test -p gateway`

Expected: PASS.

Run: `cargo fmt --all --check`

Expected: PASS.

```bash
git add services/gateway/src/api/http.rs services/gateway/tests/http_contract.rs
```

### Task 3: Verify the Completed Phase

**Files:**
- Modify: none

**Interfaces:**
- Produces: evidence that the root Rust quality gates remain valid after both boundary changes.

- [ ] **Step 1: Run the full Rust quality gate**

Run: `cargo fmt --all --check`

Expected: PASS.

Run: `cargo clippy --all-targets -- -D warnings`

Expected: PASS.

Run: `cargo test --workspace`

Expected: PASS.

- [ ] **Step 2: Record verification in the phase report**

Use the command output in the final phase report. Do not create an empty commit.
