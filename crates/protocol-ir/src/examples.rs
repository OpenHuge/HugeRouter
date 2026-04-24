#![allow(clippy::wildcard_imports)]

use std::collections::BTreeMap;

use super::*;

pub fn sample_tenant() -> Tenant {
    Tenant::new(
        TenantId::parse("tenant_acme").unwrap(),
        "acme-platform",
        "Acme Platform",
        3,
        "2026-04-20T00:00:00Z",
        "2026-04-21T08:30:00Z",
    )
    .unwrap()
}

pub fn sample_project() -> Project {
    Project::new(
        ProjectId::parse("proj_core").unwrap(),
        TenantId::parse("tenant_acme").unwrap(),
        "core-routing",
        "Core Routing",
        9,
        "2026-04-20T00:00:00Z",
        "2026-04-21T09:45:00Z",
    )
    .unwrap()
}

pub fn sample_provider_resource() -> ProviderResource {
    ProviderResource {
        provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: Some(ProjectId::parse("proj_core").unwrap()),
        provider_id: "openai".to_string(),
        name: "openai-us-east-primary".to_string(),
        status: core_domain::ProviderResourceStatus::Active,
        provenance_class: core_domain::ProvenanceClass::OfficialApi,
        credential_owner_type: core_domain::CredentialOwnerType::Platform,
        deployment_scope: core_domain::DeploymentScope::Shared,
        region: "us-east-1".to_string(),
        endpoint_base_url: "https://api.openai.com/v1".to_string(),
        auth_kind: core_domain::AuthKind::ApiKey,
        health_state: core_domain::HealthState::Healthy,
        health_message: Some("probe latency within SLO".to_string()),
        quarantine_reason: None,
        budget_policy_id: Some(core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap()),
        capabilities: core_domain::ProviderCapabilities {
            supports_streaming: true,
            supports_tool_calling: true,
            supports_json_mode: true,
            supports_realtime: false,
            supports_response_model_metadata: true,
        },
        supported_protocol_families: vec![
            "openai_chat".to_string(),
            "openai_responses".to_string(),
        ],
        is_transit_gateway: false,
        version: 7,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-21T11:15:00Z".to_string(),
    }
}

pub fn sample_bedrock_provider_resource() -> ProviderResource {
    ProviderResource {
        provider_resource_id: ProviderResourceId::parse("prvrsrc_bedrock_claude").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: Some(ProjectId::parse("proj_acme_ops").unwrap()),
        provider_id: "bedrock".to_string(),
        name: "Bedrock Claude".to_string(),
        status: core_domain::ProviderResourceStatus::Active,
        provenance_class: core_domain::ProvenanceClass::OfficialApi,
        credential_owner_type: core_domain::CredentialOwnerType::Platform,
        deployment_scope: core_domain::DeploymentScope::Shared,
        region: "us-east-1".to_string(),
        endpoint_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
        auth_kind: core_domain::AuthKind::ApiKey,
        health_state: core_domain::HealthState::Healthy,
        health_message: Some("aws credential chain available".to_string()),
        quarantine_reason: None,
        budget_policy_id: None,
        capabilities: core_domain::ProviderCapabilities {
            supports_streaming: false,
            supports_tool_calling: false,
            supports_json_mode: false,
            supports_realtime: false,
            supports_response_model_metadata: true,
        },
        supported_protocol_families: vec!["openai_chat".to_string()],
        is_transit_gateway: false,
        version: 1,
        created_at: "2026-04-22T00:00:00Z".to_string(),
        updated_at: "2026-04-22T00:00:00Z".to_string(),
    }
}

