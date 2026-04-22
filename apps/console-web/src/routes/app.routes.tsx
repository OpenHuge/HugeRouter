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
          description="Review active route policies, required capabilities, and selected targets from the current config snapshot."
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
        description="Review active route policies, required capabilities, and selected targets from the current config snapshot."
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
                <Table.Th>Selected providers</Table.Th>
                <Table.Th>Preferred regions</Table.Th>
                <Table.Th>Capabilities</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {routePolicies.map((policy) => (
                <Table.Tr key={policy.id}>
                  <Table.Td>{policy.name}</Table.Td>
                  <Table.Td>{policy.modelAlias}</Table.Td>
                  <Table.Td>
                    {policy.selectedProviders.length > 0
                      ? policy.selectedProviders.join(', ')
                      : 'Inactive snapshot'}
                  </Table.Td>
                  <Table.Td>{policy.preferredRegions.join(', ') || 'Any region'}</Table.Td>
                  <Table.Td>{policy.requiredCapabilities.join(', ')}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  )
}
