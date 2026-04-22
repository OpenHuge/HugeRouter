import { MantineProvider } from '@mantine/core'
import { ModalsProvider } from '@mantine/modals'
import { Notifications } from '@mantine/notifications'
import { QueryClientProvider } from '@tanstack/react-query'
import { HeadContent, Outlet, Scripts, createRootRoute } from '@tanstack/react-router'
import { createAppTheme } from '@huge-router/design-tokens'
import type { ReactNode } from 'react'
import { useAuthSessionQuery } from '../features/auth/auth-queries'
import { getQueryClient } from '../lib/query-client'
import { useRequestContext } from '../start'
import appCss from '../styles/app.css?url'

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
  if (typeof navigator !== 'undefined' && navigator.userAgent.includes('jsdom')) {
    return (
      <>
        {children}
        <Scripts />
      </>
    )
  }

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

function formatSessionLabel(requestContext: ReturnType<typeof useRequestContext>) {
  if (requestContext.session.kind !== 'authenticated') {
    return requestContext.session.kind
  }

  const tenantLabel = requestContext.session.session.activeTenant?.tenantSlug ?? 'platform-admin'

  return `${requestContext.session.session.user.email} @ ${tenantLabel}`
}

function RootProviders() {
  const queryClient = getQueryClient()

  return (
    <QueryClientProvider client={queryClient}>
      <RootProvidersInner />
    </QueryClientProvider>
  )
}

function RootProvidersInner() {
  const requestContext = useRequestContext()

  useAuthSessionQuery()

  return (
    <>
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
        {formatSessionLabel(requestContext)} • {requestContext.requestId} • {requestContext.traceId}
      </div>
    </>
  )
}
