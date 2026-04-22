import {
  configSnapshotResponseSchema,
  errorEnvelopeSchema,
  gatewayChatRequestSchema,
  gatewayChatResponseSchema,
  projectsResponseSchema,
  providerResourceSchema,
  providerResourcesResponseSchema,
  routePoliciesResponseSchema,
  routeReceiptResponseSchema,
  routeSimulationRequestSchema,
  routeSimulationResponseSchema,
  tenantsResponseSchema,
  type ErrorEnvelope,
  type GatewayChatRequest,
  type GatewayChatResponse,
  type Project,
  type ProviderResource,
  type RoutePolicy,
  type RouteReceipt,
  type RouteSimulationRequest,
  type RouteSimulationResponse,
  type Tenant
} from '@huge-router/ts-shared-schema'
import {
  CONTRACT_DIGEST,
  CONTRACT_VERSION,
  CONTROL_PLANE_OPERATIONS,
  GATEWAY_OPERATIONS
} from './generated/operation-meta.ts'

type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
type OperationId = (typeof CONTROL_PLANE_OPERATIONS)[number]['id']
type GatewayOperationId = (typeof GATEWAY_OPERATIONS)[number]['id']

export class ContractApiError extends Error {
  readonly status: number
  readonly envelope: ErrorEnvelope

  constructor(status: number, envelope: ErrorEnvelope) {
    super(envelope.error.message)
    this.status = status
    this.envelope = envelope
  }
}

export type ClientOptions = {
  baseUrl: string
  fetch?: FetchLike
  headers?: HeadersInit
}

export type ControlPlaneClient = {
  readonly contractVersion: typeof CONTRACT_VERSION
  readonly contractDigest: typeof CONTRACT_DIGEST
  listTenants: () => Promise<Tenant[]>
  listProjects: () => Promise<Project[]>
  listProviderResources: () => Promise<ProviderResource[]>
  getProviderResource: (providerResourceId: string) => Promise<ProviderResource>
  listRoutePolicies: () => Promise<RoutePolicy[]>
  getConfigSnapshot: (configSnapshotId: string) => Promise<
    ReturnType<typeof configSnapshotResponseSchema.parse>['config_snapshot']
  >
  activateConfigSnapshot: (configSnapshotId: string) => Promise<
    ReturnType<typeof configSnapshotResponseSchema.parse>['config_snapshot']
  >
  simulateRoute: (
    request: RouteSimulationRequest
  ) => Promise<RouteSimulationResponse>
  getRouteReceipt: (routeReceiptId: string) => Promise<RouteReceipt>
}

export type GatewayClient = {
  readonly contractVersion: typeof CONTRACT_VERSION
  readonly contractDigest: typeof CONTRACT_DIGEST
  createChatCompletion: (
    request: GatewayChatRequest
  ) => Promise<GatewayChatResponse>
}

const resolveOperation = (id: OperationId) => {
  const operation = CONTROL_PLANE_OPERATIONS.find((candidate) => candidate.id === id)
  if (!operation) {
    throw new Error(`Missing control-plane operation metadata for ${id}`)
  }
  return operation
}

const resolveGatewayOperation = (id: GatewayOperationId) => {
  const operation = GATEWAY_OPERATIONS.find((candidate) => candidate.id === id)
  if (!operation) {
    throw new Error(`Missing gateway operation metadata for ${id}`)
  }
  return operation
}

const buildUrl = (
  baseUrl: string,
  pathTemplate: string,
  params?: Record<string, string>
) => {
  const path = Object.entries(params ?? {}).reduce(
    (currentPath, [key, value]) =>
      currentPath.replace(`{${key}}`, encodeURIComponent(value)),
    pathTemplate
  )

  return new URL(path, ensureTrailingSlash(baseUrl)).toString()
}

const ensureTrailingSlash = (baseUrl: string) =>
  baseUrl.endsWith('/') ? baseUrl : `${baseUrl}/`

