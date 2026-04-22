use serde::{Deserialize, Serialize};

const DEFAULT_MAX_TOKENS: u32 = 1024;
const NON_STREAMING_MAX_TOKENS_MIN: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedAnthropicRequest {
    pub model_alias: String,
    pub messages: Vec<NormalizedAnthropicMessage>,
    pub system_prompt: Option<String>,
    pub max_output_tokens: u32,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u16>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedAnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedAnthropicResponse {
    pub response_id: Option<String>,
    pub model: String,
    pub output_text: String,
    pub stop_reason: Option<String>,
    pub usage: NormalizedUsage,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NormalizedUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnthropicMessageRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub top_k: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    PlainText(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum AnthropicContentBlock {
    Text {
        text: String,
    },
    #[serde(other)]
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnthropicMessageResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub content: Vec<AnthropicResponseContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum AnthropicResponseContentBlock {
    Text {
        text: String,
    },
    #[serde(other)]
    Unsupported,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnthropicUsage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MappingError {
    #[error("model must not be empty")]
    EmptyModel,
    #[error("messages must include at least one item")]
    EmptyMessages,
    #[error("stream=true is intentionally not supported in this minimal slice")]
    StreamingNotSupported,
    #[error("messages[{index}].role must not be empty")]
    EmptyRole { index: usize },
    #[error("messages[{index}].content must not be empty")]
    EmptyContent { index: usize },
    #[error("messages[{index}] has unsupported content block for this minimal implementation")]
    UnsupportedContentBlock { index: usize },
    #[error("messages[{index}].content text blocks must be representable as text")]
    UnsupportedContentTextStructure { index: usize },
    #[error("response does not contain assistant text content")]
    ResponseHasNoText,
    #[error("unsupported message role at messages[{index}]: {role}")]
    UnsupportedRole { index: usize, role: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamAnthropicRequest {
    pub model: String,
    pub messages: Vec<UpstreamAnthropicMessage>,
    pub max_tokens: u32,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpstreamAnthropicMessage {
    pub role: String,
    pub content: String,
}

/// # Errors
///
/// Returns an error when the request cannot be represented as the minimal
/// non-streaming Anthropic text-generation slice.
pub fn normalize_request(
    raw: AnthropicMessageRequest,
) -> Result<NormalizedAnthropicRequest, MappingError> {
    if raw.model.trim().is_empty() {
        return Err(MappingError::EmptyModel);
    }
    if raw.messages.is_empty() {
        return Err(MappingError::EmptyMessages);
    }
    if raw.stream.unwrap_or(false) {
        return Err(MappingError::StreamingNotSupported);
    }

    let mut messages = Vec::with_capacity(raw.messages.len());
    for (index, message) in raw.messages.into_iter().enumerate() {
        if message.role.trim().is_empty() {
            return Err(MappingError::EmptyRole { index });
        }
        if !matches!(message.role.as_str(), "user" | "assistant") {
            return Err(MappingError::UnsupportedRole {
                index,
                role: message.role,
            });
        }
        let content = extract_message_text(message.content, index)?;
        if content.trim().is_empty() {
            return Err(MappingError::EmptyContent { index });
        }
        messages.push(NormalizedAnthropicMessage {
            role: message.role,
            content,
        });
    }

    Ok(NormalizedAnthropicRequest {
        model_alias: raw.model,
        messages,
        system_prompt: raw.system.filter(|system| !system.trim().is_empty()),
        max_output_tokens: raw
            .max_tokens
            .unwrap_or(DEFAULT_MAX_TOKENS)
            .max(NON_STREAMING_MAX_TOKENS_MIN),
        temperature: raw.temperature,
        top_p: raw.top_p,
        top_k: raw.top_k,
        stream: false,
    })
}

#[must_use]
pub fn estimate_prompt_tokens(request: &NormalizedAnthropicRequest) -> u32 {
    let mut total = 0u32;
    for message in &request.messages {
        total += estimate_text_tokens(&message.content);
    }
    if let Some(system_prompt) = &request.system_prompt {
        total += estimate_text_tokens(system_prompt);
    }
    total
}

#[must_use]
pub fn map_to_provider_request(request: &NormalizedAnthropicRequest) -> UpstreamAnthropicRequest {
    let messages = request
        .messages
        .iter()
        .map(|message| UpstreamAnthropicMessage {
            role: message.role.clone(),
            content: message.content.clone(),
        })
        .collect();

    UpstreamAnthropicRequest {
        model: request.model_alias.clone(),
        messages,
        max_tokens: request.max_output_tokens,
        stream: request.stream,
    }
}

/// # Errors
///
/// Returns an error when the response contains no text content in the minimal
/// supported content block set.
pub fn response_text(response: &AnthropicMessageResponse) -> Result<String, MappingError> {
    let mut chunks: Vec<&str> = Vec::new();
    for block in &response.content {
        match block {
            AnthropicResponseContentBlock::Text { text } => chunks.push(text.as_str()),
            AnthropicResponseContentBlock::Unsupported => {}
        }
    }

    let output_text = chunks.join("");
    if output_text.trim().is_empty() {
        return Err(MappingError::ResponseHasNoText);
    }
    Ok(output_text)
}

/// # Errors
///
/// Returns an error when the upstream response cannot be normalized into the
/// minimal Anthropic response model.
pub fn normalize_response(
    request: &AnthropicMessageRequest,
    response: &AnthropicMessageResponse,
) -> Result<NormalizedAnthropicResponse, MappingError> {
    let output_text = response_text(response)?;
    Ok(NormalizedAnthropicResponse {
        response_id: response.id.clone(),
        model: response
            .model
            .clone()
            .unwrap_or_else(|| request.model.clone()),
        output_text,
        stop_reason: response.stop_reason.clone(),
        usage: NormalizedUsage {
            input_tokens: response
                .usage
                .as_ref()
                .map_or(0, |usage| usage.input_tokens),
            output_tokens: response
                .usage
                .as_ref()
                .map_or(0, |usage| usage.output_tokens),
        },
    })
}

fn extract_message_text(content: AnthropicContent, index: usize) -> Result<String, MappingError> {
    match content {
        AnthropicContent::PlainText(text) => Ok(text),
        AnthropicContent::Blocks(blocks) => {
            let mut chunks = Vec::new();
            for block in blocks {
                match block {
                    AnthropicContentBlock::Text { text } => chunks.push(text),
                    AnthropicContentBlock::Unsupported => {
                        return Err(MappingError::UnsupportedContentBlock { index });
                    }
                }
            }
            Ok(chunks.join(""))
        }
    }
}

fn estimate_text_tokens(text: &str) -> u32 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        0
    } else {
        u32::try_from(trimmed.len() / 4).unwrap_or(u32::MAX).max(1)
    }
}

impl AnthropicMessageRequest {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        model: impl Into<String>,
        messages: Vec<AnthropicMessage>,
        max_tokens: Option<u32>,
        system: Option<String>,
        stream: bool,
        temperature: Option<f32>,
        top_p: Option<f32>,
        top_k: Option<u16>,
    ) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens,
            system,
            stream: Some(stream),
            temperature,
            top_p,
            top_k,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_plain_text_messages_into_minimal_request() {
        let request = AnthropicMessageRequest {
            model: "claude-opus-4-20250201".to_string(),
            messages: vec![
                AnthropicMessage {
                    role: "user".to_string(),
                    content: AnthropicContent::PlainText("Hello, Claude.".to_string()),
                },
                AnthropicMessage {
                    role: "assistant".to_string(),
                    content: AnthropicContent::PlainText("Replying now.".to_string()),
                },
            ],
            max_tokens: Some(2048),
            system: Some("System prompt".to_string()),
            stream: Some(false),
            temperature: Some(0.9),
            top_p: Some(0.95),
            top_k: Some(32),
        };

        let normalized = normalize_request(request).unwrap();
        let upstream = map_to_provider_request(&normalized);

        assert_eq!(normalized.model_alias, "claude-opus-4-20250201");
        assert_eq!(normalized.system_prompt.as_deref(), Some("System prompt"));
        assert_eq!(normalized.max_output_tokens, 2048);
        assert_eq!(upstream.max_tokens, 2048);
        assert!(estimate_prompt_tokens(&normalized) > 0);
        assert_eq!(upstream.messages[1].content, "Replying now.");
    }

    #[test]
    fn rejects_streaming_requests_in_non_streaming_minimum_path() {
        let request = AnthropicMessageRequest {
            model: "claude-opus-4".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::PlainText("Hey".to_string()),
            }],
            max_tokens: None,
            system: None,
            stream: Some(true),
            temperature: None,
            top_p: None,
            top_k: None,
        };

        let err = normalize_request(request).unwrap_err();
        assert!(matches!(err, MappingError::StreamingNotSupported));
    }

    #[test]
    fn extracts_text_content_from_blocks_and_joins_content() {
        let request = AnthropicMessageRequest {
            model: "claude-opus-4".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Blocks(vec![
                    AnthropicContentBlock::Text {
                        text: "Part one. ".to_string(),
                    },
                    AnthropicContentBlock::Text {
                        text: "Part two.".to_string(),
                    },
                ]),
            }],
            max_tokens: None,
            system: None,
            stream: None,
            temperature: None,
            top_p: None,
            top_k: None,
        };

        let normalized = normalize_request(request).unwrap();
        let text = response_text(&AnthropicMessageResponse {
            id: None,
            model: Some("claude-opus-4".to_string()),
            content: vec![
                AnthropicResponseContentBlock::Text {
                    text: "prefix ".to_string(),
                },
                AnthropicResponseContentBlock::Text {
                    text: "suffix".to_string(),
                },
            ],
            stop_reason: Some("end_turn".to_string()),
            usage: Some(AnthropicUsage {
                input_tokens: 16,
                output_tokens: 8,
            }),
        })
        .unwrap();

        assert_eq!(normalized.messages[0].content, "Part one. Part two.");
        assert_eq!(text, "prefix suffix");
    }

    #[test]
    fn maps_response_text_and_usage_and_model() {
        let request = AnthropicMessageRequest {
            model: "claude-opus-4".to_string(),
            messages: vec![],
            max_tokens: None,
            system: None,
            stream: None,
            temperature: None,
            top_p: None,
            top_k: None,
        };
        let response = AnthropicMessageResponse {
            id: Some("msg_123".to_string()),
            model: None,
            content: vec![AnthropicResponseContentBlock::Text {
                text: "final answer".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: Some(AnthropicUsage {
                input_tokens: 7,
                output_tokens: 9,
            }),
        };

        let normalized = normalize_response(&request, &response).unwrap();

        assert_eq!(normalized.response_id.as_deref(), Some("msg_123"));
        assert_eq!(normalized.model, "claude-opus-4");
        assert_eq!(normalized.output_text, "final answer");
        assert_eq!(normalized.usage.input_tokens, 7);
        assert_eq!(normalized.usage.output_tokens, 9);
        assert_eq!(normalized.stop_reason.as_deref(), Some("end_turn"));
    }
}
