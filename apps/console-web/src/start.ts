import { createMiddleware, createStart } from '@tanstack/react-start'

export type AppRequestContext = {
  authState: 'anonymous'
  requestId: string
  traceId: string
}

const placeholderContext: AppRequestContext = {
  authState: 'anonymous',
  requestId: 'req_bootstrap_placeholder',
  traceId: 'trace_bootstrap_placeholder'
}

const bootstrapMiddleware = createMiddleware().server(async ({ next }) => {
  return next()
})

export const startInstance = createStart(() => ({
  defaultSsr: false,
  requestMiddleware: [bootstrapMiddleware]
}))

export function createRequestContext(): AppRequestContext {
  return placeholderContext
}
