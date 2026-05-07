use anyhow::{Context, Result};
use async_nats::Client as NatsClient;
use protocol_ir::{MessageEnvelope, MessageType};
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, run_nats_consumer,
};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

mod processor;

use processor::{IngestionOutcome, UsagePersistenceOutcome, handle_usage_event_recorded};

const WORKER_PREFIX: &str = "LEDGER_WORKER";
const SERVICE_NAME: &str = "ledger-worker";
const WORKER_ROLE: &str = "usage-ledger-worker";
const DEFAULT_NATS_SUBJECT: &str = "events.usage_event.recorded";
const DEFAULT_BUDGET_THRESHOLD_SUBJECT: &str = "events.budget_threshold.exceeded";

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
    processor::ensure_projection_tables(&pool)
        .await
        .context("failed to create ledger projection tables")?;

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        queue_group = config.nats.queue_group,
        "ledger worker ready"
    );

    let shutdown = install_shutdown_listener();
    let pool = Arc::new(pool);
    let publisher = Arc::new(
        async_nats::connect(&config.nats.url)
            .await
            .context("failed to connect ledger worker publisher to NATS")?,
    );
    let budget_threshold_subject = std::env::var("LEDGER_WORKER_BUDGET_THRESHOLD_SUBJECT")
        .or_else(|_| std::env::var("BUDGET_THRESHOLD_SUBJECT"))
        .unwrap_or_else(|_| DEFAULT_BUDGET_THRESHOLD_SUBJECT.to_string());
    let handler = make_usage_event_handler(
        Arc::clone(&pool),
        Arc::clone(&publisher),
        budget_threshold_subject,
    );
    run_nats_consumer(&config.nats, handler, shutdown).await
}

fn make_usage_event_handler(
    pool: Arc<PgPool>,
    publisher: Arc<NatsClient>,
    budget_threshold_subject: String,
) -> MessageHandler {
    std::sync::Arc::new(move |message| {
        let pool = Arc::clone(&pool);
        let publisher = Arc::clone(&publisher);
        let budget_threshold_subject = budget_threshold_subject.clone();
        Box::pin(async move {
            match handle_usage_event_recorded(&pool, &message.payload).await {
                Ok(UsagePersistenceOutcome {
                    ingestion: IngestionOutcome::Inserted,
                    budget_threshold_event,
                }) => {
                    info!(
                        bytes = message.payload.len(),
                        "ledger worker persisted usage event"
                    );
                    if let Some(event) = budget_threshold_event {
                        publish_budget_threshold_event(
                            &publisher,
                            &budget_threshold_subject,
                            &event,
                        )
                        .await?;
                    }
                }
                Ok(UsagePersistenceOutcome {
                    ingestion: IngestionOutcome::Duplicate,
                    ..
                }) => {
                    info!(
                        bytes = message.payload.len(),
                        "ledger worker deduplicated usage event"
                    );
                }
                Err(error) => {
                    tracing::warn!(error = ?error, error_chain = %format!("{error:#}"), "ledger worker failed to handle usage event");
                }
            }

            Ok(())
        })
    })
}

async fn publish_budget_threshold_event(
    publisher: &NatsClient,
    subject: &str,
    event: &processor::BudgetThresholdEventRecord,
) -> Result<()> {
    let envelope = MessageEnvelope::new(
        format!("msg_{}", event.budget_threshold_event_id),
        MessageType::BudgetThresholdExceeded,
        chrono::Utc::now().to_rfc3339(),
        core_domain::ServiceName::parse(SERVICE_NAME).expect("service name should be valid"),
        event.budget_threshold_event_id.clone(),
        serde_json::json!({
            "tenant_id": event.tenant_id,
            "project_id": event.project_id,
            "owner_account_id": event.owner_account_id,
            "currency": event.currency,
            "threshold_status": event.threshold_status,
            "billable_cost_micros": event.billable_cost_micros,
            "configured_budget_micros": event.configured_budget_micros,
            "reason": "billable spend crossed configured threshold"
        }),
    );
    publisher
        .publish(
            subject.to_string(),
            serde_json::to_vec(&envelope)
                .context("failed to serialize budget threshold envelope")?
                .into(),
        )
        .await
        .context("failed to publish budget threshold event")?;
    publisher
        .flush()
        .await
        .context("failed to flush budget threshold event")?;
    Ok(())
}
