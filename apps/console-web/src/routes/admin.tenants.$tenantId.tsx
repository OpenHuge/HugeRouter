import {
  UiChip,
  UiSurface,
  UiInline,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
import { Link, createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/admin/tenants/$tenantId")({
  loader: ({ params }) =>
    loadRouteData(() =>
      getConsoleDataService().getTenantDetail(params.tenantId),
    ),
  pendingComponent: () => <RouteLoadingState label="Loading tenant detail" />,
  pendingMs: 0,
  component: TenantDetailPage,
});

function TenantDetailPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Inspect the selected tenant workspace, project coverage, provider access, and route policy state."
          title="Tenant detail"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="The selected tenant could not be loaded. Return to the tenant list and try again."
          title="Tenant unavailable"
        />
      </UiStack>
    );
  }

  const tenant = result.data;

  return (
    <UiStack>
      <PageHeader
        description="Inspect the selected tenant workspace, project coverage, provider access, and route policy state."
        title={tenant.displayName}
        actions={
          tenant.activeConfigSnapshotId ? (
            <UiChip color="teal" size="lg" variant="light">
              {tenant.activeConfigSnapshotId}
            </UiChip>
          ) : (
            <UiChip color="gray" size="lg" variant="light">
              No active snapshot
            </UiChip>
          )
        }
      />
      <UiText c="dimmed" component={Link} size="sm" to="/admin/tenants">
        Back to tenant list
      </UiText>
      <UiInline grow align="stretch">
        <SummaryCard label="Version" value={String(tenant.version)} />
        <SummaryCard
          label="Selected provider"
          value={tenant.selectedProvider}
        />
        <SummaryCard
          label="Simulated cost"
          value={
            tenant.estimatedCostUsd ? `$${tenant.estimatedCostUsd}` : "n/a"
          }
        />
      </UiInline>
      <UiInline grow align="stretch">
        <UiSurface padding="lg" radius="md" shadow="sm">
          <UiText fw={700} mb="md">
            Projects
          </UiText>
          <UiStack gap="xs">
            {tenant.projects.map((project) => (
              <UiText key={project.id}>{project.name}</UiText>
            ))}
          </UiStack>
        </UiSurface>
        <UiSurface padding="lg" radius="md" shadow="sm">
          <UiText fw={700} mb="md">
            Provider resources
          </UiText>
          <UiStack gap="xs">
            {tenant.providers.map((provider) => (
              <UiText key={provider.provider_resource_id}>
                {provider.name}
              </UiText>
            ))}
          </UiStack>
        </UiSurface>
      </UiInline>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Route policies</UiText>
          <UiChip color="blue" variant="light">
            {tenant.routePolicies.length} policies
          </UiChip>
        </UiInline>
        <UiDataTable striped withTableBorder>
          <UiDataTable.Thead>
            <UiDataTable.Tr>
              <UiDataTable.Th>Name</UiDataTable.Th>
              <UiDataTable.Th>Model alias</UiDataTable.Th>
              <UiDataTable.Th>Selected providers</UiDataTable.Th>
              <UiDataTable.Th>Preferred regions</UiDataTable.Th>
            </UiDataTable.Tr>
          </UiDataTable.Thead>
          <UiDataTable.Tbody>
            {tenant.routePolicies.map((policy) => (
              <UiDataTable.Tr key={policy.id}>
                <UiDataTable.Td>{policy.name}</UiDataTable.Td>
                <UiDataTable.Td>{policy.modelAlias}</UiDataTable.Td>
                <UiDataTable.Td>
                  {policy.selectedProviders.length > 0
                    ? policy.selectedProviders.join(", ")
                    : "Inactive snapshot"}
                </UiDataTable.Td>
                <UiDataTable.Td>
                  {policy.preferredRegions.join(", ") || "Any region"}
                </UiDataTable.Td>
              </UiDataTable.Tr>
            ))}
          </UiDataTable.Tbody>
        </UiDataTable>
      </UiSurface>
    </UiStack>
  );
}

function SummaryCard({ label, value }: { label: string; value: string }) {
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
