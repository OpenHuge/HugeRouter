import {
  authLoginResultSchema,
  authProviderLinksResponseSchema,
  authProvidersResponseSchema,
  authSessionResponseSchema,
  configSnapshotResponseSchema,
  errorEnvelopeSchema,
  emailLoginCompleteRequestSchema,
  emailLoginStartRequestSchema,
  emailLoginStartResponseSchema,
  gatewayChatRequestSchema,
  gatewayChatResponseSchema,
  logoutResponseSchema,
  oauthCallbackRequestSchema,
  oauthLoginStartRequestSchema,
  oauthLoginStartResponseSchema,
  oauthProviderSchema,
  projectsResponseSchema,
  providerResourceSchema,
  providerResourcesResponseSchema,
  routeDiagnosticsResponseSchema,
  routePoliciesResponseSchema,
  routeReceiptResponseSchema,
  routeReceiptsResponseSchema,
  routeSimulationRequestSchema,
  routeSimulationResponseSchema,
  tenantsResponseSchema,
  unlinkAuthProviderResponseSchema,
  type AuthLoginResult,
  type AuthProviderLinksResponse,
  type AuthProvidersResponse,
  type AuthSessionResponse,
  type EmailLoginCompleteRequest,
  type EmailLoginStartRequest,
  type EmailLoginStartResponse,
  type ErrorEnvelope,
  type GatewayChatRequest,
  type GatewayChatResponse,
  type LogoutResponse,
  type OAuthCallbackRequest,
  type OAuthLoginStartRequest,
  type OAuthLoginStartResponse,
  type OAuthProvider,
  type Project,
  type ProviderResource,
  type RouteDiagnosticsResponse,
  type RoutePolicy,
  type RouteReceipt,
  type RouteSimulationRequest,
  type RouteSimulationResponse,
  type Tenant,
} from "@huge-router/ts-shared-schema";
import {
  CONTRACT_DIGEST,
  CONTRACT_VERSION,
  CONTROL_PLANE_OPERATIONS,
  GATEWAY_OPERATIONS,
} from "./generated/operation-meta.ts";

type FetchLike = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;
type OperationId = (typeof CONTROL_PLANE_OPERATIONS)[number]["id"];
type GatewayOperationId = (typeof GATEWAY_OPERATIONS)[number]["id"];

export class ContractApiError extends Error {
  readonly status: number;
  readonly envelope: ErrorEnvelope;

  constructor(status: number, envelope: ErrorEnvelope) {
    super(envelope.error.message);
    this.status = status;
    this.envelope = envelope;
  }
}

export class ControlPlaneClientError extends Error {
  readonly code?: string;
  readonly meta: {
    requestId?: string;
    traceId?: string;
  };
  readonly status: number;

  constructor(
    message: string,
    status: number,
    options: {
      code?: string;
      meta?: {
        requestId?: string;
        traceId?: string;
      };
    } = {},
  ) {
    super(message);
    this.code = options.code;
    this.meta = options.meta ?? {};
    this.name = "ControlPlaneClientError";
    this.status = status;
  }
}

export type ClientOptions = {
  baseUrl: string;
  fetch?: FetchLike;
  headers?: HeadersInit;
};

