use anyhow::{Context, Result};
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, run_nats_consumer,
};
use sqlx::PgPool;
use std::{env, sync::Arc};
use tracing::{info, warn};

mod processor;

const WORKER_PREFIX: &str = "NOTIFICATION_WORKER";
const SERVICE_NAME: &str = "notification-worker";
const WORKER_ROLE: &str = "notification-fanout";
const DEFAULT_NATS_SUBJECT: &str = "events.provider_resource.*";

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = ServiceRuntime {
        service_name: SERVICE_NAME,
        role: WORKER_ROLE,
    };

    init_tracing(&runtime);
    announce_startup(runtime);

    let config = load_worker_config(
        WORKER_PREFIX,
        SERVICE_NAME,
        WORKER_ROLE,
        DEFAULT_NATS_SUBJECT,
    );
    let database_url = config
        .database_url
        .clone()
        .context("NOTIFICATION_WORKER_DATABASE_URL or DATABASE_URL is required")?;
    let rate_limit_seconds = env::var("NOTIFICATION_WORKER_RATE_LIMIT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(300);

    let pool = PgPool::connect(&database_url).await?;
    processor::ensure_notification_tables(&pool).await?;

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        rate_limit_seconds,
        "notification worker ready"
    );

    let shutdown = install_shutdown_listener();
    let pool = Arc::new(pool);
    let handler: MessageHandler = Arc::new(move |message| {
        let pool = Arc::clone(&pool);
        Box::pin(async move {
            match processor::handle_notification_envelope(
                &pool,
                &message.payload,
                rate_limit_seconds,
            )
            .await
            {
                Ok(outcome) => {
                    info!(
                        incident_key = outcome.envelope.incident_key,
                        message_type = outcome.envelope.message_type,
                        severity = outcome.envelope.severity.as_str(),
                        suppressed = outcome.suppressed,
                        persisted = outcome.persisted,
                        "notification worker processed incident event"
                    );
                }
                Err(error) => {
                    warn!(error = %error, "notification worker received malformed incident envelope");
                }
            }

            Ok(())
        })
    });

    run_nats_consumer(&config.nats, handler, shutdown).await
}
