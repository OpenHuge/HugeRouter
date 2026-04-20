import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import { QueryClientProvider } from '@tanstack/react-query'
import { HeadContent, Outlet, Scripts, createRootRoute } from '@tanstack/react-router'
import { createAppTheme } from '@huge-router/design-tokens'
import type { ReactNode } from 'react'
import { getQueryClient } from '../lib/query-client'
import { createRequestContext } from '../start'
import appCss from '../styles/app.css?url'

const queryClient = getQueryClient()
const requestContext = createRequestContext()

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      { title: 'HugeRouter Console' }
    ],
    links: [{ rel: 'stylesheet', href: appCss }]
  }),
  shellComponent: RootDocument,
  component: RootProviders
})

function RootDocument({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  )
}

function RootProviders() {
  return (
    <QueryClientProvider client={queryClient}>
      <MantineProvider theme={createAppTheme()}>
        <ModalsProvider>
          <Notifications />
          <Outlet />
        </ModalsProvider>
      </MantineProvider>
      <div
        style={{
          bottom: 12,
          color: '#5c7394',
          fontSize: 12,
          position: 'fixed',
          right: 16
        }}
      >
        {requestContext.requestId} / {requestContext.traceId}
      </div>
    </QueryClientProvider>
  )
}

