use axum::{
    routing::{get, post},
    Router,
};

mod dto;
mod handlers;
mod state;

pub use dto::{CreateTask, TaskView};
pub use state::GatewayState;

use crate::handlers::{cancel_task, create_task, events, get_task, list_tasks};

pub fn app(state: GatewayState) -> Router {
    Router::new()
        .route("/api/v1/tasks", get(list_tasks).post(create_task))
        .route("/api/v1/tasks/{id}", get(get_task))
        .route("/api/v1/tasks/{id}/cancel", post(cancel_task))
        .route("/api/v1/events", get(events))
        .with_state(state)
}
