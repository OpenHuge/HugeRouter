use anyhow::Result;
use runtime_composition::{
    MessageHandler, ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener,
    load_worker_config, run_nats_consumer,
};
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

    info!(
        subject = config.nats.subject,
        nats_url = config.nats.url,
        "audit worker ready"
    );

    let shutdown = install_shutdown_listener();
    let handler: MessageHandler = std::sync::Arc::new(|message| {
        Box::pin(async move {
            match processor::handle_audit_envelope(&message.payload) {
                Ok(envelope) => {
                    info!(
                        message_type = envelope.message_type,
                        message_id = envelope.message_id,
                        producer = envelope.producer,
                        trace_id = envelope.trace_id,
                        request_id = envelope.request_id,
                        "audit worker parsed event envelope"
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
