use async_trait::async_trait;
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;

pub const CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingSupport {
    Unsupported,
    ServerSentEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterLifecycleFamily {
    Inference,
    TransitGateway,
    Realtime,
    Tool,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterStability {
    Stable,
    Beta,
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterManifest {
    pub manifest_schema_version: u16,
    pub adapter_id: &'static str,
    pub provider_kind: &'static str,
    pub display_name: &'static str,
    pub protocol_family: &'static str,
    pub supported_protocol_families: &'static [&'static str],
    pub lifecycle_family: AdapterLifecycleFamily,
    pub stability: AdapterStability,
    pub streaming_support: StreamingSupport,
    pub configuration_schema_ref: Option<&'static str>,
}

impl AdapterManifest {
    #[must_use]
    pub fn supports_protocol_family(&self, protocol_family: &str) -> bool {
        self.supported_protocol_families.contains(&protocol_family)
    }

    /// # Errors
    ///
    /// Returns [`AdapterManifestError`] when the manifest uses an unsupported
    /// schema version, omits required identity fields, or does not declare its
    /// primary protocol in the supported protocol list.
    pub fn validate(&self) -> Result<(), AdapterManifestError> {
        if self.manifest_schema_version != CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION {
            return Err(AdapterManifestError::UnsupportedSchemaVersion {
                adapter_id: self.adapter_id.to_string(),
                manifest_schema_version: self.manifest_schema_version,
            });
        }

        for (field, value) in [
            ("adapter_id", self.adapter_id),
            ("provider_kind", self.provider_kind),
            ("display_name", self.display_name),
            ("protocol_family", self.protocol_family),
        ] {
            if value.trim().is_empty() {
                return Err(AdapterManifestError::MissingRequiredField {
                    adapter_id: self.adapter_id.to_string(),
                    field,
                });
            }
        }

        if self.supported_protocol_families.is_empty() {
            return Err(AdapterManifestError::MissingSupportedProtocols {
                adapter_id: self.adapter_id.to_string(),
            });
        }

        if !self.supports_protocol_family(self.protocol_family) {
            return Err(AdapterManifestError::ProtocolFamilyNotDeclared {
                adapter_id: self.adapter_id.to_string(),
                protocol_family: self.protocol_family.to_string(),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdapterManifestError {
    #[error(
        "adapter `{adapter_id}` uses unsupported manifest schema version {manifest_schema_version}"
    )]
    UnsupportedSchemaVersion {
        adapter_id: String,
        manifest_schema_version: u16,
    },
    #[error("adapter `{adapter_id}` manifest is missing required field `{field}`")]
    MissingRequiredField {
        adapter_id: String,
        field: &'static str,
    },
    #[error("adapter `{adapter_id}` manifest does not declare any supported protocols")]
    MissingSupportedProtocols { adapter_id: String },
    #[error(
        "adapter `{adapter_id}` manifest primary protocol `{protocol_family}` is not declared in supported protocols"
    )]
    ProtocolFamilyNotDeclared {
        adapter_id: String,
        protocol_family: String,
    },
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
pub struct ProviderImageRequest {
    pub model: String,
    pub prompt: String,
    pub n: Option<u32>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub response_format: Option<String>,
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
pub struct ProviderImageData {
    pub b64_json: Option<String>,
    pub url: Option<String>,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderImageResponse {
    pub response_id: Option<String>,
    pub model: String,
    pub created: Option<u64>,
    pub images: Vec<ProviderImageData>,
    pub usage: ProviderUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEndpoint {
    pub provider_resource_id: String,
    pub endpoint_base_url: String,
    pub api_key: String,
    pub region: Option<String>,
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

    async fn execute_image_generation(
        &self,
        _request: &ProviderImageRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderImageResponse, ProviderError> {
        Err(ProviderError::new(
            ProviderErrorKind::InvalidRequest,
            "image generation is not supported by this provider adapter",
            false,
        )
        .with_detail(
            "provider_resource_id",
            &context.endpoint.provider_resource_id,
        )
        .with_detail("adapter_boundary", "image_generation_unsupported"))
    }
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
        let manifest = adapter.manifest();
        manifest.validate()?;

        let provider_kind = normalize_provider_kind(manifest.provider_kind);
        let adapter_id = manifest.adapter_id.to_string();

        if self.adapters.contains_key(&provider_kind) {
            return Err(ProviderRegistryError::DuplicateProviderKind { provider_kind });
        }
        if self
            .adapters
            .values()
            .any(|candidate| candidate.manifest().adapter_id == manifest.adapter_id)
        {
            return Err(ProviderRegistryError::DuplicateAdapterId { adapter_id });
        }

        self.adapters.insert(provider_kind, adapter);
        Ok(())
    }

    #[must_use]
    pub fn resolve(&self, provider_kind: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters
            .get(&normalize_provider_kind(provider_kind))
            .cloned()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    #[must_use]
    pub fn provider_kinds(&self) -> Vec<&str> {
        self.adapters.keys().map(String::as_str).collect()
    }

    #[must_use]
    pub fn manifests(&self) -> Vec<AdapterManifest> {
        self.adapters
            .values()
            .map(|adapter| adapter.manifest())
            .collect()
    }
}

fn normalize_provider_kind(provider_kind: &str) -> String {
    provider_kind.trim().to_ascii_lowercase()
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderRegistryError {
    #[error("provider adapter already registered for `{provider_kind}`")]
    DuplicateProviderKind { provider_kind: String },
    #[error("provider adapter id `{adapter_id}` is already registered")]
    DuplicateAdapterId { adapter_id: String },
    #[error("{0}")]
    InvalidManifest(#[from] AdapterManifestError),
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterLifecycleFamily, AdapterManifest, AdapterStability,
        CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION, ProviderAdapter, ProviderAdapterRegistry,
        ProviderError, ProviderErrorKind, ProviderExecutionContext, ProviderRequest,
        ProviderResponse, ProviderUsage, StreamingSupport, TransitGatewayKind,
        TransitProviderMetadata,
    };
    use async_trait::async_trait;
    use std::{collections::BTreeMap, sync::Arc};

    struct FakeAdapter;
    struct InvalidManifestAdapter;

    #[async_trait]
    impl ProviderAdapter for FakeAdapter {
        fn manifest(&self) -> AdapterManifest {
            AdapterManifest {
                manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
                adapter_id: "fake-openai",
                provider_kind: "openai",
                display_name: "Fake OpenAI",
                protocol_family: "openai_chat",
                supported_protocol_families: &["openai_chat"],
                lifecycle_family: AdapterLifecycleFamily::Inference,
                stability: AdapterStability::Stable,
                streaming_support: StreamingSupport::Unsupported,
                configuration_schema_ref: Some("test:fake-openai"),
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

    #[async_trait]
    impl ProviderAdapter for InvalidManifestAdapter {
        fn manifest(&self) -> AdapterManifest {
            AdapterManifest {
                manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
                adapter_id: "invalid",
                provider_kind: "invalid",
                display_name: "Invalid",
                protocol_family: "openai_chat",
                supported_protocol_families: &[],
                lifecycle_family: AdapterLifecycleFamily::Inference,
                stability: AdapterStability::Experimental,
                streaming_support: StreamingSupport::Unsupported,
                configuration_schema_ref: None,
            }
        }

        async fn execute_chat(
            &self,
            _request: &ProviderRequest,
            _context: &ProviderExecutionContext,
        ) -> Result<ProviderResponse, ProviderError> {
            unreachable!("invalid adapter should not execute in registry tests")
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
    fn registry_exposes_registered_provider_metadata() {
        let mut registry = ProviderAdapterRegistry::new();

        registry.register(Arc::new(FakeAdapter)).unwrap();

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.provider_kinds(), vec!["openai"]);
        assert_eq!(registry.manifests()[0].adapter_id, "fake-openai");
    }

    #[test]
    fn registry_resolves_provider_kind_with_case_and_whitespace_drift() {
        let mut registry = ProviderAdapterRegistry::new();

        registry.register(Arc::new(FakeAdapter)).unwrap();

        assert!(registry.resolve(" OpenAI ").is_some());
        assert_eq!(registry.provider_kinds(), vec!["openai"]);
    }

    #[test]
    fn registry_rejects_invalid_adapter_manifest() {
        let mut registry = ProviderAdapterRegistry::new();
        let error = registry
            .register(Arc::new(InvalidManifestAdapter))
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "adapter `invalid` manifest does not declare any supported protocols"
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
                region: None,
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
