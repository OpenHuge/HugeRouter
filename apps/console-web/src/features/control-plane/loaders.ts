export type RouteDataErrorKind =
  | 'access-denied'
  | 'session-expired'
  | 'not-found'
  | 'unknown'

export type RouteDataResult<T> =
  | {
      state: 'error'
      kind?: RouteDataErrorKind
      message?: string
    }
  | {
      data: T
      state: 'success'
    }

function parseErrorCode(error: unknown): string | undefined {
  if (!error || typeof error !== 'object') {
    return undefined
  }

  const payload = error as {
    code?: string
    status?: number
    error?: {
      code?: string
      message?: string
    }
  }

  return payload.code ?? payload.error?.code
}

function parseErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }

  if (!error || typeof error !== 'object') {
    return 'Request failed for an unknown reason.'
  }

  const payload = error as {
    message?: string
    error?: {
      message?: string
    }
  }

  return payload.message ?? payload.error?.message ?? 'Request failed for an unknown reason.'
}

function isExpiredAuthStatus(error: { status?: number }) {
  return error.status === 401 || error.status === 419
}

function isForbiddenAuthStatus(error: { status?: number }) {
  return error.status === 403
}

export async function loadRouteData<T>(load: () => Promise<T>): Promise<RouteDataResult<T>> {
  try {
    const data = await load()

    return {
      data,
      state: 'success'
    }
  } catch (error) {
    if (
      error &&
      typeof error === 'object' &&
      'status' in error
    ) {
      const code = parseErrorCode(error)
      const message = parseErrorMessage(error)

      if (isExpiredAuthStatus(error as { status?: number })) {
        return Promise.resolve({
          kind: 'session-expired',
          message,
          state: 'error'
        })
      }

      if (code === 'tenant_access_denied') {
        return Promise.resolve({
          kind: 'access-denied',
          message,
          state: 'error'
        })
      }

      if (isForbiddenAuthStatus(error as { status?: number }) || code === 'forbidden') {
        return Promise.resolve({
          kind: 'access-denied',
          message,
          state: 'error'
        })
      }

      if (code === 'not_found') {
        return Promise.resolve({
          kind: 'not-found',
          message,
          state: 'error'
        })
      }
    }

    return Promise.resolve({
      kind: 'unknown',
      message: parseErrorMessage(error),
      state: 'error'
    })
  }
}
