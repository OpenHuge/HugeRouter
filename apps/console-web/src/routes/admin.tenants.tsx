import {
  UiChip,
  UiSurface,
  UiInline,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
import { Link, Outlet, createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/admin/tenants")({
  loader: () => loadRouteData(() => getConsoleDataService().listTenants()),
  pendingComponent: () => <RouteLoadingState label="Loading tenants" />,
  pendingMs: 0,
  component: AdminTenantsPage,
});

function AdminTenantsPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Review tenant inventory, configured projects, provider coverage, and active snapshot status."
          title="Tenants"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="Tenant inventory could not be loaded. Retry once the control-plane surface is reachable."
          title="Tenants unavailable"
        />
      </UiStack>
    );
  }

  const tenants = result.data;

  return (
    <UiStack>
      <PageHeader
        description="Review tenant inventory, configured projects, provider coverage, and active snapshot status."
        title="Tenants"
      />
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Known tenants</UiText>
          <UiChip color="blue" variant="light">
            {tenants.length} workspaces
          </UiChip>
        </UiInline>
        {tenants.length === 0 ? (
          <EmptyCollectionState
            description="Create the first tenant in the control plane to begin routing traffic."
            title="No tenants"
          />
        ) : (
          <UiDataTable striped withTableBorder>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Tenant</UiDataTable.Th>
                <UiDataTable.Th>Projects</UiDataTable.Th>
                <UiDataTable.Th>Providers</UiDataTable.Th>
                <UiDataTable.Th>Route policies</UiDataTable.Th>
                <UiDataTable.Th>Active snapshot</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {tenants.map((tenant) => (
                <UiDataTable.Tr key={tenant.id}>
                  <UiDataTable.Td>
                    <UiStack gap={2}>
                      <Link
                        params={{ tenantId: tenant.id }}
                        to="/admin/tenants/$tenantId"
                      >
                        {tenant.displayName}
                      </Link>
                      <UiText c="dimmed" size="sm">
                        {tenant.slug}
                      </UiText>
                    </UiStack>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{tenant.projectCount}</UiDataTable.Td>
                  <UiDataTable.Td>{tenant.providerCount}</UiDataTable.Td>
                  <UiDataTable.Td>{tenant.routePolicyCount}</UiDataTable.Td>
                  <UiDataTable.Td>
                    {tenant.activeConfigSnapshotId ? (
                      <UiChip color="teal" variant="light">
                        {tenant.activeConfigSnapshotId}
                      </UiChip>
                    ) : (
                      <UiChip color="gray" variant="light">
                        No active snapshot
                      </UiChip>
                    )}
                  </UiDataTable.Td>
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiSurface>
      <Outlet />
    </UiStack>
  );
}
