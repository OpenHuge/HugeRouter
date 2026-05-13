use async_trait::async_trait;
use aws_sdk_bedrockruntime::{
    operation::converse::ConverseError,
    types::{
        ContentBlock, ConversationRole, GuardrailConfiguration, GuardrailTrace,
        InferenceConfiguration, Message, StopReason, SystemContentBlock,
    },
};
use provider_traits::{
    AdapterLifecycleFamily, AdapterManifest, AdapterStability,
    CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION, ProviderAdapter, ProviderError, ProviderErrorKind,
    ProviderExecutionContext, ProviderMessage, ProviderRequest, ProviderResponse, ProviderUsage,
    StreamingSupport,
};
use std::{env, sync::Arc};

const DEFAULT_MAX_TOKENS: i32 = 1024;
const DEFAULT_REGION: &str = "us-east-1";

#[derive(Clone)]
pub struct BedrockConverseAdapter {
    runtime: Arc<dyn BedrockRuntime>,
}

impl BedrockConverseAdapter {
    #[must_use]
    pub fn new(runtime: Arc<dyn BedrockRuntime>) -> Self {
        Self { runtime }
    }
}

impl Default for BedrockConverseAdapter {
    fn default() -> Self {
        Self::new(Arc::new(SdkBedrockRuntime))
    }
}

#[async_trait]
impl ProviderAdapter for BedrockConverseAdapter {
    fn manifest(&self) -> AdapterManifest {
        AdapterManifest {
            manifest_schema_version: CURRENT_ADAPTER_MANIFEST_SCHEMA_VERSION,
            adapter_id: "aws-bedrock-converse-v1",
            provider_kind: "bedrock",
            display_name: "AWS Bedrock Converse",
            protocol_family: "openai_chat",
            supported_protocol_families: &["openai_chat"],
            lifecycle_family: AdapterLifecycleFamily::Inference,
            stability: AdapterStability::Beta,
            streaming_support: StreamingSupport::Unsupported,
            configuration_schema_ref: Some("env:GATEWAY_BEDROCK_*"),
        }
    }

