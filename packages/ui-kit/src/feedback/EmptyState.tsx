import { Alert, Button, Group, Stack, Text, Title } from '@mantine/core'
import type { ReactNode } from 'react'

type EmptyStateProps = {
  title: string
  description: string
  action?: ReactNode
}

export function EmptyState({ action, description, title }: EmptyStateProps) {
  return (
    <Alert color="teal" radius="md" variant="light">
      <Stack gap="sm">
        <Title order={4}>{title}</Title>
        <Text>{description}</Text>
        <Group>
          {action ?? <Button variant="light">Placeholder action</Button>}
        </Group>
      </Stack>
    </Alert>
  )
}

