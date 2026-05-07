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
        grant_id: Some("grant_acme_customer".to_string()),
        owner_account_id: Some("acct_acme_owner".to_string()),
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
            reason_code: "provider_success".to_string(),
            retryable: false,
            fallback_target: None,
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
            owner_account_id: Some("acct_acme_owner".to_string()),
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
            owner_account_id: Some("acct_acme_owner".to_string()),
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

pub fn sample_create_renewal_intent_request() -> CreateRenewalIntentRequest {
    CreateRenewalIntentRequest {
        out_trade_no: "wxpay_renew_123".to_string(),
        grant_id: "opengrant_customer_123".to_string(),
        renew_expires_at: "2026-06-03T00:00:00Z".to_string(),
        reason: Some("customer renewal paid via WeChat Pay".to_string()),
    }
}

pub fn sample_renewal_intent_response() -> RenewalIntentResponse {
    RenewalIntentResponse {
        data: RenewalIntent {
            renewal_intent_id: "renewal_123".to_string(),
            out_trade_no: "wxpay_renew_123".to_string(),
            grant_id: "opengrant_customer_123".to_string(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            payment_status: "paid".to_string(),
            previous_grant_status: "expired".to_string(),
            previous_expires_at: "2026-05-03T00:00:00Z".to_string(),
            renew_expires_at: "2026-06-03T00:00:00Z".to_string(),
            status: "renewed".to_string(),
            reason_code: "payment_paid_grant_recovered".to_string(),
            reason: "Paid order matched the opening grant; only future grant state was restored."
                .to_string(),
            created_by: "user_ops_admin".to_string(),
            created_at: "2026-05-03T12:00:00Z".to_string(),
            updated_at: "2026-05-03T12:00:00Z".to_string(),
            applied_at: Some("2026-05-03T12:00:00Z".to_string()),
        },
    }
}

pub fn sample_create_delivery_request() -> CreateDeliveryRequest {
    CreateDeliveryRequest {
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider: "chatgpt".to_string(),
        customer_label: Some("Acme May browser handoff".to_string()),
        service_kind: Some("manual_browser_account".to_string()),
        service_days: 30,
        starts_at: Some("2026-05-05T10:00:00Z".to_string()),
        code_expires_at: Some("2026-06-04T10:00:00Z".to_string()),
    }
}

pub fn sample_delivery_prepare_response() -> DeliveryPrepareResponse {
    DeliveryPrepareResponse {
        data: sample_delivery_projection(),
        one_time_codes: DeliveryOneTimeCodes {
            redemption_code: "ku0-red-v2-260505-a1b2-c3d4e5f6g7h8-7b".to_string(),
            browser_file_unlock_code: "ku0-brw-v2-260505-j9k0-l1m2n3p4q5r6-76".to_string(),
        },
    }
}

pub fn sample_delivery_response() -> DeliveryResponse {
    DeliveryResponse {
        data: sample_delivery_projection(),
    }
}

pub fn sample_create_delivery_artifact_request() -> CreateDeliveryArtifactRequest {
    CreateDeliveryArtifactRequest {
        artifact_kind: Some("browser_account_bundle".to_string()),
        file_name: Some("acme-may.hcbrowser".to_string()),
        content_type: Some("application/octet-stream".to_string()),
        carrier_valid_until: Some("2026-05-20T10:00:00Z".to_string()),
        encryption_protocol: Some("delivery_account_bundle_v2".to_string()),
        encryption_version: Some("2".to_string()),
        secret_kind: Some("browser_file_unlock_code".to_string()),
        payload_base64: "ZW5jcnlwdGVkLWhjYnJvd3Nlci1wYXlsb2Fk".to_string(),
    }
}

pub fn sample_delivery_artifact_response() -> DeliveryArtifactResponse {
    DeliveryArtifactResponse {
        data: sample_delivery_artifact(),
    }
}

pub fn sample_delivery_artifacts_response() -> DeliveryArtifactsResponse {
    DeliveryArtifactsResponse {
        data: vec![sample_delivery_artifact()],
    }
}

pub fn sample_redeem_delivery_request() -> RedeemDeliveryRequest {
    RedeemDeliveryRequest {
        redemption_code: "ku0-red-v2-260505-a1b2-c3d4e5f6g7h8-7b".to_string(),
    }
}

pub fn sample_delivery_activation_response() -> DeliveryActivationResponse {
    DeliveryActivationResponse {
        data: DeliveryActivation {
            activation_id: "activation_10002".to_string(),
            delivery_id: "delivery_10001".to_string(),
            artifact_id: "artifact_10002".to_string(),
            entitlement_id: "dlvent_delivery_10001".to_string(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            provider: "chatgpt".to_string(),
            status: "activated".to_string(),
            activation_source: "redemption_code".to_string(),
            activated_at: "2026-05-05T10:10:00Z".to_string(),
            entitlement_ends_at: "2026-06-04T10:00:00Z".to_string(),
            artifact: sample_delivery_artifact(),
            created_at: "2026-05-05T10:10:00Z".to_string(),
            updated_at: "2026-05-05T10:10:00Z".to_string(),
            revoked_at: None,
            revoked_by: None,
            revoke_reason: None,
        },
    }
}

pub fn sample_delivery_activation_redeem_response() -> DeliveryActivationRedeemResponse {
    DeliveryActivationRedeemResponse {
        data: sample_delivery_activation_response().data,
        restore: DeliveryActivationRestoreInfo {
            artifact_import_secret: "ku0-brw-v2-260505-j9k0-l1m2n3p4q5r6-76".to_string(),
            secret_kind: "browser_file_unlock_code".to_string(),
            encryption_protocol: "delivery_account_bundle_v2".to_string(),
            encryption_version: "2".to_string(),
        },
    }
}

pub fn sample_create_delivery_download_grant_request() -> CreateDeliveryDownloadGrantRequest {
    CreateDeliveryDownloadGrantRequest {
        activation_id: "activation_10002".to_string(),
        redemption_code: None,
    }
}

pub fn sample_delivery_download_grant_issue_response() -> DeliveryDownloadGrantIssueResponse {
    DeliveryDownloadGrantIssueResponse {
        data: sample_delivery_download_grant(),
        download_token: "dlt_once_returned_plaintext_token_20260506".to_string(),
    }
}

pub fn sample_delivery_download_grant_response() -> DeliveryDownloadGrantResponse {
    DeliveryDownloadGrantResponse {
        data: sample_delivery_download_grant(),
    }
}

pub fn sample_delivery_download_grant_revoke_request() -> DeliveryDownloadGrantRevokeRequest {
    DeliveryDownloadGrantRevokeRequest {
        expected_version: 1,
        revoke_reason: Some("operator revoked unused download token".to_string()),
    }
}

pub fn sample_extend_delivery_entitlement_request() -> ExtendDeliveryEntitlementRequest {
    ExtendDeliveryEntitlementRequest {
        expected_version: 1,
        extend_days: 30,
        reason: Some("customer renewal applied without new redemption code".to_string()),
    }
}

pub fn sample_delivery_lifecycle_response() -> DeliveryLifecycleResponse {
    let segment = sample_delivery_service_segment();
    DeliveryLifecycleResponse {
        entitlement: sample_delivery_projection().entitlement,
        segments: vec![segment.clone()],
        events: vec![DeliveryLifecycleEvent {
            event_id: "dlvevt_dlvent_delivery_10001_1".to_string(),
            entitlement_id: "dlvent_delivery_10001".to_string(),
            segment_id: Some(segment.segment_id),
            event_type: "segment_created".to_string(),
            status: "active".to_string(),
            reason: Some("activation_segment".to_string()),
            created_by: "user_ops_admin".to_string(),
            created_at: "2026-05-05T10:10:00Z".to_string(),
            payload: BTreeMap::from([("artifact_id".to_string(), "artifact_10002".to_string())]),
        }],
    }
}

pub fn sample_delivery_service_segments_response() -> DeliveryServiceSegmentsResponse {
    DeliveryServiceSegmentsResponse {
        data: vec![sample_delivery_service_segment()],
    }
}

pub fn sample_delivery_lifecycle_events_response() -> DeliveryLifecycleEventsResponse {
    DeliveryLifecycleEventsResponse {
        data: sample_delivery_lifecycle_response().events,
    }
}

pub fn sample_delivery_operations_overview_response() -> DeliveryOperationsOverviewResponse {
    DeliveryOperationsOverviewResponse {
        data: DeliveryOperationsOverview {
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            window_start: "2026-05-01T00:00:00Z".to_string(),
            window_end: "2026-05-31T23:59:59Z".to_string(),
            totals: DeliveryOperationsTotals {
                deliveries: 1,
                artifacts: 1,
                upload_batches: 1,
                upload_items: 1,
                activations: 1,
                download_grants: 1,
                entitlements: 1,
                service_segments: 1,
                lifecycle_events: 1,
                exceptions: 1,
            },
            status_counts: vec![
                DeliveryOperationsStatusCount {
                    domain: "delivery".to_string(),
                    status: "active".to_string(),
                    count: 1,
                },
                DeliveryOperationsStatusCount {
                    domain: "download_grant".to_string(),
                    status: "used".to_string(),
                    count: 1,
                },
            ],
            recent_events: sample_delivery_operations_timeline_response().data,
            exceptions: sample_delivery_operations_exceptions_response().data,
        },
    }
}

pub fn sample_delivery_operations_timeline_response() -> DeliveryOperationsTimelineResponse {
    DeliveryOperationsTimelineResponse {
        data: vec![
            DeliveryOperationsTimelineEvent {
                event_id: "delivery_created:delivery_10001:2026-05-05T10:00:00Z".to_string(),
                event_type: "delivery_created".to_string(),
                object_type: "delivery".to_string(),
                object_id: "delivery_10001".to_string(),
                tenant_id: TenantId::parse("tenant_acme").unwrap(),
                project_id: ProjectId::parse("proj_core").unwrap(),
                delivery_id: Some("delivery_10001".to_string()),
                entitlement_id: None,
                activation_id: None,
                artifact_id: None,
                grant_id: None,
                segment_id: None,
                upload_batch_id: Some("dlvup_10004".to_string()),
                upload_item_id: None,
                status: "active".to_string(),
                occurred_at: "2026-05-05T10:00:00Z".to_string(),
                summary: "Delivery fact created".to_string(),
            },
            DeliveryOperationsTimelineEvent {
                event_id: "delivery_download_grant_used:dlgrant_10003:2026-05-05T10:11:00Z"
                    .to_string(),
                event_type: "delivery_download_grant_used".to_string(),
                object_type: "delivery_download_grant".to_string(),
                object_id: "dlgrant_10003".to_string(),
                tenant_id: TenantId::parse("tenant_acme").unwrap(),
                project_id: ProjectId::parse("proj_core").unwrap(),
                delivery_id: Some("delivery_10001".to_string()),
                entitlement_id: Some("dlvent_delivery_10001".to_string()),
                activation_id: Some("activation_10002".to_string()),
                artifact_id: Some("artifact_10002".to_string()),
                grant_id: Some("dlgrant_10003".to_string()),
                segment_id: None,
                upload_batch_id: Some("dlvup_10004".to_string()),
                upload_item_id: Some("dlvupitem_10004_1".to_string()),
                status: "used".to_string(),
                occurred_at: "2026-05-05T10:11:00Z".to_string(),
                summary: "Download grant consumed".to_string(),
            },
        ],
    }
}

pub fn sample_delivery_operations_exceptions_response() -> DeliveryOperationsExceptionsResponse {
    DeliveryOperationsExceptionsResponse {
        data: vec![DeliveryOperationsException {
            exception_id:
                "entitlement_needs_manual_supply:dlvent_delivery_10001:2026-05-05T10:12:00Z"
                    .to_string(),
            exception_type: "entitlement_needs_manual_supply".to_string(),
            severity: "critical".to_string(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            delivery_id: Some("delivery_10001".to_string()),
            entitlement_id: Some("dlvent_delivery_10001".to_string()),
            activation_id: None,
            artifact_id: None,
            grant_id: None,
            segment_id: None,
            upload_batch_id: Some("dlvup_10004".to_string()),
            upload_item_id: Some("dlvupitem_10004_1".to_string()),
            status: "needs_manual_supply".to_string(),
            reason: Some("continuation_artifact_missing".to_string()),
            occurred_at: "2026-05-05T10:12:00Z".to_string(),
            summary: "Delivery entitlement requires operator attention".to_string(),
        }],
    }
}

pub fn sample_delivery_operations_detail_response() -> DeliveryOperationsDetailResponse {
    DeliveryOperationsDetailResponse {
        data: DeliveryOperationsDetail {
            delivery: sample_delivery_projection(),
            artifacts: vec![sample_delivery_artifact()],
            upload_items: vec![sample_delivery_upload_batch_item()],
            activations: vec![sample_delivery_activation_response().data],
            download_grants: vec![sample_delivery_download_grant()],
            service_segments: vec![sample_delivery_service_segment()],
            lifecycle_events: sample_delivery_lifecycle_events_response().data,
            timeline: sample_delivery_operations_timeline_response().data,
            exceptions: sample_delivery_operations_exceptions_response().data,
        },
    }
}

pub fn sample_create_delivery_upload_batch_request() -> CreateDeliveryUploadBatchRequest {
    CreateDeliveryUploadBatchRequest {
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider: "chatgpt".to_string(),
        source_file_name: "acme-may-upload.jsonl".to_string(),
        idempotency_key: Some("ops-upload-2026-05-05-acme-001".to_string()),
        items: vec![DeliveryUploadBatchItemInput {
            delivery_id: "delivery_10001".to_string(),
            row_index: Some(1),
            artifact_kind: Some("browser_account_bundle".to_string()),
            file_name: Some("acme-may.hcbrowser".to_string()),
            content_type: Some("application/octet-stream".to_string()),
            carrier_valid_until: Some("2026-05-20T10:00:00Z".to_string()),
            payload_base64: "ZW5jcnlwdGVkLWhjYnJvd3Nlci1idW5kbGU=".to_string(),
        }],
    }
}

pub fn sample_delivery_upload_batch_response() -> DeliveryUploadBatchResponse {
    DeliveryUploadBatchResponse {
        data: DeliveryUploadBatch {
            batch_id: "dlvup_10004".to_string(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            provider: "chatgpt".to_string(),
            status: "queued".to_string(),
            source_file_name: "acme-may-upload.jsonl".to_string(),
            source_file_sha256:
                "sha256:1f39e082b7322d0386a855f3de726de051ae2bf80d8e8cf869927722d2a9f03f"
                    .to_string(),
            idempotency_key: Some("ops-upload-2026-05-05-acme-001".to_string()),
            total_count: 1,
            success_count: 0,
            failed_count: 0,
            duplicate_count: 0,
            created_by: "user_ops_admin".to_string(),
            created_at: "2026-05-05T10:04:00Z".to_string(),
            updated_at: "2026-05-05T10:04:00Z".to_string(),
            started_at: None,
            finished_at: None,
            error_summary: None,
            version: 1,
        },
    }
}

pub fn sample_delivery_upload_batch_items_response() -> DeliveryUploadBatchItemsResponse {
    DeliveryUploadBatchItemsResponse {
        data: vec![sample_delivery_upload_batch_item()],
    }
}

fn sample_delivery_upload_batch_item() -> DeliveryUploadBatchItem {
    DeliveryUploadBatchItem {
        item_id: "dlvupitem_10004_1".to_string(),
        batch_id: "dlvup_10004".to_string(),
        row_index: 1,
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        delivery_id: "delivery_10001".to_string(),
        artifact_id: Some("artifact_10002".to_string()),
        status: "accepted".to_string(),
        artifact_kind: "browser_account_bundle".to_string(),
        file_name: Some("acme-may.hcbrowser".to_string()),
        content_type: "application/octet-stream".to_string(),
        carrier_valid_until: Some("2026-05-20T10:00:00Z".to_string()),
        payload_sha256: "sha256:8f1b8a0ad5d63d57d8a0cce1e0a6a4d6f4af0c9d61678f2f48a88e3dd2dbe4f9"
            .to_string(),
        size_bytes: 27,
        error_code: None,
        error_message: None,
        created_at: "2026-05-05T10:04:00Z".to_string(),
        updated_at: "2026-05-05T10:05:00Z".to_string(),
        version: 2,
    }
}

fn sample_delivery_download_grant() -> DeliveryDownloadGrant {
    DeliveryDownloadGrant {
        grant_id: "dlgrant_10003".to_string(),
        activation_id: "activation_10002".to_string(),
        delivery_id: "delivery_10001".to_string(),
        artifact_id: "artifact_10002".to_string(),
        entitlement_id: "dlvent_delivery_10001".to_string(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider: "chatgpt".to_string(),
        status: "active".to_string(),
        token_prefix: "dlt_once_returned...".to_string(),
        token_last_four: "0506".to_string(),
        expires_at: "2026-05-05T10:25:00Z".to_string(),
        max_uses: 1,
        use_count: 0,
        artifact: sample_delivery_artifact(),
        created_by: "user_ops_admin".to_string(),
        created_at: "2026-05-05T10:10:00Z".to_string(),
        updated_at: "2026-05-05T10:10:00Z".to_string(),
        version: 1,
        used_at: None,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
    }
}

fn sample_delivery_service_segment() -> DeliveryServiceSegment {
    DeliveryServiceSegment {
        segment_id: "dlvseg_dlvent_delivery_10001_1".to_string(),
        entitlement_id: "dlvent_delivery_10001".to_string(),
        activation_id: "activation_10002".to_string(),
        delivery_id: "delivery_10001".to_string(),
        artifact_id: "artifact_10002".to_string(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        provider: "chatgpt".to_string(),
        status: "active".to_string(),
        segment_index: 1,
        effective_from: "2026-05-05T10:10:00Z".to_string(),
        effective_until: "2026-05-20T10:00:00Z".to_string(),
        carrier_valid_until: "2026-05-20T10:00:00Z".to_string(),
        created_by: "user_ops_admin".to_string(),
        created_at: "2026-05-05T10:10:00Z".to_string(),
        updated_at: "2026-05-05T10:10:00Z".to_string(),
        version: 1,
    }
}

fn sample_delivery_projection() -> DeliveryProjection {
    DeliveryProjection {
        delivery: Delivery {
            delivery_id: "delivery_10001".to_string(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            provider: "chatgpt".to_string(),
            status: "prepared".to_string(),
            operator_id: "user_ops_admin".to_string(),
            customer_label: Some("Acme May browser handoff".to_string()),
            source: "manual_operator".to_string(),
            created_at: "2026-05-05T10:00:00Z".to_string(),
            updated_at: "2026-05-05T10:00:00Z".to_string(),
            version: 1,
            revoked_at: None,
            revoked_by: None,
            revoke_reason: None,
        },
        codes: vec![
            DeliveryCode {
                code_id: "dlvcode_delivery_10001_redemption".to_string(),
                delivery_id: "delivery_10001".to_string(),
                code_type: "redemption_code".to_string(),
                code_prefix: "ku0-red-v2-260505...".to_string(),
                code_last_four: "8-7b".to_string(),
                format_version: "ku0-red-v2".to_string(),
                status: "active".to_string(),
                expires_at: "2026-06-04T10:00:00Z".to_string(),
                used_at: None,
                revoked_at: None,
                created_at: "2026-05-05T10:00:00Z".to_string(),
                updated_at: "2026-05-05T10:00:00Z".to_string(),
                version: 1,
            },
            DeliveryCode {
                code_id: "dlvcode_delivery_10001_browser_unlock".to_string(),
                delivery_id: "delivery_10001".to_string(),
                code_type: "browser_file_unlock_code".to_string(),
                code_prefix: "ku0-brw-v2-260505...".to_string(),
                code_last_four: "6-76".to_string(),
                format_version: "ku0-brw-v2".to_string(),
                status: "active".to_string(),
                expires_at: "2026-06-04T10:00:00Z".to_string(),
                used_at: None,
                revoked_at: None,
                created_at: "2026-05-05T10:00:00Z".to_string(),
                updated_at: "2026-05-05T10:00:00Z".to_string(),
                version: 1,
            },
        ],
        entitlement: DeliveryEntitlement {
            entitlement_id: "dlvent_delivery_10001".to_string(),
            delivery_id: "delivery_10001".to_string(),
            service_kind: "manual_browser_account".to_string(),
            service_days: 30,
            starts_at: "2026-05-05T10:00:00Z".to_string(),
            ends_at: "2026-06-04T10:00:00Z".to_string(),
            service_starts_at: "2026-05-05T10:00:00Z".to_string(),
            service_ends_at: "2026-06-04T10:00:00Z".to_string(),
            status: "active".to_string(),
            created_at: "2026-05-05T10:00:00Z".to_string(),
            updated_at: "2026-05-05T10:00:00Z".to_string(),
            version: 1,
        },
    }
}

fn sample_delivery_artifact() -> DeliveryArtifact {
    DeliveryArtifact {
        artifact_id: "artifact_10002".to_string(),
        delivery_id: "delivery_10001".to_string(),
        tenant_id: TenantId::parse("tenant_acme").unwrap(),
        project_id: ProjectId::parse("proj_core").unwrap(),
        artifact_kind: "browser_account_bundle".to_string(),
        provider: "chatgpt".to_string(),
        status: "active".to_string(),
        version: 1,
        file_name: Some("acme-may.hcbrowser".to_string()),
        content_type: "application/octet-stream".to_string(),
        size_bytes: 27,
        sha256: "sha256:8f1b8a0ad5d63d57d8a0cce1e0a6a4d6f4af0c9d61678f2f48a88e3dd2dbe4f9"
            .to_string(),
        storage_backend: "db_inline".to_string(),
        storage_ref: "artifact_10002".to_string(),
        carrier_valid_until: Some("2026-05-20T10:00:00Z".to_string()),
        encryption_protocol: "delivery_account_bundle_v2".to_string(),
        encryption_version: "2".to_string(),
        secret_kind: "browser_file_unlock_code".to_string(),
        created_by: "user_ops_admin".to_string(),
        created_at: "2026-05-05T10:05:00Z".to_string(),
        updated_at: "2026-05-05T10:05:00Z".to_string(),
        superseded_at: None,
        superseded_by: None,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
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
