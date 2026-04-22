use anyhow::Result;
use runtime_composition::{ServiceRuntime, announce_startup};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    announce_startup(ServiceRuntime {
        service_name: "audit-worker",
        role: "audit-pipeline",
    });

    Ok(())
}
