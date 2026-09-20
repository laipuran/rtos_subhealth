use orchestration::OrchestrationError;
use platform::TaskId;

use crate::{
    dto::{CreateTask, TaskView},
    state::AppState,
};
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

pub async fn list_tasks(State(state): State<AppState>) -> Json<Vec<TaskView>> {
    Json(state.list_tasks().await)
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(input): Json<CreateTask>,
) -> Result<(StatusCode, Json<TaskView>), StatusCode> {
    state
        .create_task(input)
        .await
        .map(|view| (StatusCode::ACCEPTED, Json(view)))
        .map_err(submission_status)
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TaskView>, StatusCode> {
    state
        .task(&TaskId(id))
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn events(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let mut receiver = state.subscribe();
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

fn submission_status(error: OrchestrationError) -> StatusCode {
    match error {
        OrchestrationError::Busy | OrchestrationError::Duplicate => StatusCode::CONFLICT,
        OrchestrationError::Execution(_) => StatusCode::BAD_GATEWAY,
        OrchestrationError::UnknownTask => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