export type ControlPlaneClient = {
  readonly contractVersion: typeof CONTRACT_VERSION;
  readonly contractDigest: typeof CONTRACT_DIGEST;
  listTenants: () => Promise<Tenant[]>;
  listProjects: () => Promise<Project[]>;
  getAuthProviders: () => Promise<AuthProvidersResponse>;
  startEmailLogin: (
    input: EmailLoginStartRequest,
  ) => Promise<EmailLoginStartResponse>;
  completeEmailLogin: (
    input: EmailLoginCompleteRequest,
  ) => Promise<AuthLoginResult>;
  startOAuthLogin: (
    provider: OAuthProvider,
    input: OAuthLoginStartRequest,
  ) => Promise<OAuthLoginStartResponse>;
  completeOAuthLogin: (
    provider: OAuthProvider,
    input: OAuthCallbackRequest,
  ) => Promise<AuthLoginResult>;
  getCurrentSession: () => Promise<AuthSessionResponse>;
  logout: () => Promise<LogoutResponse>;
  listAuthProviderLinks: () => Promise<AuthProviderLinksResponse>;
  unlinkAuthProvider: (
    provider: OAuthProvider | "email",
  ) => Promise<ReturnType<typeof unlinkAuthProviderResponseSchema.parse>>;
  listProviderResources: (
    filters?: ProviderResourceListFilters,
  ) => Promise<ProviderResource[]>;
  getProviderResource: (
    providerResourceId: string,
  ) => Promise<ProviderResource>;
  listRoutePolicies: () => Promise<RoutePolicy[]>;
  getConfigSnapshot: (
    configSnapshotId: string,
  ) => Promise<
    ReturnType<typeof configSnapshotResponseSchema.parse>["config_snapshot"]
  >;
  activateConfigSnapshot: (
    configSnapshotId: string,
  ) => Promise<
    ReturnType<typeof configSnapshotResponseSchema.parse>["config_snapshot"]
  >;
  simulateRoute: (
    request: RouteSimulationRequest,
  ) => Promise<RouteSimulationResponse>;
  listRouteReceipts: (
    filters?: RouteReceiptListFilters,
  ) => Promise<RouteReceipt[]>;
  getRouteReceipt: (routeReceiptId: string) => Promise<RouteReceipt>;
  getRouteDiagnostics: (
    routePolicyId: string,
  ) => Promise<RouteDiagnosticsResponse>;
};

export type ProviderResourceListFilters = {
  tenantId?: string;
  healthState?: string;
  protocolFamily?: string;
  capability?: string;
  transitGateway?: boolean;
  quarantined?: boolean;
};

export type RouteReceiptListFilters = {
  tenantId?: string;
  projectId?: string;
  routePolicyId?: string;
  providerResourceId?: string;
  admissionResult?: string;
  limit?: number;
};

export type GatewayClient = {
  readonly contractVersion: typeof CONTRACT_VERSION;
  readonly contractDigest: typeof CONTRACT_DIGEST;
  createChatCompletion: (
    request: GatewayChatRequest,
  ) => Promise<GatewayChatResponse>;
};

const resolveOperation = (id: OperationId) => {
  const operation = CONTROL_PLANE_OPERATIONS.find(
    (candidate) => candidate.id === id,
  );
  if (!operation) {
    throw new Error(`Missing control-plane operation metadata for ${id}`);
  }
  return operation;
};

const resolveGatewayOperation = (id: GatewayOperationId) => {
  const operation = GATEWAY_OPERATIONS.find((candidate) => candidate.id === id);
  if (!operation) {
    throw new Error(`Missing gateway operation metadata for ${id}`);
  }
  return operation;
};

const buildUrl = (
  baseUrl: string,
  pathTemplate: string,
  params?: Record<string, string>,
  query?: Record<string, string | number | boolean | undefined>,
) => {
  const path = Object.entries(params ?? {}).reduce(
    (currentPath, [key, value]) =>
      currentPath.replace(`{${key}}`, encodeURIComponent(value)),
    pathTemplate,
  );
  const search = new URLSearchParams();

  for (const [key, value] of Object.entries(query ?? {})) {
    if (value === undefined) {
      continue;
    }
    search.set(key, String(value));
  }

  if (!baseUrl) {
    return search.size > 0 ? `${path}?${search.toString()}` : path;
  }

  const url = new URL(path, ensureTrailingSlash(baseUrl));
  if (search.size > 0) {
    url.search = search.toString();
  }
  return url.toString();
};

