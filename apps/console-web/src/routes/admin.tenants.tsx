import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { Link, Outlet, createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { formatCurrency } from '../features/control-plane/format'
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
          description="Review tenant tenancy, routed projects, and the first operational signals for each workspace."
          title="Tenants"
        />
        <RouteErrorState
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
        description="Review tenant tenancy, routed projects, and the first operational signals for each workspace."
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
                <Table.Th>Plan</Table.Th>
                <Table.Th>Projects</Table.Th>
                <Table.Th>Active routes</Table.Th>
                <Table.Th>Monthly spend</Table.Th>
                <Table.Th>Status</Table.Th>
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
                  <Table.Td>{tenant.plan}</Table.Td>
                  <Table.Td>{tenant.projectCount}</Table.Td>
                  <Table.Td>{tenant.activeRoutePolicies}</Table.Td>
                  <Table.Td>{formatCurrency(tenant.monthlySpendUsd)}</Table.Td>
                  <Table.Td>
                    <Badge color={tenant.status === 'healthy' ? 'teal' : 'yellow'} variant="light">
                      {tenant.status === 'healthy' ? 'Healthy' : 'Needs attention'}
                    </Badge>
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
