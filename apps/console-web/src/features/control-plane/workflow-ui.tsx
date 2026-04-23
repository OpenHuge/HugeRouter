import { Alert, Button, Group, Text } from "@mantine/core";

export const SUPPORTED_ROUTE_CAPABILITIES = [
  "streaming",
  "tool_calling",
  "tool_related",
  "json_mode",
  "chat_completions",
  "realtime",
  "response_model_metadata",
] as const;

export const PROTOCOL_FAMILY_OPTIONS = [
  "openai_chat",
  "openai_responses",
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
    <Text c="red" size="sm">
      {error}
    </Text>
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
    <Alert color={color} radius="md" title={title} variant="light">
      <Group justify="space-between" wrap="nowrap">
        <Text c="dimmed" size="sm">
          {message}
        </Text>
        {onDismiss ? (
          <Button onClick={onDismiss} size="xs" variant="subtle">
            Dismiss
          </Button>
        ) : null}
      </Group>
    </Alert>
  );
}