    async fn execute_chat(
        &self,
        request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        if request.stream {
            return Err(base_error(
                ProviderErrorKind::InvalidRequest,
                "stream=true is not supported in Bedrock Converse adapter mode",
                false,
                context,
            ));
        }

        let converse_request = build_converse_request(request, context)?;
        let response = self
            .runtime
            .converse(converse_request, context)
            .await
            .map_err(|error| error.into_provider_error(context))?;

        response.into_provider_response(request, context)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockConverseRequest {
    pub model_id: String,
    pub messages: Vec<BedrockMessage>,
    pub system_prompts: Vec<String>,
    pub max_tokens: i32,
    pub temperature: Option<f32>,
    pub guardrail: Option<BedrockGuardrailConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockMessage {
    pub role: BedrockRole,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockGuardrailConfig {
    pub identifier: String,
    pub version: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockConverseResponse {
    pub response_id: Option<String>,
    pub model_id: String,
    pub output_text: String,
    pub finish_reason: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockConverseFailure {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
    pub upstream_code: Option<String>,
    pub upstream_status_code: Option<u16>,
}

impl BedrockConverseFailure {
    fn into_provider_error(self, context: &ProviderExecutionContext) -> ProviderError {
        let mut error = base_error(self.kind, self.message, self.retryable, context)
            .with_upstream_status(self.upstream_status_code)
            .with_upstream_code(self.upstream_code);

        if let Some(region) = context.endpoint.region.as_ref() {
            error = error.with_detail("aws_region", region);
        }

        error
    }
}

/// Minimal Bedrock runtime boundary used by the adapter.
#[async_trait]
pub trait BedrockRuntime: Send + Sync {
    /// Sends one non-streaming Converse request.
    ///
    /// # Errors
    ///
    /// Returns [`BedrockConverseFailure`] when AWS rejects the request, the SDK
    /// cannot reach the service, or Bedrock returns an unsupported response.
    async fn converse(
        &self,
        request: BedrockConverseRequest,
        context: &ProviderExecutionContext,
    ) -> Result<BedrockConverseResponse, BedrockConverseFailure>;
}

struct SdkBedrockRuntime;

#[async_trait]
impl BedrockRuntime for SdkBedrockRuntime {
    async fn converse(
        &self,
        request: BedrockConverseRequest,
        context: &ProviderExecutionContext,
    ) -> Result<BedrockConverseResponse, BedrockConverseFailure> {
        let region = resolve_region(context);
        let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.clone()))
            .load()
            .await;
        let mut config_builder = aws_sdk_bedrockruntime::config::Builder::from(&shared_config);

        if let Some(endpoint_url) = resolve_endpoint_override(context) {
            config_builder.set_endpoint_url(Some(endpoint_url));
        }

        let client = aws_sdk_bedrockruntime::Client::from_conf(config_builder.build());
        let mut builder = client
            .converse()
            .model_id(request.model_id.clone())
            .set_messages(Some(to_sdk_messages(&request)?))
            .inference_config(to_sdk_inference_config(&request));

        for system_prompt in &request.system_prompts {
            builder = builder.system(SystemContentBlock::Text(system_prompt.clone()));
        }

        if let Some(guardrail) = &request.guardrail {
            builder = builder.guardrail_config(to_sdk_guardrail_config(guardrail));
        }

        let output = builder.send().await.map_err(|error| {
            let service_error = error.into_service_error();
            map_converse_error(&service_error)
        })?;

        from_sdk_output(&output, &request.model_id)
    }
}

fn build_converse_request(
    request: &ProviderRequest,
    context: &ProviderExecutionContext,
) -> Result<BedrockConverseRequest, ProviderError> {
    let mut messages = Vec::new();
    let mut system_prompts = Vec::new();

    for (index, message) in request.messages.iter().enumerate() {
        let role = message.role.trim().to_ascii_lowercase();
        match role.as_str() {
            "system" => system_prompts.push(message.content.clone()),
            "user" => messages.push(to_bedrock_message(message, BedrockRole::User)),
            "assistant" => messages.push(to_bedrock_message(message, BedrockRole::Assistant)),
            _ => {
                return Err(base_error(
                    ProviderErrorKind::InvalidRequest,
                    format!("Bedrock Converse does not support role `{}`", message.role),
                    false,
                    context,
                )
                .with_detail("field", format!("messages[{index}].role")));
            }
        }
    }

    if messages.is_empty() {
        return Err(base_error(
            ProviderErrorKind::InvalidRequest,
            "Bedrock Converse requires at least one user or assistant message",
            false,
            context,
        ));
    }

    Ok(BedrockConverseRequest {
        model_id: request.model.clone(),
        messages,
        system_prompts,
        max_tokens: max_tokens_from_env(),
        temperature: temperature_from_env(),
        guardrail: guardrail_from_env(),
    })
}

fn to_bedrock_message(message: &ProviderMessage, role: BedrockRole) -> BedrockMessage {
    BedrockMessage {
        role,
        text: message.content.clone(),
    }
}

fn max_tokens_from_env() -> i32 {
    env::var("GATEWAY_BEDROCK_MAX_TOKENS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

fn temperature_from_env() -> Option<f32> {
    env::var("GATEWAY_BEDROCK_TEMPERATURE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| (0.0..=1.0).contains(value))
}

fn guardrail_from_env() -> Option<BedrockGuardrailConfig> {
    let identifier = env::var("GATEWAY_BEDROCK_GUARDRAIL_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let version = env::var("GATEWAY_BEDROCK_GUARDRAIL_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let trace = env::var("GATEWAY_BEDROCK_GUARDRAIL_TRACE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "disabled".to_string());

    Some(BedrockGuardrailConfig {
        identifier,
        version,
        trace,
    })
}

fn resolve_region(context: &ProviderExecutionContext) -> String {
    context
        .endpoint
        .region
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| env::var("AWS_REGION").ok())
        .or_else(|| env::var("AWS_DEFAULT_REGION").ok())
        .unwrap_or_else(|| DEFAULT_REGION.to_string())
}

fn resolve_endpoint_override(context: &ProviderExecutionContext) -> Option<String> {
    env::var("GATEWAY_BEDROCK_ENDPOINT_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            let endpoint = context.endpoint.endpoint_base_url.trim();
            endpoint.starts_with("http").then(|| endpoint.to_string())
        })
}

fn to_sdk_messages(
    request: &BedrockConverseRequest,
) -> Result<Vec<Message>, BedrockConverseFailure> {
    request
        .messages
        .iter()
        .map(|message| {
            Message::builder()
                .role(match message.role {
                    BedrockRole::User => ConversationRole::User,
                    BedrockRole::Assistant => ConversationRole::Assistant,
                })
                .content(ContentBlock::Text(message.text.clone()))
                .build()
                .map_err(|error| BedrockConverseFailure {
                    kind: ProviderErrorKind::InvalidRequest,
                    message: format!("failed to build Bedrock Converse message: {error}"),
                    retryable: false,
                    upstream_code: None,
                    upstream_status_code: None,
                })
        })
        .collect()
}

fn to_sdk_inference_config(request: &BedrockConverseRequest) -> InferenceConfiguration {
    InferenceConfiguration::builder()
        .max_tokens(request.max_tokens)
        .set_temperature(request.temperature)
        .build()
}

fn to_sdk_guardrail_config(guardrail: &BedrockGuardrailConfig) -> GuardrailConfiguration {
    GuardrailConfiguration::builder()
        .guardrail_identifier(guardrail.identifier.clone())
        .guardrail_version(guardrail.version.clone())
        .trace(GuardrailTrace::from(guardrail.trace.as_str()))
        .build()
}

fn from_sdk_output(
    output: &aws_sdk_bedrockruntime::operation::converse::ConverseOutput,
    model_id: &str,
) -> Result<BedrockConverseResponse, BedrockConverseFailure> {
    let output_text = output
        .output()
        .and_then(|value| value.as_message().ok())
        .map(Message::content)
        .unwrap_or_default()
        .iter()
        .filter_map(|block| block.as_text().ok())
        .fold(String::new(), |mut buffer, text| {
            buffer.push_str(text);
            buffer
        });

    if output_text.is_empty() {
        return Err(BedrockConverseFailure {
            kind: ProviderErrorKind::Protocol,
            message: "Bedrock Converse response did not include text content".to_string(),
            retryable: false,
            upstream_code: None,
            upstream_status_code: None,
        });
    }

    let usage = output.usage();

    Ok(BedrockConverseResponse {
        response_id: None,
        model_id: model_id.to_string(),
        output_text,
        finish_reason: stop_reason_to_str(output.stop_reason()).to_string(),
        input_tokens: usage.map_or(0, |value| non_negative_u32(value.input_tokens())),
        output_tokens: usage.map_or(0, |value| non_negative_u32(value.output_tokens())),
        cached_input_tokens: usage
            .and_then(aws_sdk_bedrockruntime::types::TokenUsage::cache_read_input_tokens)
            .map_or(0, non_negative_u32),
    })
}

fn stop_reason_to_str(stop_reason: &StopReason) -> &str {
    stop_reason.as_str()
}

fn non_negative_u32(value: i32) -> u32 {
    u32::try_from(value).unwrap_or_default()
}

fn map_converse_error(error: &ConverseError) -> BedrockConverseFailure {
    let (kind, retryable) = match error {
        ConverseError::AccessDeniedException(_) => (ProviderErrorKind::Auth, false),
        ConverseError::ValidationException(_) | ConverseError::ResourceNotFoundException(_) => {
            (ProviderErrorKind::InvalidRequest, false)
        }
        ConverseError::ThrottlingException(_) => (ProviderErrorKind::RateLimited, true),
        ConverseError::ModelTimeoutException(_) => (ProviderErrorKind::Timeout, true),
        ConverseError::InternalServerException(_)
        | ConverseError::ModelErrorException(_)
        | ConverseError::ModelNotReadyException(_)
        | ConverseError::ServiceUnavailableException(_) => (ProviderErrorKind::Unavailable, true),
        _ => (ProviderErrorKind::Protocol, false),
    };
    let metadata = error.meta();
    let message = metadata.message().map_or_else(
        || format!("AWS Bedrock Converse failed: {error}"),
        ToString::to_string,
    );

    BedrockConverseFailure {
        kind,
        message,
        retryable,
        upstream_code: metadata.code().map(ToString::to_string),
        upstream_status_code: None,
    }
}

fn base_error(
    kind: ProviderErrorKind,
    message: impl Into<String>,
    retryable: bool,
    context: &ProviderExecutionContext,
) -> ProviderError {
    ProviderError::new(kind, message, retryable)
        .with_detail(
            "provider_resource_id",
            &context.endpoint.provider_resource_id,
        )
        .with_detail("provider_kind", "bedrock")
}

impl BedrockConverseResponse {
    fn into_provider_response(
        self,
        request: &ProviderRequest,
        context: &ProviderExecutionContext,
    ) -> Result<ProviderResponse, ProviderError> {
        if self.output_text.trim().is_empty() {
            return Err(base_error(
                ProviderErrorKind::Protocol,
                "Bedrock Converse response did not include text content",
                false,
                context,
            ));
        }

        Ok(ProviderResponse {
            response_id: self.response_id,
            model: if self.model_id.is_empty() {
                request.model.clone()
            } else {
                self.model_id
            },
            output_text: self.output_text,
            finish_reason: self.finish_reason,
            usage: ProviderUsage {
                input_tokens: self.input_tokens,
                output_tokens: self.output_tokens,
                cached_input_tokens: self.cached_input_tokens,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BedrockConverseAdapter, BedrockConverseFailure, BedrockConverseRequest,
        BedrockConverseResponse, BedrockMessage, BedrockRole, BedrockRuntime,
        build_converse_request,
    };
    use async_trait::async_trait;
    use provider_traits::{
        ProviderAdapter, ProviderEndpoint, ProviderErrorKind, ProviderExecutionContext,
        ProviderMessage, ProviderRequest,
    };
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    struct RecordingRuntime {
        requests: Mutex<Vec<BedrockConverseRequest>>,
        response: Mutex<Result<BedrockConverseResponse, BedrockConverseFailure>>,
    }

    impl Default for RecordingRuntime {
        fn default() -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                response: Mutex::new(Ok(BedrockConverseResponse {
                    response_id: None,
                    model_id: "test-model".to_string(),
                    output_text: "ok".to_string(),
                    finish_reason: "end_turn".to_string(),
                    input_tokens: 0,
                    output_tokens: 0,
                    cached_input_tokens: 0,
                })),
            }
        }
    }

    #[async_trait]
    impl BedrockRuntime for RecordingRuntime {
        async fn converse(
            &self,
            request: BedrockConverseRequest,
            _context: &ProviderExecutionContext,
        ) -> Result<BedrockConverseResponse, BedrockConverseFailure> {
            self.requests.lock().unwrap().push(request);
            self.response.lock().unwrap().clone()
        }
    }

    fn context() -> ProviderExecutionContext {
        ProviderExecutionContext {
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            gateway_service_name: "gateway-api".to_string(),
            gateway_origin: None,
            request_headers: BTreeMap::new(),
            endpoint: ProviderEndpoint {
                provider_resource_id: "prvrsrc_bedrock_primary".to_string(),
                endpoint_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
                api_key: String::new(),
                region: Some("us-east-1".to_string()),
            },
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest {
            model: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
            messages: vec![
                ProviderMessage {
                    role: "system".to_string(),
                    content: "Be brief.".to_string(),
                },
                ProviderMessage {
                    role: "user".to_string(),
                    content: "hello".to_string(),
                },
            ],
            stream: false,
        }
    }

    #[test]
    fn builds_converse_request_with_system_prompt_and_text_messages() {
        let built = build_converse_request(&request(), &context()).unwrap();

        assert_eq!(built.model_id, "anthropic.claude-3-haiku-20240307-v1:0");
        assert_eq!(built.system_prompts, vec!["Be brief."]);
        assert_eq!(
            built.messages,
            vec![BedrockMessage {
                role: BedrockRole::User,
                text: "hello".to_string(),
            }]
        );
        assert_eq!(built.max_tokens, 1024);
    }

    #[tokio::test]
    async fn adapter_maps_bedrock_response_usage_to_provider_response() {
        let runtime = Arc::new(RecordingRuntime {
            requests: Mutex::new(Vec::new()),
            response: Mutex::new(Ok(BedrockConverseResponse {
                response_id: None,
                model_id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
                output_text: "hello from bedrock".to_string(),
                finish_reason: "end_turn".to_string(),
                input_tokens: 12,
                output_tokens: 5,
                cached_input_tokens: 3,
            })),
        });
        let adapter = BedrockConverseAdapter::new(runtime.clone());

        let response = adapter.execute_chat(&request(), &context()).await.unwrap();

        assert_eq!(response.output_text, "hello from bedrock");
        assert_eq!(response.usage.input_tokens, 12);
        assert_eq!(response.usage.cached_input_tokens, 3);
        assert_eq!(
            runtime.requests.lock().unwrap()[0].messages[0].text,
            "hello"
        );
    }

    #[tokio::test]
    async fn adapter_normalizes_throttling_as_retryable_rate_limit() {
        let runtime = Arc::new(RecordingRuntime {
            requests: Mutex::new(Vec::new()),
            response: Mutex::new(Err(BedrockConverseFailure {
                kind: ProviderErrorKind::RateLimited,
                message: "quota exceeded".to_string(),
                retryable: true,
                upstream_code: Some("ThrottlingException".to_string()),
                upstream_status_code: None,
            })),
        });
        let adapter = BedrockConverseAdapter::new(runtime);

        let error = adapter
            .execute_chat(&request(), &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::RateLimited);
        assert!(error.retryable);
        assert_eq!(error.upstream_code.as_deref(), Some("ThrottlingException"));
    }

    #[tokio::test]
    async fn rejects_streaming_in_non_streaming_mode() {
        let adapter = BedrockConverseAdapter::new(Arc::new(RecordingRuntime::default()));
        let mut request = request();
        request.stream = true;

        let error = adapter
            .execute_chat(&request, &context())
            .await
            .unwrap_err();

        assert_eq!(error.kind, ProviderErrorKind::InvalidRequest);
    }
}
