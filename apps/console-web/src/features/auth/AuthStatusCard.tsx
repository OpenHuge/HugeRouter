import { Alert, Card, Group, Stack, Text, Title } from '@mantine/core'
import type { ReactNode } from 'react'

type AuthStatusCardProps = {
  title: string
  description: string
  tone?: 'blue' | 'green' | 'orange' | 'red'
  actions?: ReactNode
  children?: ReactNode
}

export function AuthStatusCard({
  actions,
  children,
  description,
  title,
  tone = 'blue'
}: AuthStatusCardProps) {
  return (
    <Card maw={480} padding="xl" radius="lg" shadow="md" w="100%">
      <Stack>
        <Alert color={tone} variant="light">
          <Stack gap="xs">
            <Title order={3}>{title}</Title>
            <Text>{description}</Text>
          </Stack>
        </Alert>
        {children}
        {actions ? <Group>{actions}</Group> : null}
      </Stack>
    </Card>
  )
}
