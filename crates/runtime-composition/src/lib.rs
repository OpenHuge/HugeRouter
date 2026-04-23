use anyhow::{Context, Result};
use async_nats as nats;
use futures::{StreamExt, future::BoxFuture};
use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Notify;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Copy)]
pub struct ServiceRuntime {
    pub service_name: &'static str,
    pub role: &'static str,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub service_name: &'static str,
    pub role: &'static str,
    pub nats: NatsConfig,
    pub database_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub subject: String,
    pub queue_group: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    notified: Arc<Notify>,
    triggered: Arc<AtomicBool>,
}

impl Default for ShutdownHandle {
    fn default() -> Self {
        Self {
            notified: Arc::new(Notify::new()),
            triggered: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ShutdownHandle {
    pub fn trigger(&self) {
        self.triggered.store(true, Ordering::SeqCst);
        self.notified.notify_waiters();
    }

    #[must_use]
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    pub async fn wait(&self) {
        self.notified.notified().await;
    }
}

pub type MessageHandler =
    std::sync::Arc<dyn Fn(nats::Message) -> BoxFuture<'static, Result<()>> + Send + Sync>;

#[must_use]
pub const fn nats_default_url() -> &'static str {
    "nats://127.0.0.1:4222"
}

pub fn announce_startup(runtime: ServiceRuntime) {
    info!(
        service = runtime.service_name,
        role = runtime.role,
        "HugeRouter service bootstrap placeholder is online"
    );
}

pub fn init_tracing(runtime: &ServiceRuntime) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false),
        )
        .init();

    tracing::info!(
        service = runtime.service_name,
        role = runtime.role,
        "HugeRouter worker runtime initialized"
    );
}

#[must_use]
pub fn read_env_or_default(prefix: &str, key: &str, fallback: &str) -> String {
    let prefixed = format!("{prefix}_{key}");
    env::var(&prefixed)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var(key).ok().filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| fallback.to_string())
}

pub fn load_worker_config(
    prefix: &str,
    service_name: &'static str,
    role: &'static str,
    default_nats_subject: impl Into<String>,
) -> RuntimeConfig {
    let nats_subject = read_env_or_default(prefix, "NATS_SUBJECT", &default_nats_subject.into());
    let queue_group = env::var(format!("{prefix}_NATS_QUEUE_GROUP"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("NATS_QUEUE_GROUP")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    let database_url = env::var(format!("{prefix}_DATABASE_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("DATABASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    RuntimeConfig {
        service_name,
        role,
        nats: NatsConfig {
            url: read_env_or_default(prefix, "NATS_URL", nats_default_url()),
            subject: nats_subject,
            queue_group,
        },
        database_url,
    }
}

/// # Errors
///
/// Returns an error when the consumer cannot connect to NATS or create the
/// configured subscription.
pub async fn run_nats_consumer(
    config: &NatsConfig,
    handler: MessageHandler,
    shutdown: ShutdownHandle,
) -> Result<()> {
    let client = nats::connect(&config.url)
        .await
        .context("failed to connect to NATS")?;

    let mut subscription = if let Some(queue_group) = &config.queue_group {
        client
            .queue_subscribe(config.subject.clone(), queue_group.clone())
            .await
            .context("failed to create queue subscription")?
    } else {
        client
            .subscribe(config.subject.clone())
            .await
            .context("failed to create subscription")?
    };

    info!(
        subject = config.subject,
        queue_group = config.queue_group,
        "starting NATS consumer loop"
    );

    loop {
        tokio::select! {
            event = subscription.next() => {
                if let Some(message) = event {
                    if let Err(error) = (handler)(message).await {
                        tracing::warn!(error = %error, "NATS handler failed");
                    }
                } else {
                    break;
                }
            }
            () = shutdown.wait() => {
                break;
            }
        }
    }

    Ok(())
}

#[must_use]
pub fn install_shutdown_listener() -> ShutdownHandle {
    let signal = ShutdownHandle::default();
    let ctrl_c_signal = signal.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            ctrl_c_signal.trigger();
            tracing::info!("received termination signal, shutting down");
        }
    });

    #[cfg(unix)]
    {
        let terminate_signal = signal.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{SignalKind, signal};

            if let Ok(mut stream) = signal(SignalKind::terminate())
                && stream.recv().await.is_some()
            {
                terminate_signal.trigger();
                tracing::info!("received SIGTERM signal, shutting down");
            }
        });
    }

    signal
}

#[cfg(test)]
mod tests {
    use super::{ShutdownHandle, load_worker_config, nats_default_url, read_env_or_default};
    use std::env;
    #[test]
    fn loads_fallbacks_for_unspecified_prefix() {
        let prefix = "RUNTIME_COMPOSITION_TEST_WORKER";
        let expected_subject =
            env::var("NATS_SUBJECT").unwrap_or_else(|_| "events.default".to_string());
        let expected_url = env::var("NATS_URL").unwrap_or_else(|_| nats_default_url().to_string());

        let runtime = load_worker_config(prefix, "service", "role", "events.default");
        assert_eq!(runtime.nats.subject, expected_subject);
        assert_eq!(runtime.nats.url, expected_url);
    }

    #[tokio::test]
    async fn shutdown_signal_can_be_triggered_manually() {
        use tokio::time::{Duration, sleep, timeout};

        let signal = ShutdownHandle::default();
        assert!(!signal.is_triggered());

        let waiter_signal = signal.clone();
        let waiter = tokio::spawn(async move { waiter_signal.wait().await });
        tokio::task::yield_now().await;
        signal.trigger();
        assert!(signal.is_triggered());
        timeout(Duration::from_secs(1), waiter)
            .await
            .expect("waiter should observe shutdown")
            .expect("waiter task should complete");

        sleep(Duration::from_millis(1)).await;
        assert!(signal.is_triggered());
    }

    #[test]
    fn direct_env_reader_handles_defaults() {
        let fallback = read_env_or_default("MISSING_PREFIX", "UNKNOWN", "fallback-value");
        assert_eq!(fallback, "fallback-value");
    }
}
