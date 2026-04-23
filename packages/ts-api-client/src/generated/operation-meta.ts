export const CONTRACT_VERSION = "v1" as const;
export const CONTRACT_DIGEST = "f992dd64147e93f465e0d5a7701800bed08b1e1008fe649c6432ba0610739661" as const;
export const CONTROL_PLANE_OPERATIONS = [
{ id: 'listTenants', method: 'GET', path: '/v1/tenants' },
{ id: 'listProjects', method: 'GET', path: '/v1/projects' },
{ id: 'listProviderResources', method: 'GET', path: '/v1/provider-resources' },
{ id: 'getProviderResource', method: 'GET', path: '/v1/provider-resources/{provider_resource_id}' },
{ id: 'listRoutePolicies', method: 'GET', path: '/v1/route-policies' },
{ id: 'getConfigSnapshot', method: 'GET', path: '/v1/config-snapshots/{config_snapshot_id}' },
{ id: 'activateConfigSnapshot', method: 'POST', path: '/v1/config-snapshots/{config_snapshot_id}/activate' },
{ id: 'getUsageSummary', method: 'GET', path: '/v1/usage/summary' },
{ id: 'getUsageBreakdown', method: 'GET', path: '/v1/usage/breakdown' },
{ id: 'getBalanceProjection', method: 'GET', path: '/v1/billing/projection' },
{ id: 'getPricingCatalog', method: 'GET', path: '/v1/pricing/catalog' },
{ id: 'createPricingSimulation', method: 'POST', path: '/v1/pricing/simulations' },
{ id: 'createBillingExport', method: 'POST', path: '/v1/billing/exports' },
{ id: 'listBillingExports', method: 'GET', path: '/v1/billing/exports' },
{ id: 'getBillingExport', method: 'GET', path: '/v1/billing/exports/{export_job_id}' },
{ id: 'simulateRoute', method: 'POST', path: '/v1/route-simulations' },
{ id: 'listRouteReceipts', method: 'GET', path: '/v1/route-receipts' },
{ id: 'getRouteReceipt', method: 'GET', path: '/v1/route-receipts/{route_receipt_id}' },
{ id: 'getRouteDiagnostics', method: 'GET', path: '/v1/route-diagnostics/{route_policy_id}' },
{ id: 'getRouteReceiptDiagnostics', method: 'GET', path: '/v1/route-receipts/{route_receipt_id}/diagnostics' },
] as const;
export const GATEWAY_OPERATIONS = [
{ id: 'createChatCompletion', method: 'POST', path: '/v1/chat/completions' },
{ id: 'createAnthropicMessages', method: 'POST', path: '/v1/messages' },
{ id: 'createGeminiGenerateContent', method: 'POST', path: '/v1beta/models/{model}:generateContent' },
] as const;
