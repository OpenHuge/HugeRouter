import { QueryClient } from '@tanstack/react-query'

let browserQueryClient: QueryClient | undefined

export function createAppQueryClient() {
  return new QueryClient({
    defaultOptions: {
      mutations: {
        retry: false
      },
      queries: {
        retry: false
      }
    }
  })
}

export function getQueryClient() {
  if (typeof window === 'undefined') {
    return createAppQueryClient()
  }

  if (!browserQueryClient) {
    browserQueryClient = createAppQueryClient()
  }

  return browserQueryClient
}

export function resetQueryClientForTests() {
  browserQueryClient?.clear()
  browserQueryClient = undefined
}
