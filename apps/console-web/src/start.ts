import { createMiddleware, createStart } from '@tanstack/react-start'
import { useSyncExternalStore } from 'react'
import {
  type AuthSessionEnvelope,
  type AuthSessionState,
  getDefaultProviderAvailability
} from './features/auth/auth-contract'

export type AppRequestContext = {
  requestId: string
  session: AuthSessionState
  traceId: string
}

const listeners = new Set<() => void>()

function createTraceId(prefix: 'req' | 'trace') {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `${prefix}_${crypto.randomUUID()}`
  }

  return `${prefix}_${Math.random().toString(36).slice(2, 12)}`
}

function createAnonymousSession(): AuthSessionState {
  return {
    availableProviders: getDefaultProviderAvailability(),
    kind: 'anonymous'
  }
}

const bootstrapMiddleware = createMiddleware().server(async ({ next }) => {
  return next()
})

export const startInstance = createStart(() => ({
  defaultSsr: false,
  requestMiddleware: [bootstrapMiddleware]
}))

let requestContextSnapshot: AppRequestContext = {
  requestId: createTraceId('req'),
  session: createAnonymousSession(),
  traceId: createTraceId('trace')
}

export function createRequestContext(): AppRequestContext {
  return requestContextSnapshot
}

function emitRequestContext() {
  listeners.forEach((listener) => listener())
}

export function syncRequestContext(envelope: AuthSessionEnvelope) {
  requestContextSnapshot = {
    requestId: envelope.requestId ?? requestContextSnapshot.requestId,
    session: envelope.state,
    traceId: envelope.traceId ?? requestContextSnapshot.traceId
  }

  emitRequestContext()
}

export function resetRequestContextForTests() {
  requestContextSnapshot = {
    requestId: createTraceId('req'),
    session: createAnonymousSession(),
    traceId: createTraceId('trace')
  }

  emitRequestContext()
}

export function subscribeToRequestContext(listener: () => void) {
  listeners.add(listener)

  return () => {
    listeners.delete(listener)
  }
}

export function useRequestContext() {
  return useSyncExternalStore(
    subscribeToRequestContext,
    createRequestContext,
    createRequestContext
  )
}
