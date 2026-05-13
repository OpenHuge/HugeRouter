import {
  errorEnvelopeSchema,
  type ErrorEnvelope
} from '@huge-router/ts-shared-schema'

export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>

export class ContractApiError extends Error {
  readonly status: number
  readonly envelope: ErrorEnvelope

  constructor(status: number, envelope: ErrorEnvelope) {
    super(envelope.error.message)
    this.status = status
    this.envelope = envelope
  }
}

export class ControlPlaneClientError extends Error {
  readonly code?: string
  readonly meta: {
    requestId?: string
    traceId?: string
  }
  readonly status: number

  constructor(
    message: string,
    status: number,
    options: {
      code?: string
      meta?: {
        requestId?: string
        traceId?: string
      }
    } = {}
  ) {
    super(message)
    this.code = options.code
    this.meta = options.meta ?? {}
    this.name = 'ControlPlaneClientError'
    this.status = status
  }
}

export type ClientOptions = {
  baseUrl: string
  fetch?: FetchLike
  headers?: HeadersInit
}

export const buildUrl = (
  baseUrl: string,
  pathTemplate: string,
  params?: Record<string, string>,
  query?: Record<string, string | number | undefined>
) => {
  const path = Object.entries(params ?? {}).reduce(
    (currentPath, [key, value]) =>
      currentPath.replace(`{${key}}`, encodeURIComponent(value)),
    pathTemplate
  )

  const queryString = new URLSearchParams(
    Object.entries(query ?? {}).flatMap(([key, value]) =>
      value == null || value === '' ? [] : [[key, String(value)]]
    )
  ).toString()

  if (!baseUrl) {
    return queryString ? `${path}?${queryString}` : path
  }

  const url = new URL(path, ensureTrailingSlash(baseUrl))
  if (queryString) {
    url.search = queryString
  }
  return url.toString()
}

const ensureTrailingSlash = (baseUrl: string) =>
  baseUrl.endsWith('/') ? baseUrl : `${baseUrl}/`

export const defaultFetch = (): FetchLike => {
  if (!globalThis.fetch) {
    throw new Error('No fetch implementation was provided to the API client')
  }

  return globalThis.fetch.bind(globalThis)
}

export const buildHeaders = (
  defaults: Record<string, string>,
  headers?: HeadersInit
) => {
  const merged = new Headers(defaults)

  if (headers) {
    new Headers(headers).forEach((value, key) => {
      merged.set(key, value)
    })
  }

  return merged
}

export const requestJson = async <T>({
  baseUrl,
  fetchImpl,
  headers,
  method,
  path,
  params,
  query,
  body,
  parse
}: {
  baseUrl: string
  fetchImpl: FetchLike
  headers?: HeadersInit
  method: string
  path: string
  params?: Record<string, string>
  query?: Record<string, string | number | undefined>
  body?: unknown
  parse: (payload: unknown) => T
}) => {
  const response = await fetchImpl(buildUrl(baseUrl, path, params, query), {
    method,
    credentials: 'include',
    headers: buildHeaders(
      {
        Accept: 'application/json',
        ...(body ? { 'Content-Type': 'application/json' } : {})
      },
      headers
    ),
    body: body ? JSON.stringify(body) : undefined
  })

  const payload: unknown = await response.json()

  if (!response.ok) {
    throw new ContractApiError(response.status, errorEnvelopeSchema.parse(payload))
  }

  return parse(payload)
}

export const requestAuthJson = async <T>({
  baseUrl,
  fetchImpl,
  headers,
  method,
  path,
  body,
  parse
}: {
  baseUrl: string
  fetchImpl: FetchLike
  headers?: HeadersInit
  method: string
  path: string
  body?: unknown
  parse: (payload: unknown) => T
}) => {
  const response = await fetchImpl(buildUrl(baseUrl, path), {
    body: body ? JSON.stringify(body) : undefined,
    credentials: 'include',
    method,
    headers: buildHeaders(
      {
        Accept: 'application/json',
        ...(body ? { 'Content-Type': 'application/json' } : {})
      },
      headers
    )
  })

  const payload: unknown = await response.json()

  if (!response.ok) {
    const errorPayload = payload as
      | {
          code?: string
          error?: {
            code?: string
            message?: string
            requestId?: string
            traceId?: string
          }
          message?: string
          requestId?: string
          traceId?: string
        }
      | undefined

    throw new ControlPlaneClientError(
      errorPayload?.error?.message ??
        errorPayload?.message ??
        `Control plane auth request failed with status ${response.status}`,
      response.status,
      {
        code: errorPayload?.error?.code ?? errorPayload?.code,
        meta: {
          requestId: errorPayload?.error?.requestId ?? errorPayload?.requestId,
          traceId: errorPayload?.error?.traceId ?? errorPayload?.traceId
        }
      }
    )
  }

  return parse(payload)
}
