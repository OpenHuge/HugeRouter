const protocolLabelByFamily: Record<string, string> = {
  anthropic_messages: "Anthropic Messages",
  gemini_generate_content: "Gemini Generate Content",
  mcp_streamable_http: "MCP Streamable HTTP",
  openai_chat: "OpenAI Chat",
  openai_responses: "OpenAI Responses",
  realtime_webrtc: "Realtime WebRTC",
};

const protocolColorByFamily: Record<string, string> = {
  anthropic_messages: "orange",
  gemini_generate_content: "lime",
  mcp_streamable_http: "indigo",
  openai_chat: "blue",
  openai_responses: "violet",
  realtime_webrtc: "teal",
};

const previewProtocolFamilies = new Set([
  "anthropic_messages",
  "gemini_generate_content",
  "mcp_streamable_http",
]);

export function getProtocolLabel(protocolFamily: string) {
  return protocolLabelByFamily[protocolFamily] ?? protocolFamily;
}

export function getProtocolColor(protocolFamily: string) {
  return protocolColorByFamily[protocolFamily] ?? "gray";
}

export function isPreviewProtocolFamily(protocolFamily: string) {
  return previewProtocolFamilies.has(protocolFamily);
}

export function getProtocolOptionLabel(protocolFamily: string) {
  return isPreviewProtocolFamily(protocolFamily)
    ? `${getProtocolLabel(protocolFamily)} (Preview)`
    : getProtocolLabel(protocolFamily);
}
