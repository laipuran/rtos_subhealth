use std::net::SocketAddr;
use std::sync::Arc;

use gateway::app::{build_router, AppState};
use gateway::bridge::mock::MockBridge;
use gateway::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env();
    // The production `RosBridge` (feature `ros`) is wired up in Phase 3. Until
    // then the gateway runs with an in-process bridge so the API is usable.
    let bridge = Arc::new(MockBridge::new());
    let state = AppState::new(config, bridge)?;

    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.http_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("gateway listening on http://{addr}");
    axum::serve(listener, build_router(state)).await?;
    Ok(())
}
