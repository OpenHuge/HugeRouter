use axum::{Json, extract::State};
use provider_traits::{AdapterLifecycleFamily, AdapterManifest, StreamingSupport};
use serde::Serialize;

use crate::GatewayState;

#[derive(Debug, Clone, Serialize)]
pub struct RelayCapabilitiesResponse {
    pub service: &'static str,
    pub capability_schema_version: u16,
    pub current_protocols: Vec<RelayProtocolCapability>,
    pub future_protocols: Vec<RelayFutureProtocol>,
    pub routing_controls: Vec<&'static str>,
    pub safety_controls: Vec<&'static str>,
    pub observability_signals: Vec<&'static str>,
    pub commerce_controls: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelayProtocolCapability {
    pub protocol_family: &'static str,
    pub adapter_ids: Vec<&'static str>,
    pub lifecycle_families: Vec<&'static str>,
    pub streaming_support: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelayFutureProtocol {
    pub protocol_family: &'static str,
    pub maturity: &'static str,
    pub reason: &'static str,
    pub required_controls: Vec<&'static str>,
}

pub async fn relay_capabilities(
    State(state): State<GatewayState>,
) -> Json<RelayCapabilitiesResponse> {
    Json(relay_capabilities_response(&state))
}

fn relay_capabilities_response(state: &GatewayState) -> RelayCapabilitiesResponse {
    let manifests = state.adapter_registry.manifests();

    RelayCapabilitiesResponse {
        service: "gateway-api",
        capability_schema_version: 1,
        current_protocols: current_protocols(&manifests),
        future_protocols: future_protocols(),
        routing_controls: vec![
            "capability_aware_routing",
            "health_state_filtering",
            "retryable_fallback_chain",
            "transit_loop_prevention",
            "budget_projection_gate",
        ],
        safety_controls: vec![
            "api_key_scope_resolution",
            "provider_credential_isolation",
            "tenant_project_authorization",
            "normalized_error_boundary",
            "route_receipt_audit_trail",
            "terms_policy_gate_required_for_external_relays",
        ],
        observability_signals: vec![
            "request_id",
            "trace_id",
            "route_receipt_id",
            "provider_attempts",
            "fallback_transitions",
            "usage_event",
        ],
        commerce_controls: vec![
            "metered_usage_event",
            "billable_cost_projection",
            "provider_resource_scope",
            "merchant_replay_evidence",
        ],
    }
}

fn current_protocols(manifests: &[AdapterManifest]) -> Vec<RelayProtocolCapability> {
    let mut protocols = Vec::<RelayProtocolCapability>::new();

    for manifest in manifests {
        for protocol_family in manifest.supported_protocol_families {
            let position = protocols
                .iter()
                .position(|protocol| protocol.protocol_family == *protocol_family);

            if let Some(index) = position {
                push_unique(&mut protocols[index].adapter_ids, manifest.adapter_id);
                push_unique(
                    &mut protocols[index].lifecycle_families,
                    lifecycle_family_slug(manifest.lifecycle_family),
                );
                push_unique(
                    &mut protocols[index].streaming_support,
                    streaming_support_slug(manifest.streaming_support),
                );
            } else {
                protocols.push(RelayProtocolCapability {
                    protocol_family,
                    adapter_ids: vec![manifest.adapter_id],
                    lifecycle_families: vec![lifecycle_family_slug(manifest.lifecycle_family)],
                    streaming_support: vec![streaming_support_slug(manifest.streaming_support)],
                });
            }
        }
    }

    protocols.sort_by_key(|protocol| protocol.protocol_family);
    protocols
}

fn future_protocols() -> Vec<RelayFutureProtocol> {
    vec![
        RelayFutureProtocol {
            protocol_family: "mcp_streamable_http",
            maturity: "planned",
            reason: "Tool/resource/prompt relay needs explicit consent, tenant isolation, and audit before exposing arbitrary tool access.",
            required_controls: vec![
                "tool_allowlist",
                "user_consent_receipt",
                "resource_scope_policy",
                "tool_call_audit_log",
            ],
        },
        RelayFutureProtocol {
            protocol_family: "a2a",
            maturity: "planned",
            reason: "Agent-to-agent delegation needs capability discovery, task state, and push/event delivery semantics.",
            required_controls: vec![
                "agent_card_registry",
                "delegation_policy",
                "task_lifecycle_trace",
                "cross_agent_identity_boundary",
            ],
        },
        RelayFutureProtocol {
            protocol_family: "realtime_webrtc",
            maturity: "scaffolded",
            reason: "Realtime voice/video relay needs session limits, interruption handling, and partial usage capture.",
            required_controls: vec![
                "session_duration_budget",
                "media_redaction_policy",
                "barge_in_event_trace",
                "partial_usage_metering",
            ],
        },
    ]
}

fn push_unique(values: &mut Vec<&'static str>, value: &'static str) {
    if !values.contains(&value) {
        values.push(value);
    }
}

const fn lifecycle_family_slug(lifecycle_family: AdapterLifecycleFamily) -> &'static str {
    match lifecycle_family {
        AdapterLifecycleFamily::Inference => "inference",
        AdapterLifecycleFamily::TransitGateway => "transit_gateway",
        AdapterLifecycleFamily::Realtime => "realtime",
        AdapterLifecycleFamily::Tool => "tool",
        AdapterLifecycleFamily::Agent => "agent",
    }
}

const fn streaming_support_slug(streaming_support: StreamingSupport) -> &'static str {
    match streaming_support {
        StreamingSupport::Unsupported => "unsupported",
        StreamingSupport::ServerSentEvents => "server_sent_events",
    }
}

#[cfg(test)]
mod tests {
    use provider_traits::{
        AdapterLifecycleFamily, AdapterManifest, AdapterStability,
        CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION, StreamingSupport,
    };

    use super::current_protocols;

    #[test]
    fn current_protocols_groups_adapters_by_supported_protocol_family() {
        let protocols = current_protocols(&[
            AdapterManifest {
                manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
                adapter_id: "native-openai",
                provider_kind: "openai",
                display_name: "OpenAI",
                protocol_family: "openai_chat",
                supported_protocol_families: &["openai_chat", "openai_responses"],
                lifecycle_family: AdapterLifecycleFamily::Inference,
                stability: AdapterStability::Stable,
                streaming_support: StreamingSupport::Unsupported,
                configuration_schema_ref: None,
            },
            AdapterManifest {
                manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
                adapter_id: "relay-openai",
                provider_kind: "relay-openai",
                display_name: "Relay OpenAI",
                protocol_family: "openai_chat",
                supported_protocol_families: &["openai_chat"],
                lifecycle_family: AdapterLifecycleFamily::TransitGateway,
                stability: AdapterStability::Beta,
                streaming_support: StreamingSupport::ServerSentEvents,
                configuration_schema_ref: None,
            },
        ]);

        let openai_chat = protocols
            .iter()
            .find(|protocol| protocol.protocol_family == "openai_chat")
            .expect("openai_chat protocol should be present");

        assert_eq!(
            openai_chat.adapter_ids,
            vec!["native-openai", "relay-openai"]
        );
        assert_eq!(
            openai_chat.lifecycle_families,
            vec!["inference", "transit_gateway"]
        );
        assert_eq!(
            openai_chat.streaming_support,
            vec!["unsupported", "server_sent_events"]
        );
    }
}
