import { Card, Stack, Table, Text } from "@mantine/core";
import type {
  CardProductView,
  MerchantShopView,
  MerchantWorkspaceData,
  TrialConnectionView,
} from "../control-plane/types";
import { EmptyCollectionState } from "../control-plane/route-state";

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
                  <Table.Td>{shop.status}</Table.Td>
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
                  <Table.Td>{product.status}</Table.Td>
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
  workspace,
}: {
  workspace: MerchantWorkspaceData;
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Recent Evaluations</Text>
        {workspace.recentEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Run your first evaluation to produce replay-backed merchant evidence."
            title="No evaluations"
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
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}
