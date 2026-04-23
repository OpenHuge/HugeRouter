import {
  Badge,
  Card,
  Group,
  SimpleGrid,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import type { ProviderResourceView } from "../features/control-plane/types";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/providers")({
  loader: () =>
    loadRouteData(() => getConsoleDataService().listProviderResources()),
  pendingComponent: () => <RouteLoadingState label="Loading providers" />,
  pendingMs: 0,
  component: ProvidersPage,
});

function ProvidersPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Inspect provider capabilities, provenance, transit posture, and recent routing signals."
          title="Providers"
        />
        <RouteErrorState
          description="Provider diagnostics could not be loaded from the control-plane service."
          title="Providers unavailable"
        />
      </Stack>
    );
  }

  const providers = result.data;
  const transitCount = providers.filter(
    (provider) => provider.isTransitGateway,
  ).length;
  const realtimeReadyCount = providers.filter(
    (provider) => provider.capabilities.realtime,
  ).length;
  const quarantinedCount = providers.filter(
    (provider) => provider.healthState === "quarantined",
  ).length;

  return (
    <Stack>
      <PageHeader
        description="Inspect provider capabilities, provenance, transit posture, and recent routing signals."
        title="Providers"
      />
      <SimpleGrid cols={{ base: 1, md: 3 }}>
        <MetricCard label="Transit gateways" value={String(transitCount)} />
        <MetricCard
          label="Realtime-ready targets"
          value={String(realtimeReadyCount)}
        />
        <MetricCard
          label="Quarantined targets"
          value={String(quarantinedCount)}
        />
      </SimpleGrid>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Provider diagnostics</Text>
          <Badge color="blue" variant="light">
            {providers.length} resources
          </Badge>
        </Group>
        {providers.length === 0 ? (
          <EmptyCollectionState
            description="Register a provider resource to begin routing tenant traffic."
            title="No providers"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Target</Table.Th>
                <Table.Th>Protocols</Table.Th>
                <Table.Th>Capabilities</Table.Th>
                <Table.Th>Provenance</Table.Th>
                <Table.Th>Health</Table.Th>
                <Table.Th>Latest route signal</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {providers.map((provider) => (
                <Table.Tr key={provider.id}>
                  <Table.Td>
                    <Stack gap={2}>
                      <Group gap="xs">
                        <Text fw={600}>{provider.name}</Text>
                        {provider.isTransitGateway ? (
                          <Badge color="indigo" variant="light">
                            transit
                          </Badge>
                        ) : null}
                      </Group>
                      <Text c="dimmed" size="sm">
                        {provider.providerId} · {provider.region} ·{" "}
                        {provider.scope}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    <CapabilityBadges values={provider.protocolFamilies} />
                  </Table.Td>
                  <Table.Td>
                    <CapabilityBadges
                      values={
                        [
                          provider.capabilities.streaming ? "streaming" : null,
                          provider.capabilities.realtime ? "realtime" : null,
                          provider.capabilities.toolCalling
                            ? "tool-calling"
                            : null,
                          provider.capabilities.responseModelMetadata
                            ? "response-model-metadata"
                            : null,
                        ].filter(Boolean) as string[]
                      }
                    />
                  </Table.Td>
                  <Table.Td>
                    <Stack gap={2}>
                      <Badge color="gray" variant="light">
                        {provider.provenanceClass}
                      </Badge>
                      <Text c="dimmed" size="sm">
                        {provider.status}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    <Stack gap={4}>
                      <Badge
                        color={healthColor(provider.healthState)}
                        variant="light"
                      >
                        {provider.healthState}
                      </Badge>
                      <Text c="dimmed" size="sm">
                        {provider.quarantineReason ??
                          provider.healthMessage ??
                          "No health notes."}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    {provider.latestRoutingSignal ? (
                      <Stack gap={2}>
                        <Badge
                          color={
                            provider.latestRoutingSignal.outcome === "selected"
                              ? "teal"
                              : "orange"
                          }
                          variant="light"
                        >
                          {provider.latestRoutingSignal.outcome}
                        </Badge>
                        <Text size="sm">
                          {provider.latestRoutingSignal.routeName}
                        </Text>
                        <Text c="dimmed" size="sm">
                          {provider.latestRoutingSignal.reason}
                        </Text>
                      </Stack>
                    ) : (
                      <Text c="dimmed" size="sm">
                        No recent route receipt.
                      </Text>
                    )}
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

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Text c="dimmed" size="sm">
        {label}
      </Text>
      <Text fw={700} mt="xs" size="lg">
        {value}
      </Text>
    </Card>
  );
}

function CapabilityBadges({ values }: { values: string[] }) {
  if (values.length === 0) {
    return (
      <Text c="dimmed" size="sm">
        None
      </Text>
    );
  }

  return (
    <Group gap={4}>
      {values.map((value) => (
        <Badge color="blue" key={value} variant="light">
          {value}
        </Badge>
      ))}
    </Group>
  );
}

function healthColor(healthState: ProviderResourceView["healthState"]) {
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
