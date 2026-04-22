import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { Link, createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { loadRouteData } from '../features/control-plane/loaders'
import { RouteErrorState, RouteLoadingState } from '../features/control-plane/route-state'
import { getConsoleDataService } from '../features/control-plane/service'

export const Route = createFileRoute('/admin/tenants/$tenantId')({
  loader: ({ params }) => loadRouteData(() => getConsoleDataService().getTenantDetail(params.tenantId)),
  pendingComponent: () => <RouteLoadingState label="Loading tenant detail" />,
  pendingMs: 0,
  component: TenantDetailPage
})

function TenantDetailPage() {
  const result = Route.useLoaderData()

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Inspect the selected tenant workspace, project coverage, provider access, and route policy state."
          title="Tenant detail"
        />
        <RouteErrorState
          description="The selected tenant could not be loaded. Return to the tenant list and try again."
          title="Tenant unavailable"
        />
      </Stack>
    )
  }

  const tenant = result.data

  return (
    <Stack>
      <PageHeader
        description="Inspect the selected tenant workspace, project coverage, provider access, and route policy state."
        title={tenant.displayName}
        actions={
          tenant.activeConfigSnapshotId ? (
            <Badge color="teal" size="lg" variant="light">
              {tenant.activeConfigSnapshotId}
            </Badge>
          ) : (
            <Badge color="gray" size="lg" variant="light">
              No active snapshot
            </Badge>
          )
        }
      />
      <Text c="dimmed" component={Link} size="sm" to="/admin/tenants">
        Back to tenant list
      </Text>
      <Group grow align="stretch">
        <SummaryCard label="Version" value={String(tenant.version)} />
        <SummaryCard label="Selected provider" value={tenant.selectedProvider} />
        <SummaryCard label="Simulated cost" value={tenant.estimatedCostUsd ? `$${tenant.estimatedCostUsd}` : 'n/a'} />
      </Group>
      <Group grow align="stretch">
        <Card padding="lg" radius="md" shadow="sm">
          <Text fw={700} mb="md">
            Projects
          </Text>
          <Stack gap="xs">
            {tenant.projects.map((project) => (
              <Text key={project.id}>{project.name}</Text>
            ))}
          </Stack>
        </Card>
        <Card padding="lg" radius="md" shadow="sm">
          <Text fw={700} mb="md">
            Provider resources
          </Text>
          <Stack gap="xs">
            {tenant.providers.map((provider) => (
              <Text key={provider.provider_resource_id}>{provider.name}</Text>
            ))}
          </Stack>
        </Card>
      </Group>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Route policies</Text>
          <Badge color="blue" variant="light">
            {tenant.routePolicies.length} policies
          </Badge>
        </Group>
        <Table striped withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Name</Table.Th>
              <Table.Th>Model alias</Table.Th>
              <Table.Th>Selected providers</Table.Th>
              <Table.Th>Preferred regions</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {tenant.routePolicies.map((policy) => (
              <Table.Tr key={policy.id}>
                <Table.Td>{policy.name}</Table.Td>
                <Table.Td>{policy.modelAlias}</Table.Td>
                <Table.Td>
                  {policy.selectedProviders.length > 0
                    ? policy.selectedProviders.join(', ')
                    : 'Inactive snapshot'}
                </Table.Td>
                <Table.Td>{policy.preferredRegions.join(', ') || 'Any region'}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Card>
    </Stack>
  )
}

function SummaryCard({ label, value }: { label: string; value: string }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Text c="dimmed" size="sm">
        {label}
      </Text>
      <Text fw={700} mt="xs" size="lg">
        {value}
      </Text>
    </Card>
  )
}
