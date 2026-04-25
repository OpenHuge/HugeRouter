import { Badge, Card, Group, SimpleGrid, Stack, Text } from "@mantine/core";
import type { MerchantWorkspaceData } from "../control-plane/types";

function statusColor(status: string) {
  if (status === "active" || status === "healthy" || status === "pass") {
    return "teal";
  }

  if (status === "warning" || status === "needs_rotation") {
    return "yellow";
  }

  if (status === "fail" || status === "suspended" || status === "paused") {
    return "red";
  }

  return "gray";
}

function formatDateTime(value?: string) {
  if (!value) {
    return "not recorded";
  }

  return new Intl.DateTimeFormat("en", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "UTC",
  }).format(new Date(value));
}

function MetricBlock({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <Stack gap={2}>
      <Text c="dimmed" size="sm">
        {label}
      </Text>
      <Text fw={700} size="xl">
        {value}
      </Text>
    </Stack>
  );
}

export function MerchantEvidenceSummary({
  workspace,
}: {
  workspace: MerchantWorkspaceData;
}) {
  const activeShops = workspace.shops.filter(
    (shop) => shop.status === "active",
  ).length;
  const activeTrialConnections = workspace.trialConnections.filter(
    (connection) => connection.status === "active",
  ).length;
  const listedInventory = workspace.cardProducts.reduce(
    (total, product) => total + product.inventoryCount,
    0,
  );
  const replayableEvaluations = workspace.recentEvaluations.filter(
    (evaluation) => evaluation.replayCapsuleId,
  ).length;
  const estimatedTokensSaved = workspace.recentEvaluations.reduce(
    (total, evaluation) => total + evaluation.estimatedTokensSaved,
    0,
  );
  const latestEvaluation = workspace.recentEvaluations[0];

  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Group justify="space-between">
          <Stack gap={0}>
            <Text fw={700}>Replay Evidence</Text>
            <Text c="dimmed" size="sm">
              Tenant {workspace.tenantId} has {replayableEvaluations} replayable
              evaluations for support review.
            </Text>
          </Stack>
          <Badge
            color={workspace.merchantEnabled ? "teal" : "gray"}
            variant="light"
          >
            {workspace.merchantEnabled ? "Merchant enabled" : "Not enabled"}
          </Badge>
        </Group>
        <SimpleGrid cols={{ base: 1, sm: 2, lg: 5 }} spacing="md">
          <MetricBlock label="Active shops" value={activeShops} />
          <MetricBlock label="Listed inventory" value={listedInventory} />
          <MetricBlock
            label="Active trial relays"
            value={activeTrialConnections}
          />
          <MetricBlock label="Replay capsules" value={replayableEvaluations} />
          <MetricBlock
            label="Tokens saved"
            value={estimatedTokensSaved.toLocaleString("en")}
          />
        </SimpleGrid>
        <Group justify="space-between">
          <Text c="dimmed" size="sm">
            Latest evaluation
          </Text>
          {latestEvaluation ? (
            <Group gap="xs">
              <Badge
                color={statusColor(latestEvaluation.verdict)}
                variant="light"
              >
                {latestEvaluation.verdict}
              </Badge>
              <Text size="sm">
                {latestEvaluation.providerLabel} at{" "}
                {formatDateTime(latestEvaluation.createdAt)}
              </Text>
            </Group>
          ) : (
            <Text c="dimmed" size="sm">
              none
            </Text>
          )}
        </Group>
      </Stack>
    </Card>
  );
}
