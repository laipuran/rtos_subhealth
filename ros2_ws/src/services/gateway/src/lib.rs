use axum::{routing::get, Router};

mod dto;
mod handlers;
mod state;

pub use dto::CreateTask;
pub use state::AppState;

use crate::handlers::{create_task, events, get_task, list_tasks};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/{id}", get(get_task))
        .route("/api/v1/events", get(events))
        .with_state(state)
}
