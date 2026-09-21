use std::sync::Arc;

use execution::Execution;
use gateway::{app, AppState};
use orchestration::Orchestrator;
use platform::DeviceId;
use task_repository::InMemoryTaskRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("GATEWAY_HTTP_PORT").unwrap_or_else(|_| "5000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    tracing_subscriber::fmt::init();

    let device_id = DeviceId(
        std::env::var("GATEWAY_EXECUTION_DEVICE_ID").unwrap_or_else(|_| "mock_exec".into()),
    );
    let action_name = std::env::var("GATEWAY_EXECUTION_ACTION_NAME")
        .unwrap_or_else(|_| "/mock_exec/execute_task".into());
    let execution = Arc::new(Execution::start(device_id, action_name)?);
    let repository = Arc::new(InMemoryTaskRepository::new());
    let state = AppState::new(
        Orchestrator::new(execution.clone(), repository.clone()),
        repository,
    );
    axum::serve(listener, app(state)).await?;
    execution.shutdown()?;
    Ok(())
}
