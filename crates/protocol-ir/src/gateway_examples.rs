#![allow(clippy::wildcard_imports)]

use super::*;
use crate::examples::{sample_normalized_error, sample_route_receipt, sample_usage_event};

pub fn sample_gateway_chat_request() -> GatewayChatRequest {
    GatewayChatRequest {
        request: RequestEnvelope::new(
            ProtocolFamily::OpenAiChat,
            ServiceName::parse("gateway-api").unwrap(),
            "req_123",
            "trace_123",
            TenantId::parse("tenant_acme").unwrap(),
            ProjectId::parse("proj_core").unwrap(),
        ),
        chat: ChatRequest {
            model_alias: "reasoning-fast".to_string(),
            messages: vec![
                ChatMessage {
                    role: ChatMessageRole::System,
                    content: "You are a concise routing assistant.".to_string(),
                },
                ChatMessage {
                    role: ChatMessageRole::User,
                    content: "Summarize the last deployment incident.".to_string(),
                },
            ],
            required_capabilities: vec!["tool_calling".to_string(), "json_mode".to_string()],
            expected_prompt_tokens: 12_000,
            max_output_tokens: 1_024,
            temperature_milli: 200,
            tools: vec![ToolDefinition {
                name: "incident_lookup".to_string(),
                description: "Look up incident summaries from the operations system.".to_string(),
            }],
            conversation_id: Some("conv_ops_42".to_string()),
        },
    }
}

pub fn sample_gateway_chat_response() -> GatewayChatResponse {
    GatewayChatResponse {
        route_receipt: sample_route_receipt(),
        usage_event: sample_usage_event(),
        provider_response_id: "resp_openai_123".to_string(),
        output_text: "The incident was caused by a stale config snapshot activation.".to_string(),
    }
}

pub fn sample_gateway_anthropic_messages_request() -> GatewayAnthropicMessagesRequest {
    GatewayAnthropicMessagesRequest {
        model: "claude-3-opus".to_string(),
        messages: vec![GatewayAnthropicMessage {
            role: "user".to_string(),
            content: GatewayAnthropicMessageContent::Blocks(vec![
                GatewayAnthropicMessageContentBlock::Text {
                    text: "Summarize the last outage.".to_string(),
                },
            ]),
        }],
        max_tokens: Some(768),
        system: Some("You are an observability analyst.".to_string()),
        stream: Some(false),
        temperature: Some(0.3),
        top_p: Some(0.95),
        top_k: Some(40),
    }
}

pub fn sample_gateway_anthropic_messages_response() -> GatewayAnthropicMessagesResponse {
    GatewayAnthropicMessagesResponse {
        id: Some("msg_123".to_string()),
        model: Some("claude-3-opus".to_string()),
        content: vec![GatewayAnthropicResponseContentBlock {
            kind: "text".to_string(),
            text: "The outage was caused by a transient worker restart in eu-west.".to_string(),
        }],
        stop_reason: Some("end_turn".to_string()),
        usage: Some(GatewayAnthropicUsage {
            input_tokens: 123,
            output_tokens: 42,
        }),
    }
}

pub fn sample_gateway_anthropic_messages_error() -> GatewayAnthropicMessagesError {
    GatewayAnthropicMessagesError {
        error: sample_normalized_error("Anthropic messages validation failed"),
    }
}

pub fn sample_gateway_gemini_generate_content_request() -> GatewayGeminiGenerateContentRequest {
    GatewayGeminiGenerateContentRequest {
        model: "gemini-1.5-pro".to_string(),
        contents: vec![GatewayGeminiContent {
            role: GatewayGeminiRole::User,
            parts: vec![GatewayGeminiPart {
                text: "Summarize the last outage incident and mitigation steps.".to_string(),
            }],
        }],
        tools: Vec::new(),
        stream: false,
        system_instruction: Some(GatewayGeminiSystemInstruction {
            parts: vec![GatewayGeminiPart {
                text: "Be concise and technical.".to_string(),
            }],
        }),
        generation_config: Some(GatewayGeminiGenerationConfig {
            max_output_tokens: Some(1024),
            temperature: Some(0.4),
        }),
    }
}

pub fn sample_gateway_gemini_generate_content_response() -> GatewayGeminiGenerateContentResponse {
    GatewayGeminiGenerateContentResponse {
        response_id: Some("resp_gemini_123".to_string()),
        candidates: vec![GatewayGeminiCandidate {
            content: Some(GatewayGeminiContent {
                role: GatewayGeminiRole::Model,
                parts: vec![GatewayGeminiPart {
                    text: "The outage was likely triggered by routing policy misconfiguration."
                        .to_string(),
                }],
            }),
            finish_reason: Some("STOP".to_string()),
        }],
        usage_metadata: Some(GatewayGeminiUsageMetadata {
            prompt_token_count: 88,
            candidates_token_count: 27,
            total_token_count: 115,
            cached_content_token_count: 0,
        }),
        model_version: Some("gemini-1.5-pro-latest".to_string()),
    }
}

pub fn sample_gateway_gemini_generate_content_error() -> GatewayGeminiGenerateContentError {
    GatewayGeminiGenerateContentError {
        error: sample_normalized_error("Gemini generate-content validation failed"),
    }
}
