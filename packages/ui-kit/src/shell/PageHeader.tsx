import { Group, Stack, Text, Title } from '@mantine/core'
import type { ReactNode } from 'react'

type PageHeaderProps = {
  title: string
  description: string
  actions?: ReactNode
}

export function PageHeader({ actions, description, title }: PageHeaderProps) {
  return (
    <Group align="flex-start" justify="space-between" mb="lg">
      <Stack gap={4}>
        <Title order={2}>{title}</Title>
        <Text c="dimmed" maw={720}>
          {description}
        </Text>
      </Stack>
      {actions}
    </Group>
  )
}

