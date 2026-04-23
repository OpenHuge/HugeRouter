use anyhow::Result;
use realtime_gateway::{RealtimeGatewayConfig, serve};
use runtime_composition::{
    ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
};
use tokio::net::TcpListener;
use tracing::warn;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = ServiceRuntime {
        service_name: "realtime-gateway",
        role: "bidirectional-ingress",
    };
    init_tracing(&runtime);

    let config = RealtimeGatewayConfig::from_env()?;
    if config.uses_insecure_default_secret {
        warn!("REALTIME_GATEWAY_SIGNING_SECRET is not set; using the built-in development secret");
    }

    let listener = TcpListener::bind(config.bind_addr).await?;
    announce_startup(runtime);

    let shutdown = install_shutdown_listener();
    let server = serve(listener, config);

    tokio::select! {
        result = server => result,
        () = shutdown.wait() => Ok(()),
    }
}
