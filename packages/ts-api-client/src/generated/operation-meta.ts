export const CONTRACT_VERSION = "v1" as const;
export const CONTRACT_DIGEST = "c8fc46c752b6154769faefb25da8e50113880cd2c49150920741e5ad45bc5a54" as const;
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
{ id: 'getRouteReceiptDiagnostics', method: 'GET', path: '/v1/route-receipts/{route_receipt_id}/diagnostics' },
] as const;
export const GATEWAY_OPERATIONS = [
{ id: 'createChatCompletion', method: 'POST', path: '/v1/chat/completions' },
{ id: 'createAnthropicMessages', method: 'POST', path: '/v1/messages' },
{ id: 'createGeminiGenerateContent', method: 'POST', path: '/v1beta/models/{model}:generateContent' },
] as const;
