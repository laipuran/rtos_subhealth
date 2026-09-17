use gateway::{app, GatewayState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("GATEWAY_HTTP_PORT").unwrap_or_else(|_| "5000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing_subscriber::fmt::init();
    axum::serve(listener, app(GatewayState::new())).await?;
    Ok(())
}
