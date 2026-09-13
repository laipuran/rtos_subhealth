use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::Router;
use gateway::app::{build_router, AppState};
use gateway::bridge::mock::MockBridge;
use gateway::bridge::BridgeCommand;
use gateway::config::Config;
use gateway::model::task::TaskUpdate;
use gateway::store::task_store::TaskStore;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

struct TestApp {
    router: Router,
    bridge: Arc<MockBridge>,
    tasks: Arc<TaskStore>,
    _dir: tempfile::TempDir,
}

fn test_app(api_token: &str) -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let maps = dir.path().join("maps");
    std::fs::create_dir_all(&maps).unwrap();
    std::fs::write(
        maps.join("default.json"),
        br#"{"tags":{"1":{"name":"a","x":0.0,"y":0.0}},"edges":[],"routes":{}}"#,
    )
    .unwrap();
    let cfg = Config {
        db_dir: dir.path().to_path_buf(),
        maps_dir: maps,
        api_token: api_token.into(),
        webui_dir: None,
        ..Config::default()
    };
    let bridge = Arc::new(MockBridge::new());
    let state = AppState::new(cfg, bridge.clone()).unwrap();
    let tasks = state.tasks.clone();
    TestApp {
        router: build_router(state),
        bridge,
        tasks,
        _dir: dir,
    }
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value, HeaderMap) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body, headers)
}

fn post_json(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn post_task(app: &TestApp, body: Value) -> (StatusCode, Value, HeaderMap) {
    send(&app.router, post_json("/api/v1/tasks", &body.to_string())).await
}

async fn assert_invalid_goal(response: (StatusCode, Value, HeaderMap)) {
    assert_eq!(response.0, StatusCode::BAD_REQUEST);
    assert_eq!(response.1["error"]["code"], "INVALID_GOAL");
}

fn dispatched_goal(app: &TestApp) -> gateway::model::task::Goal {
    match app.bridge.commands().pop().unwrap() {
        BridgeCommand::SendGoal { goal, .. } => *goal,
        command => panic!("expected SendGoal, got {command:?}"),
    }
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

async fn get_map(app: &TestApp, scene: &str) -> (StatusCode, Value, HeaderMap) {
    send(&app.router, get(&format!("/api/v1/map?scene={scene}"))).await
}

async fn put_map(app: &TestApp, scene: &str, map: Value) -> (StatusCode, Value, HeaderMap) {
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/map?scene={scene}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(map.to_string()))
        .unwrap();
    send(&app.router, request).await
}

fn valid_map() -> Value {
    json!({"tags": {}, "edges": [], "routes": {}})
}

async fn assert_invalid_param(response: (StatusCode, Value, HeaderMap)) {
    assert_eq!(response.0, StatusCode::BAD_REQUEST);
    assert_eq!(response.1["error"]["code"], "INVALID_PARAM");
}

#[tokio::test]
async fn create_task_returns_201_and_dispatches_to_bridge() {
    let app = test_app("");
    let body = json!({"target_device": "mock", "goal": {"type": "go_to_tag", "target_tags": [42]}})
        .to_string();
    let (status, body, headers) = send(&app.router, post_json("/api/v1/tasks", &body)).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "accepted");
    assert_eq!(body["type"], "move_to_pose");
    assert!(!body["task_id"].as_str().unwrap().is_empty());
    assert!(body["trace_id"].is_string());
    assert_eq!(
        headers["x-trace-id"].to_str().unwrap(),
        body["trace_id"].as_str().unwrap()
    );
    assert_eq!(app.bridge.commands().len(), 1);
}

