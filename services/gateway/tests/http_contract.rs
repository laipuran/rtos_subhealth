use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::Router;
use gateway::app::{build_router, AppState};
use gateway::bridge::mock::MockBridge;
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

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn create_task_returns_201_and_dispatches_to_bridge() {
    let app = test_app("");
    let body = json!({"goal": {"type": "go_to_tag", "target_tags": [42]}}).to_string();
    let (status, body, headers) = send(&app.router, post_json("/api/v1/tasks", &body)).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "accepted");
    assert_eq!(body["type"], "go_to_tag");
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
    let body = json!({"goal": {"type": "teleport"}}).to_string();
    let (status, body, _) = send(&app.router, post_json("/api/v1/tasks", &body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_GOAL");
}

#[tokio::test]
async fn list_tasks_returns_pagination() {
    let app = test_app("");
    for _ in 0..3 {
        let body = json!({"goal": {"type": "hold"}}).to_string();
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
    let body = json!({"goal": {"type": "hold"}, "goal_id": "T-9"}).to_string();
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