pub fn sample_route_policy() -> RoutePolicy {
    RoutePolicy {
        route_policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        display_name: "Default Interactive Route".to_string(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec!["tool_calling".to_string(), "json_mode".to_string()],
        preferred_regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
        version: 4,
        created_at: "2026-04-20T00:00:00Z".to_string(),
        updated_at: "2026-04-21T12:00:00Z".to_string(),
    }
}

pub fn sample_config_snapshot() -> ConfigSnapshot {
    ConfigSnapshot {
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        revision: 12,
        status: core_domain::ConfigSnapshotStatus::Active,
        activated_at: Some("2026-04-21T12:15:00Z".to_string()),
        provider_resource_ids: vec![ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()],
        route_policy_id: core_domain::RoutePolicyId::parse("routepol_default").unwrap(),
        budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap(),
    }
}

pub fn sample_route_receipt() -> RouteReceipt {
    RouteReceipt {
        route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        request_id: "req_123".to_string(),
        trace_id: "trace_123".to_string(),
        route_policy_id: RoutePolicyId::parse("routepol_default").unwrap(),
        protocol_family: "openai_chat".to_string(),
        model_alias: "reasoning-fast".to_string(),
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        admission_result: AdmissionResult::Admitted,
        selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
        excluded_targets: vec![core_domain::ExcludedTarget {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_backup").unwrap(),
            reason_code: "rejected_provenance_class".to_string(),
            reason: "rejected_provenance_class".to_string(),
        }],
        failure_reason: None,
        score_breakdown: core_domain::ScoreBreakdown {
            latency: 0.82,
            cost: 0.66,
            health: 0.97,
            trust: 1.0,
        },
        fallback_transitions: Vec::new(),
        normalized_error: None,
        created_at: "2026-04-21T12:16:00Z".to_string(),
    }
}

pub fn sample_usage_event() -> UsageEvent {
    UsageEvent {
        usage_event_id: core_domain::UsageEventId::parse("usageevt_123").unwrap(),
        route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
        model_alias: "reasoning-fast".to_string(),
        phase: UsagePhase::Final,
        idempotency_key: "usageevt_123:final".to_string(),
        usage: core_domain::UsageMetrics {
            input_tokens: 1200,
            output_tokens: 320,
            cached_input_tokens: 64,
        },
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.1420".to_string(),
        },
        recorded_at: "2026-04-21T12:16:05Z".to_string(),
    }
}

pub fn sample_error_envelope() -> ErrorEnvelope {
    ErrorEnvelope {
        error: core_domain::NormalizedError {
            code: "validation_failed".to_string(),
            message: "Route policy requires at least one capability.".to_string(),
            request_id: "req_123".to_string(),
            retryable: false,
            upstream_code: None,
            upstream_status_code: None,
            validation_issues: vec![core_domain::ValidationIssue {
                field: "required_capabilities".to_string(),
                message: "expected at least one value".to_string(),
            }],
            details: std::collections::BTreeMap::from([(
                "resource".to_string(),
                "route_policy".to_string(),
            )]),
        },
    }
}

pub fn sample_route_simulation_request() -> RouteSimulationRequest {
    RouteSimulationRequest {
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        credential_scope: core_domain::CredentialId::parse("cred_primary").unwrap(),
        protocol_family: ProtocolFamily::OpenAiChat,
        model_alias: "reasoning-fast".to_string(),
        required_capabilities: vec!["tool_calling".to_string(), "json_mode".to_string()],
        region: "us-east-1".to_string(),
        expected_prompt_tokens: 12_000,
        expected_max_output_tokens: 4_000,
        traffic_class: "interactive".to_string(),
    }
}

pub fn sample_route_simulation_response() -> RouteSimulationResponse {
    RouteSimulationResponse {
        simulation_id: "routesim_123".to_string(),
        config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
        admission_result: AdmissionResult::Admitted,
        eligible_candidates: vec![EligibleCandidate {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            score_breakdown: core_domain::ScoreBreakdown {
                latency: 0.82,
                cost: 0.66,
                health: 0.97,
                trust: 1.0,
            },
        }],
        excluded_candidates: vec![core_domain::ExcludedTarget {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_backup").unwrap(),
            reason_code: "rejected_provenance_class".to_string(),
            reason: "rejected_provenance_class".to_string(),
        }],
        selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.1420".to_string(),
        },
    }
}