#[tokio::test]
async fn invalid_json_returns_invalid_json() {
    let app = test_app("");
    let (status, body, _) = send(&app.router, post_json("/api/v1/tasks", "{")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_JSON");
}

#[tokio::test]
async fn invalid_goal_type_returns_invalid_goal() {
    let app = test_app("");
    let body = json!({"target_device": "mock", "goal": {"type": "teleport"}}).to_string();
    let (status, body, _) = send(&app.router, post_json("/api/v1/tasks", &body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_GOAL");
}

#[tokio::test]
async fn list_tasks_returns_pagination() {
    let app = test_app("");
    for _ in 0..3 {
        let body = json!({"target_device": "mock", "goal": {"type": "hold"}}).to_string();
        let _ = send(&app.router, post_json("/api/v1/tasks", &body)).await;
    }
    let (status, body, _) = send(&app.router, get("/api/v1/tasks?offset=0&limit=2")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["tasks"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn non_integer_pagination_returns_invalid_param() {
    let app = test_app("");
    let (status, body, _) = send(&app.router, get("/api/v1/tasks?limit=abc")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_PARAM");
}

#[tokio::test]
async fn get_unknown_task_returns_404() {
    let app = test_app("");
    let (status, body, _) = send(&app.router, get("/api/v1/tasks/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn cancel_final_task_returns_invalid_state() {
    let app = test_app("");
    let body =
        json!({"target_device": "mock", "goal": {"type": "hold"}, "goal_id": "T-9"}).to_string();
    send(&app.router, post_json("/api/v1/tasks", &body)).await;
    app.tasks
        .update(
            "T-9",
            &TaskUpdate {
                state: Some("succeeded".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let (status, body, _) = send(&app.router, post_json("/api/v1/tasks/T-9/cancel", "{}")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_STATE");
}

#[tokio::test]
async fn map_supports_etag_and_304() {
    let app = test_app("");
    let (status, _body, headers) = send(&app.router, get("/api/v1/map")).await;
    assert_eq!(status, StatusCode::OK);
    let etag = headers["etag"].to_str().unwrap().to_string();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/map")
        .header("if-none-match", &etag)
        .body(Body::empty())
        .unwrap();
    let (status, body, _) = send(&app.router, req).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert!(body.is_null());
}

#[tokio::test]
async fn map_scene_rejects_traversal_for_reads_and_writes() {
    let app = test_app("");
    assert_invalid_param(get_map(&app, "../tasks").await).await;
    assert_invalid_param(put_map(&app, "../../outside", valid_map()).await).await;
}

#[tokio::test]
async fn map_scene_accepts_a_valid_named_scene() {
    let app = test_app("");
    let response = put_map(&app, "ward_2-night", valid_map()).await;
    assert_eq!(response.0, StatusCode::OK);
    assert!(app._dir.path().join("maps/ward_2-night.json").exists());
}

#[tokio::test]
async fn auth_requires_api_key_when_enabled() {
    let app = test_app("secret");
    let (status, body, _) = send(&app.router, get("/api/v1/tasks")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/tasks")
        .header("x-api-key", "secret")
        .body(Body::empty())
        .unwrap();
    let (status, _body, _) = send(&app.router, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn create_device_task_dispatches_and_persists_fields() {
    let app = test_app("");
    let body = json!({
        "device_id": "mock",
        "primitive": "execute_primitive",
        "target": {"kind": "action", "action_id": "wave"},
        "params_json": "{\"action\":\"wave\"}"
    })
    .to_string();
    let (status, body, _) = send(&app.router, post_json("/api/v1/tasks", &body)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "accepted");
    assert_eq!(body["type"], "execute_primitive");
    assert_eq!(app.bridge.commands().len(), 1);

    let goal_id = body["task_id"].as_str().unwrap();
    let (status, record, _) = send(&app.router, get(&format!("/api/v1/tasks/{goal_id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(record["device_id"], "mock");
    assert_eq!(record["primitive"], "execute_primitive");
    assert_eq!(record["target"]["action_id"], "wave");
}

#[tokio::test]
async fn invalid_device_primitive_returns_invalid_goal() {
    let app = test_app("");
    let body = json!({"device_id": "mock", "primitive": "teleport"}).to_string();
    let (status, body, _) = send(&app.router, post_json("/api/v1/tasks", &body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_GOAL");
}

#[tokio::test]
async fn canonical_navigation_dispatches_tag_target() {
    let app = test_app("");
    let body = json!({
        "device_id": "mock",
        "primitive": "move_to_pose",
        "target": {"kind": "tag", "tag_id": 12},
        "params_json": "{}",
        "constraints": {"max_speed_mps": 0.0, "min_clearance_m": 0.0, "avoid_tags": []},
        "deadline_ms": 0
    });
    let response = post_task(&app, body).await;
    assert_eq!(response.0, StatusCode::CREATED);
    let sent = dispatched_goal(&app);
    assert_eq!(sent.primitive, "move_to_pose");
    assert_eq!(sent.target.unwrap().tag_id, 12);
}

#[tokio::test]
async fn canonical_task_requires_device_and_primitive() {
    let app = test_app("");
    assert_invalid_goal(post_task(&app, json!({"primitive": "hold"})).await).await;
    assert_invalid_goal(post_task(&app, json!({"device_id": "mock"})).await).await;
}

#[tokio::test]
async fn legacy_go_to_tag_becomes_move_to_pose() {
    let app = test_app("");
    let response = post_task(
        &app,
        json!({
            "target_device": "mock",
            "goal": {"type": "go_to_tag", "target_tags": [7]}
        }),
    )
    .await;
    assert_eq!(response.0, StatusCode::CREATED);
    let sent = dispatched_goal(&app);
    assert_eq!(sent.primitive, "move_to_pose");
    assert_eq!(sent.target.unwrap().kind, "tag");
}

#[tokio::test]
async fn legacy_navigation_with_multiple_tags_is_rejected() {
    let app = test_app("");
    let response = post_task(
        &app,
        json!({
            "target_device": "mock",
            "goal": {"type": "go_to_tag", "target_tags": [7, 8]}
        }),
    )
    .await;
    assert_invalid_goal(response).await;
}

#[tokio::test]
async fn canonical_navigation_rejects_out_of_range_tag_ids_without_dispatch() {
    let app = test_app("");
    for tag_id in [
        i64::from(i32::MAX) + 1,
        i64::from(i32::MIN) - 1,
        (1_i64 << 32) + 1,
    ] {
        let response = post_task(
            &app,
            json!({
                "device_id": "mock",
                "primitive": "move_to_pose",
                "target": {"kind": "tag", "tag_id": tag_id}
            }),
        )
        .await;
        assert_invalid_goal(response).await;
    }
    assert!(app.bridge.commands().is_empty());
}

#[tokio::test]
async fn legacy_navigation_rejects_out_of_range_tag_ids_without_dispatch() {
    let app = test_app("");
    for tag_id in [
        i64::from(i32::MAX) + 1,
        i64::from(i32::MIN) - 1,
        (1_i64 << 32) + 1,
    ] {
        let response = post_task(
            &app,
            json!({
                "target_device": "mock",
                "goal": {"type": "go_to_tag", "target_tags": [tag_id]}
            }),
        )
        .await;
        assert_invalid_goal(response).await;
    }
    assert!(app.bridge.commands().is_empty());
}

#[tokio::test]
async fn canonical_task_rejects_malformed_optional_fields_without_dispatch() {
    let app = test_app("");
    for malformed_field in [
        json!({"target": "tag"}),
        json!({"params_json": 1}),
        json!({"deadline_ms": "soon"}),
        json!({"constraints": []}),
        json!({"constraints": {"avoid_tags": "not-an-array"}}),
    ] {
        let mut body = json!({"device_id": "mock", "primitive": "hold"});
        body.as_object_mut()
            .unwrap()
            .extend(malformed_field.as_object().unwrap().clone());
        assert_invalid_goal(post_task(&app, body).await).await;
    }
    assert!(app.bridge.commands().is_empty());
}

#[tokio::test]
async fn canonical_task_rejects_non_finite_f32_command_fields_without_dispatch() {
    let app = test_app("");
    for malformed_field in [
        json!({"constraints": {"max_speed_mps": 1e100}}),
        json!({"constraints": {"min_clearance_m": 1e100}}),
        json!({"target": {"position_tolerance_m": 1e100}}),
        json!({"target": {"yaw_tolerance_rad": 1e100}}),
    ] {
        let mut body = json!({"device_id": "mock", "primitive": "hold"});
        body.as_object_mut()
            .unwrap()
            .extend(malformed_field.as_object().unwrap().clone());
        assert_invalid_goal(post_task(&app, body).await).await;
    }
    assert!(app.bridge.commands().is_empty());
}
