import { Badge, Card, Group, Stack, Table, Text } from '@mantine/core'
import { createFileRoute } from '@tanstack/react-router'
import { tenantSchema } from '@huge-router/ts-shared-schema'
import { PageHeader } from '@huge-router/ui-kit'

const placeholderTenants = [
  tenantSchema.parse({
    id: 'ten_bootstrap',
    slug: 'bootstrap',
    displayName: 'Bootstrap Tenant'
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
              <Table.Tr key={tenant.id}>
                <Table.Td>{tenant.id}</Table.Td>
                <Table.Td>{tenant.slug}</Table.Td>
                <Table.Td>{tenant.displayName}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Card>
    </Stack>
  )
}

