import {
  Badge,
  Button,
  Card,
  Group,
  SegmentedControl,
  Stack,
  Table,
  Text,
} from "@mantine/core";
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
    <Group gap={4}>
      {checks.map(([label, status]) => (
        <Badge color={statusColor(status)} key={label} size="xs" variant="dot">
          {label} {status}
        </Badge>
      ))}
    </Group>
  );
}

export function MerchantShopTable({ shops }: { shops: MerchantShopView[] }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Shops</Text>
        {shops.length === 0 ? (
          <EmptyCollectionState
            description="Open your first small shop to start listing card-secret products."
            title="No shops"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Shop</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Slug</Table.Th>
                <Table.Th>Fulfillment</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {shops.map((shop) => (
                <Table.Tr key={shop.merchantShopId}>
                  <Table.Td>
                    <Text fw={600}>{shop.displayName}</Text>
                    <Text c="dimmed" size="sm">
                      {shop.merchantShopId}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={statusColor(shop.status)} variant="light">
                      {shop.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{shop.slug}</Table.Td>
                  <Table.Td>{shop.fulfillmentMode}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

export function CardProductTable({
  products,
}: {
  products: CardProductView[];
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Card Products</Text>
        {products.length === 0 ? (
          <EmptyCollectionState
            description="Create a card-secret product after your merchant shop is ready."
            title="No card products"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Product</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Inventory</Table.Th>
                <Table.Th>Face Value</Table.Th>
                <Table.Th>Retail Price</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {products.map((product) => (
                <Table.Tr key={product.cardProductId}>
                  <Table.Td>
                    <Text fw={600}>{product.title}</Text>
                    <Text c="dimmed" size="sm">
                      {product.cardProductId}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={statusColor(product.status)} variant="light">
                      {product.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{product.inventoryCount}</Table.Td>
                  <Table.Td>${product.faceValueUsd}</Table.Td>
                  <Table.Td>${product.retailPriceUsd}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

export function TrialConnectionTable({
  connections,
}: {
  connections: TrialConnectionView[];
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Trial Connections</Text>
        {connections.length === 0 ? (
          <EmptyCollectionState
            description="Attach a dedicated test relay before running evaluation."
            title="No trial connections"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Endpoint</Table.Th>
                <Table.Th>Masked Key</Table.Th>
                <Table.Th>Model</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {connections.map((connection) => (
                <Table.Tr key={connection.trialConnectionId}>
                  <Table.Td>
                    <Text fw={600}>{connection.providerLabel}</Text>
                    <Text c="dimmed" size="sm">
                      {connection.trialConnectionId}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge
                      color={statusColor(connection.status)}
                      variant="light"
                    >
                      {connection.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{connection.endpointBaseUrl}</Table.Td>
                  <Table.Td>{connection.apiKeyMasked}</Table.Td>
                  <Table.Td>{connection.targetModel}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
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
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Group justify="space-between">
          <Stack gap={0}>
            <Text fw={700}>Recent Evaluations</Text>
            <Text c="dimmed" size="sm">
              {visibleEvaluations.length} of{" "}
              {workspace.recentEvaluations.length} evaluations shown.
            </Text>
          </Stack>
          <SegmentedControl
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
        </Group>
        {workspace.recentEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Run your first evaluation to produce replay-backed merchant evidence."
            title="No evaluations"
          />
        ) : visibleEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Change the verdict filter or run another evaluation to review more evidence."
            title="No matching evaluations"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Verdict</Table.Th>
                <Table.Th>Score</Table.Th>
                <Table.Th>Replay Capsule</Table.Th>
                <Table.Th>Saved Tokens</Table.Th>
                <Table.Th />
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {visibleEvaluations.map((evaluation) => {
                const isSelected =
                  selectedReplayCapsuleId === evaluation.replayCapsuleId;

                return (
                  <Table.Tr
                    bg={isSelected ? "var(--mantine-color-blue-0)" : undefined}
                    key={evaluation.relayEvaluationId}
                  >
                    <Table.Td>
                      <Text fw={600}>{evaluation.providerLabel}</Text>
                      <Text c="dimmed" size="sm">
                        {evaluation.summary}
                      </Text>
                      <EvaluationCheckBadges evaluation={evaluation} />
                    </Table.Td>
                    <Table.Td>
                      <Badge
                        color={statusColor(evaluation.verdict)}
                        variant="light"
                      >
                        {evaluation.verdict}
                      </Badge>
                    </Table.Td>
                    <Table.Td>{evaluation.overallScore}</Table.Td>
                    <Table.Td>
                      <Group gap="xs">
                        <Text>{evaluation.replayCapsuleId}</Text>
                        {isSelected ? (
                          <Badge color="blue" size="xs" variant="light">
                            Selected
                          </Badge>
                        ) : null}
                      </Group>
                      <Text c="dimmed" size="sm">
                        {evaluation.runnerMode}
                      </Text>
                    </Table.Td>
                    <Table.Td>{evaluation.estimatedTokensSaved}</Table.Td>
                    <Table.Td>
                      <Button
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
                      </Button>
                    </Table.Td>
                  </Table.Tr>
                );
              })}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

export function ReplayCapsuleDetailCard({
  replayCapsule,
}: {
  replayCapsule: ReplayCapsuleView | null;
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Group justify="space-between">
          <Text fw={700}>Replay Capsule</Text>
          {replayCapsule ? (
            <Badge color="teal" variant="light">
              {replayCapsule.redactionTier}
            </Badge>
          ) : null}
        </Group>
        {!replayCapsule ? (
          <EmptyCollectionState
            description="Open a replay capsule from the evaluation table to inspect the redacted request shape and upstream error hint."
            title="No replay capsule selected"
          />
        ) : (
          <>
            <Group grow>
              <Stack gap={0}>
                <Text c="dimmed" size="sm">
                  Replay Capsule ID
                </Text>
                <Text fw={600}>{replayCapsule.replayCapsuleId}</Text>
              </Stack>
              <Stack gap={0}>
                <Text c="dimmed" size="sm">
                  Request / Trace
                </Text>
                <Text fw={600}>{replayCapsule.requestId}</Text>
                <Text c="dimmed" size="sm">
                  {replayCapsule.traceId}
                </Text>
              </Stack>
              <Stack gap={0}>
                <Text c="dimmed" size="sm">
                  Config Snapshot
                </Text>
                <Text fw={600}>{replayCapsule.configSnapshotId}</Text>
              </Stack>
              <Stack gap={0}>
                <Text c="dimmed" size="sm">
                  Route Receipt
                </Text>
                <Text fw={600}>{replayCapsule.routeReceiptId}</Text>
              </Stack>
            </Group>
            <Table striped withRowBorders>
              <Table.Tbody>
                <Table.Tr>
                  <Table.Th>Protocol family</Table.Th>
                  <Table.Td>
                    {replayCapsule.normalizedRequestSummary.protocolFamily}
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th>Model alias</Table.Th>
                  <Table.Td>
                    {replayCapsule.normalizedRequestSummary.modelAlias}
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th>Estimated prompt tokens</Table.Th>
                  <Table.Td>
                    {
                      replayCapsule.normalizedRequestSummary
                        .estimatedPromptTokens
                    }
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th>Upstream error hint</Table.Th>
                  <Table.Td>
                    {replayCapsule.upstreamErrorCode ?? "none"}
                  </Table.Td>
                </Table.Tr>
              </Table.Tbody>
            </Table>
          </>
        )}
      </Stack>
    </Card>
  );
}
