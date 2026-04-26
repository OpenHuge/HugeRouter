import { UiAlert, UiButton, UiInline, UiText } from "@huge-router/ui-kit";

export const SUPPORTED_ROUTE_CAPABILITIES = [
  "streaming",
  "tool_calling",
  "tool_related",
  "json_mode",
  "chat_completions",
  "image_generation",
  "realtime",
  "response_model_metadata",
] as const;

export const PROTOCOL_FAMILY_OPTIONS = [
  "openai_chat",
  "openai_responses",
  "openai_images",
  "mcp_streamable_http",
  "realtime_webrtc",
  "anthropic_messages",
  "gemini_generate_content",
] as const;

export function splitCommaSeparatedValues(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function nowUtcTimestamp() {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

export function slugifySegment(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
}

export function buildSuggestedId(prefix: string, label: string) {
  const suffix = slugifySegment(label);
  return suffix ? `${prefix}${suffix}` : prefix;
}

export function FieldErrorText({ error }: { error?: string }) {
  if (!error) {
    return null;
  }

  return (
    <UiText c="red" size="sm">
      {error}
    </UiText>
  );
}

export function ActionStatusNotice({
  error,
  onDismiss,
  success,
}: {
  error?: string | null;
  onDismiss?: () => void;
  success?: string | null;
}) {
  if (!error && !success) {
    return null;
  }

  const color = error ? "red" : "green";
  const title = error ? "Action failed" : "Action completed";
  const message = error ?? success ?? "";

  return (
    <UiAlert color={color} radius="md" title={title} variant="light">
      <UiInline justify="space-between" wrap="nowrap">
        <UiText c="dimmed" size="sm">
          {message}
        </UiText>
        {onDismiss ? (
          <UiButton onClick={onDismiss} size="xs" variant="subtle">
            Dismiss
          </UiButton>
        ) : null}
      </UiInline>
    </UiAlert>
  );
}
