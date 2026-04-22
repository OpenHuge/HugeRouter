import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import {
  configSnapshotResponseSchema,
  contractDigest,
  gatewayChatRequestSchema,
  gatewayChatResponseSchema,
  projectsResponseSchema,
  providerResourcesResponseSchema,
  routePoliciesResponseSchema,
  routeSimulationRequestSchema,
  routeSimulationResponseSchema,
  tenantsResponseSchema,
  usageEventRecordedMessageSchema
} from './index.ts'

const readJson = (relativePath: string) =>
  JSON.parse(
    readFileSync(new URL(relativePath, import.meta.url), 'utf8')
  ) as Record<string, unknown>

void test('checked-in schema manifest stays in sync with generated package metadata', () => {
  const manifest = readJson('../../../schemas/jsonschema/contracts.manifest.json')

  assert.equal(manifest.contract_digest, contractDigest)
})

void test('control-plane examples conform to shared zod schemas', () => {
  const tenants = readJson('../../../schemas/examples/control-plane/tenants.response.json')
  const projects = readJson('../../../schemas/examples/control-plane/projects.response.json')
  const resources = readJson(
    '../../../schemas/examples/control-plane/provider-resources.response.json'
  )
  const routePolicies = readJson(
    '../../../schemas/examples/control-plane/route-policies.response.json'
  )
  const snapshot = readJson(
    '../../../schemas/examples/control-plane/config-snapshot.response.json'
  )
  const simulationRequest = readJson(
    '../../../schemas/examples/control-plane/route-simulation.request.json'
  )
  const simulationResponse = readJson(
    '../../../schemas/examples/control-plane/route-simulation.response.json'
  )

  assert.equal(tenantsResponseSchema.parse(tenants).data[0]?.tenant_id, 'tenant_acme')
  assert.equal(projectsResponseSchema.parse(projects).data[0]?.project_id, 'proj_core')
  assert.equal(
    providerResourcesResponseSchema.parse(resources).data[0]?.provider_id,
    'openai'
  )
  assert.equal(
    routePoliciesResponseSchema.parse(routePolicies).data[0]?.route_policy_id,
    'routepol_default'
  )
  assert.equal(
    configSnapshotResponseSchema.parse(snapshot).config_snapshot.config_snapshot_id,
    'cfgsnap_default'
  )
  assert.equal(
    routeSimulationRequestSchema.parse(simulationRequest).protocol_family,
    'openai_chat'
  )
  assert.equal(
    routeSimulationResponseSchema.parse(simulationResponse).selected_target,
    'prvrsrc_openai_primary'
  )
})

void test('gateway and event examples conform to shared zod schemas', () => {
  const gatewayRequest = readJson('../../../schemas/examples/gateway/chat.request.json')
  const gatewayResponse = readJson('../../../schemas/examples/gateway/chat.response.json')
  const usageMessage = readJson(
    '../../../schemas/examples/events/usage-event-recorded.message.json'
  )

  assert.equal(
    gatewayChatRequestSchema.parse(gatewayRequest).request.protocol_family,
    'openai_chat'
  )
  assert.equal(
    gatewayChatResponseSchema.parse(gatewayResponse).usage_event.phase,
    'final'
  )
  assert.equal(
    usageEventRecordedMessageSchema.parse(usageMessage).message_type,
    'usage_event.recorded'
  )
})
