import { Alert, Button, Group, Loader, Stack, Text } from '@mantine/core'
import { EmptyState } from '@huge-router/ui-kit'
import { Link } from '@tanstack/react-router'
import type { RouteDataErrorKind } from './loaders'

type RouteLoadingStateProps = {
  label: string
}

type RouteErrorStateProps = {
  kind?: RouteDataErrorKind
  title: string
  description: string
  message?: string
}

export function RouteLoadingState({ label }: RouteLoadingStateProps) {
  return (
    <Group justify="center" py="xl">
      <Loader aria-label={label} size="md" />
    </Group>
  )
}

function getActionByKind(kind?: RouteDataErrorKind) {
  if (kind === 'session-expired') {
    return {
      label: 'Sign in again',
      to: '/login?reason=session-expired'
    }
  }

  if (kind === 'access-denied') {
    return {
      label: 'Try another account',
      to: '/login?reason=tenant-denied'
    }
  }

  if (kind === 'not-found') {
    return {
      label: 'Go to overview',
      to: '/app/overview'
    }
  }

  return null
}

function resolveRouteErrorText(kind: RouteDataErrorKind | undefined, fallback: string) {
  if (kind === 'session-expired') {
    return 'The console session is no longer valid. Sign in to continue.'
  }

  if (kind === 'access-denied') {
    return 'The active account does not have access to this control-plane resource.'
  }

  if (kind === 'not-found') {
    return 'That control-plane resource could not be found.'
  }

  return fallback
}

export function RouteErrorState({
  description,
  kind,
  message,
  title
}: RouteErrorStateProps) {
  const action = getActionByKind(kind)

  return (
    <Alert color="red" radius="md" title={title} variant="light">
      <Stack gap={4}>
        <Text>{resolveRouteErrorText(kind, description)}</Text>
        {message ? <Text c="dimmed" size="sm">{message}</Text> : null}
        {action ? (
          <Group mt={4}>
            <Button
              component={Link}
              size="xs"
              to={action.to}
              variant="light"
            >
              {action.label}
            </Button>
          </Group>
        ) : null}
      </Stack>
    </Alert>
  )
}

type EmptyCollectionStateProps = {
  title: string
  description: string
}

export function EmptyCollectionState({
  description,
  title
}: EmptyCollectionStateProps) {
  return <EmptyState action={null} description={description} title={title} />
}