const defaultFetch = (): FetchLike => {
  if (!globalThis.fetch) {
    throw new Error('No fetch implementation was provided to the API client')
  }

  return globalThis.fetch.bind(globalThis)
}

const requestJson = async <T>({
  baseUrl,
  fetchImpl,
  headers,
  method,
  path,
  params,
  body,
  parse
}: {
  baseUrl: string
  fetchImpl: FetchLike
  headers?: HeadersInit
  method: string
  path: string
  params?: Record<string, string>
  body?: unknown
  parse: (payload: unknown) => T
}) => {
  const response = await fetchImpl(buildUrl(baseUrl, path, params), {
    method,
    headers: {
      Accept: 'application/json',
      ...(body ? { 'Content-Type': 'application/json' } : {}),
      ...(headers ?? {})
    },
    body: body ? JSON.stringify(body) : undefined
  })

  const payload: unknown = await response.json()

  if (!response.ok) {
    throw new ContractApiError(response.status, errorEnvelopeSchema.parse(payload))
  }

  return parse(payload)
}

export const createControlPlaneClient = (
  options: ClientOptions = {
    baseUrl: ''
  }
): ControlPlaneClient => {
  const fetchImpl = options.fetch ?? defaultFetch()

  return {
    contractVersion: CONTRACT_VERSION,
    contractDigest: CONTRACT_DIGEST,
    async listTenants() {
      const operation = resolveOperation('listTenants')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => tenantsResponseSchema.parse(payload).data
      })
    },
    async listProjects() {
      const operation = resolveOperation('listProjects')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => projectsResponseSchema.parse(payload).data
      })
    },
    async listProviderResources() {
      const operation = resolveOperation('listProviderResources')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => providerResourcesResponseSchema.parse(payload).data
      })
    },
    async getProviderResource(providerResourceId) {
      const operation = resolveOperation('getProviderResource')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { provider_resource_id: providerResourceId },
        parse: (payload) => providerResourceSchema.parse(payload)
      })
    },
    async listRoutePolicies() {
      const operation = resolveOperation('listRoutePolicies')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => routePoliciesResponseSchema.parse(payload).data
      })
    },
    async getConfigSnapshot(configSnapshotId) {
      const operation = resolveOperation('getConfigSnapshot')
      const parsed = await requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { config_snapshot_id: configSnapshotId },
        parse: (payload) => configSnapshotResponseSchema.parse(payload)
      })
      return parsed.config_snapshot
    },
    async activateConfigSnapshot(configSnapshotId) {
      const operation = resolveOperation('activateConfigSnapshot')
      const parsed = await requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { config_snapshot_id: configSnapshotId },
        parse: (payload) => configSnapshotResponseSchema.parse(payload)
      })
      return parsed.config_snapshot
    },
    async simulateRoute(request) {
      const operation = resolveOperation('simulateRoute')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: routeSimulationRequestSchema.parse(request),
        parse: (payload) => routeSimulationResponseSchema.parse(payload)
      })
    },
    async getRouteReceipt(routeReceiptId) {
      const operation = resolveOperation('getRouteReceipt')
      const parsed = await requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { route_receipt_id: routeReceiptId },
        parse: (payload) => routeReceiptResponseSchema.parse(payload)
      })
      return parsed.route_receipt
    }
  }
}

export const createGatewayClient = (
  options: ClientOptions = {
    baseUrl: ''
  }
): GatewayClient => {
  const fetchImpl = options.fetch ?? defaultFetch()

  return {
    contractVersion: CONTRACT_VERSION,
    contractDigest: CONTRACT_DIGEST,
    async createChatCompletion(request) {
      const operation = resolveGatewayOperation('createChatCompletion')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: gatewayChatRequestSchema.parse(request),
        parse: (payload) => gatewayChatResponseSchema.parse(payload)
      })
    }
  }
}
