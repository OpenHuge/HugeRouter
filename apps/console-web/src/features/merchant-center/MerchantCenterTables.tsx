import { Badge, Button, Card, Group, Stack, Table, Text } from "@mantine/core";
import type {
  CardProductView,
  MerchantShopView,
  MerchantWorkspaceData,
  ReplayCapsuleView,
  TradeOrderView,
  TrialConnectionView,
} from "../control-plane/types";
import { EmptyCollectionState } from "../control-plane/route-state";

export function MerchantShopTable({ shops }: { shops: MerchantShopView[] }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Account Library Vendors</Text>
        {shops.length === 0 ? (
          <EmptyCollectionState
            description="Create a vendor profile before submitting account or access listings."
            title="No vendor profiles"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Shop</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Identity</Table.Th>
                <Table.Th>Deposit</Table.Th>
                <Table.Th>Disputes</Table.Th>
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
                  <Table.Td>{shop.status}</Table.Td>
                  <Table.Td>{shop.identityLevel}</Table.Td>
                  <Table.Td>${shop.guaranteeDepositUsd}</Table.Td>
                  <Table.Td>{(shop.disputeRateBps / 100).toFixed(2)}%</Table.Td>
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
        <Text fw={700}>Account Library Listings</Text>
        {products.length === 0 ? (
          <EmptyCollectionState
            description="Create an approved listing after the vendor profile is ready."
            title="No account listings"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Product</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Trust</Table.Th>
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
                  <Table.Td>{product.status}</Table.Td>
                  <Table.Td>
                    <Text>{product.reviewStatus}</Text>
                    <Text c="dimmed" size="sm">
                      {product.riskTier} / {product.escrowMode}
                    </Text>
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

export function TradeOrderTable({ orders }: { orders: TradeOrderView[] }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Account Library Orders</Text>
        {orders.length === 0 ? (
          <EmptyCollectionState
            description="Protected orders appear after a buyer funds an approved account listing."
            title="No protected orders"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Order</Table.Th>
                <Table.Th>State</Table.Th>
                <Table.Th>Escrow</Table.Th>
                <Table.Th>Evidence</Table.Th>
                <Table.Th>Dispute</Table.Th>
                <Table.Th>Amount</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {orders.map((order) => (
                <Table.Tr key={order.tradeOrderId}>
                  <Table.Td>
                    <Text fw={600}>{order.tradeOrderId}</Text>
                    <Text c="dimmed" size="sm">
                      {order.buyerAlias} / {order.sellerAlias}
                    </Text>
                  </Table.Td>
                  <Table.Td>{order.state}</Table.Td>
                  <Table.Td>{order.escrowMode}</Table.Td>
                  <Table.Td>{order.evidenceState}</Table.Td>
                  <Table.Td>{order.disputeState}</Table.Td>
                  <Table.Td>${order.orderAmountUsd}</Table.Td>
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
        <Text fw={700}>Relay Library Sources</Text>
        {connections.length === 0 ? (
          <EmptyCollectionState
            description="Register a dedicated test relay before running quality checks."
            title="No relay sources"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Provider</Table.Th>
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
  isLoadingReplayCapsule,
  onViewReplayCapsule,
  selectedReplayCapsuleId,
  workspace,
}: {
  isLoadingReplayCapsule: boolean;
  onViewReplayCapsule: (replayCapsuleId: string) => Promise<void>;
  selectedReplayCapsuleId: string | null;
  workspace: MerchantWorkspaceData;
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Relay Library Evaluations</Text>
        {workspace.recentEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Run your first evaluation to produce replay-backed relay trust evidence."
            title="No relay evaluations"
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
              {workspace.recentEvaluations.map((evaluation) => (
                <Table.Tr key={evaluation.relayEvaluationId}>
                  <Table.Td>
                    <Text fw={600}>{evaluation.providerLabel}</Text>
                    <Text c="dimmed" size="sm">
                      {evaluation.summary}
                    </Text>
                  </Table.Td>
                  <Table.Td>{evaluation.verdict}</Table.Td>
                  <Table.Td>{evaluation.overallScore}</Table.Td>
                  <Table.Td>
                    <Text>{evaluation.replayCapsuleId}</Text>
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
              ))}
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
          <Text fw={700}>Info Library Evidence Capsule</Text>
          {replayCapsule ? (
            <Badge color="teal" variant="light">
              {replayCapsule.redactionTier}
            </Badge>
          ) : null}
        </Group>
        {!replayCapsule ? (
          <EmptyCollectionState
            description="Open a relay evidence capsule to inspect the redacted request shape and upstream error hint."
            title="No evidence capsule selected"
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
