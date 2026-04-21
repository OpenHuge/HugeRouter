# 8. Domain Model

[Back to Docs Index](../README.md)

The platform's core domain objects are listed below.

Unless otherwise noted, the attribute lists in this document are recommended modeling fields, not a requirement that the first implementation persist every field on day one.

Preferred rule:

- model the fields needed to preserve routing, admission, billing, and diagnostics invariants first
- leave secondary analytics or ergonomics fields nullable, derived, or deferred when they are not yet on the critical path
- do not create a new entity just because a noun appears in the docs; a concept should become its own persisted model only when it has a distinct lifecycle, state owner, or audit requirement

### 8.0 Lifecycle Boundary Rule

The architecture should preserve four different lifecycle scopes even when the first implementation stores some of them together:

- `request`: one admitted ingress operation with one route decision
- `session`: a longer-lived conversational, realtime, or MCP interaction context that may span multiple requests or events
- `task`: a delegated or durable unit of work, typically relevant to A2A or agent orchestration
- `operation`: an individual side-effecting or governance-relevant step such as a tool call, browser action, memory write, or delegation transition

These scopes must not be silently conflated in identifiers or diagnostics. A request may create or attach to a session. A session may contain many operations. A task may outlive the request that submitted it.

### 8.1 Tenant

A top-level organizational boundary.

Attributes:
- `tenant_id`
- `name`
- `status`
- `billing_plan`
- `default_region_policy`
- `default_security_policy`
- `created_at`
- `updated_at`

### 8.2 Project

A scoped sub-boundary inside a tenant, often mapped to an application or environment.

Attributes:
- `project_id`
- `tenant_id`
- `name`
- `slug`
- `environment`
- `default_route_policy_id`
- `default_budget_policy_id`

### 8.3 Principal

A user, service account, API key subject, or machine identity.

Recommended attributes:
- `principal_id`
- `identity_type` such as `human`, `service`, `agent`
- `tenant_id`
- `project_id` nullable
- `display_name`
- `status`
- `delegation_depth_limit` nullable
- `default_budget_scope_id` nullable
- `created_at`
- `updated_at`

### 8.4 API Credential

Represents a northbound access credential.

Types:
- user API key
- project API key
- ephemeral session token
- JWT-based machine token
- mTLS client identity
- agent identity credential (non-human identity for autonomous agents)

Recommended attributes:
- `credential_id`
- `tenant_id`
- `project_id` nullable
- `subject_principal_id`
- `issuer_type`
- `identity_type`
- `binding_mode` such as `direct`, `delegated`, `ephemeral`
- `scopes`
- `status`
- `expires_at` nullable
- `created_at`
- `updated_at`

### 8.5 Provider

Represents an upstream provider family.

Examples:
- OpenAI
- Anthropic
- Google Gemini
- xAI
- Bedrock
- Vertex AI
- upstream gateway / transit system

### 8.6 Provider Resource

A specific upstream access resource.

Examples:
- API key
- OAuth token
- account pool entry
- subscription-backed account
- region-specific gateway endpoint

Recommended attributes:
- `provider_resource_id`
- `provider_id`
- `tenant_id` nullable for platform-managed shared resources
- `name`
- `status`
- `provenance_class`
- `credential_owner_type`
- `deployment_scope`
- `region`
- `endpoint_base_url`
- `auth_kind`
- `capability_profile_id` or inline capability descriptor
- `health_state`
- `quota_policy_id`
- `cost_policy_id`
- `metadata`
- `created_at`
- `updated_at`

### 8.7 Route Policy

Defines how requests should be matched and routed.

Recommended attributes:
- `route_policy_id`
- `tenant_id` nullable for platform defaults
- `name`
- `status`
- `priority`
- `match_criteria`
- `required_capabilities`
- `routing_strategy`
- `fallback_chain`
- `timeout_budget_ms`
- `ttft_budget_ms`
- `max_estimated_marginal_cost`
- `allowed_provenance_classes`
- `quota_reserve_strategy`
- `shadow_mode`
- `retention_tier`
- `protocol_bridge_policy`
- `operation_risk_policy`
- `approval_policy_id` nullable
- `version`
- `created_at`
- `updated_at`

### 8.8 Model Alias

A stable logical model name exposed to users.

Examples:
- `gpt-4.1` as a logical alias
- `reasoning-fast`
- `vision-default`
- `auto-chat-premium`

### 8.9 Capability Profile

A structured representation of what a route target supports.

### 8.10 Usage Event

Immutable record describing measured consumption for a request or session.

Recommended attributes:
- `usage_event_id`
- `request_id`
- `trace_id`
- `tenant_id`
- `project_id`
- `credential_id`
- `provider_resource_id`
- `route_receipt_id`
- `usage_phase` such as `reserve`, `partial`, `final`, `release`
- `usage_dimensions`
- `estimated_cost`
- `final_cost` nullable until finalized
- `occurred_at`
- `idempotency_key`

### 8.11 Ledger Entry

Immutable accounting event that affects projected balances or cost reporting.

Recommended attributes:
- `ledger_entry_id`
- `tenant_id`
- `project_id`
- `request_id` nullable for manual operations
- `usage_event_id` nullable for non-usage adjustments
- `entry_type`
- `currency`
- `amount`
- `balance_impact`
- `effective_at`
- `idempotency_key`
- `metadata`

### 8.12 Audit Event

Immutable security or governance event.

### 8.13 Budget Policy

Defines hierarchical budget and quota rules used during admission control and post-usage enforcement.

Recommended attributes:
- `budget_policy_id`
- `scope_type` such as `platform`, `tenant`, `project`, `credential`, `provider_resource`
- `scope_id`
- `status`
- `window_kind` such as `minute`, `hour`, `day`, `month`, `rolling`
- `limit_currency`
- `hard_limit_amount`
- `soft_limit_amount`
- `rpm_limit`
- `tpm_limit`
- `concurrency_limit`
- `action_on_hard_limit`
- `action_on_soft_limit`
- `version`

