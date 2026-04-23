use anyhow::{Context, Result};
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, run_nats_consumer,
};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

mod processor;

use processor::{IngestionOutcome, handle_route_receipt_recorded};

const WORKER_PREFIX: &str = "ROUTE_RECEIPT_WORKER";
const SERVICE_NAME: &str = "route-receipt-worker";
const WORKER_ROLE: &str = "route-receipt-persistence-worker";
const DEFAULT_NATS_SUBJECT: &str = "events.route_receipt.recorded";

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
        .context("ROUTE_RECEIPT_WORKER_DATABASE_URL or DATABASE_URL is required")?;

    let pool = PgPool::connect(&database_url)
        .await
        .context("failed to connect to postgres")?;
    processor::ensure_route_receipt_tables(&pool)
        .await
        .context("failed to create route receipt tables")?;

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        queue_group = config.nats.queue_group,
        "route receipt worker ready"
    );

    let shutdown = install_shutdown_listener();
    let pool = Arc::new(pool);
    let handler = make_route_receipt_handler(Arc::clone(&pool));
    run_nats_consumer(&config.nats, handler, shutdown).await
}

fn make_route_receipt_handler(pool: Arc<PgPool>) -> MessageHandler {
    std::sync::Arc::new(move |message| {
        let pool = Arc::clone(&pool);
        Box::pin(async move {
            match handle_route_receipt_recorded(&pool, &message.payload).await {
                Ok(IngestionOutcome::Inserted) => {
                    info!(
                        bytes = message.payload.len(),
                        "route receipt worker persisted route receipt"
                    );
                }
                Ok(IngestionOutcome::Duplicate) => {
                    info!(
                        bytes = message.payload.len(),
                        "route receipt worker deduplicated route receipt"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "route receipt worker failed to handle route receipt"
                    );
                }
            }

            Ok(())
        })
    })
}
