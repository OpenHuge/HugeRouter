import {
  authLoginResultSchema,
  authProviderLinksResponseSchema,
  authProvidersResponseSchema,
  authSessionResponseSchema,
  balanceProjectionResponseSchema,
  billingExportJobsResponseSchema,
  billingExportJobResponseSchema,
  billingExportRequestSchema,
  configSnapshotResponseSchema,
  errorEnvelopeSchema,
  emailLoginCompleteRequestSchema,
  emailLoginStartRequestSchema,
  emailLoginStartResponseSchema,
  gatewayAnthropicMessagesRequestSchema,
  gatewayAnthropicMessagesResponseSchema,
  gatewayChatRequestSchema,
  gatewayChatResponseSchema,
  gatewayGeminiGenerateContentRequestSchema,
  gatewayGeminiGenerateContentResponseSchema,
  logoutResponseSchema,
  oauthCallbackRequestSchema,
  oauthLoginStartRequestSchema,
  oauthLoginStartResponseSchema,
  oauthProviderSchema,
  pricingSimulationRequestSchema,
  pricingSimulationResponseSchema,
  pricingCatalogResponseSchema,
  projectsResponseSchema,
  providerResourceSchema,
  providerResourcesResponseSchema,
  routeDiagnosticsResponseSchema,
  routePoliciesResponseSchema,
  routeReceiptDiagnosticsResponseSchema,
  routeReceiptsResponseSchema,
  routeReceiptResponseSchema,
  routeSimulationRequestSchema,
  routeSimulationResponseSchema,
  tenantsResponseSchema,
  unlinkAuthProviderResponseSchema,
  usageBreakdownResponseSchema,
  usageSummaryResponseSchema,
  type AuthLoginResult,
  type AuthProviderLinksResponse,
  type AuthProvidersResponse,
  type AuthSessionResponse,
  type BalanceProjectionResponse,
  type BillingExportJobsResponse,
  type BillingExportJobResponse,
  type BillingExportRequest,
  type EmailLoginCompleteRequest,
  type EmailLoginStartRequest,
  type EmailLoginStartResponse,
  type ErrorEnvelope,
  type GatewayAnthropicMessagesRequest,
  type GatewayAnthropicMessagesResponse,
  type GatewayChatRequest,
  type GatewayChatResponse,
  type GatewayGeminiGenerateContentRequest,
  type GatewayGeminiGenerateContentResponse,
  type LogoutResponse,
  type OAuthCallbackRequest,
  type OAuthLoginStartRequest,
  type OAuthLoginStartResponse,
  type OAuthProvider,
  type PricingSimulationRequest,
  type PricingSimulationResponse,
  type PricingCatalogResponse,
  type Project,
  type ProviderResource,
  type RouteDiagnosticsResponse,
  type RoutePolicy,
  type RouteReceipt,
  type RouteReceiptDiagnosticsResponse,
  type RouteReceiptsResponse,
  type RouteSimulationRequest,
  type RouteSimulationResponse,
  type Tenant,
  type UsageBreakdownResponse,
  type UsageSummaryResponse
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

export type ControlPlaneClient = {
  readonly contractVersion: typeof CONTRACT_VERSION
  readonly contractDigest: typeof CONTRACT_DIGEST
  listTenants: () => Promise<Tenant[]>
  listProjects: () => Promise<Project[]>
  getAuthProviders: () => Promise<AuthProvidersResponse>
  startEmailLogin: (input: EmailLoginStartRequest) => Promise<EmailLoginStartResponse>
  completeEmailLogin: (input: EmailLoginCompleteRequest) => Promise<AuthLoginResult>
  startOAuthLogin: (
    provider: OAuthProvider,
    input: OAuthLoginStartRequest
  ) => Promise<OAuthLoginStartResponse>
  completeOAuthLogin: (
    provider: OAuthProvider,
    input: OAuthCallbackRequest
  ) => Promise<AuthLoginResult>
  getCurrentSession: () => Promise<AuthSessionResponse>
  logout: () => Promise<LogoutResponse>
  listAuthProviderLinks: () => Promise<AuthProviderLinksResponse>
  unlinkAuthProvider: (
    provider: OAuthProvider | 'email'
  ) => Promise<ReturnType<typeof unlinkAuthProviderResponseSchema.parse>>
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
  listRouteReceipts: () => Promise<RouteReceiptsResponse['data']>
  getRouteReceipt: (routeReceiptId: string) => Promise<RouteReceipt>
  getRouteDiagnostics: (routePolicyId: string) => Promise<RouteDiagnosticsResponse>
  getRouteReceiptDiagnostics: (
    routeReceiptId: string
  ) => Promise<RouteReceiptDiagnosticsResponse>
  getUsageSummary: (query: {
    tenant_id: string
    project_id?: string
    window_start?: string
    window_end?: string
  }) => Promise<UsageSummaryResponse>
  getUsageBreakdown: (query: {
    tenant_id: string
    project_id?: string
    window_start?: string
    window_end?: string
    group_by?: 'provider' | 'model' | 'day'
    cursor?: string
    limit?: number
  }) => Promise<UsageBreakdownResponse>
  getBalanceProjection: (query: {
    tenant_id: string
    project_id?: string
  }) => Promise<BalanceProjectionResponse>
  getPricingCatalog: () => Promise<PricingCatalogResponse>
  createPricingSimulation: (
    request: PricingSimulationRequest
  ) => Promise<PricingSimulationResponse>
  createBillingExport: (
    request: BillingExportRequest
  ) => Promise<BillingExportJobResponse>
  listBillingExports: (query?: {
    tenant_id?: string
    project_id?: string
  }) => Promise<BillingExportJobsResponse>
  getBillingExport: (exportJobId: string) => Promise<BillingExportJobResponse>
  downloadBillingExport: (exportJobId: string) => Promise<string>
}

export type GatewayClient = {
  readonly contractVersion: typeof CONTRACT_VERSION
  readonly contractDigest: typeof CONTRACT_DIGEST
  createChatCompletion: (
    request: GatewayChatRequest
  ) => Promise<GatewayChatResponse>
  createAnthropicMessages: (
    request: GatewayAnthropicMessagesRequest
  ) => Promise<GatewayAnthropicMessagesResponse>
  createGeminiGenerateContent: (
    request: GatewayGeminiGenerateContentRequest
  ) => Promise<GatewayGeminiGenerateContentResponse>
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

const requestAuthJson = async <T>({
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
    headers: {
      Accept: 'application/json',
      ...(body ? { 'Content-Type': 'application/json' } : {}),
      ...(headers ?? {})
    }
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
    async getAuthProviders() {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'GET',
        path: '/api/control-plane/auth/providers',
        parse: (payload) => authProvidersResponseSchema.parse(payload)
      })
    },
    async startEmailLogin(input) {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'POST',
        path: '/api/control-plane/auth/email/start',
        body: emailLoginStartRequestSchema.parse(input),
        parse: (payload) => emailLoginStartResponseSchema.parse(payload)
      })
    },
    async completeEmailLogin(input) {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'POST',
        path: '/api/control-plane/auth/email/complete',
        body: emailLoginCompleteRequestSchema.parse(input),
        parse: (payload) => authLoginResultSchema.parse(payload)
      })
    },
    async startOAuthLogin(provider, input) {
      const parsedProvider = oauthProviderSchema.parse(provider)
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'POST',
        path: `/api/control-plane/auth/oauth/${parsedProvider}/start`,
        body: oauthLoginStartRequestSchema.parse(input),
        parse: (payload) => oauthLoginStartResponseSchema.parse(payload)
      })
    },
    async completeOAuthLogin(provider, input) {
      const parsedProvider = oauthProviderSchema.parse(provider)
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'POST',
        path: `/api/control-plane/auth/oauth/${parsedProvider}/callback`,
        body: oauthCallbackRequestSchema.parse(input),
        parse: (payload) => authLoginResultSchema.parse(payload)
      })
    },
    async getCurrentSession() {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'GET',
        path: '/api/control-plane/auth/session',
        parse: (payload) => authSessionResponseSchema.parse(payload)
      })
    },
    async logout() {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? '',
        fetchImpl,
        headers: options.headers,
        method: 'POST',
        path: '/api/control-plane/auth/logout',
        parse: (payload) => logoutResponseSchema.parse(payload)
      })
    },
    async listAuthProviderLinks() {
      return requestAuthJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: 'GET',
        path: '/api/control-plane/auth/links',
        parse: (payload) => authProviderLinksResponseSchema.parse(payload)
      })
    },
    async unlinkAuthProvider(provider) {
      const pathProvider = provider === 'email' ? 'email' : oauthProviderSchema.parse(provider)
      return requestAuthJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: 'DELETE',
        path: `/api/control-plane/auth/links/${pathProvider}`,
        parse: (payload) => unlinkAuthProviderResponseSchema.parse(payload)
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
    async listRouteReceipts() {
      const operation = resolveOperation('listRouteReceipts')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => routeReceiptsResponseSchema.parse(payload).data
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
    },
    async getRouteDiagnostics(routePolicyId) {
      const operation = resolveOperation('getRouteDiagnostics')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { route_policy_id: routePolicyId },
        parse: (payload) => routeDiagnosticsResponseSchema.parse(payload)
      })
    },
    async getRouteReceiptDiagnostics(routeReceiptId) {
      const operation = resolveOperation('getRouteReceiptDiagnostics')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { route_receipt_id: routeReceiptId },
        parse: (payload) => routeReceiptDiagnosticsResponseSchema.parse(payload)
      })
    },
    async getUsageSummary(query) {
      const operation = resolveOperation('getUsageSummary')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        query,
        parse: (payload) => usageSummaryResponseSchema.parse(payload)
      })
    },
    async getUsageBreakdown(query) {
      const operation = resolveOperation('getUsageBreakdown')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        query,
        parse: (payload) => usageBreakdownResponseSchema.parse(payload)
      })
    },
    async getBalanceProjection(query) {
      const operation = resolveOperation('getBalanceProjection')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        query,
        parse: (payload) => balanceProjectionResponseSchema.parse(payload)
      })
    },
    async getPricingCatalog() {
      const operation = resolveOperation('getPricingCatalog')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => pricingCatalogResponseSchema.parse(payload)
      })
    },
    async createPricingSimulation(request) {
      const operation = resolveOperation('createPricingSimulation')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: pricingSimulationRequestSchema.parse(request),
        parse: (payload) => pricingSimulationResponseSchema.parse(payload)
      })
    },
    async createBillingExport(request) {
      const operation = resolveOperation('createBillingExport')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: billingExportRequestSchema.parse(request),
        parse: (payload) => billingExportJobResponseSchema.parse(payload)
      })
    },
    async listBillingExports(query = {}) {
      const operation = resolveOperation('listBillingExports')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        query,
        parse: (payload) => billingExportJobsResponseSchema.parse(payload)
      })
    },
    async getBillingExport(exportJobId) {
      const operation = resolveOperation('getBillingExport')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { export_job_id: exportJobId },
        parse: (payload) => billingExportJobResponseSchema.parse(payload)
      })
    },
    async downloadBillingExport(exportJobId) {
      const response = await fetchImpl(
        buildUrl(options.baseUrl, '/v1/billing/exports/{export_job_id}/download', {
          export_job_id: exportJobId
        }),
        {
          method: 'GET',
          headers: {
            Accept: 'text/csv, text/plain, */*',
            ...(options.headers ?? {})
          }
        }
      )

      if (!response.ok) {
        const payload: unknown = await response.json()
        throw new ContractApiError(response.status, errorEnvelopeSchema.parse(payload))
      }

      return response.text()
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
    },
    async createAnthropicMessages(request) {
      const operation = resolveGatewayOperation('createAnthropicMessages')
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: gatewayAnthropicMessagesRequestSchema.parse(request),
        parse: (payload) => gatewayAnthropicMessagesResponseSchema.parse(payload)
      })
    },
    async createGeminiGenerateContent(request) {
      const operation = resolveGatewayOperation('createGeminiGenerateContent')
      const parsedRequest = gatewayGeminiGenerateContentRequestSchema.parse(request)
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: {
          model: parsedRequest.model
        },
        body: parsedRequest,
        parse: (payload) => gatewayGeminiGenerateContentResponseSchema.parse(payload)
      })
    }
  }
}

