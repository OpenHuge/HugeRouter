#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::derive_partial_eq_without_eq,
    clippy::filter_map_next,
    clippy::manual_clamp,
    clippy::map_unwrap_or,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]

use protocol_ir::{ChatMessage, ChatMessageRole, ChatRequest};
use provider_traits::{ProviderMessage, ProviderRequest, ProviderResponse, ProviderUsage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

pub const PROTOCOL_FAMILY_GEMINI: &str = "gemini_generate_content";

const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 1024;
const DEFAULT_TEMPERATURE_MILLI: u16 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityErrorKind {
    EmptyModel,
    MissingContents,
    EmptyContent,
    MissingTextPart,
    UnsupportedPartType,
    UnsupportedRole,
    UnsupportedTools,
    UnsupportedStreaming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityError {
    pub kind: CompatibilityErrorKind,
    pub field: &'static str,
    pub detail: String,
}

impl CompatibilityError {
    fn new(kind: CompatibilityErrorKind, field: &'static str, detail: impl Into<String>) -> Self {
        Self {
            kind,
            field,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "gemini mapping error ({:?}) on {}: {}",
            self.kind, self.field, self.detail,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub model: String,
    pub contents: Vec<Content>,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(rename = "systemInstruction", default)]
    pub system_instruction: Option<SystemInstruction>,
    #[serde(default)]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub role: GeminiRole,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GeminiRole {
    User,
    Assistant,
    Model,
    System,
}

impl GeminiRole {
    fn as_provider_role(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant | Self::Model => "assistant",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Part {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(flatten)]
    pub unsupported: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemInstruction {
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(rename = "temperature")]
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    #[serde(rename = "responseId", default)]
    pub response_id: Option<String>,
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata", default)]
    pub usage_metadata: Option<UsageMetadata>,
    #[serde(rename = "modelVersion", default)]
    pub model_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    #[serde(default)]
    pub content: Option<Content>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount", default)]
    pub total_token_count: u32,
    #[serde(rename = "cachedContentTokenCount", default)]
    pub cached_content_token_count: u32,
}

pub const COMPATIBILITY_NOTES: [&str; 4] = [
    "Non-streaming text generation only (stream=true is rejected).",
    "Tool declarations are not supported in this minimal slice.",
    "Only plain text parts are supported in Gemini contents.",
    "System instructions may only use plain text parts and map to an internal system message.",
];

pub fn compatibility_notes() -> &'static [&'static str] {
    &COMPATIBILITY_NOTES
}

fn clamp_or_default_temperature_milli(temperature: Option<f32>) -> u16 {
    temperature
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0).min(1000.0).round() as u16)
        .unwrap_or(DEFAULT_TEMPERATURE_MILLI)
}

pub fn validate_generate_content_request(
    request: &GenerateContentRequest,
) -> Result<(), CompatibilityError> {
    if request.model.trim().is_empty() {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::EmptyModel,
            "model",
            "model must be provided",
        ));
    }

    if request.stream {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::UnsupportedStreaming,
            "stream",
            "streaming is not supported in this minimal non-streaming implementation",
        ));
    }

    if !request.tools.is_empty() {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::UnsupportedTools,
            "tools",
            "tool use is not supported in the current minimal integration",
        ));
    }

    if request.contents.is_empty() {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::MissingContents,
            "contents",
            "at least one content block is required",
        ));
    }

    if request
        .contents
        .iter()
        .any(|content| content.parts.is_empty())
    {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::EmptyContent,
            "contents[*].parts",
            "at least one part is required in each content entry",
        ));
    }

    for content in &request.contents {
        for part in &content.parts {
            if part.text.is_none() || !part.unsupported.is_empty() {
                return Err(CompatibilityError::new(
                    CompatibilityErrorKind::UnsupportedPartType,
                    "contents[*].parts[*]",
                    "content part requires plain text only",
                ));
            }
        }
    }

    if let Some(system_instruction) = &request.system_instruction {
        if system_instruction.parts.is_empty() {
            return Err(CompatibilityError::new(
                CompatibilityErrorKind::EmptyContent,
                "systemInstruction.parts",
                "system instruction must include at least one part when provided",
            ));
        }

        for part in &system_instruction.parts {
            if part.text.is_none() || !part.unsupported.is_empty() {
                return Err(CompatibilityError::new(
                    CompatibilityErrorKind::UnsupportedPartType,
                    "systemInstruction.parts[*]",
                    "system instruction supports only plain text part",
                ));
            }
        }
    }

    Ok(())
}

