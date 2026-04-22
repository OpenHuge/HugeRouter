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

export const Route = createFileRoute('/app/api-keys')({
  loader: () => loadRouteData(() => getConsoleDataService().listApiKeys()),
  pendingComponent: () => <RouteLoadingState label="Loading API keys" />,
  pendingMs: 0,
  component: ApiKeysPage
})

function ApiKeysPage() {
  const result = Route.useLoaderData()
  const router = useRouter()
  const [revokingApiKeyId, setRevokingApiKeyId] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  if (!result || result.state === 'error') {
    return (
      <Stack>
        <PageHeader
          description="Manage API key lifecycle and observe key activity at tenant level."
          title="API Keys"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === 'error' ? result?.message : undefined}
          description="API keys could not be loaded from the control-plane service."
          title="API keys unavailable"
        />
      </Stack>
    )
  }

  const keys = result.data

  const onRevoke = async (apiKeyId: string, version: number) => {
    setRevokingApiKeyId(apiKeyId)
    setActionError(null)

    try {
      await getConsoleDataService().revokeApiKey(apiKeyId, version)
      await router.invalidate()
    } catch (error) {
      setActionError(error instanceof Error ? error.message : 'Failed to revoke API key.')
    } finally {
      setRevokingApiKeyId(null)
    }
  }

  return (
    <Stack>
      <PageHeader
        description="Manage API key lifecycle and observe key activity at tenant level."
        title="API Keys"
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
          <Text fw={700}>API keys</Text>
          <Badge color="blue" variant="light">
            {keys.length} keys
          </Badge>
        </Group>
        {keys.length === 0 ? (
          <EmptyCollectionState
            description="No API keys are available for this tenant."
            title="No API keys"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Display name</Table.Th>
                <Table.Th>Key prefix</Table.Th>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Created</Table.Th>
                <Table.Th>Updated</Table.Th>
                <Table.Th>Actions</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {keys.map((key) => (
                <Table.Tr key={key.apiKeyId}>
                  <Table.Td>{key.displayName}</Table.Td>
                  <Table.Td>{key.keyPrefix}</Table.Td>
                  <Table.Td>{key.providerResourceId}</Table.Td>
                  <Table.Td>
                    <Badge color={key.isActive ? 'teal' : 'gray'} variant="light">
                      {key.isActive ? 'Active' : 'Revoked'}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{key.createdAt}</Table.Td>
                  <Table.Td>{key.updatedAt}</Table.Td>
                  <Table.Td>
                    <Button
                      disabled={!key.canRevoke || !key.isActive || revokingApiKeyId === key.apiKeyId}
                      loading={revokingApiKeyId === key.apiKeyId}
                      onClick={() => void onRevoke(key.apiKeyId, key.version)}
                      size="sm"
                      variant="light"
                    >
                      Revoke
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