pub fn sample_route_receipt_diagnostics() -> RouteReceiptDiagnosticsResponse {
    RouteReceiptDiagnosticsResponse {
        route_receipt: sample_route_receipt(),
        decision_timeline: vec![
            RouteReceiptDecisionTraceStep {
                stage: "admission".to_string(),
                status: "passed".to_string(),
                message: "Tenant policy accepted request".to_string(),
                score: Some(1.0),
                notes: vec!["all constraints satisfied".to_string()],
            },
            RouteReceiptDecisionTraceStep {
                stage: "candidate_selection".to_string(),
                status: "passed".to_string(),
                message: "Selected openai-us-east-primary".to_string(),
                score: Some(0.91),
                notes: Vec::new(),
            },
        ],
        policy_checks: vec![RouteReceiptPolicyCheck {
            policy_id: RoutePolicyId::parse("routepol_default").unwrap(),
            status: "passed".to_string(),
            reason: Some("policy satisfied".to_string()),
        }],
        provider_attempts: vec![RouteReceiptProviderAttempt {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            attempt: 1,
            status: "succeeded".to_string(),
            started_at: "2026-04-21T12:16:01Z".to_string(),
            finished_at: "2026-04-21T12:16:03Z".to_string(),
            latency_ms: 1100,
            reason: "succeeded with output".to_string(),
        }],
        metadata: BTreeMap::from([
            ("policy_cache_hit".to_string(), "true".to_string()),
            ("candidate_pool_size".to_string(), "3".to_string()),
        ]),
    }
}

pub fn sample_route_diagnostics_response() -> RouteDiagnosticsResponse {
    let provider_resource = sample_provider_resource();
    let route_policy = sample_route_policy();
    let route_receipt = sample_route_receipt();

    RouteDiagnosticsResponse {
        route_policy,
        active_snapshot: Some(sample_config_snapshot()),
        active_snapshot_matches_route_policy: true,
        last_route_receipt: Some(RouteReceiptSummary {
            route_receipt_id: route_receipt.route_receipt_id.clone(),
            admission_result: route_receipt.admission_result,
            selected_target: route_receipt.selected_target.clone(),
            failure_reason: route_receipt.failure_reason.clone(),
            created_at: route_receipt.created_at.clone(),
        }),
        recent_receipts: vec![RouteReceiptSummary {
            route_receipt_id: route_receipt.route_receipt_id.clone(),
            admission_result: route_receipt.admission_result,
            selected_target: route_receipt.selected_target.clone(),
            failure_reason: route_receipt.failure_reason.clone(),
            created_at: route_receipt.created_at.clone(),
        }],
        targets: vec![RouteDiagnosticTarget {
            provider_resource,
            decision: RouteDiagnosticDecision::Selected,
            in_active_snapshot: true,
            supports_protocol_family: true,
            capability_gaps: Vec::new(),
            reason_code: "selected_recent_receipt".to_string(),
            reason: "Selected by the most recent route receipt.".to_string(),
            recent_receipt_id: Some(route_receipt.route_receipt_id),
            recent_receipt_reason: None,
        }],
    }
}

pub fn sample_usage_summary_response() -> UsageSummaryResponse {
    UsageSummaryResponse {
        data: UsageSummary {
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            window_start: "2026-04-21T00:00:00Z".to_string(),
            window_end: "2026-04-21T23:59:59Z".to_string(),
            currency: "USD".to_string(),
            event_count: 14,
            input_tokens: 18_420,
            output_tokens: 6_245,
            cached_input_tokens: 1_220,
            provider_cost: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.124500".to_string(),
            },
            billable_price: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.152025".to_string(),
            },
        },
    }
}

