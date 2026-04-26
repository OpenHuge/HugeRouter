import {
  UiChip,
  UiSurface,
  UiInline,
  UiGrid,
  UiStack,
  UiText,
} from "@huge-router/ui-kit";
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
    <UiStack gap={2}>
      <UiText c="dimmed" size="sm">
        {label}
      </UiText>
      <UiText fw={700} size="xl">
        {value}
      </UiText>
    </UiStack>
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
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiInline justify="space-between">
          <UiStack gap={0}>
            <UiText fw={700}>Replay Evidence</UiText>
            <UiText c="dimmed" size="sm">
              Tenant {workspace.tenantId} has {replayableEvaluations} replayable
              evaluations for support review.
            </UiText>
          </UiStack>
          <UiChip
            color={workspace.merchantEnabled ? "teal" : "gray"}
            variant="light"
          >
            {workspace.merchantEnabled ? "Merchant enabled" : "Not enabled"}
          </UiChip>
        </UiInline>
        <UiGrid cols={{ base: 1, sm: 2, lg: 5 }} spacing="md">
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
        </UiGrid>
        <UiInline justify="space-between">
          <UiText c="dimmed" size="sm">
            Latest evaluation
          </UiText>
          {latestEvaluation ? (
            <UiInline gap="xs">
              <UiChip
                color={statusColor(latestEvaluation.verdict)}
                variant="light"
              >
                {latestEvaluation.verdict}
              </UiChip>
              <UiText size="sm">
                {latestEvaluation.providerLabel} at{" "}
                {formatDateTime(latestEvaluation.createdAt)}
              </UiText>
            </UiInline>
          ) : (
            <UiText c="dimmed" size="sm">
              none
            </UiText>
          )}
        </UiInline>
      </UiStack>
    </UiSurface>
  );
}