pub fn to_upstream_generate_content_request(
    request: &ProviderRequest,
) -> Result<GenerateContentRequest, CompatibilityError> {
    if request.model.trim().is_empty() {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::EmptyModel,
            "model",
            "model must be provided",
        ));
    }

    if request.messages.is_empty() {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::MissingContents,
            "messages",
            "at least one message is required",
        ));
    }

    let mut contents = Vec::with_capacity(request.messages.len());

    for message in &request.messages {
        if message.content.trim().is_empty() {
            return Err(CompatibilityError::new(
                CompatibilityErrorKind::MissingTextPart,
                "messages[*].content",
                "message content must not be empty",
            ));
        }

        let role = match message.role.as_str() {
            "system" => GeminiRole::System,
            "user" => GeminiRole::User,
            "assistant" => GeminiRole::Assistant,
            _ => {
                return Err(CompatibilityError::new(
                    CompatibilityErrorKind::UnsupportedRole,
                    "messages[*].role",
                    "message role is not supported in Gemini adapter input",
                ));
            }
        };

        contents.push(Content {
            role,
            parts: vec![Part {
                text: Some(message.content.clone()),
                unsupported: BTreeMap::new(),
            }],
        });
    }

    Ok(GenerateContentRequest {
        model: request.model.clone(),
        contents,
        tools: Vec::new(),
        stream: false,
        system_instruction: None,
        generation_config: None,
    })
}

pub fn to_provider_request(
    request: &GenerateContentRequest,
) -> Result<ProviderRequest, CompatibilityError> {
    validate_generate_content_request(request)?;

    let mut messages = Vec::with_capacity(
        request.contents.len() + usize::from(request.system_instruction.is_some()),
    );

    if let Some(system_instruction) = &request.system_instruction {
        let content =
            extract_text_from_parts(&system_instruction.parts, "systemInstruction.parts")?;
        messages.push(ProviderMessage {
            role: "system".to_string(),
            content,
        });
    }

    for content in &request.contents {
        let content_text = extract_text_from_parts(&content.parts, "contents[*].parts")?;
        messages.push(ProviderMessage {
            role: content.role.as_provider_role().to_string(),
            content: content_text,
        });
    }

    Ok(ProviderRequest {
        model: request.model.clone(),
        messages,
        stream: false,
    })
}

pub fn to_protocol_ir_request(
    request: &GenerateContentRequest,
) -> Result<ChatRequest, CompatibilityError> {
    let provider_request = to_provider_request(request)?;

    let messages = provider_request
        .messages
        .into_iter()
        .map(|message| {
            let role = match message.role.as_str() {
                "system" => ChatMessageRole::System,
                "user" => ChatMessageRole::User,
                "assistant" => ChatMessageRole::Assistant,
                _ => panic!("validated role should be one of system/user/assistant"),
            };

            ChatMessage {
                role,
                content: message.content,
            }
        })
        .collect::<Vec<_>>();

    Ok(ChatRequest {
        model_alias: provider_request.model,
        messages,
        required_capabilities: Vec::new(),
        expected_prompt_tokens: 0,
        max_output_tokens: request
            .generation_config
            .as_ref()
            .and_then(|config| config.max_output_tokens)
            .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
        temperature_milli: clamp_or_default_temperature_milli(
            request
                .generation_config
                .as_ref()
                .and_then(|config| config.temperature),
        ),
        tools: Vec::new(),
        conversation_id: None,
    })
}

pub fn from_provider_response(response: &ProviderResponse) -> GenerateContentResponse {
    GenerateContentResponse {
        response_id: response.response_id.clone(),
        candidates: vec![Candidate {
            content: Some(Content {
                role: GeminiRole::Model,
                parts: vec![Part {
                    text: Some(response.output_text.clone()),
                    unsupported: BTreeMap::new(),
                }],
            }),
            finish_reason: Some(response.finish_reason.clone()),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: response.usage.input_tokens,
            candidates_token_count: response.usage.output_tokens,
            total_token_count: response
                .usage
                .input_tokens
                .saturating_add(response.usage.output_tokens),
            cached_content_token_count: response.usage.cached_input_tokens,
        }),
        model_version: Some(response.model.clone()),
    }
}

pub fn to_provider_response(
    response: &GenerateContentResponse,
    request_model: &str,
) -> Option<ProviderResponse> {
    let output_text = response
        .candidates
        .iter()
        .filter_map(|candidate| candidate.content.as_ref())
        .filter_map(|content| {
            content
                .parts
                .iter()
                .filter_map(|part| part.text.as_deref())
                .map(str::trim)
                .find(|text| !text.is_empty())
                .map(ToOwned::to_owned)
        })
        .next();

    let output_text = output_text?;
    let usage = response.usage_metadata.as_ref();
    let finish_reason = response
        .candidates
        .iter()
        .find_map(|candidate| candidate.finish_reason.clone())
        .unwrap_or_else(|| "stop".to_string());

    Some(ProviderResponse {
        response_id: response.response_id.clone(),
        model: response
            .model_version
            .clone()
            .unwrap_or_else(|| request_model.to_string()),
        output_text,
        finish_reason,
        usage: ProviderUsage {
            input_tokens: usage.map_or(0, |metadata| metadata.prompt_token_count),
            output_tokens: usage.map_or(0, |metadata| metadata.candidates_token_count),
            cached_input_tokens: usage.map_or(0, |metadata| metadata.cached_content_token_count),
        },
    })
}

