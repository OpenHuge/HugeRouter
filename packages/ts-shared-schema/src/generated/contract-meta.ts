export const CONTRACT_VERSION = "v1" as const;
export const CONTRACT_DIGEST = "7a5b1498c203949271aaef30fa8a86cc07e407f730335054ffb4432551e7a8cd" as const;
export const COMPATIBILITY_RULES = [
'Breaking changes require a new explicit contract version or a new endpoint family.',
'Additive fields must remain optional until all first-party consumers can tolerate them.',
'Enum expansions are additive only when consumers treat unknown values defensively.',
'Serialization key changes are always breaking for v1 contracts.',
'Checked-in schemas, examples, and generated package metadata must be regenerated together.',
] as const;
export const PROTOCOL_FAMILIES = ['openai_chat', 'openai_responses', 'openai_images', 'mcp_streamable_http', 'realtime_webrtc', 'anthropic_messages', 'gemini_generate_content'] as const;
export const ADMISSION_RESULTS = ['admitted', 'rejected_budget', 'rejected_rate_limit', 'rejected_concurrency', 'rejected_policy', 'rejected_no_candidate'] as const;
export const PROVIDER_RESOURCE_STATUSES = ['active', 'disabled', 'draining', 'quarantined', 'deleted'] as const;
export const USAGE_PHASES = ['reserve', 'partial', 'final', 'release'] as const;
