import { Badge, Card, Group, Stack, Table, Text } from "@mantine/core";
import { Link, createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/route-diagnostics/$routePolicyId")({
  loader: ({ params }) =>
    loadRouteData(() =>
      getConsoleDataService().getRouteDiagnostics(params.routePolicyId),
    ),
  pendingComponent: () => (
    <RouteLoadingState label="Loading route diagnostics" />
  ),
  pendingMs: 0,
  component: RouteDiagnosticsPage,
});

function RouteDiagnosticsPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Explain why HugeRouter selected or excluded each target for the current route."
          title="Route diagnostics"
        />
        <RouteErrorState
          description="The route diagnostics drill-down could not be loaded from the control-plane service."
          title="Diagnostics unavailable"
        />
      </Stack>
    );
  }

  const { activeSnapshotId, activeSnapshotMatchesRoutePolicy, diagnostics } =
    result.data;

  return (
    <Stack>
      <PageHeader
        description="Explain why HugeRouter selected or excluded each target for the current route."
        title={diagnostics.route_policy.display_name}
      />
      <Group justify="space-between">
        <Stack gap={2}>
          <Text c="dimmed" size="sm">
            {diagnostics.route_policy.protocol_family} ·{" "}
            {diagnostics.route_policy.model_alias}
          </Text>
          <Text c="dimmed" size="sm">
            Required capabilities:{" "}
            {diagnostics.route_policy.required_capabilities.join(", ")}
          </Text>
        </Stack>
        <Badge
          color={activeSnapshotMatchesRoutePolicy ? "teal" : "gray"}
          variant="light"
        >
          {activeSnapshotMatchesRoutePolicy
            ? `active snapshot ${activeSnapshotId}`
            : activeSnapshotId
              ? `active snapshot ${activeSnapshotId} is routing a different policy`
              : "no active snapshot"}
        </Badge>
      </Group>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Target decision matrix</Text>
          <Badge color="blue" variant="light">
            {diagnostics.targets.length} targets
          </Badge>
        </Group>
        <Table striped withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Target</Table.Th>
              <Table.Th>Decision</Table.Th>
              <Table.Th>Protocol</Table.Th>
              <Table.Th>Capability gaps</Table.Th>
              <Table.Th>Health</Table.Th>
              <Table.Th>Reason</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {diagnostics.targets.map((target) => (
              <Table.Tr key={target.provider_resource.provider_resource_id}>
                <Table.Td>
                  <Stack gap={2}>
                    <Group gap="xs">
                      <Text fw={600}>{target.provider_resource.name}</Text>
                      {target.provider_resource.is_transit_gateway ? (
                        <Badge color="indigo" variant="light">
                          transit
                        </Badge>
                      ) : null}
                    </Group>
                    <Text c="dimmed" size="sm">
                      {target.provider_resource.provider_id} ·{" "}
                      {target.provider_resource.region}
                    </Text>
                  </Stack>
                </Table.Td>
                <Table.Td>
                  <Badge color={decisionColor(target.decision)} variant="light">
                    {target.decision}
                  </Badge>
                </Table.Td>
                <Table.Td>
                  <Badge
                    color={target.supports_protocol_family ? "teal" : "red"}
                    variant="light"
                  >
                    {target.supports_protocol_family
                      ? "supported"
                      : "unsupported"}
                  </Badge>
                </Table.Td>
                <Table.Td>
                  {target.capability_gaps.length > 0
                    ? target.capability_gaps.join(", ")
                    : "None"}
                </Table.Td>
                <Table.Td>
                  <Stack gap={2}>
                    <Badge
                      color={healthColor(target.provider_resource.health_state)}
                      variant="light"
                    >
                      {target.provider_resource.health_state}
                    </Badge>
                    <Text c="dimmed" size="sm">
                      {target.provider_resource.quarantine_reason ??
                        target.provider_resource.health_message ??
                        "No health notes."}
                    </Text>
                  </Stack>
                </Table.Td>
                <Table.Td>
                  <Stack gap={2}>
                    <Text size="sm">{target.reason}</Text>
                    <Text c="dimmed" size="sm">
                      {target.in_active_snapshot
                        ? "Included in active snapshot"
                        : "Not in active snapshot set"}
                    </Text>
                    {target.recent_receipt_id ? (
                      <Text c="dimmed" size="sm">
                        Receipt: {target.recent_receipt_id}
                      </Text>
                    ) : null}
                  </Stack>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Recent receipts</Text>
          <Badge color="gray" variant="light">
            {diagnostics.recent_receipts.length}
          </Badge>
        </Group>
        {diagnostics.recent_receipts.length === 0 ? (
          <Text c="dimmed" size="sm">
            No recent receipts for this route policy.
          </Text>
        ) : (
          <Stack gap="xs">
            {diagnostics.recent_receipts.map((receipt) => (
              <Card
                key={receipt.route_receipt_id}
                padding="md"
                radius="md"
                withBorder
              >
                <Group justify="space-between">
                  <Stack gap={2}>
                    <Text fw={600}>{receipt.route_receipt_id}</Text>
                    <Text c="dimmed" size="sm">
                      {receipt.created_at}
                    </Text>
                  </Stack>
                  <Badge
                    color={
                      receipt.admission_result === "admitted"
                        ? "teal"
                        : "orange"
                    }
                    variant="light"
                  >
                    {receipt.admission_result}
                  </Badge>
                </Group>
                <Text c="dimmed" mt="xs" size="sm">
                  {receipt.failure_reason ?? "Successful routing receipt."}
                </Text>
              </Card>
            ))}
          </Stack>
        )}
      </Card>
      <Text c="dimmed" size="sm">
        Return to <Link to="/app/routes">Routes</Link> or{" "}
        <Link to="/app/receipts">Receipts</Link>.
      </Text>
    </Stack>
  );
}

function decisionColor(decision: string) {
  switch (decision) {
    case "selected":
      return "teal";
    case "eligible":
      return "blue";
    default:
      return "orange";
  }
}

function healthColor(healthState: string) {
  switch (healthState) {
    case "healthy":
      return "teal";
    case "degraded":
      return "yellow";
    case "quarantined":
      return "red";
    case "draining":
      return "orange";
    default:
      return "gray";
  }
}
