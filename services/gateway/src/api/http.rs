//! RFC-005 HTTP API. Paths, status codes, error envelope, pagination, ETag,
//! `trace_id` and `X-API-Key` auth all match the legacy Flask implementation.

use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use md5::{Digest, Md5};
use serde_json::{json, Value};

use crate::api::{
    check_auth, error_response, etag_header, json_response, parse_pagination, trace_id,
};
use crate::app::AppState;
use crate::error::{ApiError, ErrorCode};
use crate::model::task::{Goal, TaskRecord};

const VALID_PRIMITIVES: [&str; 5] = [
    "move_to_pose",
    "set_velocity",
    "hold",
    "stop",
    "execute_primitive",
];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/{goal_id}", get(get_task))
        .route("/api/v1/tasks/{goal_id}/cancel", post(cancel_task))
        .route("/api/v1/map", get(get_map).put(put_map))
        .route(
            "/api/v1/diagnostics",
            get(list_diagnoses).post(trigger_diagnosis),
        )
        .route("/api/v1/diagnostics/{diagnosis_id}", get(get_diagnosis))
        .route("/api/v1/events", get(crate::api::ws::ws_handler))
}

async fn create_task(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    let value: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return error_response(
                &ApiError::new(ErrorCode::InvalidJson, "request body is not valid JSON"),
                &tid,
            )
        }
    };
    let goal = if value.get("goal").is_some() {
        match Goal::from_legacy_value(&value) {
            Ok(goal) => goal,
            Err(error) => return error_response(&error, &tid),
        }
    } else {
        match Goal::from_canonical_value(&value) {
            Ok(goal) if VALID_PRIMITIVES.contains(&goal.primitive.as_str()) => goal,
            Ok(goal) => {
                return error_response(
                    &ApiError::new(
                        ErrorCode::InvalidGoal,
                        format!(
                            "primitive '{}' must be one of {VALID_PRIMITIVES:?}",
                            goal.primitive
                        ),
                    ),
                    &tid,
                )
            }
            Err(error) => return error_response(&error, &tid),
        }
    };

    let goal_id = value
        .get("goal_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let record = TaskRecord::new(&goal_id, goal.clone());
    if let Err(e) = state.tasks.add(&record) {
        return error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid);
    }
    state.bridge.send_goal(&goal_id, goal.clone());

    json_response(
        201,
        json!({"task_id": goal_id, "status": "accepted", "type": goal.type_}),
        &tid,
    )
}

async fn list_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    let (offset, limit) = match parse_pagination(&query) {
        Some(v) => v,
        None => {
            return error_response(
                &ApiError::new(ErrorCode::InvalidParam, "offset and limit must be integers"),
                &tid,
            )
        }
    };
    let records = match state.tasks.list_all() {
        Ok(r) => r,
        Err(e) => return error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid),
    };
    let total = records.len() as i64;
    let page: Vec<Value> = records
        .iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(TaskRecord::to_wire_dict)
        .collect();
    json_response(
        200,
        json!({"tasks": page, "total": total, "offset": offset, "limit": limit}),
        &tid,
    )
}

async fn get_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(goal_id): Path<String>,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    match state.tasks.get(&goal_id) {
        Ok(Some(record)) => json_response(200, record.to_wire_dict(), &tid),
        Ok(None) => error_response(
            &ApiError::new(ErrorCode::NotFound, format!("task {goal_id} not found")),
            &tid,
        ),
        Err(e) => error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid),
    }
}

async fn cancel_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(goal_id): Path<String>,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    let record = match state.tasks.get(&goal_id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return error_response(
                &ApiError::new(ErrorCode::NotFound, format!("task {goal_id} not found")),
                &tid,
            )
        }
        Err(e) => return error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid),
    };
    if matches!(record.state.as_str(), "succeeded" | "failed" | "canceled") {
        return error_response(
            &ApiError::new(
                ErrorCode::InvalidState,
                format!("task already in final state: {}", record.state),
            ),
            &tid,
        );
    }
    state.bridge.cancel(&goal_id);
    json_response(
        200,
        json!({"task_id": goal_id, "status": "cancel_accepted"}),
        &tid,
    )
}

async fn get_map(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    let scene = query.get("scene").map(String::as_str).unwrap_or("default");
    let path = match scene_path(&state.config.maps_dir, scene) {
        Ok(path) => path,
        Err(error) => return error_response(&error, &tid),
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            return error_response(
                &ApiError::new(ErrorCode::NotFound, format!("map '{scene}' not found")),
                &tid,
            )
        }
    };
    let etag = format!("{:x}", Md5::digest(&bytes));
    let if_none_match = headers
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if if_none_match == etag {
        let mut resp = StatusCode::NOT_MODIFIED.into_response();
        if let Ok(v) = axum::http::HeaderValue::from_str(&tid) {
            resp.headers_mut().insert("x-trace-id", v);
        }
        return resp;
    }
    let data: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    let mut resp = json_response(200, data, &tid);
    if let Some((name, value)) = etag_header(&etag) {
        resp.headers_mut().insert(name, value);
    }
    resp
}

