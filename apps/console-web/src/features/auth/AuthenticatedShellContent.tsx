import { Badge, Button, Card, Group, Stack, Text } from '@mantine/core'
import type { ReactNode } from 'react'
import { useLogoutMutation } from './auth-queries'
import { useRequestContext } from '../../start'

type AuthenticatedShellContentProps = {
  children: ReactNode
}

export function AuthenticatedShellContent({
  children
}: AuthenticatedShellContentProps) {
  const requestContext = useRequestContext()
  const logoutMutation = useLogoutMutation()

  return (
    <Stack gap="lg">
      {requestContext.session.kind === 'authenticated' ? (
        <Card padding="lg" radius="md" shadow="sm">
          <Group justify="space-between" wrap="wrap">
            <div>
              <Text fw={700}>{requestContext.session.session.user.displayName}</Text>
              <Text c="dimmed" size="sm">
                {requestContext.session.session.user.email}
              </Text>
            </div>
            <Group gap="sm">
              <Badge color="blue" variant="light">
                {requestContext.session.session.user.isPlatformAdmin
                  ? 'platform-admin'
                  : requestContext.session.session.activeTenant?.tenantSlug ?? 'tenant'}
              </Badge>
              <Button
                loading={logoutMutation.isPending}
                onClick={() => void logoutMutation.mutateAsync()}
                variant="light"
              >
                Sign out
              </Button>
            </Group>
          </Group>
        </Card>
      ) : null}
      {children}
    </Stack>
  )
}
