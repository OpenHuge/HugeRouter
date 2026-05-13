use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use provider_traits::{
    AdapterLifecycleFamily, AdapterManifest, AdapterStability, ProviderAdapterRegistry,
    StreamingSupport,
};
use serde::Serialize;

use crate::GatewayState;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAdapterManifestsResponse {
    pub adapters: Vec<ProviderAdapterManifestDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAdapterManifestDto {
    pub manifest_schema_version: u16,
    pub adapter_id: &'static str,
    pub provider_kind: &'static str,
    pub display_name: &'static str,
    pub protocol_family: &'static str,
    pub supported_protocol_families: Vec<&'static str>,
    pub capability_hints: Vec<&'static str>,
    pub lifecycle_family: &'static str,
    pub stability: &'static str,
    pub streaming_support: &'static str,
    pub configuration_schema_ref: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAdapterManifestLookupError {
    pub code: &'static str,
    pub message: String,
    pub provider_kind: String,
    pub available_provider_kinds: Vec<String>,
}

pub async fn provider_adapters(
    State(state): State<GatewayState>,
) -> Json<ProviderAdapterManifestsResponse> {
    Json(provider_adapter_manifests_response(&state.adapter_registry))
}

pub async fn provider_adapter(
    State(state): State<GatewayState>,
    Path(provider_kind): Path<String>,
) -> Response {
    let provider_kind = provider_kind.trim().to_ascii_lowercase();

    state.adapter_registry.resolve(&provider_kind).map_or_else(
        || {
            let available_provider_kinds = state
                .adapter_registry
                .provider_kinds()
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            (
                StatusCode::NOT_FOUND,
                Json(ProviderAdapterManifestLookupError {
                    code: "provider_adapter_not_found",
                    message: format!("provider adapter `{provider_kind}` is not loaded"),
                    provider_kind,
                    available_provider_kinds,
                }),
            )
                .into_response()
        },
        |adapter| {
            let manifest = adapter.manifest();
            Json(provider_adapter_manifest_dto(&manifest)).into_response()
        },
    )
}

fn provider_adapter_manifests_response(
    registry: &ProviderAdapterRegistry,
) -> ProviderAdapterManifestsResponse {
    let manifests = registry.manifests();

    ProviderAdapterManifestsResponse {
        adapters: manifests
            .iter()
            .map(provider_adapter_manifest_dto)
            .collect(),
    }
}

fn provider_adapter_manifest_dto(manifest: &AdapterManifest) -> ProviderAdapterManifestDto {
    ProviderAdapterManifestDto {
        manifest_schema_version: manifest.manifest_schema_version,
        adapter_id: manifest.adapter_id,
        provider_kind: manifest.provider_kind,
        display_name: manifest.display_name,
        protocol_family: manifest.protocol_family,
        supported_protocol_families: manifest.supported_protocol_families.to_vec(),
        capability_hints: adapter_capability_hints(manifest),
        lifecycle_family: lifecycle_family_slug(manifest.lifecycle_family),
        stability: adapter_stability_slug(manifest.stability),
        streaming_support: streaming_support_slug(manifest.streaming_support),
        configuration_schema_ref: manifest.configuration_schema_ref,
    }
}

fn adapter_capability_hints(manifest: &AdapterManifest) -> Vec<&'static str> {
    let mut hints = Vec::new();

    for protocol_family in manifest.supported_protocol_families {
        match *protocol_family {
            "openai_chat" => push_unique(&mut hints, "chat_completions"),
            "openai_responses" => push_unique(&mut hints, "responses_api"),
            "openai_images" => push_unique(&mut hints, "image_generation"),
            "anthropic_messages" => push_unique(&mut hints, "anthropic_messages"),
            "gemini_generate_content" => push_unique(&mut hints, "gemini_generate_content"),
            "mcp_streamable_http" => push_unique(&mut hints, "mcp_streamable_http"),
            "realtime_webrtc" => push_unique(&mut hints, "realtime_webrtc"),
            _ => {}
        }
    }

    if manifest.streaming_support != StreamingSupport::Unsupported {
        push_unique(&mut hints, "streaming");
    }
    if manifest.lifecycle_family == AdapterLifecycleFamily::TransitGateway {
        push_unique(&mut hints, "transit_gateway");
    }

    hints
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

const fn adapter_stability_slug(stability: AdapterStability) -> &'static str {
    match stability {
        AdapterStability::Stable => "stable",
        AdapterStability::Beta => "beta",
        AdapterStability::Experimental => "experimental",
    }
}

const fn streaming_support_slug(streaming_support: StreamingSupport) -> &'static str {
    match streaming_support {
        StreamingSupport::Unsupported => "unsupported",
        StreamingSupport::ServerSentEvents => "server_sent_events",
    }
}
