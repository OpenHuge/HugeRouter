use async_trait::async_trait;
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingSupport {
    Unsupported,
    ServerSentEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterManifest {
    pub adapter_id: &'static str,
    pub provider_kind: &'static str,
    pub display_name: &'static str,
    pub protocol_family: &'static str,
    pub streaming_support: StreamingSupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequest {
    pub model: String,
    pub messages: Vec<ProviderMessage>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResponse {
    pub response_id: Option<String>,
    pub model: String,
    pub output_text: String,
    pub finish_reason: String,
    pub usage: ProviderUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEndpoint {
    pub provider_resource_id: String,
    pub endpoint_base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTargetKind {
    Native,
    TransitGateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitGatewayKind {
    OpenAiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitProviderMetadata {
    pub gateway_kind: TransitGatewayKind,
    pub gateway_name: String,
    pub route_cost_scope: String,
    pub transit_hops: u8,
    pub preserves_error_diagnostics: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderExecutionContext {
    pub request_id: String,
    pub trace_id: String,
    pub gateway_service_name: String,
    pub gateway_origin: Option<String>,
    pub request_headers: BTreeMap<String, String>,
    pub endpoint: ProviderEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    Auth,
    InvalidRequest,
    RateLimited,
    Timeout,
    Unavailable,
    Protocol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
    pub upstream_code: Option<String>,
    pub upstream_status_code: Option<u16>,
    pub details: BTreeMap<String, String>,
}

impl ProviderError {
    #[must_use]
    pub fn new(kind: ProviderErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
            upstream_code: None,
            upstream_status_code: None,
            details: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn with_upstream_status(mut self, status: Option<u16>) -> Self {
        self.upstream_status_code = status;
        self
    }

    #[must_use]
    pub fn with_upstream_code(mut self, code: Option<String>) -> Self {
        self.upstream_code = code;
        self
    }

    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn manifest(&self) -> AdapterManifest;

    async fn execute_chat(
        &self,
        request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError>;
}

#[derive(Clone, Default)]
pub struct ProviderAdapterRegistry {
    adapters: BTreeMap<String, Arc<dyn ProviderAdapter>>,
}

impl ProviderAdapterRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a provider adapter by its declared provider kind.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderRegistryError::DuplicateProviderKind`] when another
    /// adapter has already been registered for the same provider kind.
    pub fn register(
        &mut self,
        adapter: Arc<dyn ProviderAdapter>,
    ) -> Result<(), ProviderRegistryError> {
        let provider_kind = adapter.manifest().provider_kind.to_string();

        if self.adapters.contains_key(&provider_kind) {
            return Err(ProviderRegistryError::DuplicateProviderKind { provider_kind });
        }

        self.adapters.insert(provider_kind, adapter);
        Ok(())
    }

    #[must_use]
    pub fn resolve(&self, provider_kind: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_kind).cloned()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderRegistryError {
    #[error("provider adapter already registered for `{provider_kind}`")]
    DuplicateProviderKind { provider_kind: String },
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterManifest, ProviderAdapter, ProviderAdapterRegistry, ProviderError,
        ProviderErrorKind, ProviderExecutionContext, ProviderRequest, ProviderResponse,
        ProviderUsage, StreamingSupport, TransitGatewayKind, TransitProviderMetadata,
    };
    use async_trait::async_trait;
    use std::{collections::BTreeMap, sync::Arc};

    struct FakeAdapter;

    #[async_trait]
    impl ProviderAdapter for FakeAdapter {
        fn manifest(&self) -> AdapterManifest {
            AdapterManifest {
                adapter_id: "fake-openai",
                provider_kind: "openai",
                display_name: "Fake OpenAI",
                protocol_family: "openai_chat",
                streaming_support: StreamingSupport::Unsupported,
            }
        }

        async fn execute_chat(
            &self,
            _request: &ProviderRequest,
            _context: &ProviderExecutionContext,
        ) -> Result<ProviderResponse, ProviderError> {
            Ok(ProviderResponse {
                response_id: Some("resp_123".to_string()),
                model: "gpt-4.1-mini".to_string(),
                output_text: "ok".to_string(),
                finish_reason: "stop".to_string(),
                usage: ProviderUsage {
                    input_tokens: 10,
                    output_tokens: 4,
                    cached_input_tokens: 0,
                },
            })
        }
    }

    #[test]
    fn registry_rejects_duplicate_provider_kind() {
        let mut registry = ProviderAdapterRegistry::new();

        registry.register(Arc::new(FakeAdapter)).unwrap();
        let error = registry.register(Arc::new(FakeAdapter)).unwrap_err();

        assert_eq!(
            error.to_string(),
            "provider adapter already registered for `openai`"
        );
    }

    #[test]
    fn provider_error_can_capture_upstream_metadata() {
        let error = ProviderError::new(ProviderErrorKind::RateLimited, "limited", true)
            .with_upstream_status(Some(429))
            .with_upstream_code(Some("rate_limit_exceeded".to_string()))
            .with_detail("provider_resource_id", "prvrsrc_openai_primary");

        assert_eq!(error.upstream_status_code, Some(429));
        assert_eq!(
            error.details.get("provider_resource_id"),
            Some(&"prvrsrc_openai_primary".to_string())
        );
    }

    #[test]
    fn transit_metadata_is_typed() {
        let metadata = TransitProviderMetadata {
            gateway_kind: TransitGatewayKind::OpenAiCompatible,
            gateway_name: "edge transit".to_string(),
            route_cost_scope: "openai_chat".to_string(),
            transit_hops: 1,
            preserves_error_diagnostics: true,
        };

        assert_eq!(metadata.gateway_kind, TransitGatewayKind::OpenAiCompatible);
        assert_eq!(metadata.transit_hops, 1);
    }

    #[test]
    fn execution_context_can_carry_request_headers_and_origin() {
        let context = ProviderExecutionContext {
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            gateway_service_name: "gateway-api".to_string(),
            gateway_origin: Some("https://router.example.com/v1".to_string()),
            request_headers: BTreeMap::from([("x-request-id".to_string(), "req_123".to_string())]),
            endpoint: super::ProviderEndpoint {
                provider_resource_id: "prvrsrc_openai_primary".to_string(),
                endpoint_base_url: "https://api.example.com/v1".to_string(),
                api_key: "secret".to_string(),
            },
        };

        assert_eq!(
            context.gateway_origin.as_deref(),
            Some("https://router.example.com/v1")
        );
        assert_eq!(
            context.request_headers.get("x-request-id"),
            Some(&"req_123".to_string())
        );
    }
}
