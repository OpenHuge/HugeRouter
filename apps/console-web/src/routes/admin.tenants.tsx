import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { Link, Outlet, createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { loadRouteData } from '../features/control-plane/loaders'
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState
} from '../features/control-plane/route-state'
import { getConsoleDataService } from '../features/control-plane/service'

export const Route = createFileRoute('/admin/tenants')({
  loader: () => loadRouteData(() => getConsoleDataService().listTenants()),
  pendingComponent: () => <RouteLoadingState label="Loading tenants" />,
  pendingMs: 0,
  component: AdminTenantsPage
})

function AdminTenantsPage() {
  const result = Route.useLoaderData()

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Review tenant inventory, configured projects, provider coverage, and active snapshot status."
          title="Tenants"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === 'error' ? result?.message : undefined}
          description="Tenant inventory could not be loaded. Retry once the control-plane surface is reachable."
          title="Tenants unavailable"
        />
      </Stack>
    )
  }

  const tenants = result.data

  return (
    <Stack>
      <PageHeader
        description="Review tenant inventory, configured projects, provider coverage, and active snapshot status."
        title="Tenants"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Known tenants</Text>
          <Badge color="blue" variant="light">
            {tenants.length} workspaces
          </Badge>
        </Group>
        {tenants.length === 0 ? (
          <EmptyCollectionState
            description="Create the first tenant in the control plane to begin routing traffic."
            title="No tenants"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Tenant</Table.Th>
                <Table.Th>Projects</Table.Th>
                <Table.Th>Providers</Table.Th>
                <Table.Th>Route policies</Table.Th>
                <Table.Th>Active snapshot</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {tenants.map((tenant) => (
                <Table.Tr key={tenant.id}>
                  <Table.Td>
                    <Stack gap={2}>
                      <Link params={{ tenantId: tenant.id }} to="/admin/tenants/$tenantId">
                        {tenant.displayName}
                      </Link>
                      <Text c="dimmed" size="sm">
                        {tenant.slug}
                      </Text>
                    </Stack>
                  </Table.Td>
                  <Table.Td>{tenant.projectCount}</Table.Td>
                  <Table.Td>{tenant.providerCount}</Table.Td>
                  <Table.Td>{tenant.routePolicyCount}</Table.Td>
                  <Table.Td>
                    {tenant.activeConfigSnapshotId ? (
                      <Badge color="teal" variant="light">
                        {tenant.activeConfigSnapshotId}
                      </Badge>
                    ) : (
                      <Badge color="gray" variant="light">
                        No active snapshot
                      </Badge>
                    )}
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
      <Outlet />
    </Stack>
  )
}
