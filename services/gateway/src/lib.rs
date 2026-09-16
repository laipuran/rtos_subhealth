use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use domain_contract::TaskId;
use event_contract::SystemEvent;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use task_contract::{Primitive, Task, TaskState, TaskTarget};
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct GatewayState {
    tasks: Arc<RwLock<HashMap<TaskId, TaskView>>>,
    sequence: Arc<AtomicU64>,
    events: broadcast::Sender<(u64, SystemEvent)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskView {
    pub task: Task,
    pub state: TaskState,
    pub progress: f32,
    pub phase: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTask {
    pub device_id: Option<domain_contract::DeviceId>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    pub primitive: Primitive,
    #[serde(default = "default_target")]
    pub target: TaskTarget,
    #[serde(default)]
    pub parameters: serde_json::Value,
    pub deadline_ms: Option<u64>,
}

fn default_target() -> TaskTarget {
    TaskTarget::None
}

impl GatewayState {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            sequence: Arc::new(AtomicU64::new(0)),
            events,
        }
    }

    async fn create(&self, input: CreateTask) -> TaskView {
        let id = TaskId(format!(
            "task-{}",
            self.sequence.fetch_add(1, Ordering::Relaxed) + 1
        ));
        let task = Task {
            id: id.clone(),
            device_id: input.device_id,
            required_capabilities: input.required_capabilities,
            primitive: input.primitive,
            target: input.target,
            parameters: input.parameters,
            deadline_ms: input.deadline_ms,
        };
        let view = TaskView {
            task,
            state: TaskState::Accepted,
            progress: 0.0,
            phase: "accepted".into(),
        };
        self.tasks.write().await.insert(id, view.clone());
        view
    }
}

impl Default for GatewayState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/{id}", get(get_task))
        .route("/api/v1/tasks/{id}/cancel", post(cancel_task))
        .route("/api/v1/events", get(events))
        .with_state(state)
}

async fn list_tasks(State(state): State<GatewayState>) -> Json<Vec<TaskView>> {
    Json(state.tasks.read().await.values().cloned().collect())
}

async fn create_task(
    State(state): State<GatewayState>,
    Json(input): Json<CreateTask>,
) -> (StatusCode, Json<TaskView>) {
    (StatusCode::ACCEPTED, Json(state.create(input).await))
}

async fn get_task(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> Result<Json<TaskView>, StatusCode> {
    state
        .tasks
        .read()
        .await
        .get(&TaskId(id))
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn cancel_task(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> Result<Json<TaskView>, StatusCode> {
    let mut tasks = state.tasks.write().await;
    let task = tasks.get_mut(&TaskId(id)).ok_or(StatusCode::NOT_FOUND)?;
    task.state = TaskState::Canceled;
    task.phase = "canceled".into();
    Ok(Json(task.clone()))
}

async fn events(State(state): State<GatewayState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let mut receiver = state.events.subscribe();
    ws.on_upgrade(move |mut socket| async move {
        while let Ok((sequence, event)) = receiver.recv().await {
            let payload = serde_json::json!({ "sequence": sequence, "event": event });
            if socket
                .send(axum::extract::ws::Message::Text(payload.to_string().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn canonical_task_api_stores_device_neutral_task() {
        let response = app(GatewayState::new())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"primitive":"stop"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }
}
