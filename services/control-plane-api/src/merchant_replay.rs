use core_domain::{
    AdmissionResult, ConfigSnapshot, RelayCheckStatus, RelayEvaluation, RelayEvaluationId,
    RelayEvaluationRunnerMode, RelayEvaluationVerdict, ReplayCapsuleId, RouteReceipt,
    RouteReceiptId, ScoreBreakdown, TenantId, TrialConnection,
};
use time::OffsetDateTime;

pub fn next_id_suffix() -> String {
    OffsetDateTime::now_utc().unix_timestamp_nanos().to_string()
}

pub fn replay_upstream_error_code(connection: &TrialConnection) -> String {
    let endpoint = connection.endpoint_base_url.to_ascii_lowercase();

    if endpoint.contains("vertex") {
        "provider_signature_mismatch".to_string()
    } else if endpoint.contains("bedrock") {
        "provider_channel_proxy".to_string()
    } else {
        "protocol_shape_warning".to_string()
    }
}

pub fn build_merchant_replay_route_receipt(
    tenant_id: &TenantId,
    config_snapshot: &ConfigSnapshot,
    route_receipt_id: RouteReceiptId,
    request_id: String,
    trace_id: String,
    model_alias: String,
    created_at: String,
) -> RouteReceipt {
    RouteReceipt {
        route_receipt_id,
        tenant_id: tenant_id.clone(),
        project_id: config_snapshot.project_id.clone(),
        route_policy_id: config_snapshot.route_policy_id.clone(),
        request_id,
        trace_id,
        protocol_family: "openai_chat".to_string(),
        model_alias,
        config_snapshot_id: config_snapshot.config_snapshot_id.clone(),
        admission_result: AdmissionResult::Admitted,
        selected_target: config_snapshot.provider_resource_ids.first().cloned(),
        excluded_targets: Vec::new(),
        score_breakdown: ScoreBreakdown {
            latency: 0.8,
            cost: 0.7,
            health: 1.0,
            trust: 0.9,
        },
        fallback_transitions: Vec::new(),
        normalized_error: None,
        failure_reason: Some("merchant_replay_evaluation".to_string()),
        created_at,
    }
}

pub fn build_relay_evaluation(
    tenant_id: &TenantId,
    connection: &TrialConnection,
    replay_capsule_id: ReplayCapsuleId,
    created_at: String,
) -> RelayEvaluation {
    let endpoint = connection.endpoint_base_url.to_ascii_lowercase();
    let detected_channel = if endpoint.contains("vertex") {
        Some("vertex".to_string())
    } else if endpoint.contains("bedrock") {
        Some("aws-bedrock".to_string())
    } else {
        None
    };
    let protocol_status = if detected_channel.is_some() {
        RelayCheckStatus::Warning
    } else {
        RelayCheckStatus::Pass
    };
    let token_status = if connection
        .target_model
        .to_ascii_lowercase()
        .contains("flash")
    {
        RelayCheckStatus::Warning
    } else {
        RelayCheckStatus::Pass
    };
    let verdict = if detected_channel.is_some() {
        RelayEvaluationVerdict::Warning
    } else {
        RelayEvaluationVerdict::Healthy
    };
    let overall_score = if detected_channel.is_some() { 82 } else { 91 };

    RelayEvaluation {
        relay_evaluation_id: RelayEvaluationId::parse(format!("reval_{}", next_id_suffix()))
            .unwrap(),
        tenant_id: tenant_id.clone(),
        trial_connection_id: connection.trial_connection_id.clone(),
        replay_capsule_id,
        provider_label: connection.provider_label.clone(),
        endpoint_base_url: connection.endpoint_base_url.clone(),
        target_model: connection.target_model.clone(),
        runner_mode: RelayEvaluationRunnerMode::Simulated,
        sample_request_count: 5,
        estimated_tokens_saved: 2400,
        overall_score,
        verdict,
        fingerprint_status: RelayCheckStatus::Pass,
        protocol_status,
        token_status,
        multimodal_status: RelayCheckStatus::NotTested,
        detected_channel,
        summary:
            "Replay-ready evaluation recorded. Review protocol consistency before spending live token budget."
                .to_string(),
        created_at,
    }
}
