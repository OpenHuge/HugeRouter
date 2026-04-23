import {
  Alert,
  Button,
  Card,
  Center,
  Divider,
  Group,
  Stack,
  Text,
  TextInput,
  Title
} from '@mantine/core'
import type { AuthSessionState } from './auth-contract'
import { useState } from 'react'
import type { AuthSessionEnvelope, AuthProvider, LoginSearch } from './auth-contract'
import { getAuthProviderLabel } from './auth-contract'
import {
  getAuthErrorMessage,
  getAuthErrorReason,
  useEmailLoginCompleteMutation,
  useEmailLoginMutation,
  useProviderLoginMutation
} from './auth-queries'

function getSignInCopy(
  configuredProviderCount: number
) {
  if (configuredProviderCount === 0) {
    return 'No sign-in methods are currently configured for this workspace.'
  }

  return 'Use a configured sign-in method below. Only providers enabled for this workspace are shown.'
}

type LoginPageProps = {
  onAuthenticated?: (
    state: Extract<AuthSessionState, { kind: 'authenticated' }>
  ) => Promise<void> | void
  search: LoginSearch
  sessionEnvelope: AuthSessionEnvelope
}

function getLoginStatusCopy(
  search: LoginSearch,
  sessionEnvelope: AuthSessionEnvelope
) {
  if (sessionEnvelope.state.kind === 'tenant_access_denied') {
    return {
      description: sessionEnvelope.state.message,
      tone: 'orange' as const,
      title: 'Tenant access is not available for this account'
    }
  }

  if (sessionEnvelope.state.kind === 'tenant_selection_required') {
    return {
      description: sessionEnvelope.state.message,
      tone: 'blue' as const,
      title: 'Choose a workspace to continue'
    }
  }

  if (search.reason === 'provider-disabled') {
    return {
      description:
        search.message ??
        `${search.provider ? getAuthProviderLabel(search.provider) : 'That provider'} is currently disabled for this workspace.`,
      tone: 'orange' as const,
      title: 'Provider unavailable'
    }
  }

  if (search.reason === 'tenant-denied') {
    return {
      description:
        search.message ??
        'Your identity is valid, but HugeRouter could not map it to an allowed tenant membership.',
      tone: 'orange' as const,
      title: 'Tenant access denied'
    }
  }

  if (search.reason === 'session-expired') {
    return {
      description:
        search.message ??
        'Your previous HugeRouter session expired. Sign in again to continue.',
      tone: 'blue' as const,
      title: 'Session expired'
    }
  }

  if (search.reason === 'signed-out') {
    return {
      description:
        search.message ?? 'Your HugeRouter session has been closed on this device.',
      tone: 'green' as const,
      title: 'Signed out'
    }
  }

  return null
}

