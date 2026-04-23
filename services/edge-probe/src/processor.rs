use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeAssessment {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeMode {
    CheapHealth,
    BillableSynthetic,
}

impl ProbeMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CheapHealth => "cheap_health",
            Self::BillableSynthetic => "billable_synthetic",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeEventPayload {
    pub provider_resource_id: String,
    pub target_url: String,
    pub probe_mode: ProbeMode,
    pub assessment: ProbeAssessment,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishedProbeStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl PublishedProbeStatus {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedProbePayload {
    pub provider_resource_id: String,
    pub probe_mode: ProbeMode,
    pub observed_status: PublishedProbeStatus,
    pub latency_ms: u32,
    pub reason: Option<String>,
    pub target_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedProbeEvent {
    pub message_id: String,
    pub message_type: String,
    pub occurred_at: String,
    pub producer: String,
    pub payload: PublishedProbePayload,
}

pub fn assess_probe_result(
    http_status: Option<u16>,
    latency_ms: u32,
    transport_error: Option<String>,
    degraded_latency_ms: u32,
) -> ProbeAssessment {
    if let Some(error) = transport_error {
        return ProbeAssessment::Unhealthy { reason: error };
    }

    let Some(http_status) = http_status else {
        return ProbeAssessment::Unhealthy {
            reason: "probe did not return a status code".to_string(),
        };
    };

    if http_status >= 500 {
        return ProbeAssessment::Unhealthy {
            reason: format!("upstream returned HTTP {http_status}"),
        };
    }

    if http_status >= 400 {
        return ProbeAssessment::Degraded {
            reason: format!("upstream returned HTTP {http_status}"),
        };
    }

    if latency_ms > degraded_latency_ms {
        return ProbeAssessment::Degraded {
            reason: format!("latency budget exceeded at {latency_ms}ms"),
        };
    }

    ProbeAssessment::Healthy
}

#[must_use]
pub fn parse_probe_mode(value: &str) -> Option<ProbeMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "cheap" | "cheap_health" | "health" => Some(ProbeMode::CheapHealth),
        "billable" | "billable_synthetic" | "synthetic" => Some(ProbeMode::BillableSynthetic),
        _ => None,
    }
}

pub fn build_probe_event(payload: ProbeEventPayload) -> PublishedProbeEvent {
    let (observed_status, reason) = match payload.assessment {
        ProbeAssessment::Healthy => (PublishedProbeStatus::Healthy, None),
        ProbeAssessment::Degraded { reason } => (PublishedProbeStatus::Degraded, Some(reason)),
        ProbeAssessment::Unhealthy { reason } => (PublishedProbeStatus::Unhealthy, Some(reason)),
    };

    PublishedProbeEvent {
        message_id: format!(
            "msg_probe_{}_{}",
            payload.provider_resource_id,
            Utc::now().timestamp_millis()
        ),
        message_type: "provider_probe.observed".to_string(),
        occurred_at: Utc::now().to_rfc3339(),
        producer: "edge-probe".to_string(),
        payload: PublishedProbePayload {
            provider_resource_id: payload.provider_resource_id,
            probe_mode: payload.probe_mode,
            observed_status,
            latency_ms: payload.latency_ms,
            reason,
            target_url: payload.target_url,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_slow_success_as_degraded() {
        let assessment = assess_probe_result(Some(200), 2_100, None, 1_500);
        assert_eq!(
            assessment,
            ProbeAssessment::Degraded {
                reason: "latency budget exceeded at 2100ms".to_string()
            }
        );
    }

    #[test]
    fn classifies_transport_failure_as_unhealthy() {
        let assessment =
            assess_probe_result(None, 120, Some("connection refused".to_string()), 1_500);
        assert_eq!(
            assessment,
            ProbeAssessment::Unhealthy {
                reason: "connection refused".to_string()
            }
        );
    }

    #[test]
    fn builds_probe_event_with_expected_status() {
        let event = build_probe_event(ProbeEventPayload {
            provider_resource_id: "prvrsrc_openai_primary".to_string(),
            target_url: "http://127.0.0.1:8080/healthz".to_string(),
            probe_mode: ProbeMode::CheapHealth,
            assessment: ProbeAssessment::Healthy,
            latency_ms: 32,
        });

        assert_eq!(event.message_type, "provider_probe.observed");
        assert_eq!(event.payload.probe_mode, ProbeMode::CheapHealth);
        assert_eq!(event.payload.observed_status, PublishedProbeStatus::Healthy);
        assert_eq!(event.payload.reason, None);
    }

    #[test]
    fn parses_probe_mode_aliases() {
        assert_eq!(parse_probe_mode("health"), Some(ProbeMode::CheapHealth));
        assert_eq!(
            parse_probe_mode("billable"),
            Some(ProbeMode::BillableSynthetic)
        );
        assert_eq!(parse_probe_mode("unknown"), None);
    }
}
