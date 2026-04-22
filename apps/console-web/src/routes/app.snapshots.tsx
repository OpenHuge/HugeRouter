import { Alert, Badge, Button, Card, Group, Stack, Table, Text } from '@mantine/core'
import { useState } from 'react'
import { createFileRoute, useRouter } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { loadRouteData } from '../features/control-plane/loaders'
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState
} from '../features/control-plane/route-state'
import { getConsoleDataService } from '../features/control-plane/service'

export const Route = createFileRoute('/app/snapshots')({
  loader: () => loadRouteData(() => getConsoleDataService().listConfigSnapshots()),
  pendingComponent: () => <RouteLoadingState label="Loading snapshots" />,
  pendingMs: 0,
  component: ConfigSnapshotsPage
})

function ConfigSnapshotsPage() {
  const result = Route.useLoaderData()
  const router = useRouter()
  const [activatingSnapshotId, setActivatingSnapshotId] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Review config snapshots by tenant, and activate one snapshot as the live control-plane source."
          title="Snapshots"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === 'error' ? result?.message : undefined}
          description="Snapshot catalog could not be loaded from the control-plane service."
          title="Snapshots unavailable"
        />
      </Stack>
    )
  }

  const snapshots = result.data

  const onActivate = async (configSnapshotId: string) => {
    setActivatingSnapshotId(configSnapshotId)
    setActionError(null)

    try {
      await getConsoleDataService().activateConfigSnapshot(configSnapshotId)
      await router.invalidate()
    } catch (error) {
      setActionError(
        error instanceof Error ? error.message : 'Failed to activate config snapshot.'
      )
    } finally {
      setActivatingSnapshotId(null)
    }
  }

  return (
    <Stack>
      <PageHeader
        description="Review config snapshots by tenant, and activate one snapshot as the live control-plane source."
        title="Snapshots"
      />
      {actionError ? (
        <Alert color="red" radius="md" title="Action failed" variant="light">
          <Text c="dimmed" size="sm">
            {actionError}
          </Text>
        </Alert>
      ) : null}
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Config snapshots</Text>
          <Badge color="blue" variant="light">
            {snapshots.length} items
          </Badge>
        </Group>
        {snapshots.length === 0 ? (
          <EmptyCollectionState
            description="No snapshots have been created yet."
            title="No snapshots"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Snapshot ID</Table.Th>
                <Table.Th>Revision</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Route policy</Table.Th>
                <Table.Th>Providers</Table.Th>
                <Table.Th>Activated at</Table.Th>
                <Table.Th>Actions</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {snapshots.map((snapshot) => (
                <Table.Tr key={snapshot.configSnapshotId}>
                  <Table.Td>{snapshot.configSnapshotId}</Table.Td>
                  <Table.Td>{snapshot.revision}</Table.Td>
                  <Table.Td>
                    <Badge color={snapshot.status === 'active' ? 'teal' : 'gray'} variant="light">
                      {snapshot.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{snapshot.routePolicyId}</Table.Td>
                  <Table.Td>
                    {snapshot.providerResourceIds.join(', ') || 'No provider assignments'}
                  </Table.Td>
                  <Table.Td>{snapshot.activatedAt ?? 'not yet activated'}</Table.Td>
                  <Table.Td>
                    <Button
                      disabled={snapshot.status === 'active' || activatingSnapshotId === snapshot.configSnapshotId}
                      loading={activatingSnapshotId === snapshot.configSnapshotId}
                      onClick={() => void onActivate(snapshot.configSnapshotId)}
                      size="sm"
                      variant="light"
                    >
                      Activate
                    </Button>
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
