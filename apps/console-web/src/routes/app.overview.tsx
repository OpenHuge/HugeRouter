import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { loadRouteData } from '../features/control-plane/loaders'
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState
} from '../features/control-plane/route-state'
import { getConsoleDataService } from '../features/control-plane/service'

export const Route = createFileRoute('/app/overview')({
  loader: () => loadRouteData(() => getConsoleDataService().getOverview()),
  pendingComponent: () => <RouteLoadingState label="Loading overview" />,
  pendingMs: 0,
  component: OverviewPage
})

function OverviewPage() {
  const result = Route.useLoaderData()

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Monitor the current tenant workspace, active routing state, and control-plane configuration freshness."
          title="Overview"
        />
        <RouteErrorState
          description="The console could not load the current workspace summary. Try again after the control-plane API is available."
          title="Overview unavailable"
        />
      </Stack>
    )
  }

  const { data } = result

  return (
    <Stack>
      <PageHeader
        description="Monitor the current tenant workspace, active routing state, and control-plane configuration freshness."
        title="Overview"
      />
      <Group grow align="stretch">
        <MetricCard label="Active providers" value={String(data.activeProviders)} />
        <MetricCard label="Active routes" value={String(data.activeRoutes)} />
        <MetricCard label="Active snapshot" value={data.activeSnapshotId} />
      </Group>
      <Group grow align="stretch">
        <Card padding="lg" radius="md" shadow="sm">
          <Group justify="space-between" mb="md">
            <Text fw={700}>Workspace summary</Text>
            <Badge color="teal" variant="light">
              {data.workspace}
            </Badge>
          </Group>
          <Stack gap="xs">
            <Text>{data.tenantLabel} is using the control-plane-backed MVP slice.</Text>
            <Text c="dimmed" size="sm">
              Selected provider: {data.selectedProvider}
            </Text>
            <Text c="dimmed" size="sm">
              Simulated request cost: ${data.estimatedCostUsd}
            </Text>
          </Stack>
        </Card>
        <Card padding="lg" radius="md" shadow="sm">
          <Group justify="space-between" mb="md">
            <Text fw={700}>Projects</Text>
            <Badge color="blue" variant="light">
              {data.projects.length} tracked
            </Badge>
          </Group>
          {data.projects.length === 0 ? (
            <EmptyCollectionState
              description="This workspace does not have any routed projects yet."
              title="No projects"
            />
          ) : (
            <Table striped withTableBorder>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Project ID</Table.Th>
                  <Table.Th>Name</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {data.projects.map((project) => (
                  <Table.Tr key={project.id}>
                    <Table.Td>{project.id}</Table.Td>
                    <Table.Td>{project.name}</Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          )}
        </Card>
      </Group>
    </Stack>
  )
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
  )
}
