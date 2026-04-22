use anyhow::{Context, Result};
use async_nats::Client;
use reqwest::Client as HttpClient;
use runtime_composition::{
    ServiceRuntime, announce_startup, init_tracing, install_shutdown_listener, load_worker_config,
    read_env_or_default,
};
use std::{env, time::Duration};
use tracing::{info, warn};

mod processor;

use processor::{ProbeEventPayload, assess_probe_result, build_probe_event};

const WORKER_PREFIX: &str = "EDGE_PROBE";
const SERVICE_NAME: &str = "edge-probe";
const WORKER_ROLE: &str = "synthetic-probe";
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

    let target_url =
        read_env_or_default(WORKER_PREFIX, "TARGET_URL", "http://127.0.0.1:8080/healthz");
    let provider_resource_id = read_env_or_default(
        WORKER_PREFIX,
        "PROVIDER_RESOURCE_ID",
        "prvrsrc_openai_primary",
    );
    let interval_ms = env::var("EDGE_PROBE_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30_000);
    let timeout_ms = env::var("EDGE_PROBE_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2_000);
    let degraded_latency_ms = env::var("EDGE_PROBE_DEGRADED_LATENCY_MS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1_500);

    let publisher = async_nats::connect(&config.nats.url)
        .await
        .context("failed to connect edge-probe to NATS")?;
    let http = HttpClient::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .context("failed to create edge-probe HTTP client")?;

    info!(
        target_url,
        provider_resource_id,
        interval_ms,
        timeout_ms,
        degraded_latency_ms,
        nats_subject = config.nats.subject,
        "edge probe ready"
    );

    let shutdown = install_shutdown_listener();

    loop {
        tokio::select! {
            _ = shutdown.wait() => break,
            _ = tokio::time::sleep(Duration::from_millis(interval_ms)) => {
                if let Err(error) = run_probe_iteration(
                    &http,
                    &publisher,
                    &config.nats.subject,
                    &provider_resource_id,
                    &target_url,
                    degraded_latency_ms,
                ).await {
                    warn!(error = %error, "edge probe iteration failed");
                }
            }
        }
    }

    Ok(())
}

async fn run_probe_iteration(
    http: &HttpClient,
    publisher: &Client,
    subject: &str,
    provider_resource_id: &str,
    target_url: &str,
    degraded_latency_ms: u32,
) -> Result<()> {
    let started = std::time::Instant::now();
    let response = http.get(target_url).send().await;
    let latency_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);

    let assessment = match response {
        Ok(response) => {
            let status_code = response.status().as_u16();
            assess_probe_result(Some(status_code), latency_ms, None, degraded_latency_ms)
        }
        Err(error) => assess_probe_result(
            None,
            latency_ms,
            Some(error.to_string()),
            degraded_latency_ms,
        ),
    };

    let payload = ProbeEventPayload {
        provider_resource_id: provider_resource_id.to_string(),
        target_url: target_url.to_string(),
        assessment,
        latency_ms,
    };
    let event = build_probe_event(payload);
    let raw = serde_json::to_vec(&event).context("serializing probe event failed")?;
    publisher
        .publish(subject.to_string(), raw.into())
        .await
        .context("publishing probe event failed")?;
    publisher
        .flush()
        .await
        .context("flushing probe event failed")?;

    info!(
        provider_resource_id,
        observed_status = event.payload.observed_status.as_str(),
        latency_ms = event.payload.latency_ms,
        "edge probe published observation"
    );

    Ok(())
}
