import { Button, Card, Center, PasswordInput, Stack, TextInput, Title } from '@mantine/core'

export function LoginPage() {
  return (
    <Center mih="100vh" px="md">
      <Card maw={420} padding="xl" radius="lg" shadow="md" w="100%">
        <Stack>
          <Title order={1}>Sign in</Title>
          <TextInput label="Workspace" placeholder="platform-admin" />
          <TextInput label="Email" placeholder="ops@huge-router.dev" />
          <PasswordInput label="Password" placeholder="********" />
          <Button fullWidth variant="filled">
            Placeholder authentication
          </Button>
        </Stack>
      </Card>
    </Center>
  )
}
