import {
  Anchor,
  Badge,
  Button,
  Card,
  Group,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { Link, createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/routes")({
  loader: () =>
    loadRouteData(() => getConsoleDataService().listRoutePolicies()),
  pendingComponent: () => <RouteLoadingState label="Loading routes" />,
  pendingMs: 0,
  component: RoutePoliciesPage,
});

function RoutePoliciesPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Review route policies, selected targets, and the latest operator-visible routing outcomes."
          title="Routes"
        />
        <RouteErrorState
          description="Route policy diagnostics could not be loaded from the control-plane service."
          title="Routes unavailable"
        />
      </Stack>
    );
  }

  const routePolicies = result.data;

  return (
    <Stack>
      <PageHeader
        description="Review route policies, selected targets, and the latest operator-visible routing outcomes."
        title="Routes"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Route diagnostics summary</Text>
          <Badge color="blue" variant="light">
            {routePolicies.length} policies
          </Badge>
        </Group>
        {routePolicies.length === 0 ? (
          <EmptyCollectionState
            description="Create a route policy to select providers for the current tenant."
            title="No route policies"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Route</Table.Th>
                <Table.Th>Protocol</Table.Th>
                <Table.Th>Required capability</Table.Th>
                <Table.Th>Snapshot targets</Table.Th>
                <Table.Th>Latest receipt</Table.Th>
                <Table.Th />
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {routePolicies.map((policy) => (
                <Table.Tr key={policy.id}>
                  <Table.Td>
                    <Stack gap={2}>
                      <Text fw={600}>{policy.name}</Text>
                      <Text c="dimmed" size="sm">
                        {policy.modelAlias}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    <Badge color="gray" variant="light">
                      {policy.protocolFamily}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{policy.requiredCapabilities.join(", ")}</Table.Td>
                  <Table.Td>
                    {policy.selectedProviders.length > 0
                      ? policy.selectedProviders.join(", ")
                      : "Inactive snapshot"}
                  </Table.Td>
                  <Table.Td>
                    <Stack gap={2}>
                      <Badge
                        color={
                          policy.lastReceiptOutcome === "admitted"
                            ? "teal"
                            : "orange"
                        }
                        variant="light"
                      >
                        {policy.lastReceiptOutcome ?? "no receipt"}
                      </Badge>
                      <Text c="dimmed" size="sm">
                        {policy.lastFailureReason ??
                          "No recent failure reason."}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    <Button
                      component="a"
                      href={`/app/route-diagnostics/${policy.id}`}
                      size="xs"
                      variant="light"
                    >
                      Inspect diagnostics
                    </Button>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Text fw={700} mb="xs">
          How to read this page
        </Text>
        <Text c="dimmed" size="sm">
          Use the diagnostics drill-down to see protocol support, capability
          gaps, health or quarantine blockers, and the most recent receipt that
          explains why HugeRouter selected or excluded each target.
        </Text>
        <Anchor component={Link} mt="md" size="sm" to="/app/receipts">
          Open recent route receipts
        </Anchor>
      </Card>
    </Stack>
  );
}
