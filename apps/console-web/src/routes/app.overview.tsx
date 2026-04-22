import {
  Badge,
  Card,
  Group,
  List,
  SimpleGrid,
  Stack,
  Table,
  Text
} from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { formatCurrency } from '../features/control-plane/format'
import { loadRouteData } from '../features/control-plane/loaders'
import { EmptyCollectionState, RouteErrorState, RouteLoadingState } from '../features/control-plane/route-state'
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
          description="Monitor the current tenant workspace, active routes, and recent project coverage."
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
        description="Monitor the current tenant workspace, active routes, and recent project coverage."
        title="Overview"
      />
      <SimpleGrid cols={{ base: 1, md: 3 }}>
        <MetricCard label="Active providers" value={String(data.activeProviders)} />
        <MetricCard label="Active routes" value={String(data.activeRoutes)} />
        <MetricCard label="Monthly spend" value={formatCurrency(data.monthlySpendUsd)} />
      </SimpleGrid>
      <SimpleGrid cols={{ base: 1, lg: 2 }}>
        <Card padding="lg" radius="md" shadow="sm">
          <Group justify="space-between" mb="md">
            <Text fw={700}>Workspace summary</Text>
            <Badge color="teal" variant="light">
              {data.workspace}
            </Badge>
          </Group>
          <List spacing="sm">
            <List.Item>{data.tenantLabel} is using the current seeded control-plane slice.</List.Item>
            <List.Item>Route policy health and provider inventory are now loaded through route loaders.</List.Item>
            <List.Item>Projects are resolved through the published typed client surface where it exists today.</List.Item>
          </List>
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
      </SimpleGrid>
    </Stack>
  )
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Text c="dimmed" size="sm">
        {label}
      </Text>
      <Text fw={700} mt="xs" size="xl">
        {value}
      </Text>
    </Card>
  )
}
