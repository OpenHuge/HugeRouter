import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import {
  ContractApiError,
  createControlPlaneClient,
  createGatewayClient
} from './index.ts'

const readJson = (relativePath: string) =>
  JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), 'utf8')
  ) as Record<string, unknown>

const jsonResponse = (status: number, payload: unknown) =>
  new Response(JSON.stringify(payload), {
    status,
    headers: {
      'content-type': 'application/json'
    }
  })

const resolveRequestUrl = (input: RequestInfo | URL) =>
  typeof input === 'string'
    ? input
    : input instanceof URL
      ? input.toString()
      : input.url

void test('generated operation metadata stays in sync with the schema manifest', () => {
  const manifest = readJson('../../../schemas/jsonschema/contracts.manifest.json')
  const client = createControlPlaneClient({
    baseUrl: 'https://api.example.test'
  })

  assert.equal(manifest.contract_digest, client.contractDigest)
})

void test('control-plane client resolves the documented endpoints and parses responses', async () => {
  const tenantsResponse = readJson('../../../schemas/examples/control-plane/tenants.response.json')
  const projectsResponse = readJson('../../../schemas/examples/control-plane/projects.response.json')
  const providerResourcesResponse = readJson(
    '../../../schemas/examples/control-plane/provider-resources.response.json'
  )
  const routeReceiptResponse = readJson(
    '../../../schemas/examples/control-plane/route-receipt.response.json'
  )

  const calls: Array<{ url: string; method?: string }> = []
  const fetchImpl = (input: RequestInfo | URL, init?: RequestInit) => {
    const url = resolveRequestUrl(input)
    calls.push({ url, method: init?.method })

    if (url.endsWith('/v1/tenants')) {
      return Promise.resolve(jsonResponse(200, tenantsResponse))
    }

    if (url.endsWith('/v1/projects')) {
      return Promise.resolve(jsonResponse(200, projectsResponse))
    }

    if (url.endsWith('/v1/provider-resources')) {
      return Promise.resolve(jsonResponse(200, providerResourcesResponse))
    }

    if (url.endsWith('/v1/route-receipts/routercpt_123')) {
      return Promise.resolve(jsonResponse(200, routeReceiptResponse))
    }
    return Promise.resolve(
      jsonResponse(404, readJson('../../../schemas/examples/gateway/error.response.json'))
    )
  }

  const client = createControlPlaneClient({
    baseUrl: 'https://api.example.test',
    fetch: fetchImpl
  })

  const tenants = await client.listTenants()
  const projects = await client.listProjects()
  const resources = await client.listProviderResources()
  const receipt = await client.getRouteReceipt('routercpt_123')

  assert.equal(tenants[0]?.tenant_id, 'tenant_acme')
  assert.equal(projects[0]?.project_id, 'proj_core')
  assert.equal(resources[0]?.provider_resource_id, 'prvrsrc_openai_primary')
  assert.equal(receipt.route_receipt_id, 'routercpt_123')
  assert.deepEqual(calls, [
    {
      url: 'https://api.example.test/v1/tenants',
      method: 'GET'
    },
    {
      url: 'https://api.example.test/v1/projects',
      method: 'GET'
    },
    {
      url: 'https://api.example.test/v1/provider-resources',
      method: 'GET'
    },
    {
      url: 'https://api.example.test/v1/route-receipts/routercpt_123',
      method: 'GET'
    }
  ])
})

void test('gateway client validates requests and normalizes contract errors', async () => {
  const gatewayRequest = readJson('../../../schemas/examples/gateway/chat.request.json')
  const gatewayResponse = readJson('../../../schemas/examples/gateway/chat.response.json')
  const errorResponse = readJson('../../../schemas/examples/gateway/error.response.json')

  const client = createGatewayClient({
    baseUrl: 'https://gateway.example.test',
    fetch: () => Promise.resolve(jsonResponse(200, gatewayResponse))
  })

  const response = await client.createChatCompletion(gatewayRequest as never)
  assert.equal(response.provider_response_id, 'resp_openai_123')

  const failingClient = createGatewayClient({
    baseUrl: 'https://gateway.example.test',
    fetch: () => Promise.resolve(jsonResponse(422, errorResponse))
  })

  await assert.rejects(
    () => failingClient.createChatCompletion(gatewayRequest as never),
    (error: unknown) => {
      assert.ok(error instanceof ContractApiError)
      assert.equal(error.status, 422)
      assert.equal(error.envelope.error.code, 'validation_failed')
      return true
    }
  )
})
