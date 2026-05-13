import {
  UiChip,
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
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/overview")({
  loader: () => loadRouteData(() => getConsoleDataService().getOverview()),
  pendingComponent: () => <RouteLoadingState label="Loading overview" />,
  pendingMs: 0,
  component: OverviewPage,
});

function OverviewPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Monitor the current tenant workspace, active routing state, and control-plane configuration freshness."
          title="Overview"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="The console could not load the current workspace summary. Try again after the control-plane API is available."
          title="Overview unavailable"
        />
      </UiStack>
    );
  }

  const { data } = result;

  return (
    <UiStack>
      <PageHeader
        description="Monitor the current tenant workspace, active routing state, and control-plane configuration freshness."
        title="Overview"
      />
      <UiInline grow align="stretch">
        <MetricCard
          label="Active providers"
          value={String(data.activeProviders)}
        />
        <MetricCard label="Active routes" value={String(data.activeRoutes)} />
        <MetricCard label="Active snapshot" value={data.activeSnapshotId} />
      </UiInline>
      <UiInline grow align="stretch">
        <UiSurface padding="lg" radius="md" shadow="sm">
          <UiInline justify="space-between" mb="md">
            <UiText fw={700}>Workspace summary</UiText>
            <UiChip color="teal" variant="light">
              {data.workspace}
            </UiChip>
          </UiInline>
          <UiStack gap="xs">
            <UiText>
              {data.tenantLabel} is using the control-plane-backed MVP slice.
            </UiText>
            <UiText c="dimmed" size="sm">
              Selected provider: {data.selectedProvider}
            </UiText>
            <UiText c="dimmed" size="sm">
              Simulated request cost: ${data.estimatedCostUsd}
            </UiText>
          </UiStack>
        </UiSurface>
        <UiSurface padding="lg" radius="md" shadow="sm">
          <UiInline justify="space-between" mb="md">
            <UiText fw={700}>Projects</UiText>
            <UiChip color="blue" variant="light">
              {data.projects.length} tracked
            </UiChip>
          </UiInline>
          {data.projects.length === 0 ? (
            <EmptyCollectionState
              description="This workspace does not have any routed projects yet."
              title="No projects"
            />
          ) : (
            <UiDataTable striped withTableBorder>
              <UiDataTable.Thead>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Project ID</UiDataTable.Th>
                  <UiDataTable.Th>Name</UiDataTable.Th>
                </UiDataTable.Tr>
              </UiDataTable.Thead>
              <UiDataTable.Tbody>
                {data.projects.map((project) => (
                  <UiDataTable.Tr key={project.id}>
                    <UiDataTable.Td>{project.id}</UiDataTable.Td>
                    <UiDataTable.Td>{project.name}</UiDataTable.Td>
                  </UiDataTable.Tr>
                ))}
              </UiDataTable.Tbody>
            </UiDataTable>
          )}
        </UiSurface>
      </UiInline>
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
