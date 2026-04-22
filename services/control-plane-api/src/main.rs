use anyhow::Result;
use control_plane_api::app;
use runtime_composition::{announce_startup, ServiceRuntime};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    announce_startup(ServiceRuntime {
        service_name: "control-plane-api",
        role: "admin-api",
    });

    let bind_address = std::env::var("CONTROL_PLANE_API_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    let listener = TcpListener::bind(&bind_address).await?;

    info!(bind_address, "control-plane-api bootstrap HTTP server listening");

    axum::serve(listener, app().await?).await?;

    Ok(())
}
