use anyhow::Result;
use gateway_api::app;
use huge_router_config::GatewayApiConfig;
use runtime_composition::{ServiceRuntime, announce_startup};
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

    let config = GatewayApiConfig::from_env()?;
    let bind_address = config.bind_address;
    let listener = TcpListener::bind(&bind_address).await?;

    info!(bind_address, "gateway-api bootstrap HTTP server listening");

    axum::serve(listener, app()).await?;

    Ok(())
}
