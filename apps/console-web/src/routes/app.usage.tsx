import {
  Badge,
  Button,
  ButtonGroup,
  Card,
  Group,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

type UsageSearch = {
  cursor?: string;
  groupBy: "provider" | "model" | "day";
  projectId?: string;
  range: "7d" | "30d" | "90d";
};

function parseUsageSearch(rawSearch: Record<string, unknown>): UsageSearch {
  const range =
    rawSearch.range === "7d" || rawSearch.range === "90d"
      ? rawSearch.range
      : "30d";
  const groupBy =
    rawSearch.groupBy === "model" || rawSearch.groupBy === "day"
      ? rawSearch.groupBy
      : "provider";
  const projectId =
    typeof rawSearch.projectId === "string" && rawSearch.projectId.length > 0
      ? rawSearch.projectId
      : undefined;
  const cursor =
    typeof rawSearch.cursor === "string" && rawSearch.cursor.length > 0
      ? rawSearch.cursor
      : undefined;

  return { cursor, groupBy, projectId, range };
}

export const Route = createFileRoute("/app/usage")({
  validateSearch: parseUsageSearch,
  loaderDeps: ({ search }) => search,
  loader: ({ deps }) =>
    loadRouteData(() =>
      getConsoleDataService().getUsageDashboard(
        deps.range,
        deps.groupBy,
        deps.projectId,
        deps.cursor,
      ),
    ),
  pendingComponent: () => <RouteLoadingState label="Loading usage" />,
  pendingMs: 0,
  component: UsagePage,
});

function UsagePage() {
  const result = Route.useLoaderData();
  const search = Route.useSearch();

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Review persisted token totals, provider and model breakdowns, and billable amounts for the selected window."
          title="Usage"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result.message : undefined}
          description="Persisted usage totals could not be loaded from the control-plane service."
          title="Usage unavailable"
        />
      </Stack>
    );
  }

  const data = result.data;

  return (
    <Stack>
      <PageHeader
        description="Review persisted token totals, provider and model breakdowns, and billable amounts for the selected window."
        title="Usage"
      />
      <Group justify="space-between" wrap="wrap">
        <ButtonGroup>
          {(["7d", "30d", "90d"] as const).map((range) => (
            <Button
              color={search.range === range ? "blue" : "gray"}
              component="a"
              href={`/app/usage?range=${range}&groupBy=${search.groupBy}${search.projectId ? `&projectId=${search.projectId}` : ""}`}
              key={range}
              variant={search.range === range ? "filled" : "light"}
            >
              {range}
            </Button>
          ))}
        </ButtonGroup>
        <ButtonGroup>
          {(["provider", "model", "day"] as const).map((groupBy) => (
            <Button
              color={search.groupBy === groupBy ? "blue" : "gray"}
              component="a"
              href={`/app/usage?range=${search.range}&groupBy=${groupBy}${search.projectId ? `&projectId=${search.projectId}` : ""}`}
              key={groupBy}
              variant={search.groupBy === groupBy ? "filled" : "light"}
            >
              {groupBy}
            </Button>
          ))}
        </ButtonGroup>
        <ButtonGroup>
          <Button
            color={!data.activeProjectId ? "blue" : "gray"}
            component="a"
            href={`/app/usage?range=${search.range}&groupBy=${search.groupBy}`}
            variant={!data.activeProjectId ? "filled" : "light"}
          >
            all projects
          </Button>
          {data.availableProjects.map((project) => (
            <Button
              color={data.activeProjectId === project.id ? "blue" : "gray"}
              component="a"
              href={`/app/usage?range=${search.range}&groupBy=${search.groupBy}&projectId=${project.id}`}
              key={project.id}
              variant={data.activeProjectId === project.id ? "filled" : "light"}
            >
              {project.name}
            </Button>
          ))}
        </ButtonGroup>
      </Group>
      <Group grow>
        <MetricCard label="Range" value={data.rangeLabel} />
        <MetricCard
          label="Project scope"
          value={
            data.availableProjects.find(
              (project) => project.id === data.activeProjectId,
            )?.name ?? "All projects"
          }
        />
        <MetricCard label="Events" value={String(data.eventCount)} />
        <MetricCard label="Provider cost" value={`$${data.providerCostUsd}`} />
        <MetricCard label="Billable total" value={`$${data.billablePriceUsd}`} />
      </Group>
      <Group grow>
        <MetricCard label="Input tokens" value={String(data.inputTokens)} />
        <MetricCard label="Output tokens" value={String(data.outputTokens)} />
        <MetricCard
          label="Cached input"
          value={String(data.cachedInputTokens)}
        />
      </Group>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Usage breakdown</Text>
          <Badge color="blue" variant="light">
            {data.breakdown.length} rows
          </Badge>
        </Group>
        <Table striped withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Bucket</Table.Th>
              <Table.Th>Provider</Table.Th>
              <Table.Th>Model</Table.Th>
              <Table.Th>Input</Table.Th>
              <Table.Th>Output</Table.Th>
              <Table.Th>Cached</Table.Th>
              <Table.Th>Provider cost</Table.Th>
              <Table.Th>Billable</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {data.breakdown.map((row) => (
              <Table.Tr
                key={`${row.bucket}-${row.providerId ?? "none"}-${row.modelAlias ?? "none"}`}
              >
                <Table.Td>{row.bucket}</Table.Td>
                <Table.Td>{row.providerId ?? "—"}</Table.Td>
                <Table.Td>{row.modelAlias ?? "—"}</Table.Td>
                <Table.Td>{row.inputTokens}</Table.Td>
                <Table.Td>{row.outputTokens}</Table.Td>
                <Table.Td>{row.cachedInputTokens}</Table.Td>
                <Table.Td>${row.providerCostUsd}</Table.Td>
                <Table.Td>${row.billablePriceUsd}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
        {data.nextCursor ? (
          <Group justify="flex-end" mt="md">
            <Button
              component="a"
              href={`/app/usage?range=${search.range}&groupBy=${search.groupBy}${search.projectId ? `&projectId=${search.projectId}` : ""}&cursor=${data.nextCursor}`}
              variant="light"
            >
              Load more
            </Button>
          </Group>
        ) : null}
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