export function LoginPage({
  onAuthenticated,
  search,
  sessionEnvelope
}: LoginPageProps) {
  const [workspaceSlug, setWorkspaceSlug] = useState('platform-admin')
  const [email, setEmail] = useState('ops@huge-router.dev')
  const [emailFlowId, setEmailFlowId] = useState<string | null>(null)
  const [emailCode, setEmailCode] = useState('')
  const [localError, setLocalError] = useState<string | null>(null)
  const emailLoginMutation = useEmailLoginMutation()
  const emailCompleteMutation = useEmailLoginCompleteMutation()
  const providerLoginMutation = useProviderLoginMutation()

  const loginStatus = getLoginStatusCopy(search, sessionEnvelope)
  const configuredProviders = sessionEnvelope.state.availableProviders.filter(
    (provider) => !provider.hidden
  )
  const emailProvider = configuredProviders.find((provider) => provider.provider === 'email')
  const visibleProviders = configuredProviders.filter(
    (provider) => provider.provider !== 'email'
  )

  async function handleEmailLogin() {
    setLocalError(null)

    if (!emailProvider) {
      setLocalError('Email sign-in is not configured for this workspace.')
      return
    }

    if (!emailProvider.enabled) {
      setLocalError(emailProvider.reason ?? 'Email sign-in is currently unavailable.')
      return
    }

    if (!email.trim()) {
      setLocalError('Email is required.')
      return
    }

    try {
      const result = await emailLoginMutation.mutateAsync({
        email,
        redirectTo: search.redirect,
        workspaceSlug
      })
      setEmailFlowId(result.flowId)
    } catch (error) {
      setLocalError(getAuthErrorMessage(error))
    }
  }

  async function handleEmailVerification() {
    setLocalError(null)

    if (!emailFlowId) {
      setLocalError('Start the email login flow before entering a verification code.')
      return
    }

    if (!emailCode.trim()) {
      setLocalError('Verification code is required.')
      return
    }

    try {
      const result = await emailCompleteMutation.mutateAsync({
        code: emailCode.trim(),
        flowId: emailFlowId
      })

      if (result.outcome === 'authenticated') {
        await onAuthenticated?.(result.state)
      } else {
        setLocalError(result.message)
      }
    } catch (error) {
      setLocalError(getAuthErrorMessage(error))
    }
  }

  async function handleProviderLogin(provider: AuthProvider) {
    setLocalError(null)

    try {
      await providerLoginMutation.mutateAsync({
        input: {
          redirectTo: search.redirect,
          workspaceSlug
        },
        provider
      })
    } catch (error) {
      const reason = getAuthErrorReason(error)
      const message = getAuthErrorMessage(error)

      setLocalError(
        reason === 'provider-disabled'
          ? `${getAuthProviderLabel(provider)} sign-in is disabled. ${message}`
          : message
      )
    }
  }

  return (
    <Center mih="100vh" px="md">
      <Card maw={460} padding="xl" radius="lg" shadow="md" w="100%">
        <Stack>
          <Title order={1}>Sign in</Title>
          <Text c="dimmed" size="sm">
            {getSignInCopy(configuredProviders.length)}
          </Text>
          {loginStatus ? (
            <Alert color={loginStatus.tone} variant="light">
              <Text fw={700}>{loginStatus.title}</Text>
              <Text mt="xs" size="sm">
                {loginStatus.description}
              </Text>
            </Alert>
          ) : null}
          {localError ? (
            <Alert color="red" variant="light">
              {localError}
            </Alert>
          ) : null}
          {emailLoginMutation.data ? (
            <Alert color="green" variant="light">
              <Text fw={700}>Check your email</Text>
              <Text mt="xs" size="sm">
                {emailLoginMutation.data.message}
              </Text>
              {emailLoginMutation.data.codeHint ? (
                <Text mt="xs" size="sm">
                  {emailLoginMutation.data.codeHint}
                </Text>
              ) : null}
            </Alert>
          ) : null}
          {configuredProviders.length > 0 ? (
            <>
              <TextInput
                description="Optional workspace hint used for email and provider start requests."
                label="Workspace"
                onChange={(event) => setWorkspaceSlug(event.currentTarget.value)}
                placeholder="platform-admin"
                value={workspaceSlug}
              />
              {emailProvider ? (
                <>
                  <TextInput
                    label="Email"
                    onChange={(event) => setEmail(event.currentTarget.value)}
                    placeholder="ops@huge-router.dev"
                    type="email"
                    value={email}
                  />
                  <Button
                    disabled={!emailProvider.enabled}
                    fullWidth
                    loading={emailLoginMutation.isPending}
                    onClick={() => void handleEmailLogin()}
                    variant="filled"
                  >
                    Continue with Email
                  </Button>
                  {!emailProvider.enabled && emailProvider.reason ? (
                    <Text c="dimmed" size="xs">
                      {emailProvider.reason}
                    </Text>
                  ) : null}
                  {emailFlowId ? (
                    <Stack gap="xs">
                      <Text fw={700} size="sm">
                        Enter verification code
                      </Text>
                      <TextInput
                        label="Verification code"
                        onChange={(event) => setEmailCode(event.currentTarget.value)}
                        placeholder="111111"
                        value={emailCode}
                      />
                      <Button
                        fullWidth
                        loading={emailCompleteMutation.isPending}
                        onClick={() => void handleEmailVerification()}
                        variant="light"
                      >
                        Complete Email Sign-In
                      </Button>
                    </Stack>
                  ) : null}
                </>
              ) : null}
              {emailProvider && visibleProviders.length > 0 ? (
                <Divider label="or" labelPosition="center" />
              ) : null}
              {visibleProviders.length > 0 ? (
                <Stack gap="xs">
                  {visibleProviders.map((provider) => (
                    <div key={provider.provider}>
                      <Button
                        disabled={!provider.enabled}
                        fullWidth
                        loading={
                          providerLoginMutation.isPending &&
                          providerLoginMutation.variables?.provider === provider.provider
                        }
                        onClick={() => void handleProviderLogin(provider.provider)}
                        variant="default"
                      >
                        Continue with {getAuthProviderLabel(provider.provider)}
                      </Button>
                      {!provider.enabled && provider.reason ? (
                        <Text c="dimmed" mt={4} size="xs">
                          {provider.reason}
                        </Text>
                      ) : null}
                    </div>
                  ))}
                </Stack>
              ) : null}
            </>
          ) : (
            <Alert color="orange" variant="light">
              Contact your workspace administrator to configure at least one sign-in method.
            </Alert>
          )}
          <Group gap="xs" justify="space-between">
            <Text c="dimmed" size="xs">
              Provider selection is separate from tenant resolution.
            </Text>
            <Text c="dimmed" size="xs">
              Request target: {search.redirect ?? '/app/overview'}
            </Text>
          </Group>
        </Stack>
      </Card>
    </Center>
  )
}