pub fn sample_usage_breakdown_response() -> UsageBreakdownResponse {
    UsageBreakdownResponse {
        data: vec![
            UsageBreakdownRow {
                bucket: "openai".to_string(),
                provider_id: Some("openai".to_string()),
                model_alias: None,
                input_tokens: 10_000,
                output_tokens: 4_000,
                cached_input_tokens: 500,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.082000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.098400".to_string(),
                },
            },
            UsageBreakdownRow {
                bucket: "reasoning-fast".to_string(),
                provider_id: None,
                model_alias: Some("reasoning-fast".to_string()),
                input_tokens: 8_420,
                output_tokens: 2_245,
                cached_input_tokens: 720,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.042500".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.053625".to_string(),
                },
            },
        ],
        next_cursor: Some("2".to_string()),
    }
}

pub fn sample_balance_projection_response() -> BalanceProjectionResponse {
    BalanceProjectionResponse {
        data: BalanceProjection {
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            currency: "USD".to_string(),
            provider_cost_total: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "1.244000".to_string(),
            },
            billable_total: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "1.540000".to_string(),
            },
            configured_budget: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "75.000000".to_string(),
            },
            remaining_budget: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "73.460000".to_string(),
            },
            threshold_status: "ok".to_string(),
            last_projected_at: "2026-04-21T12:20:00Z".to_string(),
            projection_lag_seconds: 18,
        },
    }
}

pub fn sample_pricing_catalog_response() -> PricingCatalogResponse {
    PricingCatalogResponse {
        catalog_id: "pricing_catalog_default".to_string(),
        catalog_version: 1,
        currency: "USD".to_string(),
        entries: vec![
            PricingCatalogEntry {
                dimension: "input_tokens".to_string(),
                provider_id: "openai".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 2_500,
                unit_denominator: 1_000,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "image_generations".to_string(),
                provider_id: "openai".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 18_000,
                unit_denominator: 1,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "audio_seconds".to_string(),
                provider_id: "openai".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 1_500,
                unit_denominator: 1,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "input_tokens".to_string(),
                provider_id: "bedrock".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 6_000,
                unit_denominator: 1_000,
                source: "provider_native".to_string(),
            },
            PricingCatalogEntry {
                dimension: "output_tokens".to_string(),
                provider_id: "bedrock".to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit: 30_000,
                unit_denominator: 1_000,
                source: "provider_native".to_string(),
            },
        ],
    }
}

pub fn sample_pricing_simulation_request() -> PricingSimulationRequest {
    PricingSimulationRequest {
        provider_id: "openai".to_string(),
        model_alias: "reasoning-fast".to_string(),
        usage: UsageMetrics {
            input_tokens: 1_200,
            output_tokens: 320,
            cached_input_tokens: 64,
        },
        region: Some("us-east-1".to_string()),
        image_generation_units: Some(1),
        audio_seconds: Some(8),
    }
}

pub fn sample_pricing_simulation_response() -> PricingSimulationResponse {
    PricingSimulationResponse {
        catalog_id: "pricing_catalog_default".to_string(),
        catalog_version: 1,
        currency: "USD".to_string(),
        provider_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.005188".to_string(),
        },
        billable_price: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.006225".to_string(),
        },
        line_items: vec![
            PricingSimulationLineItem {
                dimension: "input_tokens".to_string(),
                units: 1_200,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.003000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.003600".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "output_tokens".to_string(),
                units: 320,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.002720".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.003264".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "cached_input_tokens".to_string(),
                units: 64,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.000048".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.000057".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "image_generations".to_string(),
                units: 1,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.018000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.021600".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
            PricingSimulationLineItem {
                dimension: "audio_seconds".to_string(),
                units: 8,
                provider_cost: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.012000".to_string(),
                },
                billable_price: MonetaryAmount {
                    currency: "USD".to_string(),
                    amount: "0.014400".to_string(),
                },
                rate_source: "provider_native".to_string(),
            },
        ],
    }
}

pub fn sample_billing_export_request() -> BillingExportRequest {
    BillingExportRequest {
        tenant_id: Some(TenantId::parse("tenant_acme").unwrap()),
        project_id: Some(ProjectId::parse("proj_core").unwrap()),
        window_start: "2026-04-01T00:00:00Z".to_string(),
        window_end: "2026-04-30T23:59:59Z".to_string(),
        format: "csv".to_string(),
    }
}

