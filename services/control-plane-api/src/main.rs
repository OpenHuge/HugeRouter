use anyhow::Result;
use control_plane_api::{app, bootstrap, migrate, status};
use runtime_composition::{ServiceRuntime, announce_startup};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "serve".to_string());

    match command.as_str() {
        "migrate" => return migrate().await,
        "bootstrap" => return bootstrap().await,
        "status" => {
            println!("{}", status().await?);
            return Ok(());
        }
        "serve" => {}
        other => anyhow::bail!(
            "unsupported control-plane-api command `{other}`; expected serve|migrate|bootstrap|status"
        ),
    }

    announce_startup(ServiceRuntime {
        service_name: "control-plane-api",
        role: "admin-api",
    });

    let bind_address =
        std::env::var("CONTROL_PLANE_API_ADDR").unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    let listener = TcpListener::bind(&bind_address).await?;

    info!(
        bind_address,
        "control-plane-api bootstrap HTTP server listening"
    );

    axum::serve(listener, app().await?).await?;

    Ok(())
}
