import {
  type QueryClient,
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient
} from '@tanstack/react-query'
import { useRouter } from '@tanstack/react-router'
import { syncRequestContext } from '../../start'
import { getQueryClient } from '../../lib/query-client'
import { redirectToExternal } from './browser-navigation'
import { createConsoleAuthClient, type AuthApiError } from './auth-client'
import {
  type AuthProvider,
  type EmailLoginCompleteInput,
  type AuthSessionEnvelope,
  type EmailLoginInput,
  type LoginSearch,
  type ProviderLoginStartInput
} from './auth-contract'

export const authSessionQueryKey = ['auth', 'session'] as const

let authClient = createConsoleAuthClient()

function anonymousEnvelope(): AuthSessionEnvelope {
  return {
    state: {
      availableProviders: [],
      kind: 'anonymous'
    }
  }
}

export function getAuthClient() {
  return authClient
}

export function setAuthClientForTests(client: ReturnType<typeof createConsoleAuthClient> | null) {
  authClient = client ?? createConsoleAuthClient()
}

export function authSessionQueryOptions() {
  return queryOptions({
    queryFn: async () => {
      const result = await authClient.getSession()
      syncRequestContext(result.data)
      return result.data
    },
    queryKey: authSessionQueryKey,
    retry: false,
    staleTime: 30_000
  })
}

export async function loadAuthSession(queryClient: QueryClient = getQueryClient()) {
  return queryClient.fetchQuery(authSessionQueryOptions())
}

export function useAuthSessionQuery() {
  return useQuery(authSessionQueryOptions())
}

export function storeAuthSession(
  envelope: AuthSessionEnvelope,
  queryClient: QueryClient = getQueryClient()
) {
  queryClient.setQueryData(authSessionQueryKey, envelope)
  syncRequestContext(envelope)
}

export function setAnonymousSession(queryClient: QueryClient = getQueryClient()) {
  const envelope = anonymousEnvelope()
  storeAuthSession(envelope, queryClient)
}

function getLoginSearch(reason: LoginSearch['reason'], message?: string): LoginSearch {
  return {
    message,
    reason
  }
}

export function useEmailLoginMutation() {
  return useMutation({
    mutationFn: async (input: EmailLoginInput) => {
      const result = await authClient.startEmailLogin(input)
      const availableProviders =
        getQueryClient().getQueryData<AuthSessionEnvelope>(authSessionQueryKey)?.state
          .availableProviders ?? []
      syncRequestContext({
        requestId: result.meta.requestId,
        state: {
          availableProviders,
          kind: 'anonymous'
        },
        traceId: result.meta.traceId
      })
      return result.data
    }
  })
}

export function useEmailLoginCompleteMutation() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: EmailLoginCompleteInput) => {
      const result = await authClient.completeEmailLogin(input)

      if (result.data.outcome === 'authenticated') {
        storeAuthSession(
          {
            requestId: result.meta.requestId,
            state: result.data.state,
            traceId: result.meta.traceId
          },
          queryClient
        )
      }

      return result.data
    }
  })
}

export function useProviderLoginMutation() {
  return useMutation({
    mutationFn: async ({
      input,
      provider
    }: {
      input: ProviderLoginStartInput
      provider: AuthProvider
    }) => {
      const result = await authClient.startProviderLogin(provider, input)
      redirectToExternal(result.data.authorizationUrl)
      return result.data
    }
  })
}

export function useLogoutMutation() {
  const queryClient = useQueryClient()
  const router = useRouter()

  return useMutation({
    mutationFn: async () => {
      await authClient.logout()
      setAnonymousSession(queryClient)
    },
    onSuccess: async () => {
      await router.navigate({
        search: getLoginSearch('signed-out'),
        to: '/login'
      })
    }
  })
}

export function getAuthErrorMessage(error: unknown) {
  if (error && typeof error === 'object' && 'message' in error) {
    return String(error.message)
  }

  return 'Something went wrong while contacting HugeRouter auth.'
}

export function getAuthErrorReason(error: unknown): LoginSearch['reason'] | undefined {
  const apiError = error as AuthApiError | undefined

  if (apiError?.status === 401) {
    return 'session-expired'
  }

  if (apiError?.code === 'provider_disabled') {
    return 'provider-disabled'
  }

  if (apiError?.code === 'tenant_access_denied') {
    return 'tenant-denied'
  }

  return undefined
}