pub fn sample_billing_export_job_response() -> BillingExportJobResponse {
    BillingExportJobResponse {
        data: BillingExportJob {
            export_job_id: "export_123".to_string(),
            status: "queued".to_string(),
            format: "csv".to_string(),
            requested_at: "2026-04-21T12:25:00Z".to_string(),
            completed_at: None,
            error_message: None,
            tenant_id: Some(TenantId::parse("tenant_acme").unwrap()),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
        },
    }
}

pub fn sample_normalized_error(message: &str) -> core_domain::NormalizedError {
    core_domain::NormalizedError {
        code: "validation_failed".to_string(),
        message: message.to_string(),
        request_id: "req_123".to_string(),
        retryable: false,
        upstream_code: None,
        upstream_status_code: None,
        validation_issues: vec![core_domain::ValidationIssue {
            field: "messages".to_string(),
            message: "expected valid payload".to_string(),
        }],
        details: BTreeMap::from([("service".to_string(), "gateway-api".to_string())]),
    }
}

pub fn sample_usage_event_recorded_message() -> UsageEventRecordedMessage {
    let envelope = MessageEnvelope::new(
        "msg_usage_123",
        MessageType::UsageEventRecorded,
        "2026-04-21T12:16:05Z",
        ServiceName::parse("gateway-api").unwrap(),
        "usageevt_123:final",
        UsageEventRecorded {
            usage_event: sample_usage_event(),
        },
    )
    .with_request_context("trace_123", "req_123");

    UsageEventRecordedMessage {
        message_id: envelope.message_id,
        message_type: envelope.message_type,
        schema_version: envelope.schema_version,
        occurred_at: envelope.occurred_at,
        producer: envelope.producer,
        trace_id: envelope.trace_id,
        request_id: envelope.request_id,
        idempotency_key: envelope.idempotency_key,
        payload: envelope.payload,
    }
}

#[cfg(test)]
pub fn sample_route_receipt_recorded_message() -> RouteReceiptRecordedMessage {
    let diagnostics = sample_route_receipt_diagnostics();
    let route_receipt = diagnostics.route_receipt.clone();
    RouteReceiptRecordedMessage {
        message_id: "msg_routercpt_123".to_string(),
        message_type: RouteReceiptRecordedMessageType::RouteReceiptRecorded,
        schema_version: 1,
        occurred_at: route_receipt.created_at.clone(),
        producer: ServiceName::parse("gateway-api").unwrap(),
        trace_id: Some("trace_123".to_string()),
        request_id: Some("req_123".to_string()),
        idempotency_key: format!("{}:recorded", route_receipt.route_receipt_id),
        payload: RouteReceiptRecorded {
            route_receipt,
            decision_timeline: diagnostics.decision_timeline,
            policy_checks: diagnostics.policy_checks,
            provider_attempts: diagnostics.provider_attempts,
        },
    }
}

pub fn sample_config_snapshot_activated_message() -> ConfigSnapshotActivatedMessage {
    let envelope = MessageEnvelope::new(
        "msg_cfgsnap_123",
        MessageType::ConfigSnapshotActivated,
        "2026-04-21T12:15:00Z",
        ServiceName::parse("control-plane-api").unwrap(),
        "cfgsnap_default:activated",
        ConfigSnapshotActivated {
            config_snapshot: sample_config_snapshot(),
        },
    );

    ConfigSnapshotActivatedMessage {
        message_id: envelope.message_id,
        message_type: envelope.message_type,
        schema_version: envelope.schema_version,
        occurred_at: envelope.occurred_at,
        producer: envelope.producer,
        trace_id: envelope.trace_id,
        request_id: envelope.request_id,
        idempotency_key: envelope.idempotency_key,
        payload: envelope.payload,
    }
}