const ensureTrailingSlash = (baseUrl: string) =>
  baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`;

const defaultFetch = (): FetchLike => {
  if (!globalThis.fetch) {
    throw new Error("No fetch implementation was provided to the API client");
  }

  return globalThis.fetch.bind(globalThis);
};

const requestJson = async <T>({
  baseUrl,
  fetchImpl,
  headers,
  method,
  path,
  params,
  query,
  body,
  parse,
}: {
  baseUrl: string;
  fetchImpl: FetchLike;
  headers?: HeadersInit;
  method: string;
  path: string;
  params?: Record<string, string>;
  query?: Record<string, string | number | boolean | undefined>;
  body?: unknown;
  parse: (payload: unknown) => T;
}) => {
  const response = await fetchImpl(buildUrl(baseUrl, path, params, query), {
    method,
    headers: {
      Accept: "application/json",
      ...(body ? { "Content-Type": "application/json" } : {}),
      ...(headers ?? {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });

  const payload: unknown = await response.json();

  if (!response.ok) {
    throw new ContractApiError(
      response.status,
      errorEnvelopeSchema.parse(payload),
    );
  }

  return parse(payload);
};

const requestAuthJson = async <T>({
  baseUrl,
  fetchImpl,
  headers,
  method,
  path,
  body,
  parse,
}: {
  baseUrl: string;
  fetchImpl: FetchLike;
  headers?: HeadersInit;
  method: string;
  path: string;
  body?: unknown;
  parse: (payload: unknown) => T;
}) => {
  const response = await fetchImpl(buildUrl(baseUrl, path), {
    body: body ? JSON.stringify(body) : undefined,
    credentials: "include",
    method,
    headers: {
      Accept: "application/json",
      ...(body ? { "Content-Type": "application/json" } : {}),
      ...(headers ?? {}),
    },
  });

  const payload: unknown = await response.json();

  if (!response.ok) {
    const errorPayload = payload as
      | {
          code?: string;
          error?: {
            code?: string;
            message?: string;
            requestId?: string;
            traceId?: string;
          };
          message?: string;
          requestId?: string;
          traceId?: string;
        }
      | undefined;

    throw new ControlPlaneClientError(
      errorPayload?.error?.message ??
        errorPayload?.message ??
        `Control plane auth request failed with status ${response.status}`,
      response.status,
      {
        code: errorPayload?.error?.code ?? errorPayload?.code,
        meta: {
          requestId: errorPayload?.error?.requestId ?? errorPayload?.requestId,
          traceId: errorPayload?.error?.traceId ?? errorPayload?.traceId,
        },
      },
    );
  }

  return parse(payload);
};

export const createControlPlaneClient = (
  options: ClientOptions = {
    baseUrl: "",
  },
): ControlPlaneClient => {
  const fetchImpl = options.fetch ?? defaultFetch();

  return {
    contractVersion: CONTRACT_VERSION,
    contractDigest: CONTRACT_DIGEST,
    async listTenants() {
      const operation = resolveOperation("listTenants");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => tenantsResponseSchema.parse(payload).data,
      });
    },
    async listProjects() {
      const operation = resolveOperation("listProjects");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => projectsResponseSchema.parse(payload).data,
      });
    },
    async getAuthProviders() {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "GET",
        path: "/api/control-plane/auth/providers",
        parse: (payload) => authProvidersResponseSchema.parse(payload),
      });
    },
    async startEmailLogin(input) {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "POST",
        path: "/api/control-plane/auth/email/start",
        body: emailLoginStartRequestSchema.parse(input),
        parse: (payload) => emailLoginStartResponseSchema.parse(payload),
      });
    },
    async completeEmailLogin(input) {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "POST",
        path: "/api/control-plane/auth/email/complete",
        body: emailLoginCompleteRequestSchema.parse(input),
        parse: (payload) => authLoginResultSchema.parse(payload),
      });
    },
    async startOAuthLogin(provider, input) {
      const parsedProvider = oauthProviderSchema.parse(provider);
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "POST",
        path: `/api/control-plane/auth/oauth/${parsedProvider}/start`,
        body: oauthLoginStartRequestSchema.parse(input),
        parse: (payload) => oauthLoginStartResponseSchema.parse(payload),
      });
    },
    async completeOAuthLogin(provider, input) {
      const parsedProvider = oauthProviderSchema.parse(provider);
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "POST",
        path: `/api/control-plane/auth/oauth/${parsedProvider}/callback`,
        body: oauthCallbackRequestSchema.parse(input),
        parse: (payload) => authLoginResultSchema.parse(payload),
      });
    },
    async getCurrentSession() {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "GET",
        path: "/api/control-plane/auth/session",
        parse: (payload) => authSessionResponseSchema.parse(payload),
      });
    },
    async logout() {
      return requestAuthJson({
        baseUrl: options.baseUrl ?? "",
        fetchImpl,
        headers: options.headers,
        method: "POST",
        path: "/api/control-plane/auth/logout",
        parse: (payload) => logoutResponseSchema.parse(payload),
      });
    },
    async listAuthProviderLinks() {
      return requestAuthJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: "GET",
        path: "/api/control-plane/auth/links",
        parse: (payload) => authProviderLinksResponseSchema.parse(payload),
      });
    },
    async unlinkAuthProvider(provider) {
      const pathProvider =
        provider === "email" ? "email" : oauthProviderSchema.parse(provider);
      return requestAuthJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: "DELETE",
        path: `/api/control-plane/auth/links/${pathProvider}`,
        parse: (payload) => unlinkAuthProviderResponseSchema.parse(payload),
      });
    },
    async listProviderResources(filters) {
      const operation = resolveOperation("listProviderResources");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        query: {
          tenant_id: filters?.tenantId,
          health_state: filters?.healthState,
          protocol_family: filters?.protocolFamily,
          capability: filters?.capability,
          transit_gateway: filters?.transitGateway,
          quarantined: filters?.quarantined,
        },
        parse: (payload) => providerResourcesResponseSchema.parse(payload).data,
      });
    },
    async getProviderResource(providerResourceId) {
      const operation = resolveOperation("getProviderResource");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { provider_resource_id: providerResourceId },
        parse: (payload) => providerResourceSchema.parse(payload),
      });
    },
    async listRoutePolicies() {
      const operation = resolveOperation("listRoutePolicies");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        parse: (payload) => routePoliciesResponseSchema.parse(payload).data,
      });
    },
    async getConfigSnapshot(configSnapshotId) {
      const operation = resolveOperation("getConfigSnapshot");
      const parsed = await requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { config_snapshot_id: configSnapshotId },
        parse: (payload) => configSnapshotResponseSchema.parse(payload),
      });
      return parsed.config_snapshot;
    },
    async activateConfigSnapshot(configSnapshotId) {
      const operation = resolveOperation("activateConfigSnapshot");
      const parsed = await requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { config_snapshot_id: configSnapshotId },
        parse: (payload) => configSnapshotResponseSchema.parse(payload),
      });
      return parsed.config_snapshot;
    },
    async simulateRoute(request) {
      const operation = resolveOperation("simulateRoute");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: routeSimulationRequestSchema.parse(request),
        parse: (payload) => routeSimulationResponseSchema.parse(payload),
      });
    },
    async listRouteReceipts(filters) {
      const operation = resolveOperation("listRouteReceipts");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        query: {
          tenant_id: filters?.tenantId,
          project_id: filters?.projectId,
          route_policy_id: filters?.routePolicyId,
          provider_resource_id: filters?.providerResourceId,
          admission_result: filters?.admissionResult,
          limit: filters?.limit,
        },
        parse: (payload) => routeReceiptsResponseSchema.parse(payload).data,
      });
    },
    async getRouteReceipt(routeReceiptId) {
      const operation = resolveOperation("getRouteReceipt");
      const parsed = await requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { route_receipt_id: routeReceiptId },
        parse: (payload) => routeReceiptResponseSchema.parse(payload),
      });
      return parsed.route_receipt;
    },
    async getRouteDiagnostics(routePolicyId) {
      const operation = resolveOperation("getRouteDiagnostics");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        params: { route_policy_id: routePolicyId },
        parse: (payload) => routeDiagnosticsResponseSchema.parse(payload),
      });
    },
  };
};

export const createGatewayClient = (
  options: ClientOptions = {
    baseUrl: "",
  },
): GatewayClient => {
  const fetchImpl = options.fetch ?? defaultFetch();

  return {
    contractVersion: CONTRACT_VERSION,
    contractDigest: CONTRACT_DIGEST,
    async createChatCompletion(request) {
      const operation = resolveGatewayOperation("createChatCompletion");
      return requestJson({
        baseUrl: options.baseUrl,
        fetchImpl,
        headers: options.headers,
        method: operation.method,
        path: operation.path,
        body: gatewayChatRequestSchema.parse(request),
        parse: (payload) => gatewayChatResponseSchema.parse(payload),
      });
    },
  };
};

const expectType = <Expected>(value: Expected) => value;

void expectType<Project[]>(
  [] as Awaited<ReturnType<ControlPlaneClient["listProjects"]>>,
);
void expectType<Tenant[]>(
  [] as Awaited<ReturnType<ControlPlaneClient["listTenants"]>>,
);
void expectType<ProviderResource[]>(
  [] as Awaited<ReturnType<ControlPlaneClient["listProviderResources"]>>,
);
void expectType<RoutePolicy[]>(
  [] as Awaited<ReturnType<ControlPlaneClient["listRoutePolicies"]>>,
);
void expectType<GatewayChatRequest>(
  {} as Parameters<GatewayClient["createChatCompletion"]>[0],
);
