import { Button, Card, Center, Divider, Stack, Text, TextInput, Title } from '@mantine/core'

export function LoginPage() {
  return (
    <Center mih="100vh" px="md">
      <Card maw={420} padding="xl" radius="lg" shadow="md" w="100%">
        <Stack>
          <Title order={1}>Sign in</Title>
          <Text c="dimmed" size="sm">
            Use email or continue with GitHub, Google, or WeChat.
          </Text>
          <TextInput label="Workspace" placeholder="platform-admin" />
          <TextInput label="Email" placeholder="ops@huge-router.dev" />
          <Button fullWidth variant="filled">
            Continue with Email
          </Button>
          <Divider label="or" labelPosition="center" />
          <Button fullWidth variant="default">
            Continue with GitHub
          </Button>
          <Button fullWidth variant="default">
            Continue with Google
          </Button>
          <Button fullWidth variant="default">
            Continue with WeChat
          </Button>
        </Stack>
      </Card>
    </Center>
  )
}
