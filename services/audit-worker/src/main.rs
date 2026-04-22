use anyhow::{Context, Result};
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, run_nats_consumer,
};
use sqlx::PgPool;
use std::{env, sync::Arc};
use tracing::{info, warn};

mod processor;

const WORKER_PREFIX: &str = "AUDIT_WORKER";
const SERVICE_NAME: &str = "audit-worker";
const WORKER_ROLE: &str = "audit-pipeline";
const DEFAULT_NATS_SUBJECT: &str = "events.audit_event.created";

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
        .context("AUDIT_WORKER_DATABASE_URL or DATABASE_URL is required")?;
    let retention_days = env::var("AUDIT_WORKER_RETENTION_DAYS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);

    let pool = PgPool::connect(&database_url).await?;
    processor::ensure_audit_table(&pool).await?;
    let pruned = processor::prune_audit_events(&pool, retention_days).await?;

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        retention_days,
        pruned_rows = pruned,
        "audit worker ready"
    );

    let shutdown = install_shutdown_listener();
    let pool = Arc::new(pool);
    let handler: MessageHandler = std::sync::Arc::new(move |message| {
        let pool = Arc::clone(&pool);
        Box::pin(async move {
            match processor::handle_audit_envelope(&pool, &message.payload, retention_days).await {
                Ok(outcome) => {
                    info!(
                        message_type = outcome.envelope.message_type,
                        message_id = outcome.envelope.message_id,
                        producer = outcome.envelope.producer,
                        trace_id = outcome.envelope.trace_id,
                        request_id = outcome.envelope.request_id,
                        rows_pruned = outcome.pruned_rows,
                        persisted = outcome.persisted,
                        "audit worker handled event envelope"
                    );
                }
                Err(error) => {
                    warn!(error = %error, "audit worker received malformed envelope");
                }
            }
            Ok(())
        })
    });

    run_nats_consumer(&config.nats, handler, shutdown).await
}
