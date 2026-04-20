import { Badge, Card, Grid, Group, SimpleGrid, Stack, Text } from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { createControlPlaneClient } from '@huge-router/ts-api-client'
import { EmptyState, PageHeader } from '@huge-router/ui-kit'

const client = createControlPlaneClient()

export const Route = createFileRoute('/app/overview')({
  component: OverviewPage
})

function OverviewPage() {
  void client

  return (
    <Stack>
      <PageHeader
        description="Bootstrap dashboard wired to shared UI primitives, Mantine theme tokens, and generated-client placeholders."
        title="Overview"
      />
      <SimpleGrid cols={{ base: 1, md: 3 }}>
        {[
          ['Gateway readiness', 'Placeholder'],
          ['Tenant projects', '1 project'],
          ['Schema pipeline', 'Reserved for FND-006']
        ].map(([label, value]) => (
          <Card key={label} padding="lg" radius="md" shadow="sm">
            <Text c="dimmed" size="sm">
              {label}
            </Text>
            <Text fw={700} mt="xs" size="xl">
              {value}
            </Text>
          </Card>
        ))}
      </SimpleGrid>
      <Grid>
        <Grid.Col span={{ base: 12, lg: 8 }}>
          <Card padding="lg" radius="md" shadow="sm">
            <Group justify="space-between" mb="md">
              <Text fw={700}>Bootstrap status</Text>
              <Badge color="teal" variant="light">
                foundation ready
              </Badge>
            </Group>
            <Stack gap="sm">
              <Text>Shared providers: Mantine, Query, Notifications, Modals.</Text>
              <Text>Route shells: anonymous login plus authenticated tenant/admin layouts.</Text>
              <Text>Next step: wire generated API types and server-side auth middleware.</Text>
            </Stack>
          </Card>
        </Grid.Col>
        <Grid.Col span={{ base: 12, lg: 4 }}>
          <EmptyState
            description="Diagnostics, usage explorer, and billing dashboards will expand from this shell without changing the provider stack."
            title="Feature modules pending"
          />
        </Grid.Col>
      </Grid>
    </Stack>
  )
}

