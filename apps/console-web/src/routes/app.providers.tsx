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

export const Route = createFileRoute('/app/providers')({
  loader: () => loadRouteData(() => getConsoleDataService().listProviderResources()),
  pendingComponent: () => <RouteLoadingState label="Loading providers" />,
  pendingMs: 0,
  component: ProvidersPage
})

function ProvidersPage() {
  const result = Route.useLoaderData()

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Inspect provider resources, health, routing scope, and quarantine state."
          title="Providers"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === 'error' ? result?.message : undefined}
          description="Provider inventory could not be loaded from the control-plane service."
          title="Providers unavailable"
        />
      </Stack>
    )
  }

  const providers = result.data

  return (
    <Stack>
      <PageHeader
        description="Inspect provider resources, health, routing scope, and quarantine state."
        title="Providers"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Provider inventory</Text>
          <Badge color="blue" variant="light">
            {providers.length} resources
          </Badge>
        </Group>
        {providers.length === 0 ? (
          <EmptyCollectionState
            description="Register a provider resource to begin routing tenant traffic."
            title="No providers"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Name</Table.Th>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Region</Table.Th>
                <Table.Th>Scope</Table.Th>
                <Table.Th>Health</Table.Th>
                <Table.Th>Status</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {providers.map((provider) => (
                <Table.Tr key={provider.provider_resource_id}>
                  <Table.Td>{provider.name}</Table.Td>
                  <Table.Td>{provider.provider_id}</Table.Td>
                  <Table.Td>{provider.region}</Table.Td>
                  <Table.Td>{provider.deployment_scope}</Table.Td>
                  <Table.Td>
                    <Badge
                      color={
                        provider.health_state === 'healthy'
                          ? 'teal'
                          : provider.health_state === 'degraded'
                            ? 'yellow'
                            : 'red'
                      }
                      variant="light"
                    >
                      {provider.health_state}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Badge color={provider.status === 'active' ? 'blue' : 'gray'} variant="light">
                      {provider.status}
                    </Badge>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  )
}
