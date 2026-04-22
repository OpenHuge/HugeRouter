import { Badge, Card, Group, List, SimpleGrid, Stack, Table, Text } from '@mantine/core'
import { Link, createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { formatCurrency, formatPercent } from '../features/control-plane/format'
import { loadRouteData } from '../features/control-plane/loaders'
import {
  RouteErrorState,
  RouteLoadingState
} from '../features/control-plane/route-state'
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
          description="Inspect the selected tenant workspace, project coverage, provider access, and route policy health."
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
        description="Inspect the selected tenant workspace, project coverage, provider access, and route policy health."
        title={tenant.displayName}
        actions={
          <Badge color={tenant.status === 'healthy' ? 'teal' : 'yellow'} size="lg" variant="light">
            {tenant.status === 'healthy' ? 'Healthy' : 'Needs attention'}
          </Badge>
        }
      />
      <Text c="dimmed" component={Link} size="sm" to="/admin/tenants">
        Back to tenant list
      </Text>
      <SimpleGrid cols={{ base: 1, md: 3 }}>
        <SummaryCard label="Plan" value={tenant.plan} />
        <SummaryCard label="Monthly spend" value={formatCurrency(tenant.monthlySpendUsd)} />
        <SummaryCard label="Primary region" value={tenant.primaryRegion} />
      </SimpleGrid>
      <SimpleGrid cols={{ base: 1, lg: 2 }}>
        <Card padding="lg" radius="md" shadow="sm">
          <Text fw={700} mb="md">
            Workspace notes
          </Text>
          <List spacing="sm">
            <List.Item>{tenant.notes}</List.Item>
            <List.Item>{tenant.projects.length} projects are currently configured in this workspace.</List.Item>
            <List.Item>{tenant.providers.length} provider resources are visible to this tenant.</List.Item>
          </List>
        </Card>
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
      </SimpleGrid>
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
              <Table.Th>Selected provider</Table.Th>
              <Table.Th>Success rate</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {tenant.routePolicies.map((policy) => (
              <Table.Tr key={policy.id}>
                <Table.Td>{policy.name}</Table.Td>
                <Table.Td>{policy.modelAlias}</Table.Td>
                <Table.Td>{policy.selectedProvider}</Table.Td>
                <Table.Td>{formatPercent(policy.successRate)}</Table.Td>
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
      <Text fw={700} mt="xs" size="lg" tt="capitalize">
        {value}
      </Text>
    </Card>
  )
}
