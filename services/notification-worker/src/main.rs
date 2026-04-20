use anyhow::Result;
use runtime_composition::{announce_startup, ServiceRuntime};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    announce_startup(ServiceRuntime {
        service_name: "notification-worker",
        role: "notification-fanout",
    });

    Ok(())
}

