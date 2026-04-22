use anyhow::{Context, Result};
use async_nats::Client;
use chrono::Utc;
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, read_env_or_default, run_nats_consumer,
};
use sqlx::PgPool;
use std::{env, sync::Arc};
use tracing::{info, warn};

mod processor;

use processor::{RouteHealthUpdate, handle_probe_observation};

const WORKER_PREFIX: &str = "ROUTING_WORKER";
const SERVICE_NAME: &str = "routing-worker";
const WORKER_ROLE: &str = "route-health-worker";
const DEFAULT_NATS_SUBJECT: &str = "events.provider_probe.observed";

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
        .context("ROUTING_WORKER_DATABASE_URL or DATABASE_URL is required")?;
    let quarantine_threshold = env::var("ROUTING_WORKER_QUARANTINE_THRESHOLD")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(3);
    let publisher = async_nats::connect(&config.nats.url)
        .await
        .context("failed to connect routing-worker publisher to NATS")?;
    let event_subject_prefix =
        read_env_or_default(WORKER_PREFIX, "EVENT_SUBJECT_PREFIX", "events");

    let pool = PgPool::connect(&database_url)
        .await
        .context("failed to connect routing worker to postgres")?;
    processor::ensure_probe_tables(&pool).await?;

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        quarantine_threshold,
        "routing worker ready"
    );

    let shutdown = install_shutdown_listener();
    let pool = Arc::new(pool);
    let handler: MessageHandler = Arc::new(move |message| {
        let pool = Arc::clone(&pool);
        let publisher = publisher.clone();
        let event_subject_prefix = event_subject_prefix.clone();
        Box::pin(async move {
            match handle_probe_observation(&pool, &message.payload, quarantine_threshold).await {
                Ok(RouteHealthUpdate::Duplicate { provider_resource_id }) => {
                    info!(
                        provider_resource_id,
                        "routing worker ignored duplicate probe observation"
                    );
                }
                Ok(RouteHealthUpdate::Updated {
                    provider_resource_id,
                    previous_health_state,
                    next_health_state,
                    emitted_event,
                }) => {
                    info!(
                        provider_resource_id,
                        previous_health_state = ?previous_health_state,
                        next_health_state = ?next_health_state,
                        "routing worker updated provider health state"
                    );

                    if let Some(event) = emitted_event {
                        publish_incident_event(
                            &publisher,
                            &event_subject_prefix,
                            event.message_type,
                            event.payload,
                        )
                        .await?;
                    }
                }
                Err(error) => {
                    warn!(error = %error, "routing worker received malformed probe observation");
                }
            }

            Ok(())
        })
    });

    run_nats_consumer(&config.nats, handler, shutdown).await
}

async fn publish_incident_event(
    publisher: &Client,
    subject_prefix: &str,
    message_type: &str,
    payload: serde_json::Value,
) -> Result<()> {
    let subject = format!("{subject_prefix}.{message_type}");
    let envelope = serde_json::json!({
        "message_id": format!("msg_{}_{}", message_type.replace('.', "_"), Utc::now().timestamp_millis()),
        "message_type": message_type,
        "occurred_at": Utc::now().to_rfc3339(),
        "producer": SERVICE_NAME,
        "payload": payload
    });

    publisher
        .publish(subject, serde_json::to_vec(&envelope)?.into())
        .await
        .context("failed to publish routing incident event")?;
    publisher
        .flush()
        .await
        .context("failed to flush routing incident event publish")?;

    Ok(())
}
