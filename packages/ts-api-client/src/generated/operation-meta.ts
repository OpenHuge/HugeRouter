export const CONTRACT_VERSION = "v1" as const;
export const CONTRACT_DIGEST = "7d8615b5d104f0c34971ad74d2026f4e9baac0a75f05778986c5abd9176b23b3" as const;
export const CONTROL_PLANE_OPERATIONS = [
{ id: 'listTenants', method: 'GET', path: '/v1/tenants' },
{ id: 'listProjects', method: 'GET', path: '/v1/projects' },
{ id: 'listProviderResources', method: 'GET', path: '/v1/provider-resources' },
{ id: 'getProviderResource', method: 'GET', path: '/v1/provider-resources/{provider_resource_id}' },
{ id: 'listRoutePolicies', method: 'GET', path: '/v1/route-policies' },
{ id: 'getConfigSnapshot', method: 'GET', path: '/v1/config-snapshots/{config_snapshot_id}' },
{ id: 'activateConfigSnapshot', method: 'POST', path: '/v1/config-snapshots/{config_snapshot_id}/activate' },
{ id: 'simulateRoute', method: 'POST', path: '/v1/route-simulations' },
{ id: 'listRouteReceipts', method: 'GET', path: '/v1/route-receipts' },
{ id: 'getRouteReceipt', method: 'GET', path: '/v1/route-receipts/{route_receipt_id}' },
{ id: 'getRouteDiagnostics', method: 'GET', path: '/v1/route-diagnostics/{route_policy_id}' },
] as const;
export const GATEWAY_OPERATIONS = [
{ id: 'createChatCompletion', method: 'POST', path: '/v1/gateway/chat/completions' },
] as const;
