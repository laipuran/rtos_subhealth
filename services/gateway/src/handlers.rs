use platform::TaskId;

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use platform::TaskState;

use crate::{
    dto::{CreateTask, TaskView},
    state::GatewayState,
};

pub(crate) async fn list_tasks(State(state): State<GatewayState>) -> Json<Vec<TaskView>> {
    Json(state.tasks.read().await.values().cloned().collect())
}

pub(crate) async fn create_task(
    State(state): State<GatewayState>,
    Json(input): Json<CreateTask>,
) -> (StatusCode, Json<TaskView>) {
    (StatusCode::ACCEPTED, Json(state.create_task(input).await))
}

pub(crate) async fn get_task(
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

pub(crate) async fn cancel_task(
    State(state): State<GatewayState>,
    Path(id): Path<String>,
) -> Result<Json<TaskView>, StatusCode> {
    let mut tasks = state.tasks.write().await;
    let task = tasks.get_mut(&TaskId(id)).ok_or(StatusCode::NOT_FOUND)?;
    task.state = TaskState::Canceled;
    task.phase = "canceled".into();
    Ok(Json(task.clone()))
}

pub(crate) async fn events(
    State(state): State<GatewayState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
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
