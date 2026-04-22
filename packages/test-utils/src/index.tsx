import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import {
  render,
  type RenderOptions,
  type RenderResult
} from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createAppTheme } from '@huge-router/design-tokens'
import type { PropsWithChildren, ReactElement } from 'react'

export function createTestQueryClient() {
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

type TestProvidersProps = PropsWithChildren<{
  queryClient?: QueryClient
}>

export function TestProviders({
  children,
  queryClient = createTestQueryClient()
}: TestProvidersProps) {
  return (
    <QueryClientProvider client={queryClient}>
      <MantineProvider theme={createAppTheme()}>
        <ModalsProvider>
          <Notifications />
          {children}
        </ModalsProvider>
      </MantineProvider>
    </QueryClientProvider>
  )
}

type ExtendedRenderOptions = Omit<RenderOptions, 'wrapper'> & {
  queryClient?: QueryClient
}

export function renderWithProviders(
  ui: ReactElement,
  { queryClient = createTestQueryClient(), ...options }: ExtendedRenderOptions = {}
): RenderResult {
  const Wrapper = ({ children }: PropsWithChildren) => (
    <TestProviders queryClient={queryClient}>{children}</TestProviders>
  )

  return render(ui, {
    wrapper: Wrapper,
    ...options
  })
}
