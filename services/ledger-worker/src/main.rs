use anyhow::{Context, Result};
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, run_nats_consumer,
};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

mod processor;

use processor::{IngestionOutcome, handle_usage_event_recorded};

const WORKER_PREFIX: &str = "LEDGER_WORKER";
const SERVICE_NAME: &str = "ledger-worker";
const WORKER_ROLE: &str = "usage-ledger-worker";
const DEFAULT_NATS_SUBJECT: &str = "events.usage_event.recorded";

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
        .context("LEDGER_WORKER_DATABASE_URL or DATABASE_URL is required")?;

    let pool = PgPool::connect(&database_url)
        .await
        .context("failed to connect to postgres")?;
    processor::ensure_ledger_table(&pool)
        .await
        .context("failed to create ledger_entries table")?;

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        queue_group = config.nats.queue_group,
        "ledger worker ready"
    );

    let shutdown = install_shutdown_listener();
    let pool = Arc::new(pool);
    let handler = make_usage_event_handler(Arc::clone(&pool));
    run_nats_consumer(&config.nats, handler, shutdown).await
}

fn make_usage_event_handler(pool: Arc<PgPool>) -> MessageHandler {
    std::sync::Arc::new(move |message| {
        let pool = Arc::clone(&pool);
        Box::pin(async move {
            match handle_usage_event_recorded(&pool, &message.payload).await {
                Ok(IngestionOutcome::Inserted) => {
                    info!(
                        bytes = message.payload.len(),
                        "ledger worker persisted usage event"
                    );
                }
                Ok(IngestionOutcome::Duplicate) => {
                    info!(
                        bytes = message.payload.len(),
                        "ledger worker deduplicated usage event"
                    );
                }
                Err(error) => {
                    tracing::warn!(error = %error, "ledger worker failed to handle usage event");
                }
            }

            Ok(())
        })
    })
}