fn extract_text_from_parts(
    parts: &[Part],
    field: &'static str,
) -> Result<String, CompatibilityError> {
    let text = parts
        .iter()
        .filter_map(|part| part.text.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err(CompatibilityError::new(
            CompatibilityErrorKind::MissingTextPart,
            field,
            "content resolved to empty output text",
        ));
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> GenerateContentRequest {
        GenerateContentRequest {
            model: "gemini-1.5-flash-latest".to_string(),
            contents: vec![Content {
                role: GeminiRole::User,
                parts: vec![Part {
                    text: Some("Hello".to_string()),
                    unsupported: BTreeMap::new(),
                }],
            }],
            tools: Vec::new(),
            stream: false,
            system_instruction: None,
            generation_config: None,
        }
    }

    #[test]
    fn map_request_to_provider_request() {
        let request = sample_request();
        let provider_request = to_provider_request(&request).expect("request should map");

        assert_eq!(provider_request.model, "gemini-1.5-flash-latest");
        assert_eq!(provider_request.messages.len(), 1);
        assert_eq!(
            provider_request.messages,
            vec![provider_traits::ProviderMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }]
        );
    }

    #[test]
    fn map_provider_request_to_upstream_request() {
        let request = ProviderRequest {
            model: "gemini-1.5-flash-latest".to_string(),
            messages: vec![provider_traits::ProviderMessage {
                role: "user".to_string(),
                content: "Hello from provider".to_string(),
            }],
            stream: false,
        };

        let upstream =
            to_upstream_generate_content_request(&request).expect("provider request should map");

        assert_eq!(upstream.model, "gemini-1.5-flash-latest");
        assert_eq!(upstream.contents.len(), 1);
        assert_eq!(
            upstream.contents[0].parts,
            vec![Part {
                text: Some("Hello from provider".to_string()),
                unsupported: BTreeMap::new(),
            }]
        );
    }

    #[test]
    fn rejects_streaming_requests() {
        let mut request = sample_request();
        request.stream = true;

        let error = validate_generate_content_request(&request).unwrap_err();

        assert_eq!(error.kind, CompatibilityErrorKind::UnsupportedStreaming);
    }

    #[test]
    fn rejects_tools() {
        let mut request = sample_request();
        request
            .tools
            .push(serde_json::json!({"name": "calculator"}));

        let error = validate_generate_content_request(&request).unwrap_err();

        assert_eq!(error.kind, CompatibilityErrorKind::UnsupportedTools);
    }

    #[test]
    fn rejects_non_text_parts() {
        let mut request = sample_request();
        request.contents[0].parts[0].text = None;

        let error = validate_generate_content_request(&request).unwrap_err();

        assert_eq!(error.kind, CompatibilityErrorKind::UnsupportedPartType);
    }

    #[test]
    fn map_provider_response_to_gemini_response() {
        let response = ProviderResponse {
            response_id: Some("resp-1".to_string()),
            model: "gemini-1.5-flash-latest".to_string(),
            output_text: "Done".to_string(),
            finish_reason: "stop".to_string(),
            usage: ProviderUsage {
                input_tokens: 6,
                output_tokens: 4,
                cached_input_tokens: 2,
            },
        };

        let mapped = from_provider_response(&response);

        assert_eq!(mapped.response_id.as_deref(), Some("resp-1"));
        let candidate = mapped.candidates.first().expect("candidate exists");
        let part = candidate
            .content
            .as_ref()
            .expect("content exists")
            .parts
            .first()
            .expect("part exists");
        assert_eq!(part.text.as_deref(), Some("Done"));
        assert_eq!(mapped.usage_metadata.unwrap().cached_content_token_count, 2);
    }

    #[test]
    fn map_upstream_response_to_provider_response() {
        let response = GenerateContentResponse {
            response_id: Some("resp-2".to_string()),
            candidates: vec![Candidate {
                content: Some(Content {
                    role: GeminiRole::Model,
                    parts: vec![Part {
                        text: Some("Hello Gemini".to_string()),
                        unsupported: BTreeMap::new(),
                    }],
                }),
                finish_reason: Some("stop".to_string()),
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 5,
                candidates_token_count: 7,
                total_token_count: 12,
                cached_content_token_count: 1,
            }),
            model_version: Some("gemini-1.5-flash-latest".to_string()),
        };

        let provider_response = to_provider_response(&response, "fallback-model")
            .expect("provider response should map");

        assert_eq!(provider_response.response_id.as_deref(), Some("resp-2"));
        assert_eq!(provider_response.model.as_str(), "gemini-1.5-flash-latest");
        assert_eq!(provider_response.output_text, "Hello Gemini");
        assert_eq!(provider_response.usage.input_tokens, 5);
        assert_eq!(provider_response.usage.output_tokens, 7);
        assert_eq!(provider_response.usage.cached_input_tokens, 1);
    }

    #[test]
    fn map_request_to_protocol_ir() {
        let request = sample_request();
        let mapped = to_protocol_ir_request(&request).expect("request should map");

        assert_eq!(mapped.model_alias, "gemini-1.5-flash-latest");
        assert_eq!(mapped.max_output_tokens, DEFAULT_MAX_OUTPUT_TOKENS);
        assert_eq!(mapped.messages[0].content, "Hello");
        assert_eq!(mapped.temperature_milli, DEFAULT_TEMPERATURE_MILLI);
    }
}