async fn put_map(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    let scene = query.get("scene").map(String::as_str).unwrap_or("default");
    let path = match scene_path(&state.config.maps_dir, scene) {
        Ok(path) => path,
        Err(error) => return error_response(&error, &tid),
    };
    let new: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return error_response(
                &ApiError::new(ErrorCode::InvalidJson, "request body is not valid JSON"),
                &tid,
            )
        }
    };
    let old: Option<Value> = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    if let Some(old) = old {
        let active = state.tasks.list_active().unwrap_or_default();
        if let Some(details) = check_conflicts(&old, &new, &active) {
            return error_response(
                &ApiError::new(
                    ErrorCode::Conflict,
                    "cannot delete items used by active task(s)",
                )
                .with_details(details),
                &tid,
            );
        }
    }
    if let Err(e) = atomic_write_json(&path, &new) {
        return error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid);
    }
    let etag = std::fs::read(&path)
        .map(|b| format!("{:x}", Md5::digest(&b)))
        .unwrap_or_default();
    let mut resp = json_response(200, json!({"status": "saved", "scene": scene}), &tid);
    if let Some((name, value)) = etag_header(&etag) {
        resp.headers_mut().insert(name, value);
    }
    resp
}

fn scene_path(maps_dir: &FsPath, scene: &str) -> Result<PathBuf, ApiError> {
    let bytes = scene.as_bytes();
    let valid = bytes.len() <= 64
        && matches!(bytes.first(), Some(first) if first.is_ascii_alphanumeric())
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
    if !valid {
        return Err(ApiError::new(
            ErrorCode::InvalidParam,
            "scene must match [A-Za-z0-9][A-Za-z0-9_-]{0,63}",
        ));
    }
    Ok(maps_dir.join(format!("{scene}.json")))
}

fn atomic_write_json(path: &FsPath, value: &Value) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "map path has no parent")
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("map");
    let temporary = parent.join(format!(".{name}.{}", uuid::Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    let _ = std::fs::File::open(parent).and_then(|directory| directory.sync_all());
    Ok(())
}

async fn trigger_diagnosis(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    state.bridge.trigger_diagnosis(&format!("manual:{tid}"));
    json_response(
        202,
        json!({"status": "triggered", "trigger_type": "manual"}),
        &tid,
    )
}

async fn list_diagnoses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    let (offset, limit) = match parse_pagination(&query) {
        Some(v) => v,
        None => {
            return error_response(
                &ApiError::new(ErrorCode::InvalidParam, "offset and limit must be integers"),
                &tid,
            )
        }
    };
    let records = match state.diagnoses.list_all(offset, limit) {
        Ok(r) => r,
        Err(e) => return error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid),
    };
    let total = state.diagnoses.count().unwrap_or(0);
    let page: Vec<Value> = records.iter().map(|r| r.to_wire_dict()).collect();
    json_response(
        200,
        json!({"diagnoses": page, "total": total, "offset": offset, "limit": limit}),
        &tid,
    )
}

async fn get_diagnosis(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(diagnosis_id): Path<String>,
) -> Response {
    let tid = trace_id(&headers);
    if let Some(resp) = check_auth(&state.config.api_token, &headers, &tid) {
        return resp;
    }
    match state.diagnoses.get(&diagnosis_id) {
        Ok(Some(rec)) => json_response(200, rec.to_wire_dict(), &tid),
        Ok(None) => error_response(
            &ApiError::new(
                ErrorCode::NotFound,
                format!("diagnosis {diagnosis_id} not found"),
            ),
            &tid,
        ),
        Err(e) => error_response(&ApiError::new(ErrorCode::Internal, e.to_string()), &tid),
    }
}

fn tag_keys(value: &Value) -> BTreeSet<String> {
    value
        .get("tags")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

fn edge_keys(value: &Value) -> BTreeSet<(i64, i64)> {
    value
        .get("edges")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| Some((e.get("from")?.as_i64()?, e.get("to")?.as_i64()?)))
                .collect()
        })
        .unwrap_or_default()
}

fn route_keys(value: &Value) -> BTreeSet<String> {
    value
        .get("routes")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// RFC-007 §5.3: reject deletions that active tasks still reference.
fn check_conflicts(old: &Value, new: &Value, active: &[TaskRecord]) -> Option<Value> {
    let deleted_tags: BTreeSet<String> =
        tag_keys(old).difference(&tag_keys(new)).cloned().collect();
    let deleted_edges = edge_keys(old)
        .difference(&edge_keys(new))
        .cloned()
        .collect::<Vec<_>>();
    let deleted_routes: BTreeSet<String> = route_keys(old)
        .difference(&route_keys(new))
        .cloned()
        .collect();
    if deleted_tags.is_empty() && deleted_edges.is_empty() && deleted_routes.is_empty() {
        return None;
    }

    let mut blocking = Vec::new();
    for task in active {
        let mut reasons = Vec::new();
        for tag in &task.goal.target_tags {
            if deleted_tags.contains(&tag.to_string()) {
                reasons.push(format!("target_tag {tag}"));
            }
        }
        if !task.goal.route_id.is_empty() && deleted_routes.contains(&task.goal.route_id) {
            reasons.push(format!("route_id {}", task.goal.route_id));
        }
        if !reasons.is_empty() {
            blocking.push(json!({
                "goal_id": task.goal_id,
                "type": task.goal.type_,
                "target_tags": task.goal.target_tags,
                "route_id": task.goal.route_id,
                "state": task.state,
                "reasons": reasons,
            }));
        }
    }
    if blocking.is_empty() {
        return None;
    }
    Some(json!({
        "deleted_tags": deleted_tags,
        "deleted_edges": deleted_edges
            .iter()
            .map(|(from, to)| json!({"from": from, "to": to}))
            .collect::<Vec<_>>(),
        "deleted_routes": deleted_routes,
        "blocking_tasks": blocking,
    }))
}

/// Hard-coded default map location used by `main` when no override is given.
pub fn default_maps_dir() -> PathBuf {
    PathBuf::from("config/maps")
}
