use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("expected id with prefix `{expected_prefix}` but received `{actual}`")]
    InvalidPrefixedId {
        expected_prefix: &'static str,
        actual: String,
    },
    #[error("field `{field}` must not be empty")]
    EmptyField { field: &'static str },
    #[error("field `{field}` must be a lowercase slug with digits or hyphens")]
    InvalidSlug { field: &'static str, actual: String },
    #[error("field `{field}` must use an https url")]
    InvalidHttpsUrl { field: &'static str, actual: String },
    #[error("field `{field}` must not be empty")]
    EmptyCollection { field: &'static str },
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::EmptyField { field })
    } else {
        Ok(())
    }
}

fn validate_slug(field: &'static str, value: &str) -> Result<(), DomainError> {
    validate_non_empty(field, value)?;

    let is_valid = value.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    });

    if is_valid {
        Ok(())
    } else {
        Err(DomainError::InvalidSlug {
            field,
            actual: value.to_string(),
        })
    }
}

fn validate_https_url(field: &'static str, value: &str) -> Result<(), DomainError> {
    validate_non_empty(field, value)?;
    if value.starts_with("https://") {
        Ok(())
    } else {
        Err(DomainError::InvalidHttpsUrl {
            field,
            actual: value.to_string(),
        })
    }
}

const fn validate_non_empty_slice<T>(field: &'static str, values: &[T]) -> Result<(), DomainError> {
    if values.is_empty() {
        Err(DomainError::EmptyCollection { field })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, ToSchema)]
#[schemars(with = "String")]
#[schema(value_type = String)]
pub struct ServiceName(pub String);

impl ServiceName {
    /// # Errors
    ///
    /// Returns an error when the provided service name is blank.
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate_non_empty("service_name", &value)?;
        Ok(Self(value))
    }
}

