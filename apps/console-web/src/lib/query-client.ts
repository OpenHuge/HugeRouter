import { QueryClient } from '@tanstack/react-query'

let browserQueryClient: QueryClient | undefined

export function getQueryClient() {
  if (typeof window === 'undefined') {
    return new QueryClient()
  }

  if (!browserQueryClient) {
    browserQueryClient = new QueryClient()
  }

  return browserQueryClient
}