### 8.14 Route Receipt

The canonical per-request record explaining routing and admission outcomes.

Recommended attributes:
- `route_receipt_id`
- `request_id`
- `trace_id`
- `tenant_id`
- `project_id`
- `policy_snapshot_id`
- `routing_strategy`
- `candidate_targets`
- `excluded_targets`
- `score_breakdown`
- `selected_target`
- `admission_result`
- `fallback_transitions`
- `final_outcome`
- `created_at`

### 8.15 Config Snapshot

An immutable point-in-time materialization of the effective configuration used by a request or rollout.

Recommended attributes:
- `config_snapshot_id`
- `scope_type`
- `scope_id`
- `source_versions`
- `content_hash`
- `status`
- `created_at`
- `created_by`

### 8.16 Replay Capsule

A redacted support and diagnostics artifact sufficient to explain a request without storing full payload content.

Recommended attributes:
- `replay_capsule_id`
- `request_id`
- `trace_id`
- `route_receipt_id`
- `config_snapshot_id`
- `redaction_tier`
- `normalized_request_summary`
- `policy_decision_summary`
- `upstream_error_summary`
- `ledger_correlation_ids`
- `created_at`

### 8.17 Canonical Enums for V1

To reduce naming drift during implementation, the first version should standardize the following enums:

- `provider_resource.status`: `active | disabled | draining | quarantined | deleted`
- `provider_resource.provenance_class`: `official_api | official_gateway | byo_customer_credential | dedicated_managed_account | shared_brokered_pool | unofficial_client_channel`
- `provider_resource.credential_owner_type`: `platform | tenant | project | partner`
- `route_policy.protocol_bridge_policy`: `disallow | allow_listed | require_approval`
- `route_receipt.admission_result`: `admitted | rejected_budget | rejected_rate_limit | rejected_concurrency | rejected_policy | rejected_no_candidate`
- `usage_event.usage_phase`: `reserve | partial | final | release`
- `budget_policy.action_on_hard_limit`: `reject_new | terminate_inflight | require_override`
- `budget_policy.action_on_soft_limit`: `log_only | notify | degrade_route | require_override`
- `a2a_task.status`: `submitted | working | completed | failed | cancelled`
- `credential.identity_type`: `human | service | agent`
- `runtime_session.lifecycle_state`: `active | paused | completed | failed | expired`
- `approval_checkpoint.status`: `pending | approved | rejected | expired | bypassed`
- `governed_operation.operation_kind`: `inference | tool_read | tool_write | browser_read | browser_write | memory_recall | memory_retain | memory_reflect | delegation_submit | delegation_continue | delegation_cancel`
- `governed_operation.side_effect_level`: `none | low | material | high`

### 8.18 Agent Card (A2A)

A machine-readable manifest describing an agent's capabilities, reachability, and authentication requirements for inter-agent coordination.

Recommended attributes:
- `agent_card_id`
- `tenant_id`
- `agent_name`
- `agent_url`
- `capabilities`
- `auth_methods` (OAuth 2.0, API keys, mTLS)
- `supported_input_formats`
- `supported_output_formats`
- `status`
- `trust_level`
- `created_at`
- `updated_at`

### 8.19 A2A Task

A task delegated between agents via the A2A protocol.

Recommended attributes:
- `a2a_task_id`
- `tenant_id`
- `delegating_agent_id`
- `target_agent_card_id`
- `task_status` (`submitted`, `working`, `completed`, `failed`, `cancelled`)
- `task_description`
- `delegation_chain_depth`
- `budget_allocation`
- `timeout_ms`
- `result_summary`
- `created_at`
- `completed_at`

### 8.20 Semantic Cache Entry

A cached response keyed by semantic similarity to previous requests.

Recommended attributes:
- `cache_entry_id`
- `tenant_id`
- `project_id`
- `embedding_vector`
- `similarity_threshold`
- `cached_response_summary`
- `model_alias`
- `ttl`
- `hit_count`
- `created_at`
- `last_hit_at`

### 8.21 Runtime Session

A durable or semi-durable context for stateful protocols such as Realtime, MCP, browser-assisted runs, or multi-step agent execution.

Recommended attributes:
- `runtime_session_id`
- `tenant_id`
- `project_id`
- `credential_id`
- `principal_id`
- `protocol_family`
- `lifecycle_state`
- `started_by_request_id`
- `delegated_subject_id` nullable
- `session_budget_policy_id` nullable
- `approval_state` nullable
- `created_at`
- `updated_at`
- `ended_at` nullable

### 8.22 Approval Checkpoint

A durable pause or decision point where human or policy approval is required before execution may continue.

Recommended attributes:
- `approval_checkpoint_id`
- `tenant_id`
- `project_id`
- `runtime_session_id` nullable
- `request_id` nullable
- `a2a_task_id` nullable
- `governed_operation_id`
- `approval_reason_code`
- `status`
- `requested_at`
- `resolved_at` nullable
- `resolved_by_principal_id` nullable
- `resolution_summary` nullable

### 8.23 Governed Operation

A normalized description of one execution step that may carry side effects, delegated authority, or special retention and approval rules.

Recommended attributes:
- `governed_operation_id`
- `tenant_id`
- `project_id`
- `request_id` nullable
- `runtime_session_id` nullable
- `a2a_task_id` nullable
- `operation_kind`
- `operation_name`
- `side_effect_level`
- `target_system_class`
- `input_summary`
- `output_summary` nullable
- `policy_outcome`
- `approval_checkpoint_id` nullable
- `started_at`
- `completed_at` nullable

---
