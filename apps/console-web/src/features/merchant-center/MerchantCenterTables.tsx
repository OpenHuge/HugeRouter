import {
  UiChip,
  UiButton,
  UiSurface,
  UiInline,
  UiSegmented,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
import type {
  CardProductView,
  MerchantShopView,
  MerchantWorkspaceData,
  ReplayCapsuleView,
  RelayEvaluationView,
  TrialConnectionView,
} from "../control-plane/types";
import { EmptyCollectionState } from "../control-plane/route-state";

type RelayEvaluationVerdictFilter = "all" | RelayEvaluationView["verdict"];

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

function EvaluationCheckBadges({
  evaluation,
}: {
  evaluation: RelayEvaluationView;
}) {
  const checks = [
    ["Fingerprint", evaluation.fingerprintStatus],
    ["Protocol", evaluation.protocolStatus],
    ["Token", evaluation.tokenStatus],
    ["Multimodal", evaluation.multimodalStatus],
  ] as const;

  return (
    <UiInline gap={4}>
      {checks.map(([label, status]) => (
        <UiChip color={statusColor(status)} key={label} size="xs" variant="dot">
          {label} {status}
        </UiChip>
      ))}
    </UiInline>
  );
}

export function MerchantShopTable({ shops }: { shops: MerchantShopView[] }) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiText fw={700}>Evidence Workspaces</UiText>
        {shops.length === 0 ? (
          <EmptyCollectionState
            description="Create an evidence workspace before attaching supplier test endpoints."
            title="No evidence workspaces"
          />
        ) : (
          <UiDataTable striped withRowBorders>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Workspace</UiDataTable.Th>
                <UiDataTable.Th>Status</UiDataTable.Th>
                <UiDataTable.Th>Slug</UiDataTable.Th>
                <UiDataTable.Th>Fulfillment</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {shops.map((shop) => (
                <UiDataTable.Tr key={shop.merchantShopId}>
                  <UiDataTable.Td>
                    <UiText fw={600}>{shop.displayName}</UiText>
                    <UiText c="dimmed" size="sm">
                      {shop.merchantShopId}
                    </UiText>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip color={statusColor(shop.status)} variant="light">
                      {shop.status}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{shop.slug}</UiDataTable.Td>
                  <UiDataTable.Td>{shop.fulfillmentMode}</UiDataTable.Td>
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiStack>
    </UiSurface>
  );
}

export function CardProductTable({
  products,
}: {
  products: CardProductView[];
}) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiInline justify="space-between">
          <UiText fw={700}>Legacy Card Inventory</UiText>
          <UiChip color="gray" variant="light">
            Private workflow
          </UiChip>
        </UiInline>
        {products.length === 0 ? (
          <EmptyCollectionState
            description="Legacy card-secret inventory is intentionally kept out of the public ku0 Trust Layer path."
            title="No legacy card inventory"
          />
        ) : (
          <UiDataTable striped withRowBorders>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Product</UiDataTable.Th>
                <UiDataTable.Th>Status</UiDataTable.Th>
                <UiDataTable.Th>Sale</UiDataTable.Th>
                <UiDataTable.Th>Inventory</UiDataTable.Th>
                <UiDataTable.Th>Face Value</UiDataTable.Th>
                <UiDataTable.Th>Retail Price</UiDataTable.Th>
                <UiDataTable.Th>CNY Price</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {products.map((product) => (
                <UiDataTable.Tr key={product.cardProductId}>
                  <UiDataTable.Td>
                    <UiText fw={600}>{product.title}</UiText>
                    <UiText c="dimmed" size="sm">
                      {product.cardProductId}
                    </UiText>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip color={statusColor(product.status)} variant="light">
                      {product.status}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
                      color={product.saleEnabled ? "teal" : "gray"}
                      variant="light"
                    >
                      {product.saleEnabled ? "enabled" : "disabled"}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{product.inventoryCount}</UiDataTable.Td>
                  <UiDataTable.Td>${product.faceValueUsd}</UiDataTable.Td>
                  <UiDataTable.Td>${product.retailPriceUsd}</UiDataTable.Td>
                  <UiDataTable.Td>
                    {product.retailPriceCnyTotal == null
                      ? "-"
                      : `¥${(product.retailPriceCnyTotal / 100).toFixed(2)}`}
                  </UiDataTable.Td>
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiStack>
    </UiSurface>
  );
}

export function TrialConnectionTable({
  connections,
}: {
  connections: TrialConnectionView[];
}) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiText fw={700}>Supplier Test Endpoints</UiText>
        {connections.length === 0 ? (
          <EmptyCollectionState
            description="Attach a dedicated supplier test endpoint before recording evidence."
            title="No supplier test endpoints"
          />
        ) : (
          <UiDataTable striped withRowBorders>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Provider</UiDataTable.Th>
                <UiDataTable.Th>Status</UiDataTable.Th>
                <UiDataTable.Th>Endpoint</UiDataTable.Th>
                <UiDataTable.Th>Masked Key</UiDataTable.Th>
                <UiDataTable.Th>Model</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {connections.map((connection) => (
                <UiDataTable.Tr key={connection.trialConnectionId}>
                  <UiDataTable.Td>
                    <UiText fw={600}>{connection.providerLabel}</UiText>
                    <UiText c="dimmed" size="sm">
                      {connection.trialConnectionId}
                    </UiText>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
                      color={statusColor(connection.status)}
                      variant="light"
                    >
                      {connection.status}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{connection.endpointBaseUrl}</UiDataTable.Td>
                  <UiDataTable.Td>{connection.apiKeyMasked}</UiDataTable.Td>
                  <UiDataTable.Td>{connection.targetModel}</UiDataTable.Td>
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiStack>
    </UiSurface>
  );
}

export function RelayEvaluationTable({
  evaluationVerdictFilter,
  isLoadingReplayCapsule,
  onViewReplayCapsule,
  setEvaluationVerdictFilter,
  selectedReplayCapsuleId,
  workspace,
}: {
  evaluationVerdictFilter: string;
  isLoadingReplayCapsule: boolean;
  onViewReplayCapsule: (replayCapsuleId: string) => Promise<void>;
  setEvaluationVerdictFilter: (filter: RelayEvaluationVerdictFilter) => void;
  selectedReplayCapsuleId: string | null;
  workspace: MerchantWorkspaceData;
}) {
  const visibleEvaluations =
    evaluationVerdictFilter === "all"
      ? workspace.recentEvaluations
      : workspace.recentEvaluations.filter(
          (evaluation) => evaluation.verdict === evaluationVerdictFilter,
        );

  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiInline justify="space-between">
          <UiStack gap={0}>
            <UiText fw={700}>Recent Supplier Evidence</UiText>
            <UiText c="dimmed" size="sm">
              {visibleEvaluations.length} of{" "}
              {workspace.recentEvaluations.length} evidence records shown.
            </UiText>
          </UiStack>
          <UiSegmented
            data={[
              { label: "All", value: "all" },
              { label: "Healthy", value: "healthy" },
              { label: "Warning", value: "warning" },
              { label: "Fail", value: "fail" },
            ]}
            onChange={(value) =>
              setEvaluationVerdictFilter(value as RelayEvaluationVerdictFilter)
            }
            size="xs"
            value={evaluationVerdictFilter}
          />
        </UiInline>
        {workspace.recentEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Run your first evidence check to produce replay-backed supplier evidence."
            title="No supplier evidence"
          />
        ) : visibleEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Change the verdict filter or run another evaluation to review more evidence."
            title="No matching evaluations"
          />
        ) : (
          <UiDataTable striped withRowBorders>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Provider</UiDataTable.Th>
                <UiDataTable.Th>Verdict</UiDataTable.Th>
                <UiDataTable.Th>Score</UiDataTable.Th>
                <UiDataTable.Th>Evidence Source</UiDataTable.Th>
                <UiDataTable.Th>Replay Capsule</UiDataTable.Th>
                <UiDataTable.Th>Saved Tokens</UiDataTable.Th>
                <UiDataTable.Th />
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {visibleEvaluations.map((evaluation) => {
                const isSelected =
                  selectedReplayCapsuleId === evaluation.replayCapsuleId;

                return (
                  <UiDataTable.Tr
                    bg={isSelected ? "var(--accent)" : undefined}
                    key={evaluation.relayEvaluationId}
                  >
                    <UiDataTable.Td>
                      <UiText fw={600}>{evaluation.providerLabel}</UiText>
                      <UiText c="dimmed" size="sm">
                        {evaluation.summary}
                      </UiText>
                      <EvaluationCheckBadges evaluation={evaluation} />
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiChip
                        color={statusColor(evaluation.verdict)}
                        variant="light"
                      >
                        {evaluation.verdict}
                      </UiChip>
                    </UiDataTable.Td>
                    <UiDataTable.Td>{evaluation.overallScore}</UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiStack gap={2}>
                        <UiChip color="yellow" size="xs" variant="light">
                          {evaluation.runnerMode}
                        </UiChip>
                        <UiText c="dimmed" size="xs">
                          Not live upstream evidence
                        </UiText>
                      </UiStack>
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiInline gap="xs">
                        <UiText>{evaluation.replayCapsuleId}</UiText>
                        {isSelected ? (
                          <UiChip color="blue" size="xs" variant="light">
                            Selected
                          </UiChip>
                        ) : null}
                      </UiInline>
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      {evaluation.estimatedTokensSaved}
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiButton
                        loading={
                          isLoadingReplayCapsule &&
                          selectedReplayCapsuleId === evaluation.replayCapsuleId
                        }
                        onClick={() =>
                          void onViewReplayCapsule(evaluation.replayCapsuleId)
                        }
                        size="xs"
                        variant="light"
                      >
                        View replay
                      </UiButton>
                    </UiDataTable.Td>
                  </UiDataTable.Tr>
                );
              })}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiStack>
    </UiSurface>
  );
}

export function ReplayCapsuleDetailCard({
  replayCapsule,
}: {
  replayCapsule: ReplayCapsuleView | null;
}) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiInline justify="space-between">
          <UiText fw={700}>Replay Capsule</UiText>
          {replayCapsule ? (
            <UiChip color="teal" variant="light">
              {replayCapsule.redactionTier}
            </UiChip>
          ) : null}
        </UiInline>
        {!replayCapsule ? (
          <EmptyCollectionState
            description="Open a replay capsule from the evaluation table to inspect the redacted request shape and upstream error hint."
            title="No replay capsule selected"
          />
        ) : (
          <>
            <UiInline grow>
              <UiStack gap={0}>
                <UiText c="dimmed" size="sm">
                  Replay Capsule ID
                </UiText>
                <UiText fw={600}>{replayCapsule.replayCapsuleId}</UiText>
              </UiStack>
              <UiStack gap={0}>
                <UiText c="dimmed" size="sm">
                  Request / Trace
                </UiText>
                <UiText fw={600}>{replayCapsule.requestId}</UiText>
                <UiText c="dimmed" size="sm">
                  {replayCapsule.traceId}
                </UiText>
              </UiStack>
              <UiStack gap={0}>
                <UiText c="dimmed" size="sm">
                  Config Snapshot
                </UiText>
                <UiText fw={600}>{replayCapsule.configSnapshotId}</UiText>
              </UiStack>
              <UiStack gap={0}>
                <UiText c="dimmed" size="sm">
                  Route Receipt
                </UiText>
                <UiText fw={600}>{replayCapsule.routeReceiptId}</UiText>
              </UiStack>
            </UiInline>
            <UiDataTable striped withRowBorders>
              <UiDataTable.Tbody>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Protocol family</UiDataTable.Th>
                  <UiDataTable.Td>
                    {replayCapsule.normalizedRequestSummary.protocolFamily}
                  </UiDataTable.Td>
                </UiDataTable.Tr>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Model alias</UiDataTable.Th>
                  <UiDataTable.Td>
                    {replayCapsule.normalizedRequestSummary.modelAlias}
                  </UiDataTable.Td>
                </UiDataTable.Tr>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Estimated prompt tokens</UiDataTable.Th>
                  <UiDataTable.Td>
                    {
                      replayCapsule.normalizedRequestSummary
                        .estimatedPromptTokens
                    }
                  </UiDataTable.Td>
                </UiDataTable.Tr>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Upstream error hint</UiDataTable.Th>
                  <UiDataTable.Td>
                    {replayCapsule.upstreamErrorCode ?? "none"}
                  </UiDataTable.Td>
                </UiDataTable.Tr>
              </UiDataTable.Tbody>
            </UiDataTable>
          </>
        )}
      </UiStack>
    </UiSurface>
  );
}
