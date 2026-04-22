import { Anchor, Button, Center, Loader, Stack, Text } from '@mantine/core'
import { Link, createFileRoute, redirect } from '@tanstack/react-router'
import { AuthStatusCard } from '../features/auth/AuthStatusCard'
import { AuthApiError } from '../features/auth/auth-client'
import { type AuthProvider, parseAuthCallbackSearch } from '../features/auth/auth-contract'
import { getAuthClient, storeAuthSession } from '../features/auth/auth-queries'
import { getPostLoginDestination } from '../features/auth/auth-routing'
import { getQueryClient } from '../lib/query-client'

function throwRedirect(options: Parameters<typeof redirect>[0]): never {
  throw redirect(options) as unknown as Error
}

type CallbackStatusView = {
  description: string
  provider?: AuthProvider
  title: string
  tone: 'blue' | 'orange' | 'red'
}

function formatProvider(provider?: AuthProvider) {
  if (provider === 'wechat') {
    return 'WeChat'
  }

  return provider ? provider.charAt(0).toUpperCase() + provider.slice(1) : 'Provider'
}

function createFailureView(
  provider: AuthProvider | undefined,
  title: string,
  description: string,
  tone: CallbackStatusView['tone'] = 'red'
): CallbackStatusView {
  return {
    description,
    provider,
    title,
    tone
  }
}

export const Route = createFileRoute('/login/callback')({
  validateSearch: parseAuthCallbackSearch,
  loaderDeps: ({ search }) => search,
  pendingComponent: CallbackPending,
  loader: async ({ deps: search }) => {
    const provider = search.provider

    if (!provider) {
      return createFailureView(
        undefined,
        'Invalid callback link',
        'The callback is missing a provider identifier. Start the sign-in flow again from /login.'
      )
    }

    if (search.error === 'access_denied') {
      return createFailureView(
        provider,
        `${formatProvider(provider)} sign-in was cancelled`,
        search.errorDescription ?? 'The provider returned access_denied before HugeRouter could create a session.',
        'orange'
      )
    }

    if (!search.code && !search.state) {
      return createFailureView(
        provider,
        `${formatProvider(provider)} sign-in failed`,
        search.errorDescription ?? 'The provider callback did not include a usable authorization payload.'
      )
    }

    try {
      const result = await getAuthClient().completeAuthCallback(provider, {
        code: search.code,
        error: search.error,
        errorDescription: search.errorDescription,
        redirectTo: search.redirect,
        state: search.state
      })

      if (result.data.outcome === 'authenticated') {
        storeAuthSession(
          {
            requestId: result.meta.requestId,
            state: result.data.state,
            traceId: result.meta.traceId
          },
          getQueryClient()
        )

        throwRedirect({
          to: getPostLoginDestination(result.data.state, result.data.redirectTo ?? search.redirect)
        })
      }

      if (result.data.outcome === 'provider_disabled') {
        return createFailureView(
          provider,
          `${formatProvider(provider)} sign-in is disabled`,
          result.data.message,
          'orange'
        )
      }

      if (result.data.outcome === 'tenant_denied') {
        return createFailureView(
          provider,
          'Tenant access denied',
          result.data.message,
          'orange'
        )
      }

      if (result.data.outcome === 'cancelled') {
        return createFailureView(
          provider,
          `${formatProvider(provider)} sign-in was cancelled`,
          result.data.message,
          'orange'
        )
      }

      return createFailureView(
        provider,
        `${formatProvider(provider)} sign-in failed`,
        result.data.message
      )
    } catch (error) {
      if (error instanceof AuthApiError) {
        return createFailureView(
          provider,
          `${formatProvider(provider)} sign-in failed`,
          error.message
        )
      }

      throw error
    }
  },
  component: CallbackRoute
})

function CallbackPending() {
  return (
    <Center mih="100vh" px="md">
      <AuthStatusCard
        description="Completing your HugeRouter session and loading tenant access."
        title="Finishing sign-in"
      >
        <Stack align="center" py="md">
          <Loader />
          <Text c="dimmed" size="sm">
            Waiting for the provider callback to settle.
          </Text>
        </Stack>
      </AuthStatusCard>
    </Center>
  )
}

function CallbackRoute() {
  const result = Route.useLoaderData()

  return (
    <Center mih="100vh" px="md">
      <AuthStatusCard
        description={result.description}
        title={result.title}
        tone={result.tone}
        actions={
          <>
            <Button component={Link} to="/login">
              Back to login
            </Button>
            <Anchor component={Link} c="dimmed" size="sm" to="/login">
              Start over
            </Anchor>
          </>
        }
      />
    </Center>
  )
}