impl From<&str> for ServiceName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl TryFrom<String> for ServiceName {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<ServiceName> for String {
    fn from(value: ServiceName) -> Self {
        value.0
    }
}

impl Serialize for ServiceName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ServiceName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

macro_rules! prefixed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema, ToSchema)]
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub struct $name(String);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            /// # Errors
            ///
            /// Returns an error when the provided id does not use the expected prefix.
            pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
                let value = value.into();
                if value.len() > Self::PREFIX.len() && value.starts_with(Self::PREFIX) {
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

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResourceStatus {
    Active,
    Disabled,
    Draining,
    Quarantined,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceClass {
    OfficialApi,
    OfficialGateway,
    ByoCustomerCredential,
    DedicatedManagedAccount,
    SharedBrokeredPool,
    UnofficialClientChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CredentialOwnerType {
    Platform,
    Tenant,
    Project,
    Partner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Degraded,
    Quarantined,
    Draining,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionResult {
    Admitted,
    RejectedBudget,
    RejectedRateLimit,
    RejectedConcurrency,
    RejectedPolicy,
    RejectedNoCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuotaReserveStrategy {
    None,
    EstimateThenReserve,
    FixedReserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsagePhase {
    Reserve,
    Partial,
    Final,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RedactionTier {
    MetadataOnly,
    StructuredRedacted,
    FullPayloadRetention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentScope {
    Shared,
    TenantDedicated,
    ProjectDedicated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    ApiKey,
    OAuthClientCredentials,
    SessionBroker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigSnapshotStatus {
    Draft,
    Active,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Tenant {
    pub tenant_id: TenantId,
    pub slug: String,
    pub display_name: String,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl Tenant {
    /// # Errors
    ///
    /// Returns an error when the tenant slug or display name is invalid.
    pub fn new(
        tenant_id: TenantId,
        slug: impl Into<String>,
        display_name: impl Into<String>,
        version: u64,
        created_at: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let tenant = Self {
            tenant_id,
            slug: slug.into(),
            display_name: display_name.into(),
            version,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        };
        tenant.validate()?;
        Ok(tenant)
    }

    /// # Errors
    ///
    /// Returns an error when the tenant payload is invalid.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_slug("slug", &self.slug)?;
        validate_non_empty("display_name", &self.display_name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct Project {
    pub project_id: ProjectId,
    pub tenant_id: TenantId,
    pub slug: String,
    pub display_name: String,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    /// # Errors
    ///
    /// Returns an error when the project slug or display name is invalid.
    pub fn new(
        project_id: ProjectId,
        tenant_id: TenantId,
        slug: impl Into<String>,
        display_name: impl Into<String>,
        version: u64,
        created_at: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let project = Self {
            project_id,
            tenant_id,
            slug: slug.into(),
            display_name: display_name.into(),
            version,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        };
        project.validate()?;
        Ok(project)
    }

    /// # Errors
    ///
    /// Returns an error when the project payload is invalid.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_slug("slug", &self.slug)?;
        validate_non_empty("display_name", &self.display_name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_tool_calling: bool,
    pub supports_json_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ProviderResource {
    pub provider_resource_id: ProviderResourceId,
    pub tenant_id: TenantId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub provider_id: String,
    pub name: String,
    pub status: ProviderResourceStatus,
    pub provenance_class: ProvenanceClass,
    pub credential_owner_type: CredentialOwnerType,
    pub deployment_scope: DeploymentScope,
    pub region: String,
    pub endpoint_base_url: String,
    pub auth_kind: AuthKind,
    pub health_state: HealthState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_policy_id: Option<BudgetPolicyId>,
    pub capabilities: ProviderCapabilities,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl ProviderResource {
    /// # Errors
    ///
    /// Returns an error when the provider resource contains invalid operator-facing fields.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_non_empty("provider_id", &self.provider_id)?;
        validate_non_empty("name", &self.name)?;
        validate_non_empty("region", &self.region)?;
        validate_https_url("endpoint_base_url", &self.endpoint_base_url)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BudgetPolicy {
    pub budget_policy_id: BudgetPolicyId,
    pub tenant_id: TenantId,
    pub display_name: String,
    pub quota_reserve_strategy: QuotaReserveStrategy,
    pub currency: String,
    pub monthly_limit_micros: i64,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RoutePolicy {
    pub route_policy_id: RoutePolicyId,
    pub tenant_id: TenantId,
    pub display_name: String,
    pub protocol_family: String,
    pub model_alias: String,
    pub required_capabilities: Vec<String>,
    pub preferred_regions: Vec<String>,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl RoutePolicy {
    /// # Errors
    ///
    /// Returns an error when the route policy is missing required identifiers or capabilities.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_non_empty("display_name", &self.display_name)?;
        validate_non_empty("protocol_family", &self.protocol_family)?;
        validate_non_empty("model_alias", &self.model_alias)?;
        validate_non_empty_slice("required_capabilities", &self.required_capabilities)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ConfigSnapshot {
    pub config_snapshot_id: ConfigSnapshotId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub revision: u64,
    pub status: ConfigSnapshotStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<String>,
    pub provider_resource_ids: Vec<ProviderResourceId>,
    pub route_policy_id: RoutePolicyId,
    pub budget_policy_id: BudgetPolicyId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MonetaryAmount {
    pub currency: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ScoreBreakdown {
    pub latency: f32,
    pub cost: f32,
    pub health: f32,
    pub trust: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ExcludedTarget {
    pub provider_resource_id: ProviderResourceId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FallbackTransition {
    pub from_provider_resource_id: ProviderResourceId,
    pub to_provider_resource_id: ProviderResourceId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NormalizedError {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_status_code: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_issues: Vec<ValidationIssue>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, String>,
}

impl NormalizedError {
    /// # Errors
    ///
    /// Returns an error when the normalized error payload is missing core fields.
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_non_empty("code", &self.code)?;
        validate_non_empty("message", &self.message)?;
        validate_non_empty("request_id", &self.request_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ErrorEnvelope {
    pub error: NormalizedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RouteReceipt {
    pub route_receipt_id: RouteReceiptId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub request_id: String,
    pub trace_id: String,
    pub protocol_family: String,
    pub model_alias: String,
    pub config_snapshot_id: ConfigSnapshotId,
    pub admission_result: AdmissionResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<ProviderResourceId>,
    pub excluded_targets: Vec<ExcludedTarget>,
    pub score_breakdown: ScoreBreakdown,
    pub fallback_transitions: Vec<FallbackTransition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_error: Option<NormalizedError>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageMetrics {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UsageEvent {
    pub usage_event_id: UsageEventId,
    pub route_receipt_id: RouteReceiptId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider_resource_id: ProviderResourceId,
    pub model_alias: String,
    pub phase: UsagePhase,
    pub idempotency_key: String,
    pub usage: UsageMetrics,
    pub estimated_cost: MonetaryAmount,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LedgerEntry {
    pub ledger_entry_id: LedgerEntryId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_event_id: Option<UsageEventId>,
    pub ledger_entry_type: LedgerEntryType,
    pub amount_micros: i64,
    pub currency: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct AuditEvent {
    pub audit_event_id: String,
    pub actor: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub trace_id: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NormalizedRequestSummary {
    pub protocol_family: String,
    pub model_alias: String,
    pub estimated_prompt_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct UpstreamErrorSummary {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
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
        AdmissionResult, AuthKind, ConfigSnapshotId, CredentialOwnerType, DeploymentScope,
        DomainError, ErrorEnvelope, HealthState, MonetaryAmount, NormalizedError, ProjectId,
        ProvenanceClass, ProviderCapabilities, ProviderResource, ProviderResourceId,
        ProviderResourceStatus, RoutePolicy, RouteReceipt, RouteReceiptId, ScoreBreakdown, Tenant,
        TenantId, UsageEvent, UsageEventId, UsageMetrics, UsagePhase,
    };
    use serde_json::{Value, json};

    #[test]
    fn provider_resource_id_rejects_wrong_prefix() {
        let error = ProviderResourceId::parse("tenant_wrong").unwrap_err();

        assert_eq!(
            error,
            DomainError::InvalidPrefixedId {
                expected_prefix: "prvrsrc_",
                actual: "tenant_wrong".to_string(),
            }
        );
    }

    #[test]
    fn tenant_validation_rejects_invalid_slug() {
        let error = Tenant::new(
            TenantId::parse("tenant_acme").unwrap(),
            "Acme Ops",
            "Acme",
            1,
            "2026-04-20T00:00:00Z",
            "2026-04-20T00:00:00Z",
        )
        .unwrap_err();

        assert_eq!(
            error,
            DomainError::InvalidSlug {
                field: "slug",
                actual: "Acme Ops".to_string(),
            }
        );
    }

    #[test]
    fn provider_resource_validation_rejects_non_https_endpoints() {
        let resource = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(ProjectId::parse("proj_core").unwrap()),
            provider_id: "openai".to_string(),
            name: "openai-us-east-primary".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Platform,
            deployment_scope: DeploymentScope::Shared,
            region: "us-east-1".to_string(),
            endpoint_base_url: "http://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Healthy,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
            },
            version: 7,
            created_at: "2026-04-20T00:00:00Z".to_string(),
            updated_at: "2026-04-20T00:00:00Z".to_string(),
        };

        assert_eq!(
            resource.validate().unwrap_err(),
            DomainError::InvalidHttpsUrl {
                field: "endpoint_base_url",
                actual: "http://api.openai.com/v1".to_string(),
            }
        );
    }

    #[test]
    fn route_policy_requires_capabilities() {
        let policy = RoutePolicy {
            route_policy_id: super::RoutePolicyId::parse("routepol_default").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            display_name: "Default Route".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            required_capabilities: Vec::new(),
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: "2026-04-20T00:00:00Z".to_string(),
            updated_at: "2026-04-20T00:00:00Z".to_string(),
        };

        assert_eq!(
            policy.validate().unwrap_err(),
            DomainError::EmptyCollection {
                field: "required_capabilities",
            }
        );
    }

    #[test]
    fn route_receipt_serializes_with_documented_keys() {
        let receipt = RouteReceipt {
            route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            request_id: "req_123".to_string(),
            trace_id: "trace_123".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
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
            normalized_error: None,
            created_at: "2026-04-20T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&receipt).unwrap();

        assert_eq!(json["route_receipt_id"], "routercpt_123");
        assert_eq!(json["tenant_id"], "tenant_acme");
        assert_eq!(json["config_snapshot_id"], "cfgsnap_123");
        assert_eq!(json["admission_result"], "admitted");
        assert_eq!(json["selected_target"], "prvrsrc_123");
    }

    #[test]
    fn normalized_error_envelope_round_trips() {
        let envelope = ErrorEnvelope {
            error: NormalizedError {
                code: "validation_failed".to_string(),
                message: "Route policy requires at least one capability.".to_string(),
                request_id: "req_123".to_string(),
                retryable: false,
                upstream_code: None,
                upstream_status_code: None,
                validation_issues: vec![super::ValidationIssue {
                    field: "required_capabilities".to_string(),
                    message: "expected at least one value".to_string(),
                }],
                details: std::collections::BTreeMap::from([(
                    "resource".to_string(),
                    "route_policy".to_string(),
                )]),
            },
        };

        envelope.error.validate().unwrap();

        let json = serde_json::to_value(&envelope).unwrap();
        let reparsed: ErrorEnvelope = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(reparsed, envelope);
        assert_eq!(json["error"]["code"], "validation_failed");
        assert_eq!(
            json["error"]["validation_issues"][0]["field"],
            "required_capabilities"
        );
    }

    #[test]
    fn usage_event_round_trips() {
        let event = UsageEvent {
            usage_event_id: UsageEventId::parse("usageevt_123").unwrap(),
            route_receipt_id: RouteReceiptId::parse("routercpt_123").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            provider_resource_id: ProviderResourceId::parse("prvrsrc_123").unwrap(),
            model_alias: "reasoning-fast".to_string(),
            phase: UsagePhase::Final,
            idempotency_key: "usageevt_123:final".to_string(),
            usage: UsageMetrics {
                input_tokens: 1200,
                output_tokens: 320,
                cached_input_tokens: 64,
            },
            estimated_cost: MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.1420".to_string(),
            },
            recorded_at: "2026-04-20T00:00:05Z".to_string(),
        };

        let value = serde_json::to_value(&event).unwrap();
        let reparsed: UsageEvent = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(reparsed, event);
        assert_eq!(value["phase"], "final");
        assert_eq!(
            value["usage"],
            json!({
                "input_tokens": 1200,
                "output_tokens": 320,
                "cached_input_tokens": 64,
            })
        );
    }

    #[test]
    fn prefixed_id_deserialization_enforces_validation() {
        let value: Value = json!("tenant_valid");
        let id: TenantId = serde_json::from_value(value).unwrap();
        assert_eq!(id.as_str(), "tenant_valid");

        let invalid = serde_json::from_value::<TenantId>(json!("tenant_"));
        assert!(invalid.is_err());
    }
}
