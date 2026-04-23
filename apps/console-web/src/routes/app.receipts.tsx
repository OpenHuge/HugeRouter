import { Badge, Card, Group, Stack, Table, Text } from "@mantine/core";
import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/receipts")({
  loader: () =>
    loadRouteData(() => getConsoleDataService().listRouteReceipts()),
  pendingComponent: () => <RouteLoadingState label="Loading route receipts" />,
  pendingMs: 0,
  component: RouteReceiptsPage,
});

function RouteReceiptsPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Inspect recent route receipts, failures, and target exclusion reasons."
          title="Route receipts"
        />
        <RouteErrorState
          description="Recent route receipts could not be loaded from the control-plane service."
          title="Receipts unavailable"
        />
      </Stack>
    );
  }

  const receipts = result.data;

  return (
    <Stack>
      <PageHeader
        description="Inspect recent route receipts, failures, and target exclusion reasons."
        title="Route receipts"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Recent receipts</Text>
          <Badge color="blue" variant="light">
            {receipts.length}
          </Badge>
        </Group>
        {receipts.length === 0 ? (
          <EmptyCollectionState
            description="Route receipts will appear after the first routed request is recorded."
            title="No route receipts"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Receipt</Table.Th>
                <Table.Th>Route</Table.Th>
                <Table.Th>Result</Table.Th>
                <Table.Th>Selected target</Table.Th>
                <Table.Th>Failure or exclusion reason</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {receipts.map((receipt) => (
                <Table.Tr key={receipt.receiptId}>
                  <Table.Td>
                    <Stack gap={2}>
                      <Text fw={600}>{receipt.receiptId}</Text>
                      <Text c="dimmed" size="sm">
                        {receipt.createdAt}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    <Stack gap={2}>
                      <Text fw={600}>{receipt.routeName}</Text>
                      <Text c="dimmed" size="sm">
                        {receipt.protocolFamily} · {receipt.modelAlias}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    <Badge
                      color={
                        receipt.admissionResult === "admitted"
                          ? "teal"
                          : "orange"
                      }
                      variant="light"
                    >
                      {receipt.admissionResult}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    {receipt.selectedTargetName ?? "No selected target"}
                  </Table.Td>
                  <Table.Td>
                    <Stack gap={2}>
                      <Text size="sm">
                        {receipt.failureReason ??
                          receipt.excludedTargets[0]?.reason ??
                          "Successful receipt with no exclusions."}
                      </Text>
                      {receipt.excludedTargets.length > 0 ? (
                        <Text c="dimmed" size="sm">
                          Excluded:{" "}
                          {receipt.excludedTargets
                            .map((target) => target.reason_code)
                            .join(", ")}
                        </Text>
                      ) : null}
                    </Stack>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  );
}
