import { Alert, Group, Loader, Stack, Text } from '@mantine/core'
import { EmptyState } from '@huge-router/ui-kit'

type RouteLoadingStateProps = {
  label: string
}

type RouteErrorStateProps = {
  title: string
  description: string
}

export function RouteLoadingState({ label }: RouteLoadingStateProps) {
  return (
    <Group justify="center" py="xl">
      <Loader aria-label={label} size="md" />
    </Group>
  )
}

export function RouteErrorState({ description, title }: RouteErrorStateProps) {
  return (
    <Alert color="red" radius="md" title={title} variant="light">
      <Stack gap={4}>
        <Text>{description}</Text>
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
