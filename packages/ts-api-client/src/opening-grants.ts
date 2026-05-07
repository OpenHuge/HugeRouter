import {
  createOpeningGrantRequestSchema,
  openingGrantCreateResponseSchema,
  openingGrantRevokeRequestSchema,
  openingGrantSchema,
  openingGrantsResponseSchema,
  type CreateOpeningGrantRequest,
  type OpeningGrant,
  type OpeningGrantCreateResponse
} from '@huge-router/ts-shared-schema'
import type { CONTROL_PLANE_OPERATIONS } from './generated/operation-meta.ts'
import type { FetchLike } from './http.ts'

type OperationId = (typeof CONTROL_PLANE_OPERATIONS)[number]['id']

export type OpeningGrantsClient = {
  listOpeningGrants: () => Promise<OpeningGrant[]>
  createOpeningGrant: (
    request: CreateOpeningGrantRequest
  ) => Promise<OpeningGrantCreateResponse>
  revokeOpeningGrant: (
    grantId: string,
    expectedVersion: number
  ) => Promise<OpeningGrant>
}

export const createOpeningGrantOperations = ({
  baseUrl,
  fetchImpl,
  headers,
  requestJson,
  resolveOperation
}: {
  baseUrl: string
  fetchImpl: FetchLike
  headers?: HeadersInit
  requestJson: <T>(input: {
    baseUrl: string
    fetchImpl: FetchLike
    headers?: HeadersInit
    method: string
    path: string
    params?: Record<string, string>
    body?: unknown
    parse: (payload: unknown) => T
  }) => Promise<T>
  resolveOperation: (id: OperationId) => { method: string; path: string }
}): OpeningGrantsClient => ({
  async listOpeningGrants() {
    const operation = resolveOperation('listOpeningGrants')
    return requestJson({
      baseUrl,
      fetchImpl,
      headers,
      method: operation.method,
      path: operation.path,
      parse: (payload) => openingGrantsResponseSchema.parse(payload).data
    })
  },
  async createOpeningGrant(request) {
    const operation = resolveOperation('createOpeningGrant')
    return requestJson({
      baseUrl,
      fetchImpl,
      headers,
      method: operation.method,
      path: operation.path,
      body: createOpeningGrantRequestSchema.parse(request),
      parse: (payload) => openingGrantCreateResponseSchema.parse(payload)
    })
  },
  async revokeOpeningGrant(grantId, expectedVersion) {
    const operation = resolveOperation('revokeOpeningGrant')
    return requestJson({
      baseUrl,
      fetchImpl,
      headers,
      method: operation.method,
      path: operation.path,
      params: { grant_id: grantId },
      body: openingGrantRevokeRequestSchema.parse({
        expected_version: expectedVersion
      }),
      parse: (payload) => openingGrantSchema.parse(payload)
    })
  }
})
