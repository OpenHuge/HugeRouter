import { beforeEach, afterEach, vi } from 'vitest'

const tenantsResponse = {
  data: [
    {
      tenant_id: 'tenant_platform',
      slug: 'platform-admin',
      display_name: 'Platform Admin',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      tenant_id: 'tenant_acme',
      slug: 'acme-retail',
      display_name: 'Acme Retail',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      tenant_id: 'tenant_northstar',
      slug: 'northstar-labs',
      display_name: 'Northstar Labs',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    }
  ]
}

const projectsResponse = {
  data: [
    {
      project_id: 'proj_core',
      tenant_id: 'tenant_acme',
      slug: 'core-gateway',
      display_name: 'Core Gateway',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      project_id: 'proj_acme_ops',
      tenant_id: 'tenant_acme',
      slug: 'retail-ops',
      display_name: 'Retail Operations Control',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      project_id: 'proj_acme_support',
      tenant_id: 'tenant_acme',
      slug: 'store-support',
      display_name: 'Store Support Agent',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      project_id: 'proj_ns_research',
      tenant_id: 'tenant_northstar',
      slug: 'research-qa',
      display_name: 'Research QA',
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    }
  ]
}

const providerResourcesResponse = {
  data: [
    {
      provider_resource_id: 'prvrsrc_openai_primary',
      tenant_id: 'tenant_acme',
      project_id: 'proj_core',
      provider_id: 'openai',
      name: 'OpenAI Primary',
      status: 'active',
      provenance_class: 'official_api',
      credential_owner_type: 'platform',
      deployment_scope: 'shared',
      region: 'us-east-1',
      endpoint_base_url: 'https://api.openai.com/v1',
      auth_kind: 'api_key',
      health_state: 'healthy',
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: true
      },
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      provider_resource_id: 'prvrsrc_openai_backup',
      tenant_id: 'tenant_acme',
      project_id: 'proj_core',
      provider_id: 'openai',
      name: 'OpenAI Backup',
      status: 'active',
      provenance_class: 'official_api',
      credential_owner_type: 'platform',
      deployment_scope: 'shared',
      region: 'us-west-2',
      endpoint_base_url: 'https://api.openai.com/v1',
      auth_kind: 'api_key',
      health_state: 'healthy',
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: true,
        supports_json_mode: true
      },
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      provider_resource_id: 'prvrsrc_openai_research',
      tenant_id: 'tenant_northstar',
      project_id: 'proj_ns_research',
      provider_id: 'openai',
      name: 'OpenAI Research',
      status: 'active',
      provenance_class: 'official_api',
      credential_owner_type: 'tenant',
      deployment_scope: 'tenant_dedicated',
      region: 'us-west-2',
      endpoint_base_url: 'https://api.openai.com/v1',
      auth_kind: 'api_key',
      health_state: 'degraded',
      capabilities: {
        supports_streaming: true,
        supports_tool_calling: false,
        supports_json_mode: true
      },
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    }
  ]
}

const routePoliciesResponse = {
  data: [
    {
      route_policy_id: 'routepol_openai_chat_default',
      tenant_id: 'tenant_acme',
      display_name: 'Acme Reasoning Fast',
      protocol_family: 'openai_chat',
      model_alias: 'reasoning-fast',
      required_capabilities: ['json_mode'],
      preferred_regions: ['us-east-1'],
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      route_policy_id: 'routepol_acme_support',
      tenant_id: 'tenant_acme',
      display_name: 'Acme Support Safe',
      protocol_family: 'openai_chat',
      model_alias: 'support-safe',
      required_capabilities: ['json_mode'],
      preferred_regions: ['us-west-2'],
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    },
    {
      route_policy_id: 'routepol_northstar_research',
      tenant_id: 'tenant_northstar',
      display_name: 'Northstar Research',
      protocol_family: 'openai_chat',
      model_alias: 'research-fast',
      required_capabilities: ['json_mode'],
      preferred_regions: ['us-west-2'],
      version: 1,
      created_at: '2026-04-22T00:00:00Z',
      updated_at: '2026-04-22T00:00:00Z'
    }
  ]
}

const configSnapshotResponse = {
  config_snapshot: {
    config_snapshot_id: 'cfgsnap_gateway_v1',
    tenant_id: 'tenant_acme',
    project_id: 'proj_core',
    revision: 1,
    status: 'active',
    activated_at: '2026-04-22T00:00:00Z',
    provider_resource_ids: ['prvrsrc_openai_primary', 'prvrsrc_openai_backup'],
    route_policy_id: 'routepol_openai_chat_default',
    budget_policy_id: 'budgetpol_default'
  }
}

const routeSimulationResponse = {
  simulation_id: 'sim_reasoning-fast',
  config_snapshot_id: 'cfgsnap_gateway_v1',
  admission_result: 'admitted',
  eligible_candidates: [
    {
      provider_resource_id: 'prvrsrc_openai_primary',
      score_breakdown: {
        latency: 0.95,
        cost: 0.7,
        health: 1,
        trust: 1
      }
    }
  ],
  excluded_candidates: [],
  selected_target: 'prvrsrc_openai_primary',
  estimated_cost: {
    currency: 'USD',
    amount: '0.000210'
  }
}

function jsonResponse(status: number, payload: unknown) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      'content-type': 'application/json'
    }
  })
}

function resolvePath(input: RequestInfo | URL) {
  if (typeof input === 'string') {
    return input
  }

  if (input instanceof URL) {
    return input.pathname
  }

  return new URL(input.url, 'http://127.0.0.1').pathname
}

export function createControlPlaneFetchMock() {
  return vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const path = resolvePath(input)

    if (path === '/v1/tenants') {
      return Promise.resolve(jsonResponse(200, tenantsResponse))
    }

    if (path === '/v1/projects') {
      return Promise.resolve(jsonResponse(200, projectsResponse))
    }

    if (path === '/v1/provider-resources') {
      return Promise.resolve(jsonResponse(200, providerResourcesResponse))
    }

    if (path === '/v1/route-policies') {
      return Promise.resolve(jsonResponse(200, routePoliciesResponse))
    }

    if (path === '/v1/config-snapshots/active') {
      return Promise.resolve(jsonResponse(200, configSnapshotResponse))
    }

    if (path === '/v1/route-simulations' && init?.method === 'POST') {
      return Promise.resolve(jsonResponse(200, routeSimulationResponse))
    }

    return Promise.resolve(
      jsonResponse(404, {
        error: {
          code: 'not_found',
          message: `Unhandled test path ${path}`,
          request_id: 'req_test',
          retryable: false
        }
      })
    )
  })
}

export function useControlPlaneFetchMock() {
  let fetchMock: ReturnType<typeof createControlPlaneFetchMock>

  beforeEach(() => {
    fetchMock = createControlPlaneFetchMock()
    vi.stubGlobal('fetch', fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  return () => fetchMock
}
