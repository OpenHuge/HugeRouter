use anyhow::Result;
use gateway_api::app;
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
        service_name: "gateway-api",
        role: "northbound-api",
    });

    let bind_address =
        std::env::var("GATEWAY_API_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = TcpListener::bind(&bind_address).await?;

    info!(bind_address, "gateway-api bootstrap HTTP server listening");

    axum::serve(listener, app()).await?;

    Ok(())
}
