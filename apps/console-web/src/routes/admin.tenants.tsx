import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { tenantSchema } from '@huge-router/ts-shared-schema'
import { PageHeader } from '@huge-router/ui-kit'

const placeholderTenants = [
  tenantSchema.parse({
    tenant_id: 'tenant_bootstrap',
    slug: 'bootstrap',
    display_name: 'Bootstrap Tenant',
    version: 1,
    created_at: '2026-04-22T00:00:00Z',
    updated_at: '2026-04-22T00:00:00Z'
  })
]

export const Route = createFileRoute('/admin/tenants')({
  component: AdminTenantsPage
})

function AdminTenantsPage() {
  return (
    <Stack>
      <PageHeader
        description="Admin tenancy placeholder demonstrating shared schemas and shell primitives."
        title="Tenants"
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Known tenants</Text>
          <Badge color="blue" variant="light">
            placeholder dataset
          </Badge>
        </Group>
        <Table striped withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>ID</Table.Th>
              <Table.Th>Slug</Table.Th>
              <Table.Th>Display Name</Table.Th>
            </Table.Tr>
          </Table.Thead>
            <Table.Tbody>
              {placeholderTenants.map((tenant) => (
              <Table.Tr key={tenant.tenant_id}>
                <Table.Td>{tenant.tenant_id}</Table.Td>
                <Table.Td>{tenant.slug}</Table.Td>
                <Table.Td>{tenant.display_name}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Card>
    </Stack>
  )
}
