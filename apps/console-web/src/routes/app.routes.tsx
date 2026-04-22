import { Badge, Card, Group, List, Stack, Table, Text } from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { PageHeader } from '@huge-router/ui-kit'
import { loadRouteData } from '../features/control-plane/loaders'
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState
} from '../features/control-plane/route-state'
import type {
  RouteReceiptDiagnosticView,
  RoutePolicyView
} from '../features/control-plane/types'
import { getConsoleDataService } from '../features/control-plane/service'

const protocolLabelByFamily: Record<string, string> = {
  mcp_streamable_http: 'MCP Streamable HTTP',
  openai_chat: 'OpenAI Chat',
  openai_responses: 'OpenAI Responses',
  realtime_webrtc: 'Realtime WebRTC'
}

const protocolColorByFamily: Record<string, string> = {
  mcp_streamable_http: 'indigo',
  openai_chat: 'blue',
  openai_responses: 'violet',
  realtime_webrtc: 'teal'
}

const protocolOrder = [
  'openai_chat',
  'openai_responses',
  'mcp_streamable_http',
  'realtime_webrtc'
]

type RoutePoliciesPageData = {
  routePolicies: RoutePolicyView[]
  routeReceipts: RouteReceiptDiagnosticView[]
}

function protocolDisplay(protocolFamily: string) {
  return protocolLabelByFamily[protocolFamily] ?? protocolFamily
}

function protocolColor(protocolFamily: string) {
  return protocolColorByFamily[protocolFamily] ?? 'gray'
}

function routePoliciesByProtocol(routePolicies: RoutePolicyView[]) {
  return routePolicies.reduce<Record<string, RoutePolicyView[]>>((acc, policy) => {
    const group = policy.protocolFamily
    acc[group] ??= []
    acc[group].push(policy)
    return acc
  }, {})
}

function listSortedProtocols(groups: Record<string, RoutePolicyView[]>) {
  return Object.entries(groups).sort(([left], [right]) => {
    const leftIndex = protocolOrder.indexOf(left)
    const rightIndex = protocolOrder.indexOf(right)

    if (leftIndex >= 0 && rightIndex >= 0) {
      return leftIndex - rightIndex
    }

    if (leftIndex >= 0) {
      return -1
    }

    if (rightIndex >= 0) {
      return 1
    }

    return left.localeCompare(right)
  })
}

export const Route = createFileRoute('/app/routes')({
  loader: () =>
    loadRouteData(async () => {
      const [routePolicies, routeReceipts] = await Promise.all([
        getConsoleDataService().listRoutePolicies(),
        getConsoleDataService().listRouteReceipts()
      ])

      return {
        routePolicies,
        routeReceipts
      } as RoutePoliciesPageData
    }),
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
          description="Review protocol-aware route policies and recent route diagnostics."
          title="Routes"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === 'error' ? result?.message : undefined}
          description="Route policy and diagnostics data could not be loaded from the control-plane service."
          title="Routes unavailable"
        />
      </Stack>
    )
  }

  const { routePolicies, routeReceipts } = result.data
  const groupedPolicies = routePoliciesByProtocol(routePolicies)
  const protocolGroups = listSortedProtocols(groupedPolicies)

  return (
    <Stack>
      <PageHeader
        description="Review protocol-aware route policies and recent route diagnostics."
        title="Routes"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Route policy config</Text>
          <Badge color="blue" variant="light">
            {routePolicies.length} policies
          </Badge>
        </Group>
        {routePolicies.length === 0 ? (
          <EmptyCollectionState
            description="Create a route policy to define protocol-aware provider resolution."
            title="No route policies"
          />
        ) : (
          protocolGroups.map(([protocolLabel, policies]) => (
            <Stack key={protocolLabel} gap="sm">
              <Group>
                <Badge color={protocolColor(protocolLabel)} size="md">
                  {protocolLabelByFamily[protocolLabel] ?? protocolLabel}
                </Badge>
                <Text c="dimmed" size="sm">
                  {policies.length} polic{policies.length === 1 ? 'y' : 'ies'}
                </Text>
              </Group>
              <Table mb="md" striped withTableBorder>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>Policy</Table.Th>
                    <Table.Th>Protocol</Table.Th>
                    <Table.Th>Model alias</Table.Th>
                    <Table.Th>Selected providers</Table.Th>
                    <Table.Th>Preferred regions</Table.Th>
                    <Table.Th>Required capabilities</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {policies.map((policy) => (
                    <Table.Tr key={policy.id}>
                      <Table.Td>{policy.name}</Table.Td>
                      <Table.Td>{protocolDisplay(policy.protocolFamily)}</Table.Td>
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
            </Stack>
          ))
        )}
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Recent route receipts</Text>
          <Badge color="blue" variant="light">
            {routeReceipts.length}
          </Badge>
        </Group>
        {routeReceipts.length === 0 ? (
          <EmptyCollectionState
            description="No recent route receipts are available yet."
            title="No route receipts"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Receipt ID</Table.Th>
                <Table.Th>Protocol</Table.Th>
                <Table.Th>Model alias</Table.Th>
                <Table.Th>Selected target</Table.Th>
                <Table.Th>Excluded targets</Table.Th>
                <Table.Th>Fallback transitions</Table.Th>
                <Table.Th>Normalized error</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {routeReceipts.map((receipt) => (
                <Table.Tr key={receipt.routeReceiptId}>
                  <Table.Td>{receipt.routeReceiptId}</Table.Td>
                  <Table.Td>{protocolDisplay(receipt.protocolFamily)}</Table.Td>
                  <Table.Td>{receipt.modelAlias}</Table.Td>
                  <Table.Td>
                    {receipt.selectedTargetLabel}
                  </Table.Td>
                  <Table.Td>
                    {receipt.excludedTargets.length === 0 ? (
                      <Text c="dimmed" size="sm">
                        None
                      </Text>
                    ) : (
                      <List size="sm" withPadding>
                        {receipt.excludedTargets.map((target) => (
                          <List.Item key={target.providerResourceId}>
                            {target.providerLabel} ({target.reason})
                          </List.Item>
                        ))}
                      </List>
                    )}
                  </Table.Td>
                  <Table.Td>
                    {receipt.fallbackTransitions.length === 0 ? (
                      <Text c="dimmed" size="sm">
                        None
                      </Text>
                    ) : (
                      <List size="sm" withPadding>
                        {receipt.fallbackTransitions.map((transition) => (
                          <List.Item
                            key={`${transition.fromProviderResourceId}-${transition.toProviderResourceId}-${transition.reason}`}
                          >
                            {transition.fromProviderLabel} &rarr; {transition.toProviderLabel} (
                            {transition.reason})
                          </List.Item>
                        ))}
                      </List>
                    )}
                  </Table.Td>
                  <Table.Td>
                    {receipt.normalizedError ? (
                      <Stack gap={4}>
                        <Text fw={500} size="sm">
                          {receipt.normalizedError.code}
                        </Text>
                        <Text c="dimmed" size="sm">
                          {receipt.normalizedError.message}
                        </Text>
                      </Stack>
                    ) : (
                      <Text c="dimmed" size="sm">
                        None
                      </Text>
                    )}
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
