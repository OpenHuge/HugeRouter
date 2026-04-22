use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceName(pub String);

impl From<&str> for ServiceName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("expected id with prefix `{expected_prefix}` but received `{actual}`")]
    InvalidPrefixedId {
        expected_prefix: &'static str,
        actual: String,
    },
}

macro_rules! prefixed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            /// # Errors
            ///
            /// Returns an error when the provided id does not use the expected prefix.
            pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
                let value = value.into();
                if value.starts_with(Self::PREFIX) {
                    Ok(Self(value))
                } else {
                    Err(DomainError::InvalidPrefixedId {
                        expected_prefix: Self::PREFIX,
                        actual: value,
                    })
                }
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = DomainError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = DomainError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

prefixed_id!(TenantId, "tenant_");
prefixed_id!(ProjectId, "proj_");
prefixed_id!(CredentialId, "cred_");
prefixed_id!(ProviderResourceId, "prvrsrc_");
prefixed_id!(RoutePolicyId, "routepol_");
prefixed_id!(BudgetPolicyId, "budgetpol_");
prefixed_id!(ConfigSnapshotId, "cfgsnap_");
prefixed_id!(RouteReceiptId, "routercpt_");
prefixed_id!(UsageEventId, "usageevt_");
prefixed_id!(LedgerEntryId, "ledger_");
prefixed_id!(ReplayCapsuleId, "replay_");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResourceStatus {
    Active,
    Disabled,
    Draining,
    Quarantined,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceClass {
    OfficialApi,
    OfficialGateway,
    ByoCustomerCredential,
    DedicatedManagedAccount,
    SharedBrokeredPool,
    UnofficialClientChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialOwnerType {
    Platform,
    Tenant,
    Project,
    Partner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Degraded,
    Quarantined,
    Draining,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionResult {
    Admitted,
    RejectedBudget,
    RejectedRateLimit,
    RejectedConcurrency,
    RejectedPolicy,
    RejectedNoCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaReserveStrategy {
    None,
    EstimateThenReserve,
    FixedReserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsagePhase {
    Reserve,
    Partial,
    Final,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryType {
    UsageDebit,
    ManualCredit,
    PromoCredit,
    RefundReversal,
    MinimumFee,
    ReserveHold,
    ReserveRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionTier {
    MetadataOnly,
    StructuredRedacted,
    FullPayloadRetention,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderResource {
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub status: ProviderResourceStatus,
    pub provenance_class: ProvenanceClass,
    pub credential_owner_type: CredentialOwnerType,
    pub health_state: HealthState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetPolicy {
    pub budget_policy_id: BudgetPolicyId,
    pub display_name: String,
    pub quota_reserve_strategy: QuotaReserveStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePolicy {
    pub route_policy_id: RoutePolicyId,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub config_snapshot_id: ConfigSnapshotId,
    pub activated_at: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub latency: f32,
    pub cost: f32,
    pub health: f32,
    pub trust: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedTarget {
    pub provider_resource_id: ProviderResourceId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackTransition {
    pub from_provider_resource_id: ProviderResourceId,
    pub to_provider_resource_id: ProviderResourceId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteReceipt {
    pub route_receipt_id: RouteReceiptId,
    pub request_id: String,
    pub trace_id: String,
    pub config_snapshot_id: ConfigSnapshotId,
    pub admission_result: AdmissionResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<ProviderResourceId>,
    pub excluded_targets: Vec<ExcludedTarget>,
    pub score_breakdown: ScoreBreakdown,
    pub fallback_transitions: Vec<FallbackTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub usage_event_id: UsageEventId,
    pub route_receipt_id: RouteReceiptId,
    pub phase: UsagePhase,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub ledger_entry_id: LedgerEntryId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_event_id: Option<UsageEventId>,
    pub ledger_entry_type: LedgerEntryType,
    pub amount_micros: i64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub audit_event_id: String,
    pub actor: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRequestSummary {
    pub protocol_family: String,
    pub model_alias: String,
    pub estimated_prompt_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpstreamErrorSummary {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayCapsule {
    pub replay_capsule_id: ReplayCapsuleId,
    pub request_id: String,
    pub trace_id: String,
    pub route_receipt_id: RouteReceiptId,
    pub config_snapshot_id: ConfigSnapshotId,
    pub redaction_tier: RedactionTier,
    pub normalized_request_summary: NormalizedRequestSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_error_summary: Option<UpstreamErrorSummary>,
}

#[cfg(test)]
mod tests {
    use super::{
        AdmissionResult, ConfigSnapshotId, ProviderResourceId, RedactionTier, ReplayCapsule,
        ReplayCapsuleId, RouteReceipt, RouteReceiptId, ScoreBreakdown,
    };

    #[test]
    fn provider_resource_id_rejects_wrong_prefix() {
        let error = ProviderResourceId::parse("tenant_wrong").unwrap_err();

        assert_eq!(
            error,
            super::DomainError::InvalidPrefixedId {
                expected_prefix: "prvrsrc_",
                actual: "tenant_wrong".to_string(),
            }
        );
    }

    #[test]
    fn route_receipt_serializes_with_documented_keys() {
        let receipt = RouteReceipt {
            route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_123").unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(ProviderResourceId::parse("prvrsrc_123").unwrap()),
            excluded_targets: Vec::new(),
            score_breakdown: ScoreBreakdown {
                latency: 0.82,
                cost: 0.66,
                health: 0.97,
                trust: 1.0,
            },
            fallback_transitions: Vec::new(),
        };

        let json = serde_json::to_value(&receipt).unwrap();

        assert_eq!(json["route_receipt_id"], "routercpt_123");
        assert_eq!(json["config_snapshot_id"], "cfgsnap_123");
        assert_eq!(json["admission_result"], "admitted");
        assert_eq!(json["selected_target"], "prvrsrc_123");
    }

    #[test]
    fn replay_capsule_matches_redaction_first_shape() {
        let capsule = ReplayCapsule {
            replay_capsule_id: ReplayCapsuleId::parse("replay_123").unwrap(),
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_123").unwrap(),
            redaction_tier: RedactionTier::MetadataOnly,
            normalized_request_summary: super::NormalizedRequestSummary {
                protocol_family: "openai_chat".to_string(),
                model_alias: "reasoning-fast".to_string(),
                estimated_prompt_tokens: 12_000,
            },
            upstream_error_summary: Some(super::UpstreamErrorSummary {
                code: "upstream_timeout".to_string(),
            }),
        };

        let json = serde_json::to_value(&capsule).unwrap();

        assert_eq!(json["replay_capsule_id"], "replay_123");
        assert_eq!(json["redaction_tier"], "metadata_only");
        assert_eq!(json["normalized_request_summary"]["estimated_prompt_tokens"], 12_000);
        assert_eq!(json["upstream_error_summary"]["code"], "upstream_timeout");
    }
}
