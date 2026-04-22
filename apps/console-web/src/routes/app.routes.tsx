import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { formatPercent } from '../features/control-plane/format'
import { loadRouteData } from '../features/control-plane/loaders'
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState
} from '../features/control-plane/route-state'
import { getConsoleDataService } from '../features/control-plane/service'

export const Route = createFileRoute('/app/routes')({
  loader: () => loadRouteData(() => getConsoleDataService().listRoutePolicies()),
  pendingComponent: () => <RouteLoadingState label="Loading routes" />,
  pendingMs: 0,
  component: RoutePoliciesPage
})

function RoutePoliciesPage() {
  const result = Route.useLoaderData()

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Review the first route-management slice, including model aliases, selected targets, and delivery health."
          title="Routes"
        />
        <RouteErrorState
          description="Route policy data could not be loaded from the control-plane service."
          title="Routes unavailable"
        />
      </Stack>
    )
  }

  const routePolicies = result.data

  return (
    <Stack>
      <PageHeader
        description="Review the first route-management slice, including model aliases, selected targets, and delivery health."
        title="Routes"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Route policies</Text>
          <Badge color="blue" variant="light">
            {routePolicies.length} policies
          </Badge>
        </Group>
        {routePolicies.length === 0 ? (
          <EmptyCollectionState
            description="Create a route policy to select providers for the current tenant."
            title="No route policies"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Name</Table.Th>
                <Table.Th>Model alias</Table.Th>
                <Table.Th>Selected provider</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Success rate</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {routePolicies.map((policy) => (
                <Table.Tr key={policy.id}>
                  <Table.Td>{policy.name}</Table.Td>
                  <Table.Td>{policy.modelAlias}</Table.Td>
                  <Table.Td>{policy.selectedProvider}</Table.Td>
                  <Table.Td>
                    <Badge color={policy.status === 'active' ? 'teal' : 'yellow'} variant="light">
                      {policy.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{formatPercent(policy.successRate)}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  )
}
