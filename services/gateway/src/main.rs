use gateway::app;
use gateway::GatewayState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("GATEWAY_HTTP_PORT").unwrap_or_else(|_| "5000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    tracing_subscriber::fmt::init();

    let state = GatewayState::new();
    axum::serve(listener, app(state)).await?;
    Ok(())
}
