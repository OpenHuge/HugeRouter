import {
  UiChip,
  UiButton,
  UiButtonGroup,
  UiSurface,
  UiInline,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
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
      <UiStack>
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
      </UiStack>
    );
  }

  const data = result.data;

  return (
    <UiStack>
      <PageHeader
        description="Review persisted token totals, provider and model breakdowns, and billable amounts for the selected window."
        title="Usage"
      />
      <UiInline justify="space-between" wrap="wrap">
        <UiButtonGroup>
          {(["7d", "30d", "90d"] as const).map((range) => (
            <UiButton
              color={search.range === range ? "blue" : "gray"}
              component="a"
              href={`/app/usage?range=${range}&groupBy=${search.groupBy}${search.projectId ? `&projectId=${search.projectId}` : ""}`}
              key={range}
              variant={search.range === range ? "filled" : "light"}
            >
              {range}
            </UiButton>
          ))}
        </UiButtonGroup>
        <UiButtonGroup>
          {(["provider", "model", "day"] as const).map((groupBy) => (
            <UiButton
              color={search.groupBy === groupBy ? "blue" : "gray"}
              component="a"
              href={`/app/usage?range=${search.range}&groupBy=${groupBy}${search.projectId ? `&projectId=${search.projectId}` : ""}`}
              key={groupBy}
              variant={search.groupBy === groupBy ? "filled" : "light"}
            >
              {groupBy}
            </UiButton>
          ))}
        </UiButtonGroup>
        <UiButtonGroup>
          <UiButton
            color={!data.activeProjectId ? "blue" : "gray"}
            component="a"
            href={`/app/usage?range=${search.range}&groupBy=${search.groupBy}`}
            variant={!data.activeProjectId ? "filled" : "light"}
          >
            all projects
          </UiButton>
          {data.availableProjects.map((project) => (
            <UiButton
              color={data.activeProjectId === project.id ? "blue" : "gray"}
              component="a"
              href={`/app/usage?range=${search.range}&groupBy=${search.groupBy}&projectId=${project.id}`}
              key={project.id}
              variant={data.activeProjectId === project.id ? "filled" : "light"}
            >
              {project.name}
            </UiButton>
          ))}
        </UiButtonGroup>
      </UiInline>
      <UiInline grow>
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
        <MetricCard
          label="Billable total"
          value={`$${data.billablePriceUsd}`}
        />
      </UiInline>
      <UiInline grow>
        <MetricCard label="Input tokens" value={String(data.inputTokens)} />
        <MetricCard label="Output tokens" value={String(data.outputTokens)} />
        <MetricCard
          label="Cached input"
          value={String(data.cachedInputTokens)}
        />
      </UiInline>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Usage breakdown</UiText>
          <UiChip color="blue" variant="light">
            {data.breakdown.length} rows
          </UiChip>
        </UiInline>
        <UiDataTable striped withTableBorder>
          <UiDataTable.Thead>
            <UiDataTable.Tr>
              <UiDataTable.Th>Bucket</UiDataTable.Th>
              <UiDataTable.Th>Provider</UiDataTable.Th>
              <UiDataTable.Th>Model</UiDataTable.Th>
              <UiDataTable.Th>Input</UiDataTable.Th>
              <UiDataTable.Th>Output</UiDataTable.Th>
              <UiDataTable.Th>Cached</UiDataTable.Th>
              <UiDataTable.Th>Provider cost</UiDataTable.Th>
              <UiDataTable.Th>Billable</UiDataTable.Th>
            </UiDataTable.Tr>
          </UiDataTable.Thead>
          <UiDataTable.Tbody>
            {data.breakdown.map((row) => (
              <UiDataTable.Tr
                key={`${row.bucket}-${row.providerId ?? "none"}-${row.modelAlias ?? "none"}`}
              >
                <UiDataTable.Td>{row.bucket}</UiDataTable.Td>
                <UiDataTable.Td>{row.providerId ?? "—"}</UiDataTable.Td>
                <UiDataTable.Td>{row.modelAlias ?? "—"}</UiDataTable.Td>
                <UiDataTable.Td>{row.inputTokens}</UiDataTable.Td>
                <UiDataTable.Td>{row.outputTokens}</UiDataTable.Td>
                <UiDataTable.Td>{row.cachedInputTokens}</UiDataTable.Td>
                <UiDataTable.Td>${row.providerCostUsd}</UiDataTable.Td>
                <UiDataTable.Td>${row.billablePriceUsd}</UiDataTable.Td>
              </UiDataTable.Tr>
            ))}
          </UiDataTable.Tbody>
        </UiDataTable>
        {data.nextCursor ? (
          <UiInline justify="flex-end" mt="md">
            <UiButton
              component="a"
              href={`/app/usage?range=${search.range}&groupBy=${search.groupBy}${search.projectId ? `&projectId=${search.projectId}` : ""}&cursor=${data.nextCursor}`}
              variant="light"
            >
              Load more
            </UiButton>
          </UiInline>
        ) : null}
      </UiSurface>
    </UiStack>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiText c="dimmed" size="sm">
        {label}
      </UiText>
      <UiText fw={700} mt="xs" size="lg">
        {value}
      </UiText>
    </UiSurface>
  );
}