const expectType = <Expected>(value: Expected) => value

void expectType<Project[]>([] as Awaited<ReturnType<ControlPlaneClient['listProjects']>>)
void expectType<Tenant[]>([] as Awaited<ReturnType<ControlPlaneClient['listTenants']>>)
void expectType<ProviderResource[]>(
  [] as Awaited<ReturnType<ControlPlaneClient['listProviderResources']>>
)
void expectType<RoutePolicy[]>(
  [] as Awaited<ReturnType<ControlPlaneClient['listRoutePolicies']>>
)
void expectType<RouteReceipt>(
  {} as Awaited<ReturnType<ControlPlaneClient['getRouteReceipt']>>
)
void expectType<RouteReceipt[]>(
  [] as Awaited<ReturnType<ControlPlaneClient['listRouteReceipts']>>
)
void expectType<RouteReceiptDiagnosticsResponse>(
  {} as Awaited<ReturnType<ControlPlaneClient['getRouteReceiptDiagnostics']>>
)
void expectType<PricingCatalogResponse>(
  {} as Awaited<ReturnType<ControlPlaneClient['getPricingCatalog']>>
)
void expectType<BillingExportJobsResponse>(
  {} as Awaited<ReturnType<ControlPlaneClient['listBillingExports']>>
)
void expectType<BillingExportJobResponse>(
  {} as Awaited<ReturnType<ControlPlaneClient['getBillingExport']>>
)
void expectType<GatewayAnthropicMessagesRequest>(
  {} as Parameters<GatewayClient['createAnthropicMessages']>[0]
)
void expectType<GatewayGeminiGenerateContentRequest>(
  {} as Parameters<GatewayClient['createGeminiGenerateContent']>[0]
)
void expectType<GatewayChatRequest>({} as Parameters<GatewayClient['createChatCompletion']>[0])
