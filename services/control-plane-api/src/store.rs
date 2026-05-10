#![allow(
    clippy::assigning_clones,
    clippy::branches_sharing_code,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::format_collect,
    clippy::format_push_string,
    clippy::items_after_test_module,
    clippy::match_like_matches_macro,
    clippy::needless_raw_string_hashes,
    clippy::needless_pass_by_value,
    clippy::option_as_ref_deref,
    clippy::or_fun_call,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::significant_drop_tightening,
    clippy::struct_field_names,
    clippy::unused_async,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::trivially_copy_pass_by_ref
)]

use crate::merchant_replay::{
    build_merchant_replay_route_receipt, build_relay_evaluation, next_id_suffix,
    replay_upstream_error_code,
};
use crate::pricing_catalog::{
    default_pricing_catalog_response, load_pricing_catalog, pricing_catalog_response,
    seed_default_pricing_catalog, simulate_pricing, simulate_pricing_with_catalog,
};
use crate::store_schema::{MIGRATIONS, REQUIRED_TABLES};
use anyhow::{Context, Result, anyhow};
use core_domain::{
    AdmissionResult, AuthKind, AuthLoginResult, AuthProvider, AuthProviderAvailability,
    AuthProviderLink, AuthSession, AuthSessionId, AuthSessionState, BudgetPolicyId,
    CardDeliveryKind, CardProduct, CardProductId, CardProductStatus, ConfigSnapshot,
    ConfigSnapshotId, ConfigSnapshotStatus, CredentialOwnerType, DEFAULT_OWNER_ACCOUNT_ID,
    DeploymentScope, HealthState, LogoutResponse, MerchantFulfillmentMode, MerchantShop,
    MerchantShopId, MerchantShopStatus, MonetaryAmount, NormalizedRequestSummary, OAuthProvider,
    Project, ProjectId, ProvenanceClass, ProviderCapabilities, ProviderResource,
    ProviderResourceId, ProviderResourceStatus, RedactionTier, RelayCheckStatus, RelayEvaluation,
    RelayEvaluationId, RelayEvaluationRunnerMode, RelayEvaluationVerdict, ReplayCapsule,
    ReplayCapsuleId, RoutePolicy, RoutePolicyId, RouteReceipt, RouteReceiptId, Tenant, TenantId,
    TenantMembership, TenantMembershipId, TenantMembershipRole, TenantMembershipStatus,
    TenantSummary, TrialConnection, TrialConnectionId, TrialConnectionStatus,
    UnlinkAuthProviderResponse, UpstreamErrorSummary, UsageMetrics, UserId, UserIdentity,
};
use metering::{PricingCatalog, default_budget_micros_for_scope};
use protocol_ir::{
    BalanceProjection, BalanceProjectionResponse, BillingExportJob, BillingExportJobResponse,
    BillingExportJobsResponse, BillingExportRequest, ConfigSnapshotResponse, Delivery,
    DeliveryActivation, DeliveryActivationResponse, DeliveryArtifact, DeliveryArtifactResponse,
    DeliveryArtifactsResponse, DeliveryCode, DeliveryDownloadGrant,
    DeliveryDownloadGrantIssueResponse, DeliveryDownloadGrantResponse, DeliveryEntitlement,
    DeliveryLifecycleEvent, DeliveryLifecycleEventsResponse, DeliveryLifecycleResponse,
    DeliveryOneTimeCodes, DeliveryOperationsDetail, DeliveryOperationsDetailResponse,
    DeliveryOperationsException, DeliveryOperationsExceptionsResponse, DeliveryOperationsOverview,
    DeliveryOperationsOverviewResponse, DeliveryOperationsStatusCount,
    DeliveryOperationsTimelineEvent, DeliveryOperationsTimelineResponse, DeliveryOperationsTotals,
    DeliveryPrepareResponse, DeliveryProjection, DeliveryResponse, DeliveryServiceSegment,
    DeliveryServiceSegmentsResponse, DeliveryUploadBatch, DeliveryUploadBatchItem,
    DeliveryUploadBatchItemsResponse, DeliveryUploadBatchResponse, OpeningGrant,
    OpeningGrantsResponse, PricingCatalogResponse, PricingSimulationRequest,
    PricingSimulationResponse, ProjectsResponse, ProtocolFamily, ProviderResourcesResponse,
    RenewalIntent, RenewalIntentsResponse, RouteDiagnosticDecision, RouteDiagnosticTarget,
    RouteDiagnosticsResponse, RoutePoliciesResponse, RouteReceiptDecisionTraceStep,
    RouteReceiptDiagnosticsResponse, RouteReceiptPolicyCheck, RouteReceiptProviderAttempt,
    RouteReceiptResponse, RouteReceiptSummary, RouteSimulationRequest, RouteSimulationResponse,
    SaleReadiness, SaleReadyCheck, SaleReadyHandoff, SaleReadyPackageResponse, TenantsResponse,
    UsageBreakdownResponse, UsageBreakdownRow, UsageSummary, UsageSummaryResponse,
};
use routing_engine::{
    ProviderTargetKind, ProviderTargetRuntime, RoutingConfig, RoutingRequest, health_state_slug,
    is_health_blocked, provider_capability_gaps, provider_supports_capability,
    provider_supports_protocol_family, route_capability_supported_by_provider_capabilities,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres, Row, postgres::PgPoolOptions, types::Json};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, RwLock},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

mod delivery_redemption_units;
mod merchant_product_orders;
mod merchant_product_orders_postgres;
mod opening_grants;

pub use delivery_redemption_units::{
    DeliveryRedemptionUnitDraft, DeliveryRedemptionUnitsPrepareResult,
    owner_redemption_capacity_blocker_for_memory, owner_redemption_capacity_blocker_for_postgres,
};
pub use merchant_product_orders::{
    MERCHANT_ORDER_STATUS_FULFILLED, MerchantPickupResponse, MerchantProductFulfillmentDraft,
    MerchantProductFulfillmentResult, MerchantProductInventoryRecord,
    MerchantProductOrderCreateResult, MerchantProductOrderDraft, MerchantProductOrderRecord,
    MerchantProductOrderResponse, MerchantProductPrepayDraft, MerchantPublicShopResponse,
};
use opening_grants::{
    OPENING_GRANT_OWNER_LOCK_INSERT_SQL, OPENING_GRANT_OWNER_LOCK_SELECT_SQL,
    opening_grant_create_blocker,
};
pub use opening_grants::{OpeningGrantCreateResult, OpeningGrantDraft, OpeningGrantRecord};

pub const SESSION_TTL_SECONDS: u64 = 60 * 60 * 8;
pub const ACTIVE_CONFIG_ALIAS: &str = "active";
const DEFAULT_OAUTH_POOL_RUNTIME_LEASE_TTL_SECONDS: u64 = 120;
const DEFAULT_OAUTH_POOL_SESSION_BINDING_TTL_SECONDS: u64 = 60 * 60 * 24;
const OAUTH_POOL_COOLDOWN_SECONDS: u64 = 60 * 5;
const OAUTH_POOL_MAX_COOLDOWN_SECONDS: u64 = 60 * 60;
const PROVIDER_RESOURCE_INTAKE_PENDING_PROBE: &str = "manual_intake_pending_live_probe";
const PROVIDER_RESOURCE_INTAKE_PENDING_MESSAGE: &str =
    "Resource registered but not route eligible until live probe or operator approval.";
pub const DELIVERY_SOURCE_MANUAL_OPERATOR: &str = "manual_operator";
pub const DELIVERY_STATUS_PREPARED: &str = "prepared";
pub const DELIVERY_STATUS_REVOKED: &str = "revoked";
pub const DELIVERY_STATUS_EXPIRED: &str = "expired";
pub const DELIVERY_CODE_TYPE_REDEMPTION: &str =
    crate::delivery_redemption_policy::DELIVERY_CODE_TYPE_REDEMPTION;
pub const DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK: &str =
    crate::delivery_redemption_policy::DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK;
pub const DELIVERY_CODE_STATUS_ACTIVE: &str = "active";
pub const DELIVERY_CODE_STATUS_USED: &str = "used";
pub const DELIVERY_CODE_FORMAT_REDEMPTION_V2: &str =
    crate::delivery_redemption_policy::DELIVERY_CODE_FORMAT_REDEMPTION_V2;
pub const DELIVERY_CODE_FORMAT_BROWSER_FILE_UNLOCK_V2: &str =
    crate::delivery_redemption_policy::DELIVERY_CODE_FORMAT_BROWSER_FILE_UNLOCK_V2;
pub const DELIVERY_ENTITLEMENT_STATUS_ACTIVE: &str = "active";
pub const DELIVERY_ENTITLEMENT_STATUS_PENDING_ACTIVATION: &str = "pending_activation";
pub const DELIVERY_ENTITLEMENT_STATUS_REVOKED: &str = "revoked";
pub const DELIVERY_ENTITLEMENT_STATUS_SUSPENDED: &str = "suspended";
pub const DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY: &str = "needs_manual_supply";
pub const DELIVERY_ARTIFACT_KIND_BROWSER_ACCOUNT_BUNDLE: &str = "browser_account_bundle";
pub const DELIVERY_ARTIFACT_STATUS_ACTIVE: &str = "active";
pub const DELIVERY_ARTIFACT_STATUS_SUPERSEDED: &str = "superseded";
pub const DELIVERY_ARTIFACT_STATUS_REVOKED: &str = "revoked";
pub const DELIVERY_ARTIFACT_STORAGE_BACKEND_DB_INLINE: &str = "db_inline";
pub const DELIVERY_ARTIFACT_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_PROTOCOL_V2: &str =
    crate::delivery_redemption_policy::DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_PROTOCOL_V2;
pub const DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_VERSION_V2: &str =
    crate::delivery_redemption_policy::DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_VERSION_V2;
pub const DELIVERY_ACTIVATION_STATUS_ACTIVATED: &str = "activated";
pub const DELIVERY_ACTIVATION_SOURCE_REDEMPTION_CODE: &str = "redemption_code";
pub const DELIVERY_DOWNLOAD_GRANT_STATUS_ACTIVE: &str = "active";
pub const DELIVERY_DOWNLOAD_GRANT_STATUS_USED: &str = "used";
pub const DELIVERY_DOWNLOAD_GRANT_STATUS_EXPIRED: &str = "expired";
pub const DELIVERY_DOWNLOAD_GRANT_STATUS_REVOKED: &str = "revoked";
pub const DELIVERY_DOWNLOAD_GRANT_TTL_SECONDS: u64 = 15 * 60;
pub const DELIVERY_DOWNLOAD_GRANT_MAX_USES: u32 = 1;
pub const DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE: &str = "active";
pub const DELIVERY_SERVICE_SEGMENT_STATUS_SCHEDULED: &str = "scheduled";
pub const DELIVERY_SERVICE_SEGMENT_STATUS_EXPIRED: &str = "expired";
pub const DELIVERY_SERVICE_SEGMENT_STATUS_REVOKED: &str = "revoked";
pub const DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED: &str = "segment_created";
pub const DELIVERY_LIFECYCLE_EVENT_NEEDS_MANUAL_SUPPLY: &str = "needs_manual_supply";
pub const DELIVERY_LIFECYCLE_EVENT_RENEWED: &str = "entitlement_renewed";
pub const DELIVERY_LIFECYCLE_EVENT_EXPIRED: &str = "entitlement_expired";
pub const DELIVERY_UPLOAD_BATCH_STATUS_QUEUED: &str = "queued";
pub const DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING: &str = "processing";
pub const DELIVERY_UPLOAD_BATCH_STATUS_SUCCEEDED: &str = "succeeded";
pub const DELIVERY_UPLOAD_BATCH_STATUS_PARTIALLY_FAILED: &str = "partially_failed";
pub const DELIVERY_UPLOAD_BATCH_STATUS_FAILED: &str = "failed";
pub const DELIVERY_UPLOAD_ITEM_STATUS_PENDING: &str = "pending";
pub const DELIVERY_UPLOAD_ITEM_STATUS_ACCEPTED: &str = "accepted";
pub const DELIVERY_UPLOAD_ITEM_STATUS_DUPLICATE: &str = "duplicate";
pub const DELIVERY_UPLOAD_ITEM_STATUS_REJECTED: &str = "rejected";
pub const DELIVERY_UPLOAD_ITEM_STATUS_FAILED: &str = "failed";
const PROVIDER_CATALOG: &[(AuthProvider, &str, &str)] = &[
    (
        AuthProvider::Email,
        "Continue with Email",
        "/api/control-plane/auth/email/start",
    ),
    (
        AuthProvider::Github,
        "Continue with GitHub",
        "/api/control-plane/auth/oauth/github/start",
    ),
    (
        AuthProvider::Google,
        "Continue with Google",
        "/api/control-plane/auth/oauth/google/start",
    ),
    (
        AuthProvider::Wechat,
        "Continue with WeChat",
        "/api/control-plane/auth/oauth/wechat/start",
    ),
];

const OIDC_PROVIDER_CATALOG_ENTRY: (AuthProvider, &str, &str) = (
    AuthProvider::Oidc,
    "Continue with Enterprise SSO",
    "/api/control-plane/auth/oauth/oidc/start",
);

#[derive(Debug, Clone)]
pub enum StoreMode {
    Memory(Arc<RwLock<MemoryStore>>),
    Postgres(PostgresStore),
}

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pub(crate) pool: Pool<Postgres>,
}

#[derive(Debug, Default)]
pub struct MemoryStore {
    tenants: Vec<Tenant>,
    projects: Vec<Project>,
    provider_resources: Vec<ProviderResource>,
    route_policies: Vec<RoutePolicy>,
    config_snapshots: Vec<ConfigSnapshot>,
    merchant_shops: Vec<MerchantShop>,
    card_products: Vec<CardProduct>,
    merchant_product_inventory: Vec<MerchantProductInventoryRecord>,
    merchant_product_orders: Vec<MerchantProductOrderRecord>,
    wechat_user_openids: HashMap<String, String>,
    trial_connections: Vec<TrialConnection>,
    relay_evaluations: Vec<RelayEvaluation>,
    replay_capsules: HashMap<String, ReplayCapsule>,
    active_config_snapshot_id: String,
    users: HashMap<String, UserIdentity>,
    memberships_by_user: HashMap<String, Vec<TenantMembership>>,
    provider_links_by_user: HashMap<String, Vec<AuthProviderLink>>,
    email_identity_to_user_id: HashMap<String, String>,
    provider_subject_to_user_id: HashMap<String, String>,
    sessions: HashMap<String, StoredSession>,
    login_flows: HashMap<String, LoginFlow>,
    route_receipts: HashMap<String, RouteReceipt>,
    billing_export_jobs: Vec<BillingExportJobRecord>,
    wechat_payment_orders: HashMap<String, WechatPaymentOrderRecord>,
    alipay_payment_orders: HashMap<String, AlipayPaymentOrderRecord>,
    renewal_intents: Vec<RenewalIntent>,
    route_policy_disabled_ids: HashSet<String>,
    api_keys: Vec<ApiKeyRecord>,
    opening_grants: Vec<OpeningGrantRecord>,
    deliveries: Vec<DeliveryRecord>,
    delivery_codes: Vec<DeliveryCodeRecord>,
    delivery_secret_plaintexts: HashMap<String, String>,
    delivery_entitlements: Vec<DeliveryEntitlementRecord>,
    delivery_artifacts: Vec<DeliveryArtifactRecord>,
    delivery_activations: Vec<DeliveryActivationRecord>,
    delivery_download_grants: Vec<DeliveryDownloadGrantRecord>,
    delivery_service_segments: Vec<DeliveryServiceSegmentRecord>,
    delivery_lifecycle_events: Vec<DeliveryLifecycleEventRecord>,
    delivery_upload_batches: Vec<DeliveryUploadBatchRecord>,
    delivery_upload_batch_items: Vec<DeliveryUploadBatchItemRecord>,
    codex_auth_accounts: Vec<CodexAuthAccountRecord>,
    oauth_sharing_leases: Vec<OAuthSharingLeaseRecord>,
    oauth_carpools: Vec<OAuthCarpoolRecord>,
    oauth_pool_runtime_leases: Vec<OAuthPoolRuntimeLeaseRecord>,
    oauth_pool_session_bindings: Vec<OAuthPoolSessionBindingRecord>,
    oauth_sharing_audit_events: Vec<OAuthSharingAuditEventRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub api_key_id: String,
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub key_prefix: String,
    pub hash: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct DeliveryPrepareDraft {
    pub delivery_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub owner_account_id: String,
    pub redemption_batch_id: Option<String>,
    pub provider: String,
    pub operator_id: String,
    pub customer_label: Option<String>,
    pub service_kind: String,
    pub service_days: u32,
    pub starts_at: String,
    pub ends_at: String,
    pub code_expires_at: String,
    pub enforce_owner_redemption_capacity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    pub delivery_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    #[serde(default = "default_delivery_owner_account_id")]
    pub owner_account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redemption_batch_id: Option<String>,
    pub provider: String,
    pub status: String,
    pub operator_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_label: Option<String>,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryCodeRecord {
    pub code_id: String,
    pub delivery_id: String,
    pub code_type: String,
    pub code_hash: String,
    pub code_prefix: String,
    pub code_last_four: String,
    pub format_version: String,
    pub status: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryEntitlementRecord {
    pub entitlement_id: String,
    pub delivery_id: String,
    pub service_kind: String,
    pub service_days: u32,
    pub starts_at: String,
    pub ends_at: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_record_version")]
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct DeliveryArtifactDraft {
    pub artifact_id: String,
    pub delivery_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub artifact_kind: String,
    pub provider: String,
    pub file_name: Option<String>,
    pub content_type: String,
    pub carrier_valid_until: Option<String>,
    pub encryption_protocol: String,
    pub encryption_version: String,
    pub secret_kind: String,
    pub ciphertext: Vec<u8>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryArtifactRecord {
    pub artifact_id: String,
    pub delivery_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub artifact_kind: String,
    pub provider: String,
    pub status: String,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub content_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub storage_backend: String,
    pub storage_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier_valid_until: Option<String>,
    #[serde(default = "default_delivery_account_bundle_encryption_protocol")]
    pub encryption_protocol: String,
    #[serde(default = "default_delivery_account_bundle_encryption_version")]
    pub encryption_version: String,
    #[serde(default = "default_delivery_artifact_secret_kind")]
    pub secret_kind: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
    #[serde(skip)]
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DeliveryUploadBatchDraft {
    pub batch_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub source_file_name: String,
    pub source_file_sha256: String,
    pub idempotency_key: Option<String>,
    pub items: Vec<DeliveryUploadBatchItemDraft>,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct DeliveryUploadBatchItemDraft {
    pub item_id: String,
    pub row_index: u32,
    pub delivery_id: String,
    pub artifact_kind: String,
    pub file_name: Option<String>,
    pub content_type: String,
    pub carrier_valid_until: Option<String>,
    pub encryption_protocol: String,
    pub encryption_version: String,
    pub secret_kind: String,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryUploadBatchRecord {
    pub batch_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub source_file_name: String,
    pub source_file_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub total_count: u32,
    pub success_count: u32,
    pub failed_count: u32,
    pub duplicate_count: u32,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_summary: Option<String>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryUploadBatchItemRecord {
    pub item_id: String,
    pub batch_id: String,
    pub row_index: u32,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub delivery_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    pub status: String,
    pub artifact_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier_valid_until: Option<String>,
    #[serde(default = "default_delivery_account_bundle_encryption_protocol")]
    pub encryption_protocol: String,
    #[serde(default = "default_delivery_account_bundle_encryption_version")]
    pub encryption_version: String,
    #[serde(default = "default_delivery_artifact_secret_kind")]
    pub secret_kind: String,
    pub payload_sha256: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(skip)]
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryServiceSegmentRecord {
    pub segment_id: String,
    pub entitlement_id: String,
    pub activation_id: String,
    pub delivery_id: String,
    pub artifact_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub segment_index: u32,
    pub effective_from: String,
    pub effective_until: String,
    pub carrier_valid_until: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryLifecycleEventRecord {
    pub event_id: String,
    pub entitlement_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    pub event_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub created_by: String,
    pub created_at: String,
    #[serde(default)]
    pub payload: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryActivationRecord {
    pub activation_id: String,
    pub delivery_id: String,
    pub code_id: String,
    pub entitlement_id: String,
    pub artifact_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub activation_source: String,
    pub activated_at: String,
    pub entitlement_ends_at: String,
    pub artifact: DeliveryArtifact,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeliveryDownloadGrantDraft {
    pub grant_id: String,
    pub activation_id: String,
    pub token_plaintext: String,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryDownloadGrantRecord {
    pub grant_id: String,
    pub activation_id: String,
    pub delivery_id: String,
    pub artifact_id: String,
    pub entitlement_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub provider: String,
    pub status: String,
    pub token_hash: String,
    pub token_prefix: String,
    pub token_last_four: String,
    pub expires_at: String,
    pub max_uses: u32,
    pub use_count: u32,
    pub artifact: DeliveryArtifact,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeliveryDownloadArtifactPayload {
    pub grant: DeliveryDownloadGrant,
    pub file_name: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DeliveryEntitlementExtendDraft {
    pub entitlement_id: String,
    pub expected_version: u64,
    pub extend_days: u32,
    pub reason: Option<String>,
    pub actor_id: String,
}

#[derive(Debug, Clone)]
pub enum DeliveryRedeemResult {
    Activated(Box<DeliveryActivationResponse>),
    RedemptionCodeNotFound,
    RedemptionCodeUsed,
    RedemptionCodeExpired,
    RedemptionCodeRevoked,
    RedemptionCodeNotActive(String),
    DeliveryNotFound,
    DeliveryNotActive(String),
    EntitlementNotFound,
    EntitlementNotActive(String),
    ArtifactMissing,
}

#[derive(Debug, Clone)]
pub enum DeliveryDownloadGrantIssueResult {
    Issued(Box<DeliveryDownloadGrantIssueResponse>),
    ActivationNotFound,
    ActivationNotActive(String),
    EntitlementNotFound,
    EntitlementNotActive(String),
    ArtifactMissing,
    ArtifactNotAvailable(String),
    SegmentMissing,
}

#[derive(Debug, Clone)]
pub enum DeliveryLifecycleResult {
    Applied(Box<DeliveryLifecycleResponse>),
    EntitlementNotFound,
}

#[derive(Debug, Clone)]
pub enum DeliveryUploadProcessResult {
    Processed(Box<DeliveryUploadBatchResponse>),
    BatchNotFound,
    BatchAlreadyProcessing,
    ScopeBusy,
}

#[derive(Debug, Clone)]
pub enum DeliveryDownloadConsumeResult {
    Retrieved(Box<DeliveryDownloadArtifactPayload>),
    GrantNotFound,
    GrantUsed,
    GrantExpired,
    GrantRevoked,
    GrantNotActive(String),
    ActivationNotFound,
    ActivationNotActive(String),
    EntitlementNotFound,
    EntitlementNotActive(String),
    ArtifactMissing,
    ArtifactNotAvailable(String),
    SegmentMissing,
}

#[derive(Debug, Clone)]
pub enum DeliveryPrepareResult {
    Prepared(Box<DeliveryPrepareResponse>),
    OwnerAlreadyIssued {
        active_count: usize,
        capacity: usize,
    },
}

#[derive(Debug, Clone)]
pub struct PreparedDeliveryRecords {
    delivery: DeliveryRecord,
    codes: Vec<DeliveryCodeRecord>,
    entitlement: DeliveryEntitlementRecord,
    enforce_owner_redemption_capacity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPaymentOrderRecord {
    pub out_trade_no: String,
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub amount_total: u32,
    pub currency: String,
    pub channel: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepay_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct WechatPaymentOrderResponse {
    pub data: WechatPaymentOrderRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlipayPaymentOrderRecord {
    pub out_trade_no: String,
    pub tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub amount_total: u32,
    pub currency: String,
    pub channel: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepay_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlipayPaymentOrderResponse {
    pub data: AlipayPaymentOrderRecord,
}

#[derive(Debug, Clone)]
pub struct RenewalIntentDraft {
    pub renewal_intent_id: String,
    pub out_trade_no: String,
    pub grant_id: String,
    pub renew_expires_at: String,
    pub reason: Option<String>,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct RenewalIntentFilters {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub grant_id: Option<String>,
    pub out_trade_no: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum RenewalIntentCreateResult {
    Created(RenewalIntent),
    OrderNotFound,
    GrantNotFound,
    GrantMismatch,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKey {
    pub api_key_id: String,
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub key_prefix: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeysResponse {
    pub data: Vec<ApiKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecretBlob {
    pub algorithm: String,
    pub key_id: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthAccountRecord {
    pub codex_account_id: String,
    #[serde(default = "default_oauth_account_provider")]
    pub provider: String,
    pub tenant_id: TenantId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub status: String,
    #[serde(default = "default_true")]
    pub schedulable: bool,
    #[serde(default = "default_true")]
    pub credential_ready: bool,
    #[serde(default)]
    pub concurrency_limit: Option<u32>,
    #[serde(default)]
    pub active_runs: u32,
    #[serde(default)]
    pub rate_limited_until: Option<String>,
    #[serde(default)]
    pub overloaded_until: Option<String>,
    #[serde(default)]
    pub temp_unschedulable_until: Option<String>,
    #[serde(default = "default_oauth_account_health_state")]
    pub health_state: String,
    #[serde(default = "default_oauth_account_health_score")]
    pub health_score: f32,
    #[serde(default)]
    pub last_error_code: Option<String>,
    #[serde(default)]
    pub consecutive_failures: u32,
    #[serde(default)]
    pub last_success_at: Option<String>,
    pub auth_json_sha256: String,
    pub encrypted_auth_json: EncryptedSecretBlob,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leased_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexAuthAccount {
    pub codex_account_id: String,
    pub provider: String,
    pub tenant_id: TenantId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub provider_resource_id: ProviderResourceId,
    pub display_name: String,
    pub status: String,
    pub schedulable: bool,
    pub credential_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_limit: Option<u32>,
    pub active_runs: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limited_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overloaded_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_unschedulable_until: Option<String>,
    pub health_state: String,
    pub health_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_code: Option<String>,
    pub consecutive_failures: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success_at: Option<String>,
    pub auth_json_sha256: String,
    pub encrypted_auth_json_key_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leased_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexAuthAccountsResponse {
    pub data: Vec<CodexAuthAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthSharingUsageBudget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSharingLeaseRecord {
    pub lease_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<String>,
    pub borrower_workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub borrower_user_id: Option<String>,
    pub provider: String,
    pub pool_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_account_ids: Option<Vec<String>>,
    pub status: String,
    pub starts_at: String,
    pub expires_at: String,
    pub max_concurrent_runs: u32,
    #[serde(default)]
    pub usage_budget: OAuthSharingUsageBudget,
    pub policy: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthSharingLeasesResponse {
    pub data: Vec<OAuthSharingLeaseRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCarpoolRecord {
    pub carpool_id: String,
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub member_workspace_ids: Vec<String>,
    #[serde(default)]
    pub pool_ids: Vec<String>,
    pub strategy: String,
    #[serde(default)]
    pub member_weights: BTreeMap<String, u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_member_concurrency_limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_member_turn_budget: Option<u64>,
    pub enabled: bool,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthCarpoolsResponse {
    pub data: Vec<OAuthCarpoolRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSharingAuditEventRecord {
    pub audit_event_id: String,
    pub event_type: String,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carpool_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthPoolRuntimeLeaseRecord {
    pub runtime_lease_id: String,
    pub account_id: String,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carpool_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    pub status: String,
    pub expires_at: String,
    pub heartbeat_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
    pub fencing_token: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthPoolSessionBindingRecord {
    pub binding_id: String,
    pub session_key: String,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub binding_policy: String,
    pub expires_at: String,
    pub last_seen_at: String,
    pub rebind_count: u64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthPoolAccountFeedback {
    #[serde(default)]
    pub runtime_lease_id: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub status_code: Option<u16>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub retry_after_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct OAuthSharingUsageSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carpool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    pub turns: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthSharingUsageResponse {
    pub data: Vec<OAuthSharingUsageSummary>,
    pub audit_events: Vec<OAuthSharingAuditEventRecord>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OAuthSharingUsageFilters<'a> {
    pub lease_id: Option<&'a str>,
    pub carpool_id: Option<&'a str>,
    pub workspace_id: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub account_id: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthPoolSelectionRequest {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub provider_resource_id: Option<String>,
    #[serde(default)]
    pub pool_id: Option<String>,
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub carpool_id: Option<String>,
    #[serde(default)]
    pub borrower_workspace_id: Option<String>,
    #[serde(default)]
    pub borrower_user_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub holder_id: Option<String>,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub lease_ttl_seconds: Option<u64>,
    #[serde(default)]
    pub binding_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthPoolSelection {
    pub account: Option<CodexAuthAccountRecord>,
    pub reason: String,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carpool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_lease_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fencing_token: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteReceiptsResponse {
    pub data: Vec<RouteReceipt>,
}

#[derive(Debug, Clone)]
pub struct BillingExportJobRecord {
    pub job: BillingExportJob,
    pub content: Option<String>,
    pub content_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageBreakdownGroupBy {
    Provider,
    Model,
    Day,
}

#[derive(Debug, Clone)]
pub struct ResolvedApiKey {
    pub api_key_id: String,
    pub grant_id: Option<String>,
    pub owner_account_id: Option<String>,
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub is_active: bool,
    pub status: String,
    pub config_snapshot_id: Option<ConfigSnapshotId>,
    pub route_policy_id: Option<RoutePolicyId>,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSnapshotsResponse {
    pub data: Vec<ConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantWorkspaceResponse {
    pub merchant_enabled: bool,
    pub tenant_id: TenantId,
    pub shops: Vec<MerchantShop>,
    pub card_products: Vec<CardProduct>,
    pub trial_connections: Vec<TrialConnection>,
    pub recent_evaluations: Vec<RelayEvaluation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MerchantWorkspaceEnvelope {
    pub data: MerchantWorkspaceResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayCapsuleResponse {
    pub replay_capsule: ReplayCapsule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrencyResult<T> {
    Applied(T),
    NotFound,
    VersionConflict,
}

impl ApiKeyRecord {
    fn public_view(&self) -> ApiKey {
        ApiKey {
            api_key_id: self.api_key_id.clone(),
            provider_resource_id: self.provider_resource_id.clone(),
            display_name: self.display_name.clone(),
            key_prefix: self.key_prefix.clone(),
            is_active: self.is_active,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }

    fn to_resolved(&self, provider_resources: &[ProviderResource]) -> Option<ResolvedApiKey> {
        if !self.is_active {
            return None;
        }
        let provider_resource = provider_resources
            .iter()
            .find(|resource| resource.provider_resource_id == self.provider_resource_id)?;
        Some(ResolvedApiKey {
            api_key_id: self.api_key_id.clone(),
            grant_id: None,
            owner_account_id: None,
            tenant_id: provider_resource.tenant_id.clone(),
            project_id: provider_resource.project_id.clone(),
            is_active: true,
            status: "active".to_string(),
            config_snapshot_id: None,
            route_policy_id: None,
            scopes: Vec::new(),
            expires_at: None,
        })
    }
}

impl DeliveryRecord {
    fn effective_status(&self, entitlement: &DeliveryEntitlementRecord) -> String {
        if self.status == DELIVERY_STATUS_REVOKED {
            DELIVERY_STATUS_REVOKED.to_string()
        } else if entitlement.effective_status() == DELIVERY_STATUS_EXPIRED {
            DELIVERY_STATUS_EXPIRED.to_string()
        } else {
            self.status.clone()
        }
    }

    fn public_view(&self, entitlement: &DeliveryEntitlementRecord) -> Delivery {
        Delivery {
            delivery_id: self.delivery_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            owner_account_id: self.owner_account_id.clone(),
            redemption_batch_id: self.redemption_batch_id.clone(),
            provider: self.provider.clone(),
            status: self.effective_status(entitlement),
            operator_id: self.operator_id.clone(),
            customer_label: self.customer_label.clone(),
            source: self.source.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
            revoked_at: self.revoked_at.clone(),
            revoked_by: self.revoked_by.clone(),
            revoke_reason: self.revoke_reason.clone(),
        }
    }
}

impl DeliveryCodeRecord {
    fn effective_status(&self) -> String {
        if self.status == "active" && timestamp_is_expired(&self.expires_at) {
            DELIVERY_STATUS_EXPIRED.to_string()
        } else {
            self.status.clone()
        }
    }

    fn public_view(&self) -> DeliveryCode {
        DeliveryCode {
            code_id: self.code_id.clone(),
            delivery_id: self.delivery_id.clone(),
            code_type: self.code_type.clone(),
            code_prefix: self.code_prefix.clone(),
            code_last_four: self.code_last_four.clone(),
            format_version: self.format_version.clone(),
            status: self.effective_status(),
            expires_at: self.expires_at.clone(),
            used_at: self.used_at.clone(),
            revoked_at: self.revoked_at.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }
}

impl DeliveryEntitlementRecord {
    fn effective_status(&self) -> String {
        if matches!(
            self.status.as_str(),
            DELIVERY_ENTITLEMENT_STATUS_REVOKED
                | DELIVERY_ENTITLEMENT_STATUS_SUSPENDED
                | DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY
                | DELIVERY_ENTITLEMENT_STATUS_PENDING_ACTIVATION
        ) {
            self.status.clone()
        } else if timestamp_is_expired(&self.ends_at) {
            DELIVERY_STATUS_EXPIRED.to_string()
        } else if timestamp_is_future(&self.starts_at) {
            DELIVERY_ENTITLEMENT_STATUS_PENDING_ACTIVATION.to_string()
        } else {
            DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string()
        }
    }

    fn public_view(&self) -> DeliveryEntitlement {
        DeliveryEntitlement {
            entitlement_id: self.entitlement_id.clone(),
            delivery_id: self.delivery_id.clone(),
            service_kind: self.service_kind.clone(),
            service_days: self.service_days,
            starts_at: self.starts_at.clone(),
            ends_at: self.ends_at.clone(),
            service_starts_at: self.starts_at.clone(),
            service_ends_at: self.ends_at.clone(),
            status: self.effective_status(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }
}

impl DeliveryArtifactRecord {
    fn public_view(&self) -> DeliveryArtifact {
        DeliveryArtifact {
            artifact_id: self.artifact_id.clone(),
            delivery_id: self.delivery_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            artifact_kind: self.artifact_kind.clone(),
            provider: self.provider.clone(),
            status: self.status.clone(),
            version: self.version,
            file_name: self.file_name.clone(),
            content_type: self.content_type.clone(),
            size_bytes: self.size_bytes,
            sha256: self.sha256.clone(),
            storage_backend: self.storage_backend.clone(),
            storage_ref: self.storage_ref.clone(),
            carrier_valid_until: self.carrier_valid_until.clone(),
            encryption_protocol: self.encryption_protocol.clone(),
            encryption_version: self.encryption_version.clone(),
            secret_kind: self.secret_kind.clone(),
            created_by: self.created_by.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            superseded_at: self.superseded_at.clone(),
            superseded_by: self.superseded_by.clone(),
            revoked_at: self.revoked_at.clone(),
            revoked_by: self.revoked_by.clone(),
            revoke_reason: self.revoke_reason.clone(),
        }
    }
}

impl DeliveryUploadBatchRecord {
    fn public_response(&self) -> DeliveryUploadBatchResponse {
        DeliveryUploadBatchResponse {
            data: self.public_view(),
        }
    }

    fn public_view(&self) -> DeliveryUploadBatch {
        DeliveryUploadBatch {
            batch_id: self.batch_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            provider: self.provider.clone(),
            status: self.status.clone(),
            source_file_name: self.source_file_name.clone(),
            source_file_sha256: self.source_file_sha256.clone(),
            idempotency_key: self.idempotency_key.clone(),
            total_count: self.total_count,
            success_count: self.success_count,
            failed_count: self.failed_count,
            duplicate_count: self.duplicate_count,
            created_by: self.created_by.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            error_summary: self.error_summary.clone(),
            version: self.version,
        }
    }
}

impl DeliveryUploadBatchItemRecord {
    fn public_view(&self) -> DeliveryUploadBatchItem {
        DeliveryUploadBatchItem {
            item_id: self.item_id.clone(),
            batch_id: self.batch_id.clone(),
            row_index: self.row_index,
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            delivery_id: self.delivery_id.clone(),
            artifact_id: self.artifact_id.clone(),
            status: self.status.clone(),
            artifact_kind: self.artifact_kind.clone(),
            file_name: self.file_name.clone(),
            content_type: self.content_type.clone(),
            carrier_valid_until: self.carrier_valid_until.clone(),
            encryption_protocol: self.encryption_protocol.clone(),
            encryption_version: self.encryption_version.clone(),
            secret_kind: self.secret_kind.clone(),
            payload_sha256: self.payload_sha256.clone(),
            size_bytes: self.size_bytes,
            error_code: self.error_code.clone(),
            error_message: self.error_message.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }
}

impl DeliveryServiceSegmentRecord {
    fn effective_status(&self) -> String {
        if self.status == DELIVERY_SERVICE_SEGMENT_STATUS_REVOKED {
            DELIVERY_SERVICE_SEGMENT_STATUS_REVOKED.to_string()
        } else if timestamp_is_expired(&self.effective_until) {
            DELIVERY_SERVICE_SEGMENT_STATUS_EXPIRED.to_string()
        } else if self.status == DELIVERY_SERVICE_SEGMENT_STATUS_SCHEDULED
            && !timestamp_is_future(&self.effective_from)
        {
            DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE.to_string()
        } else if self.status == DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE
            && timestamp_is_expired(&self.effective_until)
        {
            DELIVERY_SERVICE_SEGMENT_STATUS_EXPIRED.to_string()
        } else {
            self.status.clone()
        }
    }

    fn public_view(&self) -> DeliveryServiceSegment {
        DeliveryServiceSegment {
            segment_id: self.segment_id.clone(),
            entitlement_id: self.entitlement_id.clone(),
            activation_id: self.activation_id.clone(),
            delivery_id: self.delivery_id.clone(),
            artifact_id: self.artifact_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            provider: self.provider.clone(),
            status: self.effective_status(),
            segment_index: self.segment_index,
            effective_from: self.effective_from.clone(),
            effective_until: self.effective_until.clone(),
            carrier_valid_until: self.carrier_valid_until.clone(),
            created_by: self.created_by.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }
}

impl DeliveryLifecycleEventRecord {
    fn public_view(&self) -> DeliveryLifecycleEvent {
        DeliveryLifecycleEvent {
            event_id: self.event_id.clone(),
            entitlement_id: self.entitlement_id.clone(),
            segment_id: self.segment_id.clone(),
            event_type: self.event_type.clone(),
            status: self.status.clone(),
            reason: self.reason.clone(),
            created_by: self.created_by.clone(),
            created_at: self.created_at.clone(),
            payload: self.payload.clone(),
        }
    }
}

impl DeliveryActivationRecord {
    fn public_view(&self) -> DeliveryActivation {
        DeliveryActivation {
            activation_id: self.activation_id.clone(),
            delivery_id: self.delivery_id.clone(),
            artifact_id: self.artifact_id.clone(),
            entitlement_id: self.entitlement_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            provider: self.provider.clone(),
            status: self.status.clone(),
            activation_source: self.activation_source.clone(),
            activated_at: self.activated_at.clone(),
            entitlement_ends_at: self.entitlement_ends_at.clone(),
            artifact: self.artifact.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            revoked_at: self.revoked_at.clone(),
            revoked_by: self.revoked_by.clone(),
            revoke_reason: self.revoke_reason.clone(),
        }
    }
}

impl DeliveryDownloadGrantRecord {
    fn effective_status(&self) -> String {
        if self.status == DELIVERY_DOWNLOAD_GRANT_STATUS_ACTIVE
            && timestamp_is_expired(&self.expires_at)
        {
            DELIVERY_DOWNLOAD_GRANT_STATUS_EXPIRED.to_string()
        } else {
            self.status.clone()
        }
    }

    fn public_view(&self) -> DeliveryDownloadGrant {
        DeliveryDownloadGrant {
            grant_id: self.grant_id.clone(),
            activation_id: self.activation_id.clone(),
            delivery_id: self.delivery_id.clone(),
            artifact_id: self.artifact_id.clone(),
            entitlement_id: self.entitlement_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            provider: self.provider.clone(),
            status: self.effective_status(),
            token_prefix: self.token_prefix.clone(),
            token_last_four: self.token_last_four.clone(),
            expires_at: self.expires_at.clone(),
            max_uses: self.max_uses,
            use_count: self.use_count,
            artifact: self.artifact.clone(),
            created_by: self.created_by.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
            used_at: self.used_at.clone(),
            revoked_at: self.revoked_at.clone(),
            revoked_by: self.revoked_by.clone(),
            revoke_reason: self.revoke_reason.clone(),
        }
    }
}

impl CodexAuthAccountRecord {
    #[must_use]
    pub fn public_view(&self) -> CodexAuthAccount {
        CodexAuthAccount {
            codex_account_id: self.codex_account_id.clone(),
            provider: self.provider.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            provider_resource_id: self.provider_resource_id.clone(),
            display_name: self.display_name.clone(),
            status: self.status.clone(),
            schedulable: self.schedulable,
            credential_ready: self.credential_ready,
            concurrency_limit: self.concurrency_limit,
            active_runs: self.active_runs,
            rate_limited_until: self.rate_limited_until.clone(),
            overloaded_until: self.overloaded_until.clone(),
            temp_unschedulable_until: self.temp_unschedulable_until.clone(),
            health_state: self.health_state.clone(),
            health_score: self.health_score,
            last_error_code: self.last_error_code.clone(),
            consecutive_failures: self.consecutive_failures,
            last_success_at: self.last_success_at.clone(),
            auth_json_sha256: self.auth_json_sha256.clone(),
            encrypted_auth_json_key_id: self.encrypted_auth_json.key_id.clone(),
            leased_until: self.leased_until.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
        }
    }

    #[must_use]
    pub fn public_view_with_active_runs(&self, active_runs: u32) -> CodexAuthAccount {
        let mut view = self.public_view();
        view.active_runs = active_runs;
        view
    }
}

fn default_oauth_account_provider() -> String {
    "codex".to_string()
}

fn default_oauth_account_health_state() -> String {
    "healthy".to_string()
}

const fn default_oauth_account_health_score() -> f32 {
    1.0
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFlow {
    pub flow_id: String,
    pub flow_kind: LoginFlowKind,
    pub email: Option<String>,
    pub provider: Option<OAuthProvider>,
    pub workspace_slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_to: Option<String>,
    pub verification_code: Option<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoginFlowKind {
    Email,
    OAuth,
}

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub session: AuthSession,
    pub links: Vec<AuthProviderLink>,
}

#[derive(Debug, Clone)]
pub struct SeedData {
    pub tenants: Vec<Tenant>,
    pub projects: Vec<Project>,
    pub provider_resources: Vec<ProviderResource>,
    pub route_policies: Vec<RoutePolicy>,
    pub config_snapshots: Vec<ConfigSnapshot>,
    pub merchant_shops: Vec<MerchantShop>,
    pub card_products: Vec<CardProduct>,
    pub trial_connections: Vec<TrialConnection>,
    pub relay_evaluations: Vec<RelayEvaluation>,
    pub replay_capsules: Vec<ReplayCapsule>,
    pub route_receipts: Vec<RouteReceipt>,
    pub active_config_snapshot_id: String,
    pub users: Vec<UserSeed>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderResourceFilters {
    pub tenant_id: Option<String>,
    pub health_state: Option<String>,
    pub protocol_family: Option<String>,
    pub capability: Option<String>,
    pub transit_gateway: Option<bool>,
    pub quarantined: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RouteReceiptFilters {
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub protocol_family: Option<String>,
    pub route_policy_id: Option<String>,
    pub provider_resource_id: Option<String>,
    pub admission_result: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DeliveryOperationsScope {
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub window_start: String,
    pub window_end: String,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct DeliveryOperationsExceptionFilters {
    pub scope: DeliveryOperationsScope,
    pub status: Option<String>,
    pub exception_type: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DeliveryOperationsObjectRef {
    Delivery(String),
    Entitlement(String),
    Activation(String),
    Artifact(String),
    DownloadGrant(String),
    ServiceSegment(String),
}

#[derive(Debug, Clone)]
pub struct DeliveryOperationsObjectFilters {
    pub scope: DeliveryOperationsScope,
    pub object_ref: DeliveryOperationsObjectRef,
}

#[derive(Debug, Clone)]
pub struct UserSeed {
    pub user: UserIdentity,
    pub identities: Vec<UserIdentityKey>,
    pub memberships: Vec<TenantMembership>,
    pub links: Vec<AuthProviderLink>,
}

#[derive(Debug, Clone)]
pub enum UserIdentityKey {
    Email(String),
    ProviderSubject(AuthProvider, String),
}

impl SeedData {
    #[allow(clippy::too_many_lines)]
    pub fn bootstrap() -> Self {
        let now = "2026-04-22T00:00:00Z".to_string();
        let tenant_platform = Tenant::new(
            TenantId::parse("tenant_platform").unwrap(),
            "platform-admin",
            "Platform Admin",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let tenant_acme = Tenant::new(
            TenantId::parse("tenant_acme").unwrap(),
            "acme-retail",
            "Acme Retail",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let tenant_northstar = Tenant::new(
            TenantId::parse("tenant_northstar").unwrap(),
            "northstar-labs",
            "Northstar Labs",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();

        let proj_core = Project::new(
            ProjectId::parse("proj_core").unwrap(),
            tenant_acme.tenant_id.clone(),
            "core-gateway",
            "Core Gateway",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let proj_ops = Project::new(
            ProjectId::parse("proj_acme_ops").unwrap(),
            tenant_acme.tenant_id.clone(),
            "retail-ops",
            "Retail Operations Control",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let proj_support = Project::new(
            ProjectId::parse("proj_acme_support").unwrap(),
            tenant_acme.tenant_id.clone(),
            "store-support",
            "Store Support Agent",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();
        let proj_research = Project::new(
            ProjectId::parse("proj_ns_research").unwrap(),
            tenant_northstar.tenant_id.clone(),
            "research-qa",
            "Research QA",
            1,
            now.clone(),
            now.clone(),
        )
        .unwrap();

        let openai_primary = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: Some(proj_core.project_id.clone()),
            provider_id: "openai".to_string(),
            name: "OpenAI Primary".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Platform,
            deployment_scope: DeploymentScope::Shared,
            region: "us-east-1".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Healthy,
            health_message: Some("probe latency within SLO".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec![
                "openai_chat".to_string(),
                "openai_responses".to_string(),
                "openai_images".to_string(),
            ],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let openai_backup = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: Some(proj_core.project_id.clone()),
            provider_id: "openai".to_string(),
            name: "OpenAI Backup".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Platform,
            deployment_scope: DeploymentScope::Shared,
            region: "us-west-2".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Healthy,
            health_message: Some("backup target healthy".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec![
                "openai_chat".to_string(),
                "openai_responses".to_string(),
                "openai_images".to_string(),
            ],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let bedrock_claude = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_bedrock_claude").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: Some(proj_ops.project_id.clone()),
            provider_id: "bedrock".to_string(),
            name: "Bedrock Claude".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Platform,
            deployment_scope: DeploymentScope::Shared,
            region: "us-east-1".to_string(),
            endpoint_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Healthy,
            health_message: Some("aws credential chain available".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: false,
                supports_tool_calling: false,
                supports_json_mode: false,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec!["openai_chat".to_string()],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let northstar_openai = ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_research").unwrap(),
            tenant_id: tenant_northstar.tenant_id.clone(),
            project_id: Some(proj_research.project_id.clone()),
            provider_id: "openai".to_string(),
            name: "OpenAI Research".to_string(),
            status: ProviderResourceStatus::Active,
            provenance_class: ProvenanceClass::OfficialApi,
            credential_owner_type: CredentialOwnerType::Tenant,
            deployment_scope: DeploymentScope::TenantDedicated,
            region: "us-west-2".to_string(),
            endpoint_base_url: "https://api.openai.com/v1".to_string(),
            auth_kind: AuthKind::ApiKey,
            health_state: HealthState::Degraded,
            health_message: Some("research endpoint latency elevated".to_string()),
            quarantine_reason: None,
            budget_policy_id: None,
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: false,
                supports_json_mode: true,
                supports_realtime: false,
                supports_response_model_metadata: true,
            },
            supported_protocol_families: vec!["openai_chat".to_string()],
            is_transit_gateway: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let route_default = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_openai_chat_default").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            display_name: "Acme Reasoning Fast".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let route_support = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_acme_support").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            display_name: "Acme Support Safe".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "support-safe".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-west-2".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let route_bedrock = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_bedrock_claude_text").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            display_name: "Acme Bedrock Claude Text".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "claude-sonnet".to_string(),
            required_capabilities: vec!["chat_completions".to_string()],
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let route_research = RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_northstar_research").unwrap(),
            tenant_id: tenant_northstar.tenant_id.clone(),
            display_name: "Northstar Research".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "research-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-west-2".to_string()],
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let config_active = ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_gateway_v1").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: proj_core.project_id.clone(),
            revision: 1,
            status: ConfigSnapshotStatus::Active,
            activated_at: Some(now.clone()),
            provider_resource_ids: vec![
                openai_primary.provider_resource_id.clone(),
                openai_backup.provider_resource_id.clone(),
            ],
            route_policy_id: route_default.route_policy_id.clone(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };
        let config_bedrock = ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_bedrock_ops_v1").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            project_id: proj_ops.project_id.clone(),
            revision: 1,
            status: ConfigSnapshotStatus::Active,
            activated_at: Some(now.clone()),
            provider_resource_ids: vec![bedrock_claude.provider_resource_id.clone()],
            route_policy_id: route_bedrock.route_policy_id.clone(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };
        let config_research = ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_research_v1").unwrap(),
            tenant_id: tenant_northstar.tenant_id.clone(),
            project_id: proj_research.project_id.clone(),
            revision: 1,
            status: ConfigSnapshotStatus::Draft,
            activated_at: None,
            provider_resource_ids: vec![northstar_openai.provider_resource_id.clone()],
            route_policy_id: route_research.route_policy_id.clone(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };

        let merchant_shop = MerchantShop {
            merchant_shop_id: MerchantShopId::parse("mshop_acme").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            slug: "acme-small-shop".to_string(),
            display_name: "Acme Small Shop".to_string(),
            status: MerchantShopStatus::Active,
            announcement: Some(
                "Fresh relay trial cards with replay-backed evaluation.".to_string(),
            ),
            fulfillment_mode: MerchantFulfillmentMode::AutoCardSecret,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let card_product = CardProduct {
            card_product_id: CardProductId::parse("cardprod_acme_trial").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            merchant_shop_id: merchant_shop.merchant_shop_id.clone(),
            project_id: Some(proj_core.project_id.clone()),
            title: "Claude Trial Pack".to_string(),
            description: "Starter batch for relay verification and low-risk onboarding."
                .to_string(),
            status: CardProductStatus::Active,
            inventory_count: 32,
            face_value_usd: "1.00".to_string(),
            retail_price_usd: "1.99".to_string(),
            retail_price_cny_total: Some(199),
            delivery_kind: CardDeliveryKind::DirectSecret,
            supports_trial: true,
            sale_enabled: false,
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let trial_connection = TrialConnection {
            trial_connection_id: TrialConnectionId::parse("trialconn_acme_relay").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            provider_label: "Acme Relay".to_string(),
            endpoint_base_url: "https://relay.acme.example/v1".to_string(),
            api_key_masked: "sk-trial...acme".to_string(),
            target_model: "claude-sonnet".to_string(),
            status: TrialConnectionStatus::Active,
            notes: Some("Dedicated trial key only; never attach production traffic.".to_string()),
            last_verified_at: Some(now.clone()),
            version: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let replay_capsule = ReplayCapsule {
            replay_capsule_id: ReplayCapsuleId::parse("replay_acme_relay_eval").unwrap(),
            request_id: "req_merchant_eval_acme".to_string(),
            trace_id: "trace_merchant_eval_acme".to_string(),
            route_receipt_id: RouteReceiptId::parse("routercpt_acme_relay_eval").unwrap(),
            config_snapshot_id: config_active.config_snapshot_id.clone(),
            redaction_tier: RedactionTier::StructuredRedacted,
            normalized_request_summary: NormalizedRequestSummary {
                protocol_family: "openai_chat".to_string(),
                model_alias: "claude-sonnet".to_string(),
                estimated_prompt_tokens: 480,
            },
            upstream_error_summary: Some(UpstreamErrorSummary {
                code: "provider_signature_mismatch".to_string(),
            }),
        };
        let route_receipt = build_merchant_replay_route_receipt(
            &tenant_acme.tenant_id,
            &config_active,
            replay_capsule.route_receipt_id.clone(),
            replay_capsule.request_id.clone(),
            replay_capsule.trace_id.clone(),
            trial_connection.target_model.clone(),
            now.clone(),
        );
        let relay_evaluation = RelayEvaluation {
            relay_evaluation_id: RelayEvaluationId::parse("reval_acme_relay").unwrap(),
            tenant_id: tenant_acme.tenant_id.clone(),
            trial_connection_id: trial_connection.trial_connection_id.clone(),
            replay_capsule_id: replay_capsule.replay_capsule_id.clone(),
            provider_label: trial_connection.provider_label.clone(),
            endpoint_base_url: trial_connection.endpoint_base_url.clone(),
            target_model: trial_connection.target_model.clone(),
            runner_mode: RelayEvaluationRunnerMode::Simulated,
            sample_request_count: 5,
            estimated_tokens_saved: 2400,
            overall_score: 82,
            verdict: RelayEvaluationVerdict::Warning,
            fingerprint_status: RelayCheckStatus::Pass,
            protocol_status: RelayCheckStatus::Warning,
            token_status: RelayCheckStatus::Warning,
            multimodal_status: RelayCheckStatus::NotTested,
            detected_channel: Some("vertex".to_string()),
            summary:
                "Replay capsule captured; protocol and token behavior still need manual follow-up."
                    .to_string(),
            created_at: now.clone(),
        };

        let ops_user = UserIdentity {
            user_id: UserId::parse("user_ops").unwrap(),
            primary_email: Some("ops@huge-router.dev".to_string()),
            display_name: "Operations Admin".to_string(),
            avatar_url: None,
            created_at: now.clone(),
            last_login_at: None,
        };
        let developer_user = UserIdentity {
            user_id: UserId::parse("user_developer").unwrap(),
            primary_email: Some("developer@huge-router.dev".to_string()),
            display_name: "Demo Developer".to_string(),
            avatar_url: None,
            created_at: now.clone(),
            last_login_at: None,
        };

        let memberships = vec![
            membership(
                "tmemb_platform",
                &tenant_platform,
                TenantMembershipRole::Admin,
            ),
            membership("tmemb_acme", &tenant_acme, TenantMembershipRole::Admin),
            membership(
                "tmemb_northstar",
                &tenant_northstar,
                TenantMembershipRole::Member,
            ),
        ];
        let links = vec![
            link(
                AuthProvider::Email,
                "ops@huge-router.dev",
                Some("ops@huge-router.dev"),
                false,
            ),
            link(
                AuthProvider::Github,
                "github_ops",
                Some("ops@huge-router.dev"),
                true,
            ),
            link(
                AuthProvider::Google,
                "google_ops",
                Some("ops@huge-router.dev"),
                true,
            ),
            link(
                AuthProvider::Wechat,
                "wechat_ops",
                Some("ops@huge-router.dev"),
                true,
            ),
        ];

        Self {
            tenants: vec![tenant_platform, tenant_acme.clone(), tenant_northstar],
            projects: vec![proj_core, proj_ops, proj_support, proj_research],
            provider_resources: vec![
                openai_primary,
                openai_backup,
                bedrock_claude,
                northstar_openai,
            ],
            route_policies: vec![route_default, route_support, route_bedrock, route_research],
            config_snapshots: vec![config_active.clone(), config_bedrock, config_research],
            merchant_shops: vec![merchant_shop],
            card_products: vec![card_product],
            trial_connections: vec![trial_connection],
            relay_evaluations: vec![relay_evaluation],
            replay_capsules: vec![replay_capsule],
            route_receipts: vec![route_receipt],
            active_config_snapshot_id: config_active.config_snapshot_id.as_str().to_string(),
            users: vec![
                UserSeed {
                    user: ops_user,
                    identities: vec![
                        UserIdentityKey::Email("ops@huge-router.dev".to_string()),
                        UserIdentityKey::ProviderSubject(
                            AuthProvider::Github,
                            "github_ops".to_string(),
                        ),
                        UserIdentityKey::ProviderSubject(
                            AuthProvider::Google,
                            "google_ops".to_string(),
                        ),
                        UserIdentityKey::ProviderSubject(
                            AuthProvider::Wechat,
                            "wechat_ops".to_string(),
                        ),
                    ],
                    memberships,
                    links,
                },
                UserSeed {
                    user: developer_user,
                    identities: vec![UserIdentityKey::Email(
                        "developer@huge-router.dev".to_string(),
                    )],
                    memberships: vec![membership(
                        "tmemb_acme_developer",
                        &tenant_acme,
                        TenantMembershipRole::Member,
                    )],
                    links: vec![link(
                        AuthProvider::Email,
                        "developer@huge-router.dev",
                        Some("developer@huge-router.dev"),
                        false,
                    )],
                },
            ],
        }
    }
}

impl MemoryStore {
    pub fn bootstrap() -> Self {
        Self::from_seed(SeedData::bootstrap())
    }

    pub(crate) fn from_seed(seed: SeedData) -> Self {
        let mut users = HashMap::new();
        let mut memberships_by_user = HashMap::new();
        let mut provider_links_by_user = HashMap::new();
        let mut email_identity_to_user_id = HashMap::new();
        let mut provider_subject_to_user_id = HashMap::new();

        for entry in seed.users {
            let user_id = entry.user.user_id.as_str().to_string();
            memberships_by_user.insert(user_id.clone(), entry.memberships);
            provider_links_by_user.insert(user_id.clone(), entry.links);
            for identity in entry.identities {
                match identity {
                    UserIdentityKey::Email(email) => {
                        email_identity_to_user_id.insert(email.to_lowercase(), user_id.clone());
                    }
                    UserIdentityKey::ProviderSubject(provider, subject) => {
                        provider_subject_to_user_id.insert(
                            format!("{}:{}", auth_provider_slug(provider), subject),
                            user_id.clone(),
                        );
                    }
                }
            }
            users.insert(user_id, entry.user);
        }

        Self {
            tenants: seed.tenants,
            projects: seed.projects,
            provider_resources: seed.provider_resources,
            route_policies: seed.route_policies,
            config_snapshots: seed.config_snapshots,
            merchant_shops: seed.merchant_shops,
            card_products: seed.card_products,
            merchant_product_inventory: Vec::new(),
            merchant_product_orders: Vec::new(),
            trial_connections: seed.trial_connections,
            relay_evaluations: seed.relay_evaluations,
            replay_capsules: seed
                .replay_capsules
                .into_iter()
                .map(|capsule| (capsule.replay_capsule_id.as_str().to_string(), capsule))
                .collect(),
            active_config_snapshot_id: seed.active_config_snapshot_id,
            users,
            memberships_by_user,
            provider_links_by_user,
            email_identity_to_user_id,
            provider_subject_to_user_id,
            sessions: HashMap::new(),
            login_flows: HashMap::new(),
            wechat_user_openids: HashMap::new(),
            route_receipts: seed
                .route_receipts
                .into_iter()
                .map(|receipt| (receipt.route_receipt_id.as_str().to_string(), receipt))
                .collect(),
            billing_export_jobs: Vec::new(),
            wechat_payment_orders: HashMap::new(),
            alipay_payment_orders: HashMap::new(),
            renewal_intents: Vec::new(),
            route_policy_disabled_ids: HashSet::new(),
            api_keys: Vec::new(),
            opening_grants: Vec::new(),
            deliveries: Vec::new(),
            delivery_codes: Vec::new(),
            delivery_secret_plaintexts: HashMap::new(),
            delivery_entitlements: Vec::new(),
            delivery_artifacts: Vec::new(),
            delivery_activations: Vec::new(),
            delivery_download_grants: Vec::new(),
            delivery_service_segments: Vec::new(),
            delivery_lifecycle_events: Vec::new(),
            delivery_upload_batches: Vec::new(),
            delivery_upload_batch_items: Vec::new(),
            codex_auth_accounts: Vec::new(),
            oauth_sharing_leases: Vec::new(),
            oauth_carpools: Vec::new(),
            oauth_pool_runtime_leases: Vec::new(),
            oauth_pool_session_bindings: Vec::new(),
            oauth_sharing_audit_events: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_route_receipt(&mut self, route_receipt: RouteReceipt) {
        self.route_receipts.insert(
            route_receipt.route_receipt_id.as_str().to_string(),
            route_receipt,
        );
    }
}

impl StoreMode {
    pub fn memory() -> Self {
        Self::Memory(Arc::new(RwLock::new(MemoryStore::bootstrap())))
    }

    pub async fn from_env() -> Result<Self> {
        let store_mode =
            std::env::var("CONTROL_PLANE_STORE_MODE").unwrap_or_else(|_| "postgres".to_string());

        match store_mode.as_str() {
            "memory" => Ok(Self::memory()),
            "postgres" => {
                let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL").map_err(|_| {
                    anyhow!(
                        "CONTROL_PLANE_DATABASE_URL is required when CONTROL_PLANE_STORE_MODE=postgres"
                    )
                })?;
                let store = PostgresStore::connect(&database_url).await?;
                store.ensure_schema_ready().await?;
                Ok(Self::Postgres(store))
            }
            other => Err(anyhow!(
                "unsupported CONTROL_PLANE_STORE_MODE `{other}`; expected `postgres` or `memory`"
            )),
        }
    }

    pub fn provider_catalog() -> Vec<AuthProviderAvailability> {
        let mut catalog = PROVIDER_CATALOG
            .iter()
            .map(|(provider, display_name, start_path)| {
                let enabled = auth_provider_enabled(*provider);
                AuthProviderAvailability {
                    provider: *provider,
                    display_name: (*display_name).to_string(),
                    enabled,
                    start_path: (*start_path).to_string(),
                    reason_code: if enabled {
                        None
                    } else {
                        Some("provider_not_configured".to_string())
                    },
                }
            })
            .collect::<Vec<_>>();

        let oidc_available = oidc_enabled() || mock_auth_enabled();
        catalog.push(AuthProviderAvailability {
            provider: OIDC_PROVIDER_CATALOG_ENTRY.0,
            display_name: OIDC_PROVIDER_CATALOG_ENTRY.1.to_string(),
            enabled: oidc_available,
            start_path: OIDC_PROVIDER_CATALOG_ENTRY.2.to_string(),
            reason_code: if oidc_available {
                None
            } else {
                Some("provider_not_configured".to_string())
            },
        });

        catalog
    }

    pub async fn ensure_known_email(&self, email: &str) -> Result<bool> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .email_identity_to_user_id
                .contains_key(&email.to_lowercase())),
            Self::Postgres(store) => store.ensure_known_email(email).await,
        }
    }

    pub async fn create_email_flow(
        &self,
        flow_id: &str,
        email: &str,
        workspace_slug: &str,
        verification_code: &str,
        expires_at: &str,
    ) -> Result<LoginFlow> {
        let flow = LoginFlow {
            flow_id: flow_id.to_string(),
            flow_kind: LoginFlowKind::Email,
            email: Some(email.to_lowercase()),
            provider: None,
            workspace_slug: workspace_slug.to_string(),
            redirect_to: None,
            verification_code: Some(verification_code.to_string()),
            expires_at: expires_at.to_string(),
        };
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .login_flows
                    .insert(flow_id.to_string(), flow.clone());
                Ok(flow)
            }
            Self::Postgres(store) => store.create_login_flow(&flow).await,
        }
    }

    pub async fn create_oauth_flow(
        &self,
        flow_id: &str,
        provider: OAuthProvider,
        workspace_slug: &str,
        redirect_to: Option<&str>,
        expires_at: &str,
    ) -> Result<LoginFlow> {
        let flow = LoginFlow {
            flow_id: flow_id.to_string(),
            flow_kind: LoginFlowKind::OAuth,
            email: None,
            provider: Some(provider),
            workspace_slug: workspace_slug.to_string(),
            redirect_to: redirect_to.map(str::to_string),
            verification_code: None,
            expires_at: expires_at.to_string(),
        };
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .login_flows
                    .insert(flow_id.to_string(), flow.clone());
                Ok(flow)
            }
            Self::Postgres(store) => store.create_login_flow(&flow).await,
        }
    }

    pub async fn consume_login_flow(&self, flow_id: &str) -> Result<Option<LoginFlow>> {
        let pending = match self {
            Self::Memory(store) => store
                .write()
                .expect("memory store write lock")
                .login_flows
                .remove(flow_id),
            Self::Postgres(store) => store.consume_login_flow(flow_id).await?,
        };

        Ok(pending.filter(|flow| !timestamp_is_expired(&flow.expires_at)))
    }

    pub async fn workspace_exists(&self, workspace_slug: &str) -> Result<bool> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .tenants
                .iter()
                .any(|tenant| tenant_matches_workspace(tenant, workspace_slug))),
            Self::Postgres(store) => store.workspace_exists(workspace_slug).await,
        }
    }

    pub async fn issue_session(
        &self,
        session_id: &str,
        provider: AuthProvider,
        identity_key: &IdentityLookup,
        workspace_slug: &str,
        now: &str,
        expires_at: &str,
    ) -> Result<AuthLoginResult> {
        match self {
            Self::Memory(store) => issue_memory_session(
                &mut store.write().expect("memory store write lock"),
                session_id,
                provider,
                identity_key,
                workspace_slug,
                now,
                expires_at,
            ),
            Self::Postgres(store) => {
                store
                    .issue_session(
                        session_id,
                        provider,
                        identity_key,
                        workspace_slug,
                        now,
                        expires_at,
                    )
                    .await
            }
        }
    }

    pub async fn upsert_oidc_user(
        &self,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let user_id = upsert_memory_oidc_user(
                    &mut store,
                    subject,
                    email,
                    display_name,
                    workspace_slug,
                    role,
                    now,
                )?;
                store
                    .users
                    .get(&user_id)
                    .cloned()
                    .context("oidc user should exist after upsert")
            }
            Self::Postgres(store) => {
                store
                    .upsert_oidc_user(subject, email, display_name, workspace_slug, role, now)
                    .await
            }
        }
    }

    pub async fn upsert_oauth_user(
        &self,
        provider: AuthProvider,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let user_id = upsert_memory_provider_user(
                    &mut store,
                    provider,
                    subject,
                    email,
                    display_name,
                    workspace_slug,
                    role,
                    now,
                )?;
                store
                    .users
                    .get(&user_id)
                    .cloned()
                    .context("oauth user should exist after upsert")
            }
            Self::Postgres(store) => {
                store
                    .upsert_oauth_user(
                        provider,
                        subject,
                        email,
                        display_name,
                        workspace_slug,
                        role,
                        now,
                    )
                    .await
            }
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<AuthLoginResult>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let Some(stored) = store.sessions.get(session_id).cloned() else {
                    return Ok(None);
                };
                if timestamp_is_expired(&stored.session.expires_at) {
                    store.sessions.remove(session_id);
                    return Ok(None);
                }
                Ok(Some(refresh_memory_session(&store, stored)))
            }
            Self::Postgres(store) => store.get_session(session_id).await,
        }
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<LogoutResponse> {
        match self {
            Self::Memory(store) => {
                let removed = store
                    .write()
                    .expect("memory store write lock")
                    .sessions
                    .remove(session_id)
                    .is_some();
                let _ = removed;
                Ok(LogoutResponse {
                    outcome: "signed_out".to_string(),
                })
            }
            Self::Postgres(store) => store.revoke_session(session_id).await,
        }
    }

    pub async fn unlink_provider(
        &self,
        session_id: &str,
        provider: AuthProvider,
    ) -> Result<Option<UnlinkAuthProviderResponse>> {
        match self {
            Self::Memory(store) => Ok(unlink_memory_provider(
                &mut store.write().expect("memory store write lock"),
                session_id,
                provider,
            )),
            Self::Postgres(store) => store.unlink_provider(session_id, provider).await,
        }
    }

    pub async fn list_tenants(&self) -> Result<TenantsResponse> {
        match self {
            Self::Memory(store) => Ok(TenantsResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .tenants
                    .clone(),
            }),
            Self::Postgres(store) => store.list_tenants().await,
        }
    }

    pub async fn list_projects(&self) -> Result<ProjectsResponse> {
        match self {
            Self::Memory(store) => Ok(ProjectsResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .projects
                    .clone(),
            }),
            Self::Postgres(store) => store.list_projects().await,
        }
    }

    pub async fn get_merchant_workspace(
        &self,
        tenant_id: &TenantId,
    ) -> Result<MerchantWorkspaceEnvelope> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let tenant_id_str = tenant_id.as_str();
                let mut recent_evaluations = store
                    .relay_evaluations
                    .iter()
                    .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                    .cloned()
                    .collect::<Vec<_>>();
                recent_evaluations.sort_by(|left, right| right.created_at.cmp(&left.created_at));

                Ok(MerchantWorkspaceEnvelope {
                    data: MerchantWorkspaceResponse {
                        merchant_enabled: store
                            .merchant_shops
                            .iter()
                            .any(|shop| shop.tenant_id.as_str() == tenant_id_str),
                        tenant_id: tenant_id.clone(),
                        shops: store
                            .merchant_shops
                            .iter()
                            .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                            .cloned()
                            .collect(),
                        card_products: store
                            .card_products
                            .iter()
                            .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                            .cloned()
                            .collect(),
                        trial_connections: store
                            .trial_connections
                            .iter()
                            .filter(|item| item.tenant_id.as_str() == tenant_id_str)
                            .cloned()
                            .collect(),
                        recent_evaluations,
                    },
                })
            }
            Self::Postgres(store) => store.get_merchant_workspace(tenant_id).await,
        }
    }

    pub async fn create_merchant_shop(&self, mut shop: MerchantShop) -> Result<MerchantShop> {
        shop.version = 1;
        shop.created_at = now_rfc3339();
        shop.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                shop.validate()?;
                let mut store = store.write().expect("memory store write lock");
                if store.merchant_shops.iter().any(|existing| {
                    existing.tenant_id == shop.tenant_id
                        && (existing.merchant_shop_id == shop.merchant_shop_id
                            || existing.slug == shop.slug)
                }) {
                    return Err(anyhow!("merchant_shop_already_exists"));
                }
                store.merchant_shops.push(shop.clone());
                Ok(shop)
            }
            Self::Postgres(store) => store.create_merchant_shop(&shop).await,
        }
    }

    pub async fn create_card_product(&self, mut product: CardProduct) -> Result<CardProduct> {
        product.version = 1;
        product.created_at = now_rfc3339();
        product.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                if store.card_products.iter().any(|existing| {
                    existing.tenant_id == product.tenant_id
                        && existing.card_product_id == product.card_product_id
                }) {
                    return Err(anyhow!("card_product_already_exists"));
                }
                if !store.merchant_shops.iter().any(|shop| {
                    shop.merchant_shop_id == product.merchant_shop_id
                        && shop.tenant_id == product.tenant_id
                }) {
                    return Err(anyhow!("merchant_shop_not_found"));
                }
                product.validate()?;
                store.card_products.push(product.clone());
                Ok(product)
            }
            Self::Postgres(store) => store.create_card_product(&product).await,
        }
    }

    pub async fn create_trial_connection(
        &self,
        mut connection: TrialConnection,
    ) -> Result<TrialConnection> {
        connection.version = 1;
        connection.created_at = now_rfc3339();
        connection.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                connection.validate()?;
                let mut store = store.write().expect("memory store write lock");
                if store.trial_connections.iter().any(|existing| {
                    existing.tenant_id == connection.tenant_id
                        && existing.trial_connection_id == connection.trial_connection_id
                }) {
                    return Err(anyhow!("trial_connection_already_exists"));
                }
                store.trial_connections.push(connection.clone());
                Ok(connection)
            }
            Self::Postgres(store) => store.create_trial_connection(&connection).await,
        }
    }

    pub async fn create_relay_evaluation(
        &self,
        tenant_id: &TenantId,
        trial_connection_id: &str,
    ) -> Result<RelayEvaluation> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let connection = store
                    .trial_connections
                    .iter()
                    .find(|item| {
                        item.tenant_id == *tenant_id
                            && item.trial_connection_id.as_str() == trial_connection_id
                    })
                    .cloned()
                    .context("trial_connection_not_found")?;

                let created_at = now_rfc3339();
                let replay_capsule_id =
                    ReplayCapsuleId::parse(format!("replay_{}", next_id_suffix())).unwrap();
                let route_receipt_id =
                    RouteReceiptId::parse(format!("routercpt_{}", next_id_suffix())).unwrap();
                let config_snapshot = store
                    .config_snapshots
                    .iter()
                    .find(|snapshot| {
                        snapshot.tenant_id == *tenant_id
                            && snapshot.status == ConfigSnapshotStatus::Active
                    })
                    .or_else(|| {
                        store
                            .config_snapshots
                            .iter()
                            .find(|snapshot| snapshot.tenant_id == *tenant_id)
                    })
                    .cloned()
                    .context("config_snapshot_not_found")?;
                let request_id = format!("req_{}", next_id_suffix());
                let trace_id = format!("trace_{}", next_id_suffix());

                let replay_capsule = ReplayCapsule {
                    replay_capsule_id: replay_capsule_id.clone(),
                    request_id: request_id.clone(),
                    trace_id: trace_id.clone(),
                    route_receipt_id: route_receipt_id.clone(),
                    config_snapshot_id: config_snapshot.config_snapshot_id.clone(),
                    redaction_tier: RedactionTier::StructuredRedacted,
                    normalized_request_summary: NormalizedRequestSummary {
                        protocol_family: "openai_chat".to_string(),
                        model_alias: connection.target_model.clone(),
                        estimated_prompt_tokens: 480,
                    },
                    upstream_error_summary: Some(UpstreamErrorSummary {
                        code: replay_upstream_error_code(&connection),
                    }),
                };

                let evaluation =
                    build_relay_evaluation(tenant_id, &connection, replay_capsule_id, created_at);
                evaluation.validate()?;
                let route_receipt = build_merchant_replay_route_receipt(
                    tenant_id,
                    &config_snapshot,
                    route_receipt_id,
                    request_id,
                    trace_id,
                    connection.target_model.clone(),
                    evaluation.created_at.clone(),
                );

                store.replay_capsules.insert(
                    replay_capsule.replay_capsule_id.as_str().to_string(),
                    replay_capsule,
                );
                store.route_receipts.insert(
                    route_receipt.route_receipt_id.as_str().to_string(),
                    route_receipt,
                );
                store.relay_evaluations.push(evaluation.clone());
                Ok(evaluation)
            }
            Self::Postgres(store) => {
                store
                    .create_relay_evaluation(tenant_id, trial_connection_id)
                    .await
            }
        }
    }

    pub async fn get_replay_capsule(
        &self,
        tenant_id: &TenantId,
        replay_capsule_id: &str,
    ) -> Result<Option<ReplayCapsuleResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let has_tenant_evaluation = store.relay_evaluations.iter().any(|evaluation| {
                    evaluation.tenant_id == *tenant_id
                        && evaluation.replay_capsule_id.as_str() == replay_capsule_id
                });

                if !has_tenant_evaluation {
                    return Ok(None);
                }

                Ok(store
                    .replay_capsules
                    .get(replay_capsule_id)
                    .cloned()
                    .map(|replay_capsule| ReplayCapsuleResponse { replay_capsule }))
            }
            Self::Postgres(store) => store.get_replay_capsule(tenant_id, replay_capsule_id).await,
        }
    }

    pub async fn get_project(&self, project_id: &str) -> Result<Option<Project>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .projects
                .iter()
                .find(|item| item.project_id.as_str() == project_id)
                .cloned()),
            Self::Postgres(store) => store.get_project(project_id).await,
        }
    }

    pub async fn list_provider_resources(
        &self,
        filters: &ProviderResourceFilters,
    ) -> Result<ProviderResourcesResponse> {
        match self {
            Self::Memory(store) => Ok(ProviderResourcesResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .provider_resources
                    .iter()
                    .filter(|resource| provider_matches_filters(resource, filters))
                    .cloned()
                    .collect(),
            }),
            Self::Postgres(store) => store.list_provider_resources(filters).await,
        }
    }

    pub async fn get_provider_resource(
        &self,
        provider_resource_id: &str,
    ) -> Result<Option<ProviderResource>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .provider_resources
                .iter()
                .find(|item| item.provider_resource_id.as_str() == provider_resource_id)
                .cloned()),
            Self::Postgres(store) => store.get_provider_resource(provider_resource_id).await,
        }
    }

    pub async fn create_provider_resource(
        &self,
        mut provider_resource: ProviderResource,
    ) -> Result<ProviderResource> {
        fail_close_new_provider_resource(&mut provider_resource);
        provider_resource.version = 1;
        provider_resource.created_at = now_rfc3339();
        provider_resource.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .provider_resources
                    .push(provider_resource.clone());
                Ok(provider_resource)
            }
            Self::Postgres(store) => store.create_provider_resource(&provider_resource).await,
        }
    }

    pub async fn update_provider_resource(
        &self,
        provider_resource: ProviderResource,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        let mut provider_resource = provider_resource;
        provider_resource.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => Ok(update_memory_provider_resource(
                &mut store.write().expect("memory store write lock"),
                &provider_resource,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .update_provider_resource(&provider_resource, expected_version)
                    .await
            }
        }
    }

    pub async fn disable_provider_resource(
        &self,
        provider_resource_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        match self {
            Self::Memory(store) => Ok(disable_memory_provider_resource(
                &mut store.write().expect("memory store write lock"),
                provider_resource_id,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .disable_provider_resource(provider_resource_id, expected_version)
                    .await
            }
        }
    }

    pub async fn list_route_policies(&self) -> Result<RoutePoliciesResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(RoutePoliciesResponse {
                    data: store
                        .route_policies
                        .iter()
                        .filter(|policy| {
                            !store
                                .route_policy_disabled_ids
                                .contains(policy.route_policy_id.as_str())
                        })
                        .cloned()
                        .collect(),
                })
            }
            Self::Postgres(store) => store.list_route_policies().await,
        }
    }

    #[allow(dead_code)]
    pub async fn get_route_policy(&self, route_policy_id: &str) -> Result<Option<RoutePolicy>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .route_policies
                .iter()
                .find(|item| item.route_policy_id.as_str() == route_policy_id)
                .cloned()),
            Self::Postgres(store) => store.get_route_policy(route_policy_id).await,
        }
    }

    pub async fn create_route_policy(&self, mut route_policy: RoutePolicy) -> Result<RoutePolicy> {
        route_policy.version = 1;
        route_policy.created_at = now_rfc3339();
        route_policy.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .route_policies
                    .push(route_policy.clone());
                Ok(route_policy)
            }
            Self::Postgres(store) => store.create_route_policy(&route_policy).await,
        }
    }

    pub async fn update_route_policy(
        &self,
        route_policy: RoutePolicy,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        let mut route_policy = route_policy;
        route_policy.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => Ok(update_memory_route_policy(
                &mut store.write().expect("memory store write lock"),
                &route_policy,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .update_route_policy(&route_policy, expected_version)
                    .await
            }
        }
    }

    pub async fn disable_route_policy(
        &self,
        route_policy_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        match self {
            Self::Memory(store) => Ok(disable_memory_route_policy(
                &mut store.write().expect("memory store write lock"),
                route_policy_id,
                expected_version,
            )),
            Self::Postgres(store) => {
                store
                    .disable_route_policy(route_policy_id, expected_version)
                    .await
            }
        }
    }

    pub async fn list_config_snapshots(&self) -> Result<ConfigSnapshotsResponse> {
        match self {
            Self::Memory(store) => Ok(ConfigSnapshotsResponse {
                data: store
                    .read()
                    .expect("memory store read lock")
                    .config_snapshots
                    .clone(),
            }),
            Self::Postgres(store) => store.list_config_snapshots().await,
        }
    }

    pub async fn create_config_snapshot(
        &self,
        mut config_snapshot: ConfigSnapshot,
    ) -> Result<ConfigSnapshot> {
        config_snapshot.activated_at = None;
        config_snapshot.status = ConfigSnapshotStatus::Draft;
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .config_snapshots
                    .push(config_snapshot.clone());
                Ok(config_snapshot)
            }
            Self::Postgres(store) => store.create_config_snapshot(&config_snapshot).await,
        }
    }

    pub async fn create_api_key(
        &self,
        provider_resource_id: ProviderResourceId,
        display_name: &str,
        api_key: &str,
    ) -> Result<ApiKey> {
        let now = now_rfc3339();
        let prefix = api_key_prefix(api_key);
        let hash = hash_api_key(api_key);
        match self {
            Self::Memory(store) => Ok(insert_memory_api_key(
                &mut store.write().expect("memory store write lock"),
                provider_resource_id,
                display_name,
                &prefix,
                hash,
                now,
            )),
            Self::Postgres(store) => {
                store
                    .create_api_key(provider_resource_id.as_str(), display_name, &prefix, &hash)
                    .await
            }
        }
    }

    pub async fn list_api_keys(&self) -> Result<ApiKeysResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(ApiKeysResponse {
                    data: store
                        .api_keys
                        .iter()
                        .map(ApiKeyRecord::public_view)
                        .collect(),
                })
            }
            Self::Postgres(store) => store.list_api_keys().await,
        }
    }

    pub async fn create_opening_grant(
        &self,
        draft: OpeningGrantDraft,
        credential_plaintext: &str,
    ) -> Result<OpeningGrantCreateResult> {
        let now = now_rfc3339();
        let credential_hash = hash_api_key(credential_plaintext);
        let record = OpeningGrantRecord {
            grant_id: draft.grant_id,
            tenant_id: draft.tenant_id,
            project_id: draft.project_id,
            owner_account_id: draft.owner_account_id,
            grantee_kind: draft.grantee_kind,
            grantee_id: draft.grantee_id,
            grantee_label: draft.grantee_label,
            config_snapshot_id: draft.config_snapshot_id,
            route_policy_id: draft.route_policy_id,
            budget_policy_id: draft.budget_policy_id,
            provider_resource_ids: draft.provider_resource_ids,
            credential_kind: draft.credential_kind,
            credential_id: format!("cred_{}", &credential_hash[..16]),
            credential_key_prefix: api_key_prefix(credential_plaintext),
            credential_last_four: credential_last_four(credential_plaintext),
            credential_hash,
            scopes: draft.scopes,
            expires_at: draft.expires_at,
            status: "active".to_string(),
            created_by: draft.created_by,
            created_at: now.clone(),
            updated_at: now,
            version: 1,
            revoked_at: None,
            revoked_by: None,
        };
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                if let Some(blocker) = opening_grant_create_blocker(&store.opening_grants, &record)
                {
                    return Ok(blocker);
                }
                store.opening_grants.push(record.clone());
                Ok(OpeningGrantCreateResult::Created(Box::new(
                    record.public_view(),
                )))
            }
            Self::Postgres(store) => store.create_opening_grant(&record).await,
        }
    }

    pub async fn list_opening_grants(&self) -> Result<OpeningGrantsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(OpeningGrantsResponse {
                    data: store
                        .opening_grants
                        .iter()
                        .map(OpeningGrantRecord::public_view)
                        .collect(),
                })
            }
            Self::Postgres(store) => store.list_opening_grants().await,
        }
    }

    pub async fn prepare_delivery(
        &self,
        draft: DeliveryPrepareDraft,
        redemption_code: &str,
        browser_file_unlock_code: &str,
    ) -> Result<DeliveryPrepareResult> {
        let prepared = prepare_delivery_records(draft, redemption_code, browser_file_unlock_code);
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                if let Some(blocker) = owner_redemption_capacity_blocker_for_memory(
                    &store.deliveries,
                    &store.delivery_codes,
                    &prepared,
                ) {
                    return Ok(blocker);
                }
                store.deliveries.push(prepared.delivery.clone());
                store.delivery_codes.extend(prepared.codes.clone());
                store.delivery_secret_plaintexts.insert(
                    delivery_secret_key(
                        &prepared.delivery.delivery_id,
                        DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK,
                    ),
                    browser_file_unlock_code.to_string(),
                );
                store
                    .delivery_entitlements
                    .push(prepared.entitlement.clone());
                Ok(DeliveryPrepareResult::Prepared(Box::new(
                    DeliveryPrepareResponse {
                        data: delivery_projection(
                            &prepared.delivery,
                            &prepared.codes,
                            &prepared.entitlement,
                        ),
                        one_time_codes: DeliveryOneTimeCodes {
                            redemption_code: redemption_code.to_string(),
                            browser_file_unlock_code: browser_file_unlock_code.to_string(),
                        },
                    },
                )))
            }
            Self::Postgres(store) => {
                store
                    .prepare_delivery(&prepared, redemption_code, browser_file_unlock_code)
                    .await
            }
        }
    }

    pub async fn get_delivery(&self, delivery_id: &str) -> Result<Option<DeliveryResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(memory_delivery_projection(&store, delivery_id)
                    .map(|data| DeliveryResponse { data }))
            }
            Self::Postgres(store) => store.get_delivery(delivery_id).await,
        }
    }

    pub async fn get_delivery_secret_plaintext(
        &self,
        delivery_id: &str,
        code_type: &str,
    ) -> Result<Option<String>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_secret_plaintexts
                    .get(&delivery_secret_key(delivery_id, code_type))
                    .cloned())
            }
            Self::Postgres(store) => {
                store
                    .get_delivery_secret_plaintext(delivery_id, code_type)
                    .await
            }
        }
    }

    pub async fn revoke_delivery(
        &self,
        delivery_id: &str,
        expected_version: u64,
        revoked_by: &str,
        revoke_reason: Option<String>,
    ) -> Result<ConcurrencyResult<DeliveryResponse>> {
        match self {
            Self::Memory(store) => Ok(revoke_memory_delivery(
                &mut store.write().expect("memory store write lock"),
                delivery_id,
                expected_version,
                revoked_by,
                revoke_reason,
            )),
            Self::Postgres(store) => {
                store
                    .revoke_delivery(delivery_id, expected_version, revoked_by, revoke_reason)
                    .await
            }
        }
    }

    #[cfg(test)]
    pub async fn delivery_code_hashes_for_tests(&self, delivery_id: &str) -> Result<Vec<String>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_codes
                    .iter()
                    .filter(|code| code.delivery_id == delivery_id)
                    .map(|code| code.code_hash.clone())
                    .collect())
            }
            Self::Postgres(store) => {
                let rows = sqlx::query(
                    "SELECT code_hash
                       FROM delivery_codes
                      WHERE delivery_id = $1
                      ORDER BY code_type",
                )
                .bind(delivery_id)
                .fetch_all(&store.pool)
                .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| row.get::<String, _>("code_hash"))
                    .collect())
            }
        }
    }

    pub async fn create_delivery_artifact(
        &self,
        draft: DeliveryArtifactDraft,
    ) -> Result<DeliveryArtifactResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(DeliveryArtifactResponse {
                    data: insert_memory_delivery_artifact(&mut store, draft).public_view(),
                })
            }
            Self::Postgres(store) => store.create_delivery_artifact(draft).await,
        }
    }

    pub async fn create_delivery_upload_batch(
        &self,
        draft: DeliveryUploadBatchDraft,
    ) -> Result<DeliveryUploadBatchResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(insert_memory_delivery_upload_batch(&mut store, draft).public_response())
            }
            Self::Postgres(store) => store.create_delivery_upload_batch(draft).await,
        }
    }

    pub async fn get_delivery_upload_batch(
        &self,
        batch_id: &str,
    ) -> Result<Option<DeliveryUploadBatchResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_upload_batches
                    .iter()
                    .find(|batch| batch.batch_id == batch_id)
                    .map(DeliveryUploadBatchRecord::public_response))
            }
            Self::Postgres(store) => store.get_delivery_upload_batch(batch_id).await,
        }
    }

    pub async fn list_delivery_upload_batch_items(
        &self,
        batch_id: &str,
    ) -> Result<Option<DeliveryUploadBatchItemsResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                if !store
                    .delivery_upload_batches
                    .iter()
                    .any(|batch| batch.batch_id == batch_id)
                {
                    return Ok(None);
                }
                let mut data = store
                    .delivery_upload_batch_items
                    .iter()
                    .filter(|item| item.batch_id == batch_id)
                    .map(DeliveryUploadBatchItemRecord::public_view)
                    .collect::<Vec<_>>();
                data.sort_by(|left, right| left.row_index.cmp(&right.row_index));
                Ok(Some(DeliveryUploadBatchItemsResponse { data }))
            }
            Self::Postgres(store) => store.list_delivery_upload_batch_items(batch_id).await,
        }
    }

    pub async fn process_delivery_upload_batch(
        &self,
        batch_id: &str,
        actor_id: &str,
    ) -> Result<DeliveryUploadProcessResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(process_memory_delivery_upload_batch(
                    &mut store, batch_id, actor_id,
                ))
            }
            Self::Postgres(store) => {
                store
                    .process_delivery_upload_batch(batch_id, actor_id)
                    .await
            }
        }
    }

    pub async fn list_delivery_artifacts(
        &self,
        delivery_id: &str,
    ) -> Result<DeliveryArtifactsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let mut data = store
                    .delivery_artifacts
                    .iter()
                    .filter(|artifact| artifact.delivery_id == delivery_id)
                    .map(DeliveryArtifactRecord::public_view)
                    .collect::<Vec<_>>();
                data.sort_by(|left, right| {
                    right
                        .version
                        .cmp(&left.version)
                        .then_with(|| right.created_at.cmp(&left.created_at))
                });
                Ok(DeliveryArtifactsResponse { data })
            }
            Self::Postgres(store) => store.list_delivery_artifacts(delivery_id).await,
        }
    }

    pub async fn get_delivery_artifact(
        &self,
        delivery_id: &str,
        artifact_id: &str,
    ) -> Result<Option<DeliveryArtifactResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_artifacts
                    .iter()
                    .find(|artifact| {
                        artifact.delivery_id == delivery_id && artifact.artifact_id == artifact_id
                    })
                    .map(|artifact| DeliveryArtifactResponse {
                        data: artifact.public_view(),
                    }))
            }
            Self::Postgres(store) => store.get_delivery_artifact(delivery_id, artifact_id).await,
        }
    }

    pub async fn get_delivery_by_redemption_code_hash(
        &self,
        code_hash: &str,
    ) -> Result<Option<DeliveryResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let delivery_id = store
                    .delivery_codes
                    .iter()
                    .find(|code| {
                        code.code_type == DELIVERY_CODE_TYPE_REDEMPTION
                            && code.code_hash == code_hash
                    })
                    .map(|code| code.delivery_id.clone());
                Ok(delivery_id.and_then(|delivery_id| {
                    memory_delivery_projection(&store, &delivery_id)
                        .map(|data| DeliveryResponse { data })
                }))
            }
            Self::Postgres(store) => store.get_delivery_by_redemption_code_hash(code_hash).await,
        }
    }

    pub async fn get_delivery_by_entitlement_id(
        &self,
        entitlement_id: &str,
    ) -> Result<Option<DeliveryResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let delivery_id = store
                    .delivery_entitlements
                    .iter()
                    .find(|entitlement| entitlement.entitlement_id == entitlement_id)
                    .map(|entitlement| entitlement.delivery_id.clone());
                Ok(delivery_id.and_then(|delivery_id| {
                    memory_delivery_projection(&store, &delivery_id)
                        .map(|data| DeliveryResponse { data })
                }))
            }
            Self::Postgres(store) => store.get_delivery_by_entitlement_id(entitlement_id).await,
        }
    }

    pub async fn redeem_delivery_activation(
        &self,
        code_hash: &str,
        activation_id: String,
        activated_by: &str,
    ) -> Result<DeliveryRedeemResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(redeem_memory_delivery_activation(
                    &mut store,
                    code_hash,
                    activation_id,
                    activated_by,
                ))
            }
            Self::Postgres(store) => {
                store
                    .redeem_delivery_activation(code_hash, activation_id, activated_by)
                    .await
            }
        }
    }

    pub async fn get_delivery_activation(
        &self,
        activation_id: &str,
    ) -> Result<Option<DeliveryActivationResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_activations
                    .iter()
                    .find(|activation| activation.activation_id == activation_id)
                    .map(|activation| DeliveryActivationResponse {
                        data: activation.public_view(),
                    }))
            }
            Self::Postgres(store) => store.get_delivery_activation(activation_id).await,
        }
    }

    pub async fn list_delivery_service_segments(
        &self,
        entitlement_id: &str,
    ) -> Result<DeliveryServiceSegmentsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let mut data = store
                    .delivery_service_segments
                    .iter()
                    .filter(|segment| segment.entitlement_id == entitlement_id)
                    .map(DeliveryServiceSegmentRecord::public_view)
                    .collect::<Vec<_>>();
                data.sort_by(|left, right| left.segment_index.cmp(&right.segment_index));
                Ok(DeliveryServiceSegmentsResponse { data })
            }
            Self::Postgres(store) => store.list_delivery_service_segments(entitlement_id).await,
        }
    }

    pub async fn list_delivery_lifecycle_events(
        &self,
        entitlement_id: &str,
    ) -> Result<DeliveryLifecycleEventsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let mut data = store
                    .delivery_lifecycle_events
                    .iter()
                    .filter(|event| event.entitlement_id == entitlement_id)
                    .map(DeliveryLifecycleEventRecord::public_view)
                    .collect::<Vec<_>>();
                data.sort_by(|left, right| left.created_at.cmp(&right.created_at));
                Ok(DeliveryLifecycleEventsResponse { data })
            }
            Self::Postgres(store) => store.list_delivery_lifecycle_events(entitlement_id).await,
        }
    }

    pub async fn reconcile_delivery_lifecycle(
        &self,
        entitlement_id: &str,
        actor_id: &str,
    ) -> Result<DeliveryLifecycleResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(reconcile_memory_delivery_lifecycle(
                    &mut store,
                    entitlement_id,
                    actor_id,
                ))
            }
            Self::Postgres(store) => {
                store
                    .reconcile_delivery_lifecycle(entitlement_id, actor_id)
                    .await
            }
        }
    }

    pub async fn extend_delivery_entitlement(
        &self,
        draft: DeliveryEntitlementExtendDraft,
    ) -> Result<ConcurrencyResult<DeliveryLifecycleResponse>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(extend_memory_delivery_entitlement(&mut store, draft))
            }
            Self::Postgres(store) => store.extend_delivery_entitlement(draft).await,
        }
    }

    pub async fn get_delivery_operations_overview(
        &self,
        scope: DeliveryOperationsScope,
    ) -> Result<DeliveryOperationsOverviewResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(delivery_operations_overview(
                    delivery_operations_records_from_memory(&store),
                    scope,
                ))
            }
            Self::Postgres(store) => {
                let records = store.load_delivery_operations_records(&scope).await?;
                Ok(delivery_operations_overview(records, scope))
            }
        }
    }

    pub async fn get_delivery_operations_timeline(
        &self,
        filters: DeliveryOperationsObjectFilters,
    ) -> Result<Option<DeliveryOperationsTimelineResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(delivery_operations_timeline(
                    delivery_operations_records_from_memory(&store),
                    filters,
                ))
            }
            Self::Postgres(store) => {
                let records = store
                    .load_delivery_operations_records(&filters.scope)
                    .await?;
                Ok(delivery_operations_timeline(records, filters))
            }
        }
    }

    pub async fn list_delivery_operations_exceptions(
        &self,
        filters: DeliveryOperationsExceptionFilters,
    ) -> Result<DeliveryOperationsExceptionsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(delivery_operations_exceptions(
                    delivery_operations_records_from_memory(&store),
                    filters,
                ))
            }
            Self::Postgres(store) => {
                let records = store
                    .load_delivery_operations_records(&filters.scope)
                    .await?;
                Ok(delivery_operations_exceptions(records, filters))
            }
        }
    }

    pub async fn get_delivery_operations_detail(
        &self,
        filters: DeliveryOperationsObjectFilters,
    ) -> Result<Option<DeliveryOperationsDetailResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(delivery_operations_detail(
                    delivery_operations_records_from_memory(&store),
                    filters,
                ))
            }
            Self::Postgres(store) => {
                let records = store
                    .load_delivery_operations_records(&filters.scope)
                    .await?;
                Ok(delivery_operations_detail(records, filters))
            }
        }
    }

    pub async fn issue_delivery_download_grant(
        &self,
        draft: DeliveryDownloadGrantDraft,
    ) -> Result<DeliveryDownloadGrantIssueResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(issue_memory_delivery_download_grant(&mut store, draft))
            }
            Self::Postgres(store) => store.issue_delivery_download_grant(draft).await,
        }
    }

    pub async fn get_delivery_download_grant(
        &self,
        grant_id: &str,
    ) -> Result<Option<DeliveryDownloadGrantResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_download_grants
                    .iter()
                    .find(|grant| grant.grant_id == grant_id)
                    .map(|grant| DeliveryDownloadGrantResponse {
                        data: grant.public_view(),
                    }))
            }
            Self::Postgres(store) => store.get_delivery_download_grant(grant_id).await,
        }
    }

    pub async fn revoke_delivery_download_grant(
        &self,
        grant_id: &str,
        expected_version: u64,
        revoked_by: &str,
        revoke_reason: Option<String>,
    ) -> Result<ConcurrencyResult<DeliveryDownloadGrantResponse>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(revoke_memory_delivery_download_grant(
                    &mut store,
                    grant_id,
                    expected_version,
                    revoked_by,
                    revoke_reason,
                ))
            }
            Self::Postgres(store) => {
                store
                    .revoke_delivery_download_grant(
                        grant_id,
                        expected_version,
                        revoked_by,
                        revoke_reason,
                    )
                    .await
            }
        }
    }

    pub async fn consume_delivery_download_grant(
        &self,
        token_hash: &str,
    ) -> Result<DeliveryDownloadConsumeResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(consume_memory_delivery_download_grant(
                    &mut store, token_hash,
                ))
            }
            Self::Postgres(store) => store.consume_delivery_download_grant(token_hash).await,
        }
    }

    #[cfg(test)]
    pub async fn delivery_artifact_ciphertext_for_tests(
        &self,
        artifact_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_artifacts
                    .iter()
                    .find(|artifact| artifact.artifact_id == artifact_id)
                    .map(|artifact| artifact.ciphertext.clone()))
            }
            Self::Postgres(store) => {
                let row = sqlx::query(
                    "SELECT ciphertext
                       FROM delivery_artifacts
                      WHERE artifact_id = $1",
                )
                .bind(artifact_id)
                .fetch_optional(&store.pool)
                .await?;
                Ok(row.map(|row| row.get::<Vec<u8>, _>("ciphertext")))
            }
        }
    }

    #[cfg(test)]
    pub async fn delivery_download_token_hashes_for_tests(
        &self,
        grant_id: &str,
    ) -> Result<Vec<String>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_download_grants
                    .iter()
                    .filter(|grant| grant.grant_id == grant_id)
                    .map(|grant| grant.token_hash.clone())
                    .collect())
            }
            Self::Postgres(store) => {
                let rows = sqlx::query(
                    "SELECT token_hash
                       FROM delivery_download_grants
                      WHERE grant_id = $1",
                )
                .bind(grant_id)
                .fetch_all(&store.pool)
                .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| row.get::<String, _>("token_hash"))
                    .collect())
            }
        }
    }

    #[cfg(test)]
    pub async fn expire_delivery_download_grant_for_tests(&self, grant_id: &str) -> Result<()> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                if let Some(grant) = store
                    .delivery_download_grants
                    .iter_mut()
                    .find(|grant| grant.grant_id == grant_id)
                {
                    grant.expires_at = "2000-01-01T00:00:00Z".to_string();
                    grant.updated_at = now_rfc3339();
                }
                Ok(())
            }
            Self::Postgres(store) => {
                let Some(row) = sqlx::query(
                    "SELECT payload
                       FROM delivery_download_grants
                      WHERE grant_id = $1",
                )
                .bind(grant_id)
                .fetch_optional(&store.pool)
                .await?
                else {
                    return Ok(());
                };
                let mut grant = row.get::<Json<DeliveryDownloadGrantRecord>, _>("payload").0;
                grant.expires_at = "2000-01-01T00:00:00Z".to_string();
                grant.updated_at = now_rfc3339();
                sqlx::query(
                    "UPDATE delivery_download_grants
                        SET expires_at = $2, payload = $3, updated_at = $4
                      WHERE grant_id = $1",
                )
                .bind(grant_id)
                .bind(&grant.expires_at)
                .bind(Json(&grant))
                .bind(&grant.updated_at)
                .execute(&store.pool)
                .await?;
                Ok(())
            }
        }
    }

    #[cfg(test)]
    pub async fn expire_delivery_entitlement_for_tests(&self, entitlement_id: &str) -> Result<()> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                if let Some(entitlement) = store
                    .delivery_entitlements
                    .iter_mut()
                    .find(|entitlement| entitlement.entitlement_id == entitlement_id)
                {
                    entitlement.ends_at = "2000-01-01T00:00:00Z".to_string();
                    entitlement.updated_at = now_rfc3339();
                    entitlement.version = entitlement.version.saturating_add(1);
                }
                Ok(())
            }
            Self::Postgres(store) => {
                let Some(row) = sqlx::query(
                    "SELECT payload
                       FROM delivery_entitlements
                      WHERE entitlement_id = $1",
                )
                .bind(entitlement_id)
                .fetch_optional(&store.pool)
                .await?
                else {
                    return Ok(());
                };
                let mut entitlement = row.get::<Json<DeliveryEntitlementRecord>, _>("payload").0;
                entitlement.ends_at = "2000-01-01T00:00:00Z".to_string();
                entitlement.updated_at = now_rfc3339();
                entitlement.version = entitlement.version.saturating_add(1);
                sqlx::query(
                    "UPDATE delivery_entitlements
                        SET ends_at = $2, payload = $3, updated_at = $4
                      WHERE entitlement_id = $1",
                )
                .bind(entitlement_id)
                .bind(&entitlement.ends_at)
                .bind(Json(&entitlement))
                .bind(&entitlement.updated_at)
                .execute(&store.pool)
                .await?;
                Ok(())
            }
        }
    }

    #[cfg(test)]
    pub async fn delivery_activation_records_for_tests(
        &self,
        delivery_id: &str,
    ) -> Result<Vec<DeliveryActivationRecord>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(store
                    .delivery_activations
                    .iter()
                    .filter(|activation| activation.delivery_id == delivery_id)
                    .cloned()
                    .collect())
            }
            Self::Postgres(store) => {
                let rows = sqlx::query(
                    "SELECT payload
                       FROM delivery_activations
                      WHERE delivery_id = $1
                      ORDER BY activated_at",
                )
                .bind(delivery_id)
                .fetch_all(&store.pool)
                .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| row.get::<Json<DeliveryActivationRecord>, _>("payload").0)
                    .collect())
            }
        }
    }

    pub async fn list_codex_auth_accounts(&self) -> Result<CodexAuthAccountsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(CodexAuthAccountsResponse {
                    data: store
                        .codex_auth_accounts
                        .iter()
                        .map(|account| {
                            let active_runs = active_runtime_lease_count(
                                &store,
                                Some(account.codex_account_id.as_str()),
                                None,
                                None,
                                None,
                                account.provider.as_str(),
                            );
                            account.public_view_with_active_runs(
                                u32::try_from(active_runs).unwrap_or(u32::MAX),
                            )
                        })
                        .collect(),
                })
            }
            Self::Postgres(store) => store.list_codex_auth_accounts().await,
        }
    }

    pub async fn create_codex_auth_account(
        &self,
        mut record: CodexAuthAccountRecord,
    ) -> Result<CodexAuthAccount> {
        record.version = 1;
        record.created_at = now_rfc3339();
        record.updated_at = now_rfc3339();
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .codex_auth_accounts
                    .push(record.clone());
                Ok(record.public_view())
            }
            Self::Postgres(store) => store.create_codex_auth_account(&record).await,
        }
    }

    #[allow(dead_code)]
    pub async fn lease_codex_auth_account(
        &self,
        provider_resource_id: Option<&str>,
    ) -> Result<Option<CodexAuthAccountRecord>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let leased_until = lease_until_rfc3339();
                let now = now_rfc3339();
                let account = store
                    .codex_auth_accounts
                    .iter_mut()
                    .find(|account| {
                        account.status == "active"
                            && provider_resource_id.is_none_or(|provider_resource_id| {
                                account.provider_resource_id.as_str() == provider_resource_id
                            })
                    })
                    .map(|account| {
                        account.leased_until = Some(leased_until);
                        account.updated_at = now;
                        account.version = account.version.saturating_add(1);
                        account.clone()
                    });
                Ok(account)
            }
            Self::Postgres(store) => store.lease_codex_auth_account(provider_resource_id).await,
        }
    }

    pub async fn select_oauth_pool_account(
        &self,
        mut request: OAuthPoolSelectionRequest,
    ) -> Result<OAuthPoolSelection> {
        if request.pool_id.is_none() {
            request.pool_id.clone_from(&request.provider_resource_id);
        }
        if request.provider.is_none() {
            request.provider = Some("codex".to_string());
        }

        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(select_memory_oauth_pool_account(&mut store, &request))
            }
            Self::Postgres(store) => store.select_oauth_pool_account(&request).await,
        }
    }

    pub async fn list_oauth_sharing_leases(&self) -> Result<OAuthSharingLeasesResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(OAuthSharingLeasesResponse {
                    data: store.oauth_sharing_leases.clone(),
                })
            }
            Self::Postgres(store) => store.list_oauth_sharing_leases().await,
        }
    }

    pub async fn upsert_oauth_sharing_lease(
        &self,
        mut record: OAuthSharingLeaseRecord,
    ) -> Result<OAuthSharingLeaseRecord> {
        normalize_oauth_sharing_lease(&mut record);
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(upsert_memory_oauth_sharing_lease(&mut store, record))
            }
            Self::Postgres(store) => store.upsert_oauth_sharing_lease(&record).await,
        }
    }

    pub async fn revoke_oauth_sharing_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<OAuthSharingLeaseRecord>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(revoke_memory_oauth_sharing_lease(&mut store, lease_id))
            }
            Self::Postgres(store) => store.revoke_oauth_sharing_lease(lease_id).await,
        }
    }

    pub async fn list_oauth_carpools(&self) -> Result<OAuthCarpoolsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(OAuthCarpoolsResponse {
                    data: store.oauth_carpools.clone(),
                })
            }
            Self::Postgres(store) => store.list_oauth_carpools().await,
        }
    }

    pub async fn upsert_oauth_carpool(
        &self,
        mut record: OAuthCarpoolRecord,
    ) -> Result<OAuthCarpoolRecord> {
        normalize_oauth_carpool(&mut record);
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(upsert_memory_oauth_carpool(&mut store, record))
            }
            Self::Postgres(store) => store.upsert_oauth_carpool(&record).await,
        }
    }

    pub async fn remove_oauth_carpool(
        &self,
        carpool_id: &str,
    ) -> Result<Option<OAuthCarpoolRecord>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(remove_memory_oauth_carpool(&mut store, carpool_id))
            }
            Self::Postgres(store) => store.remove_oauth_carpool(carpool_id).await,
        }
    }

    pub async fn read_oauth_sharing_usage(
        &self,
        filters: OAuthSharingUsageFilters<'_>,
    ) -> Result<OAuthSharingUsageResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(read_memory_oauth_sharing_usage(&store, filters))
            }
            Self::Postgres(store) => store.read_oauth_sharing_usage(filters).await,
        }
    }

    pub async fn heartbeat_oauth_pool_runtime_lease(
        &self,
        runtime_lease_id: &str,
    ) -> Result<Option<OAuthPoolRuntimeLeaseRecord>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(heartbeat_memory_runtime_lease(&mut store, runtime_lease_id))
            }
            Self::Postgres(store) => {
                store
                    .heartbeat_oauth_pool_runtime_lease(runtime_lease_id)
                    .await
            }
        }
    }

    pub async fn release_oauth_pool_runtime_lease(
        &self,
        runtime_lease_id: &str,
    ) -> Result<Option<OAuthPoolRuntimeLeaseRecord>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(release_memory_runtime_lease(&mut store, runtime_lease_id))
            }
            Self::Postgres(store) => {
                store
                    .release_oauth_pool_runtime_lease(runtime_lease_id)
                    .await
            }
        }
    }

    pub async fn record_oauth_pool_account_feedback(
        &self,
        account_id: &str,
        feedback: OAuthPoolAccountFeedback,
    ) -> Result<Option<CodexAuthAccount>> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(
                    apply_memory_account_feedback(&mut store, account_id, feedback)
                        .map(|record| record.public_view()),
                )
            }
            Self::Postgres(store) => {
                store
                    .record_oauth_pool_account_feedback(account_id, feedback)
                    .await
            }
        }
    }

    #[cfg(test)]
    pub async fn set_codex_auth_account_runtime_state(
        &self,
        codex_account_id: &str,
        rate_limited_until: Option<String>,
        concurrency_limit: Option<u32>,
        active_runs: u32,
    ) -> Result<()> {
        match self {
            Self::Memory(store) => {
                if let Some(account) = store
                    .write()
                    .expect("memory store write lock")
                    .codex_auth_accounts
                    .iter_mut()
                    .find(|account| account.codex_account_id == codex_account_id)
                {
                    account.rate_limited_until = rate_limited_until;
                    account.concurrency_limit = concurrency_limit;
                    account.active_runs = active_runs;
                    account.updated_at = now_rfc3339();
                }
                Ok(())
            }
            Self::Postgres(_) => Ok(()),
        }
    }

    pub async fn revoke_api_key(
        &self,
        api_key_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ApiKey>> {
        match self {
            Self::Memory(store) => Ok(revoke_memory_api_key(
                &mut store.write().expect("memory store write lock"),
                api_key_id,
                expected_version,
            )),
            Self::Postgres(store) => store.revoke_api_key(api_key_id, expected_version).await,
        }
    }

    pub async fn revoke_opening_grant(
        &self,
        grant_id: &str,
        expected_version: u64,
        revoked_by: &str,
    ) -> Result<ConcurrencyResult<OpeningGrant>> {
        match self {
            Self::Memory(store) => Ok(revoke_memory_opening_grant(
                &mut store.write().expect("memory store write lock"),
                grant_id,
                expected_version,
                revoked_by,
            )),
            Self::Postgres(store) => {
                store
                    .revoke_opening_grant(grant_id, expected_version, revoked_by)
                    .await
            }
        }
    }

    pub async fn resolve_api_key(&self, api_key: &str) -> Result<Option<ResolvedApiKey>> {
        let hash = hash_api_key(api_key);
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                if let Some(grant) = store
                    .opening_grants
                    .iter()
                    .find(|item| item.credential_hash == hash)
                {
                    return Ok(Some(grant.to_resolved()));
                }
                Ok(store
                    .api_keys
                    .iter()
                    .find(|item| item.hash == hash && item.is_active)
                    .and_then(|item| item.to_resolved(&store.provider_resources)))
            }
            Self::Postgres(store) => store.resolve_api_key(&hash).await,
        }
    }

    pub async fn get_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                let snapshot_id = if config_snapshot_id == ACTIVE_CONFIG_ALIAS {
                    store.active_config_snapshot_id.as_str()
                } else {
                    config_snapshot_id
                };
                Ok(store
                    .config_snapshots
                    .iter()
                    .find(|item| item.config_snapshot_id.as_str() == snapshot_id)
                    .cloned()
                    .map(|config_snapshot| ConfigSnapshotResponse { config_snapshot }))
            }
            Self::Postgres(store) => store.get_config_snapshot(config_snapshot_id).await,
        }
    }

    pub async fn get_config_snapshot_sale_readiness(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<SaleReadyPackageResponse>> {
        let Some(snapshot) = self.get_config_snapshot(config_snapshot_id).await? else {
            return Ok(None);
        };
        let snapshot = snapshot.config_snapshot;
        let route_policy = self
            .get_route_policy(snapshot.route_policy_id.as_str())
            .await?;
        let mut provider_resources = Vec::with_capacity(snapshot.provider_resource_ids.len());
        let mut missing_provider_resource_ids = Vec::new();
        for provider_resource_id in &snapshot.provider_resource_ids {
            match self
                .get_provider_resource(provider_resource_id.as_str())
                .await?
            {
                Some(resource) => provider_resources.push(resource),
                None => missing_provider_resource_ids.push(provider_resource_id.to_string()),
            }
        }

        let active_snapshot_id = self
            .get_config_snapshot(ACTIVE_CONFIG_ALIAS)
            .await?
            .map(|response| response.config_snapshot.config_snapshot_id);
        let route_simulation =
            sale_ready_route_simulation(&snapshot, route_policy.as_ref(), &provider_resources);
        let pricing = match sale_ready_pricing_request(
            route_policy.as_ref(),
            route_simulation.as_ref(),
            &provider_resources,
        ) {
            Some(request) => Some(self.create_pricing_simulation(request).await?),
            None => None,
        };
        let budget = Some(
            self.get_balance_projection(
                snapshot.tenant_id.as_str(),
                Some(snapshot.project_id.to_string()),
                None,
            )
            .await?
            .data,
        );

        Ok(Some(build_sale_ready_package_response(
            snapshot,
            route_policy,
            provider_resources,
            missing_provider_resource_ids,
            active_snapshot_id,
            route_simulation,
            pricing,
            budget,
        )))
    }

    pub async fn activate_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        match self {
            Self::Memory(store) => Ok(activate_memory_config_snapshot(
                &mut store.write().expect("memory store write lock"),
                config_snapshot_id,
            )),
            Self::Postgres(store) => store.activate_config_snapshot(config_snapshot_id).await,
        }
    }

    pub async fn simulate_route(
        &self,
        request: RouteSimulationRequest,
    ) -> Result<RouteSimulationResponse> {
        match self {
            Self::Memory(store) => {
                simulate_memory_route(&store.read().expect("memory store read lock"), &request)
            }
            Self::Postgres(store) => store.simulate_route(request).await,
        }
    }

    pub async fn get_route_receipt(
        &self,
        route_receipt_id: &str,
    ) -> Result<Option<RouteReceiptResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .route_receipts
                .get(route_receipt_id)
                .cloned()
                .map(|route_receipt| RouteReceiptResponse { route_receipt })),
            Self::Postgres(store) => store.get_route_receipt(route_receipt_id).await,
        }
    }

    pub async fn get_route_receipt_diagnostics(
        &self,
        route_receipt_id: &str,
    ) -> Result<Option<RouteReceiptDiagnosticsResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .route_receipts
                .get(route_receipt_id)
                .cloned()
                .map(memory_route_receipt_diagnostics_response)),
            Self::Postgres(store) => store.get_route_receipt_diagnostics(route_receipt_id).await,
        }
    }

    pub async fn list_route_receipts(
        &self,
        filters: &RouteReceiptFilters,
    ) -> Result<RouteReceiptsResponse> {
        match self {
            Self::Memory(store) => {
                let mut receipts = store
                    .read()
                    .expect("memory store read lock")
                    .route_receipts
                    .values()
                    .filter(|receipt| route_receipt_matches_filters(receipt, filters))
                    .cloned()
                    .collect::<Vec<_>>();
                receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));
                if let Some(limit) = filters.limit {
                    receipts.truncate(limit);
                }
                Ok(RouteReceiptsResponse { data: receipts })
            }
            Self::Postgres(store) => store.list_route_receipts(filters).await,
        }
    }

    pub async fn get_route_diagnostics(
        &self,
        route_policy_id: &str,
    ) -> Result<Option<RouteDiagnosticsResponse>> {
        match self {
            Self::Memory(store) => Ok(build_memory_route_diagnostics(
                &store.read().expect("memory store read lock"),
                route_policy_id,
            )),
            Self::Postgres(store) => store.get_route_diagnostics(route_policy_id).await,
        }
    }

    pub async fn get_usage_summary(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        owner_account_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
    ) -> Result<UsageSummaryResponse> {
        match self {
            Self::Memory(_) => Ok(sample_usage_summary_response(
                tenant_id,
                project_id.as_deref(),
                owner_account_id.as_deref(),
                window_start.as_deref(),
                window_end.as_deref(),
            )),
            Self::Postgres(store) => {
                store
                    .get_usage_summary(
                        tenant_id,
                        project_id,
                        owner_account_id,
                        window_start,
                        window_end,
                    )
                    .await
            }
        }
    }

    pub async fn get_usage_breakdown(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        owner_account_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
        group_by: UsageBreakdownGroupBy,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<UsageBreakdownResponse> {
        match self {
            Self::Memory(_) => Ok(sample_usage_breakdown_response(
                tenant_id,
                project_id.as_deref(),
                owner_account_id.as_deref(),
                group_by,
                cursor,
                limit,
            )),
            Self::Postgres(store) => {
                store
                    .get_usage_breakdown(
                        tenant_id,
                        project_id,
                        owner_account_id,
                        window_start,
                        window_end,
                        group_by,
                        cursor,
                        limit,
                    )
                    .await
            }
        }
    }

    pub async fn get_balance_projection(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        owner_account_id: Option<String>,
    ) -> Result<BalanceProjectionResponse> {
        match self {
            Self::Memory(_) => Ok(sample_balance_projection_response(
                tenant_id,
                project_id.as_deref(),
                owner_account_id.as_deref(),
            )),
            Self::Postgres(store) => {
                store
                    .get_balance_projection(tenant_id, project_id, owner_account_id)
                    .await
            }
        }
    }

    pub async fn get_pricing_catalog(&self) -> Result<PricingCatalogResponse> {
        match self {
            Self::Memory(_) => Ok(default_pricing_catalog_response()),
            Self::Postgres(store) => store.get_pricing_catalog().await,
        }
    }

    pub async fn create_pricing_simulation(
        &self,
        request: PricingSimulationRequest,
    ) -> Result<PricingSimulationResponse> {
        match self {
            Self::Memory(_) => Ok(simulate_pricing(&request)),
            Self::Postgres(store) => store.create_pricing_simulation(&request).await,
        }
    }

    pub async fn create_billing_export(
        &self,
        request: BillingExportRequest,
    ) -> Result<BillingExportJobResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let export_job_id = format!("export_mem_{}", store.billing_export_jobs.len() + 1);
                let requested_at = now_rfc3339();
                let job = BillingExportJob {
                    export_job_id: export_job_id.clone(),
                    status: "queued".to_string(),
                    format: request.format.clone(),
                    requested_at: requested_at.clone(),
                    completed_at: None,
                    error_message: None,
                    tenant_id: request.tenant_id.clone(),
                    project_id: request.project_id.clone(),
                };
                let record = BillingExportJobRecord {
                    job: job.clone(),
                    content: Some("bucket,provider_id,model_alias,input_tokens,output_tokens,cached_input_tokens,provider_cost,billable_price\n2026-04-21,openai,reasoning-fast,18420,6245,1220,0.124500,0.152025\n".to_string()),
                    content_type: "text/csv".to_string(),
                };
                store.billing_export_jobs.push(record);
                Ok(BillingExportJobResponse { data: job })
            }
            Self::Postgres(store) => store.create_billing_export(request).await,
        }
    }

    pub async fn list_billing_exports(
        &self,
        tenant_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<BillingExportJobsResponse> {
        match self {
            Self::Memory(store) => Ok(BillingExportJobsResponse {
                data: filter_billing_export_jobs(
                    store
                        .read()
                        .expect("memory store read lock")
                        .billing_export_jobs
                        .iter()
                        .map(|record| {
                            maybe_complete_memory_export_job(
                                record.job.clone(),
                                record.content.is_some(),
                            )
                        })
                        .collect(),
                    tenant_id,
                    project_id,
                ),
            }),
            Self::Postgres(store) => store.list_billing_exports(tenant_id, project_id).await,
        }
    }

    pub async fn get_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<BillingExportJobResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .billing_export_jobs
                .iter()
                .find(|record| record.job.export_job_id == export_job_id)
                .map(|record| BillingExportJobResponse {
                    data: maybe_complete_memory_export_job(
                        record.job.clone(),
                        record.content.is_some(),
                    ),
                })),
            Self::Postgres(store) => store.get_billing_export(export_job_id).await,
        }
    }

    pub async fn download_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<(String, String)>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .billing_export_jobs
                .iter()
                .find(|record| record.job.export_job_id == export_job_id)
                .and_then(|record| {
                    record
                        .content
                        .as_ref()
                        .map(|content| (record.content_type.clone(), content.clone()))
                })),
            Self::Postgres(store) => store.download_billing_export(export_job_id).await,
        }
    }

    pub async fn create_wechat_payment_order(
        &self,
        record: WechatPaymentOrderRecord,
    ) -> Result<WechatPaymentOrderResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                store
                    .wechat_payment_orders
                    .insert(record.out_trade_no.clone(), record.clone());
                Ok(WechatPaymentOrderResponse { data: record })
            }
            Self::Postgres(store) => store.create_wechat_payment_order(record).await,
        }
    }

    pub async fn get_wechat_payment_order(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<WechatPaymentOrderResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .wechat_payment_orders
                .get(out_trade_no)
                .cloned()
                .map(|data| WechatPaymentOrderResponse { data })),
            Self::Postgres(store) => store.get_wechat_payment_order(out_trade_no).await,
        }
    }

    pub async fn update_wechat_payment_order(
        &self,
        record: WechatPaymentOrderRecord,
    ) -> Result<WechatPaymentOrderResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                store
                    .wechat_payment_orders
                    .insert(record.out_trade_no.clone(), record.clone());
                Ok(WechatPaymentOrderResponse { data: record })
            }
            Self::Postgres(store) => store.update_wechat_payment_order(record).await,
        }
    }

    pub async fn create_alipay_payment_order(
        &self,
        record: AlipayPaymentOrderRecord,
    ) -> Result<AlipayPaymentOrderResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                store
                    .alipay_payment_orders
                    .insert(record.out_trade_no.clone(), record.clone());
                Ok(AlipayPaymentOrderResponse { data: record })
            }
            Self::Postgres(store) => store.create_alipay_payment_order(record).await,
        }
    }

    pub async fn get_alipay_payment_order(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<AlipayPaymentOrderResponse>> {
        match self {
            Self::Memory(store) => Ok(store
                .read()
                .expect("memory store read lock")
                .alipay_payment_orders
                .get(out_trade_no)
                .cloned()
                .map(|data| AlipayPaymentOrderResponse { data })),
            Self::Postgres(store) => store.get_alipay_payment_order(out_trade_no).await,
        }
    }

    pub async fn update_alipay_payment_order(
        &self,
        record: AlipayPaymentOrderRecord,
    ) -> Result<AlipayPaymentOrderResponse> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                store
                    .alipay_payment_orders
                    .insert(record.out_trade_no.clone(), record.clone());
                Ok(AlipayPaymentOrderResponse { data: record })
            }
            Self::Postgres(store) => store.update_alipay_payment_order(record).await,
        }
    }

    pub async fn create_renewal_intent(
        &self,
        draft: RenewalIntentDraft,
    ) -> Result<RenewalIntentCreateResult> {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                Ok(create_memory_renewal_intent(&mut store, draft))
            }
            Self::Postgres(store) => store.create_renewal_intent(draft).await,
        }
    }

    pub async fn list_renewal_intents(
        &self,
        filters: RenewalIntentFilters,
    ) -> Result<RenewalIntentsResponse> {
        match self {
            Self::Memory(store) => {
                let store = store.read().expect("memory store read lock");
                Ok(RenewalIntentsResponse {
                    data: filter_renewal_intents(store.renewal_intents.clone(), &filters),
                })
            }
            Self::Postgres(store) => store.list_renewal_intents(filters).await,
        }
    }

    #[cfg(test)]
    pub fn insert_route_receipt_for_tests(&self, route_receipt: RouteReceipt) {
        match self {
            Self::Memory(store) => {
                store
                    .write()
                    .expect("memory store write lock")
                    .route_receipts
                    .insert(
                        route_receipt.route_receipt_id.as_str().to_string(),
                        route_receipt,
                    );
            }
            Self::Postgres(_) => panic!("test helper only supports memory store"),
        }
    }

    #[cfg(test)]
    pub fn set_provider_resource_provider_id_for_tests(
        &self,
        provider_resource_id: &str,
        provider_id: &str,
    ) {
        match self {
            Self::Memory(store) => {
                let mut store = store.write().expect("memory store write lock");
                let resource = store
                    .provider_resources
                    .iter_mut()
                    .find(|resource| resource.provider_resource_id.as_str() == provider_resource_id)
                    .expect("provider resource should exist in memory store");
                resource.provider_id = provider_id.to_string();
                resource.supported_protocol_families = match provider_id {
                    "anthropic" => vec!["anthropic_messages".to_string()],
                    "bedrock" => vec!["openai_chat".to_string()],
                    "gemini" => vec!["gemini_generate_content".to_string()],
                    _ => vec![
                        "openai_chat".to_string(),
                        "openai_responses".to_string(),
                        "openai_images".to_string(),
                    ],
                };
            }
            Self::Postgres(_) => panic!("test helper only supports memory store"),
        }
    }
}

pub async fn migrate_from_env() -> Result<()> {
    let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL")
        .map_err(|_| anyhow!("CONTROL_PLANE_DATABASE_URL is required to run migrations"))?;
    let store = PostgresStore::connect(&database_url).await?;
    store.migrate().await
}

pub async fn bootstrap_from_env() -> Result<()> {
    let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL").map_err(|_| {
        anyhow!("CONTROL_PLANE_DATABASE_URL is required to bootstrap control-plane data")
    })?;
    let store = PostgresStore::connect(&database_url).await?;
    store.migrate().await?;
    store.seed().await
}

pub async fn schema_status_from_env() -> Result<String> {
    let database_url = std::env::var("CONTROL_PLANE_DATABASE_URL").map_err(|_| {
        anyhow!("CONTROL_PLANE_DATABASE_URL is required to inspect control-plane schema")
    })?;
    let store = PostgresStore::connect(&database_url).await?;
    store.ensure_schema_ready().await?;
    Ok("control-plane schema is ready".to_string())
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .with_context(|| {
                format!("failed to connect to control-plane database at {database_url}")
            })?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        for statement in MIGRATIONS {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn ensure_schema_ready(&self) -> Result<()> {
        for table_name in REQUIRED_TABLES {
            let exists = sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)::text")
                .bind(table_name)
                .fetch_one(&self.pool)
                .await?;
            if exists.is_none() {
                return Err(anyhow!(
                    "required control-plane table `{table_name}` is missing; run `control-plane-api migrate` first"
                ));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub async fn seed(&self) -> Result<()> {
        let seed = SeedData::bootstrap();
        for tenant in seed.tenants {
            let tenant_id = tenant.tenant_id.to_string();
            sqlx::query(
                "INSERT INTO tenants (tenant_id, payload) VALUES ($1, $2)
                 ON CONFLICT (tenant_id) DO UPDATE SET payload = EXCLUDED.payload",
            )
            .bind(&tenant_id)
            .bind(Json(tenant))
            .execute(&self.pool)
            .await?;
        }

        for project in seed.projects {
            let project_id = project.project_id.to_string();
            let tenant_id = project.tenant_id.to_string();
            sqlx::query(
                "INSERT INTO projects (project_id, tenant_id, payload) VALUES ($1, $2, $3)
                 ON CONFLICT (project_id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
            )
            .bind(&project_id)
            .bind(&tenant_id)
            .bind(Json(project))
            .execute(&self.pool)
            .await?;
        }

        for provider_resource in seed.provider_resources {
            let provider_resource_id = provider_resource.provider_resource_id.to_string();
            let tenant_id = provider_resource.tenant_id.to_string();
            let project_id = provider_resource
                .project_id
                .as_ref()
                .map(ToString::to_string);
            let provider_id = provider_resource.provider_id.clone();
            sqlx::query(
                "INSERT INTO provider_resources (provider_resource_id, tenant_id, project_id, provider_id, payload)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (provider_resource_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   project_id = EXCLUDED.project_id,
                   provider_id = EXCLUDED.provider_id,
                   payload = EXCLUDED.payload",
            )
            .bind(&provider_resource_id)
            .bind(&tenant_id)
            .bind(project_id.as_deref())
            .bind(&provider_id)
            .bind(Json(provider_resource))
            .execute(&self.pool)
            .await?;
        }

        for route_policy in seed.route_policies {
            let route_policy_id = route_policy.route_policy_id.to_string();
            let tenant_id = route_policy.tenant_id.to_string();
            sqlx::query(
                "INSERT INTO route_policies (route_policy_id, tenant_id, payload) VALUES ($1, $2, $3)
                 ON CONFLICT (route_policy_id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
            )
            .bind(&route_policy_id)
            .bind(&tenant_id)
            .bind(Json(route_policy))
            .execute(&self.pool)
            .await?;
        }

        for config_snapshot in seed.config_snapshots {
            let config_snapshot_id = config_snapshot.config_snapshot_id.to_string();
            let tenant_id = config_snapshot.tenant_id.to_string();
            let project_id = config_snapshot.project_id.to_string();
            let status = config_snapshot_status_slug(config_snapshot.status);
            sqlx::query(
                "INSERT INTO config_snapshots (config_snapshot_id, tenant_id, project_id, status, payload)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (config_snapshot_id) DO UPDATE SET
                   tenant_id = EXCLUDED.tenant_id,
                   project_id = EXCLUDED.project_id,
                   status = EXCLUDED.status,
                   payload = EXCLUDED.payload",
            )
            .bind(&config_snapshot_id)
            .bind(&tenant_id)
            .bind(&project_id)
            .bind(status)
            .bind(Json(config_snapshot))
            .execute(&self.pool)
            .await?;
        }

        sqlx::query(
            "INSERT INTO active_config_pointers (pointer_key, config_snapshot_id)
             VALUES ('default', $1)
             ON CONFLICT (pointer_key) DO UPDATE SET config_snapshot_id = EXCLUDED.config_snapshot_id",
        )
        .bind(seed.active_config_snapshot_id)
        .execute(&self.pool)
        .await?;

        self.seed_merchant(
            seed.merchant_shops,
            seed.card_products,
            seed.trial_connections,
            seed.relay_evaluations,
            seed.replay_capsules,
            seed.route_receipts,
        )
        .await?;

        seed_default_pricing_catalog(&self.pool).await?;

        for user_seed in seed.users {
            let user_id = user_seed.user.user_id.as_str().to_string();
            let primary_email = user_seed.user.primary_email.clone();
            sqlx::query(
                "INSERT INTO users (user_id, primary_email, payload) VALUES ($1, $2, $3)
                 ON CONFLICT (user_id) DO UPDATE SET primary_email = EXCLUDED.primary_email, payload = EXCLUDED.payload",
            )
            .bind(&user_id)
            .bind(primary_email.as_deref())
            .bind(Json(user_seed.user))
            .execute(&self.pool)
            .await?;

            for membership in user_seed.memberships {
                let membership_id = membership.membership_id.to_string();
                let tenant_id = membership.tenant.id.to_string();
                sqlx::query(
                    "INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (membership_id) DO UPDATE SET user_id = EXCLUDED.user_id, tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
                )
                .bind(&membership_id)
                .bind(&user_id)
                .bind(&tenant_id)
                .bind(Json(membership))
                .execute(&self.pool)
                .await?;
            }

            for link in user_seed.links {
                let link_id = link.link_id.to_string();
                let provider_slug = auth_provider_slug(link.provider);
                let provider_subject = link.provider_subject.clone();
                let email = link.email.clone();
                let can_unlink = link.can_unlink;
                sqlx::query(
                    "INSERT INTO auth_provider_links (link_id, user_id, provider, provider_subject, email, can_unlink, payload)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)
                     ON CONFLICT (link_id) DO UPDATE SET
                       user_id = EXCLUDED.user_id,
                       provider = EXCLUDED.provider,
                       provider_subject = EXCLUDED.provider_subject,
                       email = EXCLUDED.email,
                       can_unlink = EXCLUDED.can_unlink,
                       payload = EXCLUDED.payload",
                )
                .bind(&link_id)
                .bind(&user_id)
                .bind(provider_slug)
                .bind(&provider_subject)
                .bind(email.as_deref())
                .bind(can_unlink)
                .bind(Json(link))
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn ensure_known_email(&self, email: &str) -> Result<bool> {
        let row =
            sqlx::query("SELECT user_id FROM users WHERE lower(primary_email) = lower($1) LIMIT 1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }

    async fn create_login_flow(&self, flow: &LoginFlow) -> Result<LoginFlow> {
        sqlx::query(
            "INSERT INTO login_flows (flow_id, flow_kind, email, provider, workspace_slug, expires_at, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (flow_id) DO UPDATE SET
               flow_kind = EXCLUDED.flow_kind,
               email = EXCLUDED.email,
               provider = EXCLUDED.provider,
               workspace_slug = EXCLUDED.workspace_slug,
               expires_at = EXCLUDED.expires_at,
               payload = EXCLUDED.payload",
        )
        .bind(&flow.flow_id)
        .bind(login_flow_kind_slug(flow.flow_kind))
        .bind(flow.email.as_deref())
        .bind(flow.provider.map(oauth_provider_slug))
        .bind(&flow.workspace_slug)
        .bind(&flow.expires_at)
        .bind(Json(flow))
        .execute(&self.pool)
        .await?;
        Ok(flow.clone())
    }

    async fn consume_login_flow(&self, flow_id: &str) -> Result<Option<LoginFlow>> {
        let row = sqlx::query("DELETE FROM login_flows WHERE flow_id = $1 RETURNING payload")
            .bind(flow_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<LoginFlow>, _>("payload").0))
    }

    async fn issue_session(
        &self,
        session_id: &str,
        provider: AuthProvider,
        identity_key: &IdentityLookup,
        workspace_slug: &str,
        now: &str,
        expires_at: &str,
    ) -> Result<AuthLoginResult> {
        let user = match self.lookup_user(identity_key).await? {
            Some(user) => user,
            None if provider == AuthProvider::Oidc => match identity_key {
                IdentityLookup::ProviderSubject(AuthProvider::Oidc, subject) => {
                    self.ensure_oidc_user(subject, workspace_slug, now).await?
                }
                _ => anyhow::bail!("identity not found"),
            },
            None => anyhow::bail!("identity not found"),
        };
        let memberships = self.list_memberships(&user.user_id).await?;
        let active_membership = memberships
            .iter()
            .find(|membership| tenant_summary_matches_workspace(&membership.tenant, workspace_slug))
            .or_else(|| memberships.first())
            .cloned()
            .context("no tenant memberships available for login")?;
        let links = self.list_links(&user.user_id).await?;

        let session = AuthSession {
            session_id: AuthSessionId::parse(session_id.to_string()).unwrap(),
            state: AuthSessionState::Active,
            user: user.clone(),
            active_tenant_id: Some(active_membership.tenant.id.clone()),
            memberships,
            authenticated_by: provider,
            created_at: now.to_string(),
            expires_at: expires_at.to_string(),
            last_authenticated_at: now.to_string(),
        };

        sqlx::query(
            "INSERT INTO sessions (session_id, user_id, state, active_tenant_id, authenticated_by, expires_at, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (session_id) DO UPDATE SET
               user_id = EXCLUDED.user_id,
               state = EXCLUDED.state,
               active_tenant_id = EXCLUDED.active_tenant_id,
               authenticated_by = EXCLUDED.authenticated_by,
               expires_at = EXCLUDED.expires_at,
               payload = EXCLUDED.payload",
        )
        .bind(session.session_id.as_str())
        .bind(user.user_id.as_str())
        .bind("active")
        .bind(session.active_tenant_id.as_ref().map(core_domain::TenantId::as_str))
        .bind(auth_provider_slug(provider))
        .bind(&session.expires_at)
        .bind(Json(session.clone()))
        .execute(&self.pool)
        .await?;

        Ok(AuthLoginResult {
            session,
            links,
            redirect_to: None,
        })
    }

    async fn ensure_oidc_user(
        &self,
        subject: &str,
        workspace_slug: &str,
        now: &str,
    ) -> Result<UserIdentity> {
        self.upsert_oidc_user(
            subject,
            std::env::var("CONTROL_PLANE_OIDC_EMAIL").ok().as_deref(),
            std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                .ok()
                .as_deref(),
            workspace_slug,
            if workspace_slug == "platform-admin" {
                TenantMembershipRole::Admin
            } else {
                TenantMembershipRole::Member
            },
            now,
        )
        .await
    }

    async fn upsert_oidc_user(
        &self,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        let row = sqlx::query(
            "SELECT payload FROM tenants
             WHERE payload->>'slug' = $1 OR tenant_id = $1 OR ($1 = 'acme' AND tenant_id = 'tenant_acme')
             LIMIT 1",
        )
            .bind(workspace_slug)
            .fetch_optional(&self.pool)
            .await?
            .context("workspace not found for oidc login")?;
        let tenant = row.get::<Json<Tenant>, _>("payload").0;

        let digest = Sha256::digest(subject.as_bytes());
        let subject_hash = digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let user_id = format!("user_{subject_hash}");
        let primary_email = email
            .map(str::to_string)
            .or_else(|| {
                std::env::var("CONTROL_PLANE_OIDC_EMAIL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| format!("{subject}@enterprise.example"));
        let display_name = display_name
            .map(str::to_string)
            .or_else(|| {
                std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| "Enterprise SSO User".to_string());

        let user = UserIdentity {
            user_id: UserId::parse(user_id.clone()).unwrap(),
            primary_email: Some(primary_email.clone()),
            display_name,
            avatar_url: None,
            created_at: now.to_string(),
            last_login_at: Some(now.to_string()),
        };

        sqlx::query(
            "INSERT INTO users (user_id, primary_email, payload) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET primary_email = EXCLUDED.primary_email, payload = EXCLUDED.payload",
        )
        .bind(&user_id)
        .bind(&primary_email)
        .bind(Json(user.clone()))
        .execute(&self.pool)
        .await?;

        let membership = membership(&format!("tmemb_oidc_{subject_hash}"), &tenant, role);
        let membership_id = membership.membership_id.to_string();
        let membership_tenant_id = membership.tenant.id.to_string();
        sqlx::query(
            "INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (membership_id) DO UPDATE SET user_id = EXCLUDED.user_id, tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
        )
        .bind(&membership_id)
        .bind(&user_id)
        .bind(&membership_tenant_id)
        .bind(Json(membership))
        .execute(&self.pool)
        .await?;

        let link = link(AuthProvider::Oidc, subject, Some(&primary_email), false);
        let link_id = link.link_id.to_string();
        sqlx::query(
            "INSERT INTO auth_provider_links (link_id, user_id, provider, provider_subject, email, can_unlink, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (link_id) DO UPDATE SET
               user_id = EXCLUDED.user_id,
               provider = EXCLUDED.provider,
               provider_subject = EXCLUDED.provider_subject,
               email = EXCLUDED.email,
               can_unlink = EXCLUDED.can_unlink,
               payload = EXCLUDED.payload",
        )
        .bind(&link_id)
        .bind(&user_id)
        .bind(auth_provider_slug(AuthProvider::Oidc))
        .bind(subject)
        .bind(Some(primary_email.as_str()))
        .bind(false)
        .bind(Json(link))
        .execute(&self.pool)
        .await?;

        Ok(user)
    }

    async fn upsert_oauth_user(
        &self,
        provider: AuthProvider,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        self.upsert_federated_user(
            provider,
            subject,
            email,
            display_name,
            workspace_slug,
            role,
            now,
        )
        .await
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AuthLoginResult>> {
        let row = sqlx::query(
            "SELECT user_id, payload FROM sessions WHERE session_id = $1 AND state = 'active'",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let session = row.get::<Json<AuthSession>, _>("payload").0;
        if timestamp_is_expired(&session.expires_at) {
            let _ = self.revoke_session(session_id).await?;
            return Ok(None);
        }
        let user_id = row.get::<String, _>("user_id");
        let user = self
            .load_user(&UserId::parse(user_id).unwrap())
            .await?
            .unwrap_or(session.user.clone());
        let memberships = self.list_memberships(&user.user_id).await?;
        let links = self.list_links(&user.user_id).await?;
        let active_tenant_id =
            resolve_active_tenant_id(session.active_tenant_id.as_ref(), &memberships);

        Ok(Some(AuthLoginResult {
            session: AuthSession {
                user,
                memberships,
                active_tenant_id,
                ..session
            },
            links,
            redirect_to: None,
        }))
    }

    async fn revoke_session(&self, session_id: &str) -> Result<LogoutResponse> {
        let revoked = sqlx::query("DELETE FROM sessions WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0;
        let _ = revoked;
        Ok(LogoutResponse {
            outcome: "signed_out".to_string(),
        })
    }

    async fn unlink_provider(
        &self,
        session_id: &str,
        provider: AuthProvider,
    ) -> Result<Option<UnlinkAuthProviderResponse>> {
        let Some(session) = self.get_session(session_id).await? else {
            return Ok(None);
        };
        let removed = if provider == AuthProvider::Email {
            false
        } else {
            sqlx::query("DELETE FROM auth_provider_links WHERE user_id = $1 AND provider = $2")
                .bind(session.session.user.user_id.as_str())
                .bind(auth_provider_slug(provider))
                .execute(&self.pool)
                .await?
                .rows_affected()
                > 0
        };
        if removed {
            self.revoke_sessions_for_user(&session.session.user.user_id)
                .await?;
        }
        Ok(Some(UnlinkAuthProviderResponse { provider, removed }))
    }

    async fn workspace_exists(&self, workspace_slug: &str) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tenants
             WHERE payload->>'slug' = $1 OR tenant_id = $1 OR ($1 = 'acme' AND tenant_id = 'tenant_acme')",
        )
            .bind(workspace_slug)
            .fetch_one(&self.pool)
            .await?
            > 0)
    }

    async fn list_tenants(&self) -> Result<TenantsResponse> {
        let rows = sqlx::query("SELECT payload FROM tenants ORDER BY tenant_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(TenantsResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<Tenant>, _>("payload").0)
                .collect(),
        })
    }

    async fn list_projects(&self) -> Result<ProjectsResponse> {
        let rows = sqlx::query("SELECT payload FROM projects ORDER BY project_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(ProjectsResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<Project>, _>("payload").0)
                .collect(),
        })
    }

    async fn get_project(&self, project_id: &str) -> Result<Option<Project>> {
        let row = sqlx::query("SELECT payload FROM projects WHERE project_id = $1")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<Project>, _>("payload").0))
    }

    async fn list_provider_resources(
        &self,
        filters: &ProviderResourceFilters,
    ) -> Result<ProviderResourcesResponse> {
        let rows =
            sqlx::query("SELECT payload FROM provider_resources ORDER BY provider_resource_id")
                .fetch_all(&self.pool)
                .await?;
        Ok(ProviderResourcesResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<ProviderResource>, _>("payload").0)
                .filter(|resource| provider_matches_filters(resource, filters))
                .collect(),
        })
    }

    async fn get_provider_resource(
        &self,
        provider_resource_id: &str,
    ) -> Result<Option<ProviderResource>> {
        let row =
            sqlx::query("SELECT payload FROM provider_resources WHERE provider_resource_id = $1")
                .bind(provider_resource_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|row| row.get::<Json<ProviderResource>, _>("payload").0))
    }

    async fn create_provider_resource(
        &self,
        provider_resource: &ProviderResource,
    ) -> Result<ProviderResource> {
        sqlx::query(
            "INSERT INTO provider_resources (provider_resource_id, tenant_id, project_id, provider_id, payload)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(provider_resource.provider_resource_id.as_str())
        .bind(provider_resource.tenant_id.as_str())
        .bind(provider_resource.project_id.as_ref().map(ProjectId::as_str))
        .bind(&provider_resource.provider_id)
        .bind(Json(provider_resource))
        .execute(&self.pool)
        .await?;
        Ok(provider_resource.clone())
    }

    async fn update_provider_resource(
        &self,
        provider_resource: &ProviderResource,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        let current = self
            .get_provider_resource(provider_resource.provider_resource_id.as_str())
            .await?;
        let Some(current) = current else {
            return Ok(ConcurrencyResult::NotFound);
        };
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut updated = provider_resource.clone();
        updated.version = expected_version.saturating_add(1);
        updated.created_at = current.created_at;
        sqlx::query(
            "UPDATE provider_resources
              SET tenant_id = $2, project_id = $3, provider_id = $4, payload = $5
              WHERE provider_resource_id = $1",
        )
        .bind(provider_resource.provider_resource_id.as_str())
        .bind(provider_resource.tenant_id.as_str())
        .bind(provider_resource.project_id.as_ref().map(ProjectId::as_str))
        .bind(&provider_resource.provider_id)
        .bind(Json(updated.clone()))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(updated))
    }

    async fn disable_provider_resource(
        &self,
        provider_resource_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ProviderResource>> {
        let current = self
            .get_provider_resource(provider_resource_id)
            .await?
            .context("provider resource not found")?;
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut disabled = current.clone();
        disabled.version = expected_version.saturating_add(1);
        disabled.status = ProviderResourceStatus::Disabled;
        disabled.updated_at = now_rfc3339();
        sqlx::query(
            "UPDATE provider_resources
             SET payload = $2
             WHERE provider_resource_id = $1",
        )
        .bind(provider_resource_id)
        .bind(Json(disabled.clone()))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(disabled))
    }

    async fn list_route_policies(&self) -> Result<RoutePoliciesResponse> {
        let rows = sqlx::query(
            "SELECT rp.payload
               FROM route_policies rp
               LEFT JOIN disabled_route_policies d ON rp.route_policy_id = d.route_policy_id
              WHERE d.route_policy_id IS NULL
              ORDER BY rp.route_policy_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(RoutePoliciesResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<RoutePolicy>, _>("payload").0)
                .collect(),
        })
    }

    async fn get_route_policy(&self, route_policy_id: &str) -> Result<Option<RoutePolicy>> {
        let row = sqlx::query("SELECT payload FROM route_policies WHERE route_policy_id = $1")
            .bind(route_policy_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<RoutePolicy>, _>("payload").0))
    }

    async fn create_route_policy(&self, route_policy: &RoutePolicy) -> Result<RoutePolicy> {
        sqlx::query(
            "INSERT INTO route_policies (route_policy_id, tenant_id, payload)
             VALUES ($1, $2, $3)",
        )
        .bind(route_policy.route_policy_id.as_str())
        .bind(route_policy.tenant_id.as_str())
        .bind(Json(route_policy))
        .execute(&self.pool)
        .await?;
        Ok(route_policy.clone())
    }

    async fn update_route_policy(
        &self,
        route_policy: &RoutePolicy,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        let current = self
            .get_route_policy(route_policy.route_policy_id.as_str())
            .await?;
        let Some(current) = current else {
            return Ok(ConcurrencyResult::NotFound);
        };
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut updated = route_policy.clone();
        updated.version = expected_version.saturating_add(1);
        updated.created_at = current.created_at;
        sqlx::query(
            "UPDATE route_policies
              SET tenant_id = $2, payload = $3
              WHERE route_policy_id = $1",
        )
        .bind(route_policy.route_policy_id.as_str())
        .bind(route_policy.tenant_id.as_str())
        .bind(Json(updated.clone()))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(updated))
    }

    async fn disable_route_policy(
        &self,
        route_policy_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<RoutePolicy>> {
        let current = self
            .get_route_policy(route_policy_id)
            .await?
            .context("route policy not found")?;
        if current.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let mut disabled = current.clone();
        disabled.version = expected_version.saturating_add(1);
        sqlx::query(
            "INSERT INTO disabled_route_policies (route_policy_id, disabled_at)
             VALUES ($1, $2)
             ON CONFLICT (route_policy_id)
             DO UPDATE SET disabled_at = EXCLUDED.disabled_at",
        )
        .bind(route_policy_id)
        .bind(now_rfc3339())
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE route_policies SET payload = $2 WHERE route_policy_id = $1")
            .bind(route_policy_id)
            .bind(Json(disabled.clone()))
            .execute(&self.pool)
            .await?;
        Ok(ConcurrencyResult::Applied(disabled))
    }

    async fn list_config_snapshots(&self) -> Result<ConfigSnapshotsResponse> {
        let rows = sqlx::query("SELECT payload FROM config_snapshots ORDER BY config_snapshot_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(ConfigSnapshotsResponse {
            data: rows
                .into_iter()
                .map(|row| row.get::<Json<ConfigSnapshot>, _>("payload").0)
                .collect(),
        })
    }

    async fn active_snapshot_id(&self) -> Result<Option<String>> {
        Ok(sqlx::query(
            "SELECT config_snapshot_id FROM active_config_pointers WHERE pointer_key = 'default'",
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|row| row.get::<String, _>("config_snapshot_id")))
    }

    async fn create_config_snapshot(
        &self,
        config_snapshot: &ConfigSnapshot,
    ) -> Result<ConfigSnapshot> {
        sqlx::query(
            "INSERT INTO config_snapshots (config_snapshot_id, tenant_id, project_id, status, payload)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(config_snapshot.config_snapshot_id.as_str())
        .bind(config_snapshot.tenant_id.as_str())
        .bind(config_snapshot.project_id.as_str())
        .bind(config_snapshot_status_slug(config_snapshot.status))
        .bind(Json(config_snapshot))
        .execute(&self.pool)
        .await?;
        Ok(config_snapshot.clone())
    }

    async fn create_api_key(
        &self,
        provider_resource_id: &str,
        display_name: &str,
        key_prefix: &str,
        hash: &str,
    ) -> Result<ApiKey> {
        let row = sqlx::query(
            "SELECT tenant_id, project_id FROM provider_resources WHERE provider_resource_id = $1",
        )
        .bind(provider_resource_id)
        .fetch_one(&self.pool)
        .await?;
        let tenant_id = TenantId::parse(row.get::<String, _>("tenant_id"))
            .context("invalid tenant id in provider resource record")?;
        let project_id = row.get::<Option<String>, _>("project_id");

        let now = now_rfc3339();
        let api_key_id = format!("ak_{}", &hash[..16]);
        sqlx::query(
            "INSERT INTO api_keys
                (api_key_id, provider_resource_id, tenant_id, project_id, display_name, key_prefix, hash, is_active, version, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, true, 1, $8, $8)",
        )
        .bind(&api_key_id)
        .bind(provider_resource_id)
        .bind(tenant_id.as_str())
        .bind(project_id.as_ref().map(String::as_str))
        .bind(display_name)
        .bind(key_prefix)
        .bind(hash)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(ApiKey {
            api_key_id,
            provider_resource_id: ProviderResourceId::parse(provider_resource_id.to_string())?,
            display_name: display_name.to_string(),
            key_prefix: key_prefix.to_string(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
            version: 1,
        })
    }

    async fn list_api_keys(&self) -> Result<ApiKeysResponse> {
        let rows = sqlx::query(
            "SELECT api_key_id, provider_resource_id, display_name, key_prefix, is_active, created_at, updated_at, version
               FROM api_keys
              ORDER BY api_key_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(ApiKeysResponse {
            data: rows
                .into_iter()
                .map(|row| ApiKey {
                    api_key_id: row.get::<String, _>("api_key_id"),
                    provider_resource_id: ProviderResourceId::parse(
                        row.get::<String, _>("provider_resource_id"),
                    )
                    .expect("stored provider_resource_id should be valid"),
                    display_name: row.get::<String, _>("display_name"),
                    key_prefix: row.get::<String, _>("key_prefix"),
                    is_active: row.get::<bool, _>("is_active"),
                    created_at: row.get::<String, _>("created_at"),
                    updated_at: row.get::<String, _>("updated_at"),
                    version: row.get::<i64, _>("version") as u64,
                })
                .collect(),
        })
    }

    async fn create_opening_grant(
        &self,
        record: &OpeningGrantRecord,
    ) -> Result<OpeningGrantCreateResult> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(OPENING_GRANT_OWNER_LOCK_INSERT_SQL)
            .bind(record.tenant_id.as_str())
            .bind(record.project_id.as_str())
            .bind(&record.owner_account_id)
            .bind(&record.created_at)
            .execute(&mut *tx)
            .await?;
        sqlx::query(OPENING_GRANT_OWNER_LOCK_SELECT_SQL)
            .bind(record.tenant_id.as_str())
            .bind(record.project_id.as_str())
            .bind(&record.owner_account_id)
            .execute(&mut *tx)
            .await?;

        let existing_rows = sqlx::query(
            "SELECT payload FROM opening_grants
              WHERE tenant_id = $1 AND project_id = $2 AND owner_account_id = $3
              FOR UPDATE",
        )
        .bind(record.tenant_id.as_str())
        .bind(record.project_id.as_str())
        .bind(&record.owner_account_id)
        .fetch_all(&mut *tx)
        .await?;
        let existing = existing_rows
            .into_iter()
            .map(|row| row.get::<Json<OpeningGrantRecord>, _>("payload").0)
            .collect::<Vec<_>>();
        if let Some(blocker) = opening_grant_create_blocker(&existing, record) {
            tx.rollback().await?;
            return Ok(blocker);
        }

        sqlx::query(
            "INSERT INTO opening_grants
                (grant_id, tenant_id, project_id, owner_account_id, config_snapshot_id, grantee_kind, grantee_id, status, credential_hash, expires_at, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(&record.grant_id)
        .bind(record.tenant_id.as_str())
        .bind(record.project_id.as_str())
        .bind(&record.owner_account_id)
        .bind(record.config_snapshot_id.as_str())
        .bind(&record.grantee_kind)
        .bind(&record.grantee_id)
        .bind(&record.status)
        .bind(&record.credential_hash)
        .bind(&record.expires_at)
        .bind(Json(record))
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(OpeningGrantCreateResult::Created(Box::new(
            record.public_view(),
        )))
    }

    async fn list_opening_grants(&self) -> Result<OpeningGrantsResponse> {
        let rows = sqlx::query("SELECT payload FROM opening_grants ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(OpeningGrantsResponse {
            data: rows
                .into_iter()
                .map(|row| {
                    row.get::<Json<OpeningGrantRecord>, _>("payload")
                        .0
                        .public_view()
                })
                .collect(),
        })
    }

    async fn prepare_delivery(
        &self,
        prepared: &PreparedDeliveryRecords,
        redemption_code: &str,
        browser_file_unlock_code: &str,
    ) -> Result<DeliveryPrepareResult> {
        let mut tx = self.pool.begin().await?;
        if let Some(blocker) =
            owner_redemption_capacity_blocker_for_postgres(&mut tx, prepared).await?
        {
            tx.rollback().await?;
            return Ok(blocker);
        }
        sqlx::query(
            "INSERT INTO deliveries
                (delivery_id, tenant_id, project_id, owner_account_id, redemption_batch_id, provider, status, operator_id, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&prepared.delivery.delivery_id)
        .bind(prepared.delivery.tenant_id.as_str())
        .bind(prepared.delivery.project_id.as_str())
        .bind(&prepared.delivery.owner_account_id)
        .bind(&prepared.delivery.redemption_batch_id)
        .bind(&prepared.delivery.provider)
        .bind(&prepared.delivery.status)
        .bind(&prepared.delivery.operator_id)
        .bind(Json(&prepared.delivery))
        .bind(&prepared.delivery.created_at)
        .bind(&prepared.delivery.updated_at)
        .execute(&mut *tx)
        .await?;

        for code in &prepared.codes {
            sqlx::query(
                "INSERT INTO delivery_codes
                    (code_id, delivery_id, code_type, code_hash, status, expires_at, payload, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(&code.code_id)
            .bind(&code.delivery_id)
            .bind(&code.code_type)
            .bind(&code.code_hash)
            .bind(&code.status)
            .bind(&code.expires_at)
            .bind(Json(code))
            .bind(&code.created_at)
            .bind(&code.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO delivery_secret_plaintexts
                (delivery_id, code_type, secret_plaintext, created_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (delivery_id, code_type) DO UPDATE
                SET secret_plaintext = EXCLUDED.secret_plaintext,
                    created_at = EXCLUDED.created_at,
                    used_at = NULL",
        )
        .bind(&prepared.delivery.delivery_id)
        .bind(DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK)
        .bind(browser_file_unlock_code)
        .bind(&prepared.delivery.created_at)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO delivery_entitlements
                (entitlement_id, delivery_id, status, ends_at, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&prepared.entitlement.entitlement_id)
        .bind(&prepared.entitlement.delivery_id)
        .bind(&prepared.entitlement.status)
        .bind(&prepared.entitlement.ends_at)
        .bind(Json(&prepared.entitlement))
        .bind(&prepared.entitlement.created_at)
        .bind(&prepared.entitlement.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(DeliveryPrepareResult::Prepared(Box::new(
            DeliveryPrepareResponse {
                data: delivery_projection(
                    &prepared.delivery,
                    &prepared.codes,
                    &prepared.entitlement,
                ),
                one_time_codes: DeliveryOneTimeCodes {
                    redemption_code: redemption_code.to_string(),
                    browser_file_unlock_code: browser_file_unlock_code.to_string(),
                },
            },
        )))
    }

    async fn get_delivery(&self, delivery_id: &str) -> Result<Option<DeliveryResponse>> {
        let Some(delivery_row) =
            sqlx::query("SELECT payload FROM deliveries WHERE delivery_id = $1")
                .bind(delivery_id)
                .fetch_optional(&self.pool)
                .await?
        else {
            return Ok(None);
        };
        let delivery = delivery_row.get::<Json<DeliveryRecord>, _>("payload").0;
        let code_rows = sqlx::query(
            "SELECT payload
               FROM delivery_codes
              WHERE delivery_id = $1
              ORDER BY code_type",
        )
        .bind(delivery_id)
        .fetch_all(&self.pool)
        .await?;
        let codes = code_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryCodeRecord>, _>("payload").0)
            .collect::<Vec<_>>();
        let entitlement_row =
            sqlx::query("SELECT payload FROM delivery_entitlements WHERE delivery_id = $1")
                .bind(delivery_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(entitlement_row) = entitlement_row else {
            return Ok(None);
        };
        let entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        Ok(Some(DeliveryResponse {
            data: delivery_projection(&delivery, &codes, &entitlement),
        }))
    }

    async fn get_delivery_secret_plaintext(
        &self,
        delivery_id: &str,
        code_type: &str,
    ) -> Result<Option<String>> {
        let row = sqlx::query(
            "SELECT secret_plaintext
               FROM delivery_secret_plaintexts
              WHERE delivery_id = $1 AND code_type = $2",
        )
        .bind(delivery_id)
        .bind(code_type)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get("secret_plaintext")))
    }

    async fn revoke_delivery(
        &self,
        delivery_id: &str,
        expected_version: u64,
        revoked_by: &str,
        revoke_reason: Option<String>,
    ) -> Result<ConcurrencyResult<DeliveryResponse>> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT payload FROM deliveries WHERE delivery_id = $1 FOR UPDATE")
            .bind(delivery_id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(ConcurrencyResult::NotFound);
        };
        let mut delivery = row.get::<Json<DeliveryRecord>, _>("payload").0;
        if delivery.version != expected_version {
            tx.commit().await?;
            return Ok(ConcurrencyResult::VersionConflict);
        }

        let now = now_rfc3339();
        delivery.status = DELIVERY_STATUS_REVOKED.to_string();
        delivery.revoked_at = Some(now.clone());
        delivery.revoked_by = Some(revoked_by.to_string());
        delivery.revoke_reason = revoke_reason;
        delivery.updated_at = now.clone();
        delivery.version = delivery.version.saturating_add(1);
        sqlx::query(
            "UPDATE deliveries
                SET status = $2, payload = $3, updated_at = $4
              WHERE delivery_id = $1",
        )
        .bind(&delivery.delivery_id)
        .bind(&delivery.status)
        .bind(Json(&delivery))
        .bind(&delivery.updated_at)
        .execute(&mut *tx)
        .await?;

        let code_rows = sqlx::query(
            "SELECT payload
               FROM delivery_codes
              WHERE delivery_id = $1
              FOR UPDATE",
        )
        .bind(delivery_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut codes = Vec::with_capacity(code_rows.len());
        for row in code_rows {
            let mut code = row.get::<Json<DeliveryCodeRecord>, _>("payload").0;
            code.status = DELIVERY_STATUS_REVOKED.to_string();
            code.revoked_at = Some(now.clone());
            code.updated_at = now.clone();
            code.version = code.version.saturating_add(1);
            sqlx::query(
                "UPDATE delivery_codes
                    SET status = $2, payload = $3, updated_at = $4
                  WHERE code_id = $1",
            )
            .bind(&code.code_id)
            .bind(&code.status)
            .bind(Json(&code))
            .bind(&code.updated_at)
            .execute(&mut *tx)
            .await?;
            codes.push(code);
        }
        codes.sort_by(|left, right| left.code_type.cmp(&right.code_type));

        let entitlement_row = sqlx::query(
            "SELECT payload FROM delivery_entitlements WHERE delivery_id = $1 FOR UPDATE",
        )
        .bind(delivery_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(entitlement_row) = entitlement_row else {
            tx.commit().await?;
            return Ok(ConcurrencyResult::NotFound);
        };
        let mut entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        entitlement.status = DELIVERY_ENTITLEMENT_STATUS_REVOKED.to_string();
        entitlement.updated_at = now;
        sqlx::query(
            "UPDATE delivery_entitlements
                SET status = $2, payload = $3, updated_at = $4
              WHERE entitlement_id = $1",
        )
        .bind(&entitlement.entitlement_id)
        .bind(&entitlement.status)
        .bind(Json(&entitlement))
        .bind(&entitlement.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(ConcurrencyResult::Applied(DeliveryResponse {
            data: delivery_projection(&delivery, &codes, &entitlement),
        }))
    }

    async fn create_delivery_artifact(
        &self,
        draft: DeliveryArtifactDraft,
    ) -> Result<DeliveryArtifactResponse> {
        let mut tx = self.pool.begin().await?;
        let existing_rows = sqlx::query(
            "SELECT payload
               FROM delivery_artifacts
              WHERE delivery_id = $1
              FOR UPDATE",
        )
        .bind(&draft.delivery_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut existing = existing_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryArtifactRecord>, _>("payload").0)
            .collect::<Vec<_>>();
        let artifact = build_delivery_artifact_record(&draft, &existing);
        let now = artifact.created_at.clone();
        for item in existing
            .iter_mut()
            .filter(|item| item.status == DELIVERY_ARTIFACT_STATUS_ACTIVE)
        {
            item.status = DELIVERY_ARTIFACT_STATUS_SUPERSEDED.to_string();
            item.superseded_at = Some(now.clone());
            item.superseded_by = Some(artifact.artifact_id.clone());
            item.updated_at = now.clone();
            sqlx::query(
                "UPDATE delivery_artifacts
                    SET status = $2, payload = $3, updated_at = $4
                  WHERE artifact_id = $1",
            )
            .bind(&item.artifact_id)
            .bind(&item.status)
            .bind(Json(&item))
            .bind(&item.updated_at)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO delivery_artifacts
                (artifact_id, delivery_id, tenant_id, project_id, status, version, sha256, size_bytes, storage_backend, storage_ref, ciphertext, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(&artifact.artifact_id)
        .bind(&artifact.delivery_id)
        .bind(artifact.tenant_id.as_str())
        .bind(artifact.project_id.as_str())
        .bind(&artifact.status)
        .bind(i64::try_from(artifact.version).unwrap_or(i64::MAX))
        .bind(&artifact.sha256)
        .bind(i64::try_from(artifact.size_bytes).unwrap_or(i64::MAX))
        .bind(&artifact.storage_backend)
        .bind(&artifact.storage_ref)
        .bind(&artifact.ciphertext)
        .bind(Json(&artifact))
        .bind(&artifact.created_at)
        .bind(&artifact.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(DeliveryArtifactResponse {
            data: artifact.public_view(),
        })
    }

    async fn create_delivery_upload_batch(
        &self,
        draft: DeliveryUploadBatchDraft,
    ) -> Result<DeliveryUploadBatchResponse> {
        let batch = build_delivery_upload_batch_record(&draft);
        let items = build_delivery_upload_batch_item_records(&draft, &batch);
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO delivery_upload_batches
                (batch_id, tenant_id, project_id, provider, status, idempotency_key, source_file_name, source_file_sha256, total_count, success_count, failed_count, duplicate_count, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT DO NOTHING",
        )
        .bind(&batch.batch_id)
        .bind(batch.tenant_id.as_str())
        .bind(batch.project_id.as_str())
        .bind(&batch.provider)
        .bind(&batch.status)
        .bind(&batch.idempotency_key)
        .bind(&batch.source_file_name)
        .bind(&batch.source_file_sha256)
        .bind(i64::from(batch.total_count))
        .bind(i64::from(batch.success_count))
        .bind(i64::from(batch.failed_count))
        .bind(i64::from(batch.duplicate_count))
        .bind(Json(&batch))
        .bind(&batch.created_at)
        .bind(&batch.updated_at)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if inserted == 0 {
            tx.commit().await?;
            return self
                .get_delivery_upload_batch_by_dedupe(
                    &draft.tenant_id,
                    &draft.project_id,
                    draft.idempotency_key.as_deref(),
                    &draft.source_file_sha256,
                )
                .await?
                .ok_or_else(|| anyhow!("delivery upload batch dedupe conflict was not readable"));
        }
        for item in items {
            sqlx::query(
                "INSERT INTO delivery_upload_batch_items
                    (item_id, batch_id, tenant_id, project_id, delivery_id, status, row_index, payload_sha256, size_bytes, ciphertext, payload, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            )
            .bind(&item.item_id)
            .bind(&item.batch_id)
            .bind(item.tenant_id.as_str())
            .bind(item.project_id.as_str())
            .bind(&item.delivery_id)
            .bind(&item.status)
            .bind(i64::from(item.row_index))
            .bind(&item.payload_sha256)
            .bind(i64::try_from(item.size_bytes).unwrap_or(i64::MAX))
            .bind(&item.ciphertext)
            .bind(Json(&item))
            .bind(&item.created_at)
            .bind(&item.updated_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(batch.public_response())
    }

    async fn get_delivery_upload_batch(
        &self,
        batch_id: &str,
    ) -> Result<Option<DeliveryUploadBatchResponse>> {
        let row = sqlx::query(
            "SELECT payload
               FROM delivery_upload_batches
              WHERE batch_id = $1",
        )
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| {
            row.get::<Json<DeliveryUploadBatchRecord>, _>("payload")
                .0
                .public_response()
        }))
    }

    async fn get_delivery_upload_batch_by_dedupe(
        &self,
        tenant_id: &TenantId,
        project_id: &ProjectId,
        idempotency_key: Option<&str>,
        source_file_sha256: &str,
    ) -> Result<Option<DeliveryUploadBatchResponse>> {
        let row = if let Some(idempotency_key) = idempotency_key {
            sqlx::query(
                "SELECT payload
                   FROM delivery_upload_batches
                  WHERE tenant_id = $1
                    AND project_id = $2
                    AND (source_file_sha256 = $3 OR idempotency_key = $4)
                  ORDER BY created_at
                  LIMIT 1",
            )
            .bind(tenant_id.as_str())
            .bind(project_id.as_str())
            .bind(source_file_sha256)
            .bind(idempotency_key)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_upload_batches
                  WHERE tenant_id = $1
                    AND project_id = $2
                    AND source_file_sha256 = $3
                  ORDER BY created_at
                  LIMIT 1",
            )
            .bind(tenant_id.as_str())
            .bind(project_id.as_str())
            .bind(source_file_sha256)
            .fetch_optional(&self.pool)
            .await?
        };
        Ok(row.map(|row| {
            row.get::<Json<DeliveryUploadBatchRecord>, _>("payload")
                .0
                .public_response()
        }))
    }

    async fn list_delivery_upload_batch_items(
        &self,
        batch_id: &str,
    ) -> Result<Option<DeliveryUploadBatchItemsResponse>> {
        if self.get_delivery_upload_batch(batch_id).await?.is_none() {
            return Ok(None);
        }
        let rows = sqlx::query(
            "SELECT payload
               FROM delivery_upload_batch_items
              WHERE batch_id = $1
              ORDER BY row_index",
        )
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(DeliveryUploadBatchItemsResponse {
            data: rows
                .into_iter()
                .map(|row| {
                    row.get::<Json<DeliveryUploadBatchItemRecord>, _>("payload")
                        .0
                        .public_view()
                })
                .collect(),
        }))
    }

    async fn process_delivery_upload_batch(
        &self,
        batch_id: &str,
        actor_id: &str,
    ) -> Result<DeliveryUploadProcessResult> {
        let mut tx = self.pool.begin().await?;
        let Some(batch_row) = sqlx::query(
            "SELECT payload
               FROM delivery_upload_batches
              WHERE batch_id = $1
              FOR UPDATE",
        )
        .bind(batch_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            tx.commit().await?;
            return Ok(DeliveryUploadProcessResult::BatchNotFound);
        };
        let mut batch = batch_row
            .get::<Json<DeliveryUploadBatchRecord>, _>("payload")
            .0;
        if delivery_upload_batch_is_terminal(&batch.status) {
            tx.commit().await?;
            return Ok(DeliveryUploadProcessResult::Processed(Box::new(
                batch.public_response(),
            )));
        }
        if batch.status == DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING {
            tx.commit().await?;
            return Ok(DeliveryUploadProcessResult::BatchAlreadyProcessing);
        }
        let scope_busy = sqlx::query(
            "SELECT batch_id
               FROM delivery_upload_batches
              WHERE tenant_id = $1 AND project_id = $2 AND status = $3 AND batch_id <> $4
              LIMIT 1
              FOR UPDATE",
        )
        .bind(batch.tenant_id.as_str())
        .bind(batch.project_id.as_str())
        .bind(DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING)
        .bind(&batch.batch_id)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
        if scope_busy {
            tx.commit().await?;
            return Ok(DeliveryUploadProcessResult::ScopeBusy);
        }

        let now = now_rfc3339();
        batch.status = DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING.to_string();
        batch.started_at = Some(now.clone());
        batch.updated_at = now;
        batch.version = batch.version.saturating_add(1);
        update_postgres_upload_batch(&mut tx, &batch).await?;

        let item_rows = sqlx::query(
            "SELECT payload, ciphertext
               FROM delivery_upload_batch_items
              WHERE batch_id = $1
              ORDER BY row_index
              FOR UPDATE",
        )
        .bind(&batch.batch_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut items = item_rows
            .into_iter()
            .map(|row| {
                hydrate_upload_batch_item_ciphertext(
                    row.get::<Json<DeliveryUploadBatchItemRecord>, _>("payload")
                        .0,
                    row.get::<Vec<u8>, _>("ciphertext"),
                )
            })
            .collect::<Vec<_>>();
        for item in &mut items {
            if item.status != DELIVERY_UPLOAD_ITEM_STATUS_PENDING {
                continue;
            }
            let outcome =
                postgres_delivery_upload_item_outcome(&mut tx, item, &batch, actor_id).await?;
            apply_delivery_upload_item_outcome(item, outcome);
            update_postgres_upload_item(&mut tx, item).await?;
        }

        finalize_postgres_delivery_upload_batch(&mut tx, &mut batch, &items).await?;
        tx.commit().await?;
        Ok(DeliveryUploadProcessResult::Processed(Box::new(
            batch.public_response(),
        )))
    }

    async fn list_delivery_artifacts(
        &self,
        delivery_id: &str,
    ) -> Result<DeliveryArtifactsResponse> {
        let rows = sqlx::query(
            "SELECT payload
               FROM delivery_artifacts
              WHERE delivery_id = $1
              ORDER BY version DESC, created_at DESC",
        )
        .bind(delivery_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(DeliveryArtifactsResponse {
            data: rows
                .into_iter()
                .map(|row| {
                    row.get::<Json<DeliveryArtifactRecord>, _>("payload")
                        .0
                        .public_view()
                })
                .collect(),
        })
    }

    async fn get_delivery_artifact(
        &self,
        delivery_id: &str,
        artifact_id: &str,
    ) -> Result<Option<DeliveryArtifactResponse>> {
        let row = sqlx::query(
            "SELECT payload
               FROM delivery_artifacts
              WHERE delivery_id = $1 AND artifact_id = $2",
        )
        .bind(delivery_id)
        .bind(artifact_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| DeliveryArtifactResponse {
            data: row
                .get::<Json<DeliveryArtifactRecord>, _>("payload")
                .0
                .public_view(),
        }))
    }

    async fn get_delivery_by_redemption_code_hash(
        &self,
        code_hash: &str,
    ) -> Result<Option<DeliveryResponse>> {
        let row = sqlx::query(
            "SELECT delivery_id
               FROM delivery_codes
              WHERE code_type = $1 AND code_hash = $2",
        )
        .bind(DELIVERY_CODE_TYPE_REDEMPTION)
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await?;
        let Some(delivery_id) = row.map(|row| row.get::<String, _>("delivery_id")) else {
            return Ok(None);
        };
        self.get_delivery(&delivery_id).await
    }

    async fn get_delivery_by_entitlement_id(
        &self,
        entitlement_id: &str,
    ) -> Result<Option<DeliveryResponse>> {
        let row = sqlx::query(
            "SELECT delivery_id
               FROM delivery_entitlements
              WHERE entitlement_id = $1",
        )
        .bind(entitlement_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(delivery_id) = row.map(|row| row.get::<String, _>("delivery_id")) else {
            return Ok(None);
        };
        self.get_delivery(&delivery_id).await
    }

    async fn redeem_delivery_activation(
        &self,
        code_hash: &str,
        activation_id: String,
        activated_by: &str,
    ) -> Result<DeliveryRedeemResult> {
        let mut tx = self.pool.begin().await?;
        let Some(code_row) = sqlx::query(
            "SELECT payload
               FROM delivery_codes
              WHERE code_type = $1 AND code_hash = $2
              FOR UPDATE",
        )
        .bind(DELIVERY_CODE_TYPE_REDEMPTION)
        .bind(code_hash)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::RedemptionCodeNotFound);
        };
        let mut code = code_row.get::<Json<DeliveryCodeRecord>, _>("payload").0;
        if let Some(result) = blocked_redemption_code_result(&code) {
            return Ok(result);
        }

        let Some(delivery_row) = sqlx::query(
            "SELECT payload
               FROM deliveries
              WHERE delivery_id = $1
              FOR UPDATE",
        )
        .bind(&code.delivery_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::DeliveryNotFound);
        };
        let delivery = delivery_row.get::<Json<DeliveryRecord>, _>("payload").0;

        let Some(entitlement_row) = sqlx::query(
            "SELECT payload
               FROM delivery_entitlements
              WHERE delivery_id = $1
              FOR UPDATE",
        )
        .bind(&code.delivery_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::EntitlementNotFound);
        };
        let mut entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_delivery_activation_result(&delivery, &entitlement) {
            return Ok(result);
        }

        let Some(artifact_row) = sqlx::query(
            "SELECT payload
               FROM delivery_artifacts
              WHERE delivery_id = $1 AND status = $2
              ORDER BY version DESC
              LIMIT 1
              FOR UPDATE",
        )
        .bind(&code.delivery_id)
        .bind(DELIVERY_ARTIFACT_STATUS_ACTIVE)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryRedeemResult::ArtifactMissing);
        };
        let artifact = artifact_row
            .get::<Json<DeliveryArtifactRecord>, _>("payload")
            .0;

        activate_delivery_entitlement(&mut entitlement, None);
        let activation = build_delivery_activation_record(
            &activation_id,
            &delivery,
            &mut code,
            &entitlement,
            &artifact,
        );
        let mut new_segments = Vec::<DeliveryServiceSegmentRecord>::new();
        let mut new_events = Vec::<DeliveryLifecycleEventRecord>::new();
        if let Some(segment) = build_postgres_service_segment_record(
            &[],
            &[],
            &entitlement,
            &activation,
            &artifact,
            activation.activated_at.clone(),
            activated_by,
        ) {
            new_events.push(postgres_lifecycle_event_record(
                &entitlement.entitlement_id,
                Some(segment.segment_id.clone()),
                DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED,
                DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE,
                Some("activation_segment".to_string()),
                activated_by,
                BTreeMap::from([
                    ("artifact_id".to_string(), artifact.artifact_id.clone()),
                    (
                        "effective_until".to_string(),
                        segment.effective_until.clone(),
                    ),
                ]),
            ));
            new_segments.push(segment);
        } else {
            entitlement.status = DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY.to_string();
            entitlement.updated_at = now_rfc3339();
            entitlement.version = entitlement.version.saturating_add(1);
            new_events.push(postgres_lifecycle_event_record(
                &entitlement.entitlement_id,
                None,
                DELIVERY_LIFECYCLE_EVENT_NEEDS_MANUAL_SUPPLY,
                DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY,
                Some("activation_artifact_missing_carrier_window".to_string()),
                activated_by,
                BTreeMap::new(),
            ));
        }
        sqlx::query(
            "UPDATE delivery_codes
                SET status = $2, payload = $3, updated_at = $4
              WHERE code_id = $1",
        )
        .bind(&code.code_id)
        .bind(&code.status)
        .bind(Json(&code))
        .bind(&code.updated_at)
        .execute(&mut *tx)
        .await?;
        update_postgres_entitlement(&mut tx, &entitlement).await?;
        sqlx::query(
            "INSERT INTO delivery_activations
                (activation_id, delivery_id, code_id, artifact_id, entitlement_id, tenant_id, project_id, status, activated_at, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&activation.activation_id)
        .bind(&activation.delivery_id)
        .bind(&activation.code_id)
        .bind(&activation.artifact_id)
        .bind(&activation.entitlement_id)
        .bind(activation.tenant_id.as_str())
        .bind(activation.project_id.as_str())
        .bind(&activation.status)
        .bind(&activation.activated_at)
        .bind(Json(&activation))
        .bind(&activation.created_at)
        .bind(&activation.updated_at)
        .execute(&mut *tx)
        .await?;
        insert_postgres_service_segments(&mut tx, &new_segments).await?;
        insert_postgres_lifecycle_events(&mut tx, &new_events).await?;
        tx.commit().await?;
        Ok(DeliveryRedeemResult::Activated(Box::new(
            DeliveryActivationResponse {
                data: activation.public_view(),
            },
        )))
    }

    async fn get_delivery_activation(
        &self,
        activation_id: &str,
    ) -> Result<Option<DeliveryActivationResponse>> {
        let row = sqlx::query(
            "SELECT payload
               FROM delivery_activations
              WHERE activation_id = $1",
        )
        .bind(activation_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| DeliveryActivationResponse {
            data: row
                .get::<Json<DeliveryActivationRecord>, _>("payload")
                .0
                .public_view(),
        }))
    }

    async fn list_delivery_service_segments(
        &self,
        entitlement_id: &str,
    ) -> Result<DeliveryServiceSegmentsResponse> {
        let rows = sqlx::query(
            "SELECT payload
               FROM delivery_service_segments
              WHERE entitlement_id = $1
              ORDER BY segment_index",
        )
        .bind(entitlement_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(DeliveryServiceSegmentsResponse {
            data: rows
                .into_iter()
                .map(|row| {
                    row.get::<Json<DeliveryServiceSegmentRecord>, _>("payload")
                        .0
                        .public_view()
                })
                .collect(),
        })
    }

    async fn list_delivery_lifecycle_events(
        &self,
        entitlement_id: &str,
    ) -> Result<DeliveryLifecycleEventsResponse> {
        let rows = sqlx::query(
            "SELECT payload
               FROM delivery_lifecycle_events
              WHERE entitlement_id = $1
              ORDER BY created_at",
        )
        .bind(entitlement_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(DeliveryLifecycleEventsResponse {
            data: rows
                .into_iter()
                .map(|row| {
                    row.get::<Json<DeliveryLifecycleEventRecord>, _>("payload")
                        .0
                        .public_view()
                })
                .collect(),
        })
    }

    async fn load_delivery_operations_records(
        &self,
        scope: &DeliveryOperationsScope,
    ) -> Result<DeliveryOperationsRecords> {
        let delivery_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM deliveries
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM deliveries
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let deliveries = delivery_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryRecord>, _>("payload").0)
            .collect::<Vec<_>>();
        let delivery_ids = deliveries
            .iter()
            .map(|delivery| delivery.delivery_id.clone())
            .collect::<HashSet<_>>();

        let code_rows = sqlx::query("SELECT payload FROM delivery_codes")
            .fetch_all(&self.pool)
            .await?;
        let codes = code_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryCodeRecord>, _>("payload").0)
            .filter(|code| delivery_ids.contains(&code.delivery_id))
            .collect::<Vec<_>>();

        let entitlement_rows = sqlx::query("SELECT payload FROM delivery_entitlements")
            .fetch_all(&self.pool)
            .await?;
        let entitlements = entitlement_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryEntitlementRecord>, _>("payload").0)
            .filter(|entitlement| delivery_ids.contains(&entitlement.delivery_id))
            .collect::<Vec<_>>();
        let entitlement_ids = entitlements
            .iter()
            .map(|entitlement| entitlement.entitlement_id.clone())
            .collect::<HashSet<_>>();

        let artifact_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM delivery_artifacts
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_artifacts
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let artifacts = artifact_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryArtifactRecord>, _>("payload").0)
            .filter(|artifact| delivery_ids.contains(&artifact.delivery_id))
            .collect::<Vec<_>>();

        let activation_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM delivery_activations
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_activations
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let activations = activation_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryActivationRecord>, _>("payload").0)
            .filter(|activation| delivery_ids.contains(&activation.delivery_id))
            .collect::<Vec<_>>();

        let grant_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM delivery_download_grants
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_download_grants
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let download_grants = grant_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryDownloadGrantRecord>, _>("payload").0)
            .filter(|grant| delivery_ids.contains(&grant.delivery_id))
            .collect::<Vec<_>>();

        let segment_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM delivery_service_segments
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_service_segments
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let service_segments = segment_rows
            .into_iter()
            .map(|row| {
                row.get::<Json<DeliveryServiceSegmentRecord>, _>("payload")
                    .0
            })
            .filter(|segment| delivery_ids.contains(&segment.delivery_id))
            .collect::<Vec<_>>();

        let event_rows = sqlx::query("SELECT payload FROM delivery_lifecycle_events")
            .fetch_all(&self.pool)
            .await?;
        let lifecycle_events = event_rows
            .into_iter()
            .map(|row| {
                row.get::<Json<DeliveryLifecycleEventRecord>, _>("payload")
                    .0
            })
            .filter(|event| entitlement_ids.contains(&event.entitlement_id))
            .collect::<Vec<_>>();

        let upload_batch_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM delivery_upload_batches
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_upload_batches
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let upload_batches = upload_batch_rows
            .into_iter()
            .map(|row| row.get::<Json<DeliveryUploadBatchRecord>, _>("payload").0)
            .collect::<Vec<_>>();
        let upload_batch_ids = upload_batches
            .iter()
            .map(|batch| batch.batch_id.clone())
            .collect::<HashSet<_>>();

        let upload_item_rows = if let Some(project_id) = scope.project_id.as_ref() {
            sqlx::query(
                "SELECT payload
                   FROM delivery_upload_batch_items
                  WHERE tenant_id = $1 AND project_id = $2",
            )
            .bind(scope.tenant_id.as_str())
            .bind(project_id.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload
                   FROM delivery_upload_batch_items
                  WHERE tenant_id = $1",
            )
            .bind(scope.tenant_id.as_str())
            .fetch_all(&self.pool)
            .await?
        };
        let upload_items = upload_item_rows
            .into_iter()
            .map(|row| {
                row.get::<Json<DeliveryUploadBatchItemRecord>, _>("payload")
                    .0
            })
            .filter(|item| {
                upload_batch_ids.contains(&item.batch_id)
                    || delivery_ids.contains(&item.delivery_id)
            })
            .collect::<Vec<_>>();

        Ok(DeliveryOperationsRecords {
            deliveries,
            codes,
            entitlements,
            artifacts,
            activations,
            download_grants,
            service_segments,
            lifecycle_events,
            upload_batches,
            upload_items,
        })
    }

    async fn reconcile_delivery_lifecycle(
        &self,
        entitlement_id: &str,
        actor_id: &str,
    ) -> Result<DeliveryLifecycleResult> {
        let mut tx = self.pool.begin().await?;
        let Some(entitlement_row) = sqlx::query(
            "SELECT payload
               FROM delivery_entitlements
              WHERE entitlement_id = $1
              FOR UPDATE",
        )
        .bind(entitlement_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            tx.commit().await?;
            return Ok(DeliveryLifecycleResult::EntitlementNotFound);
        };
        let mut entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        let mut new_segments = Vec::<DeliveryServiceSegmentRecord>::new();
        let mut new_events = Vec::<DeliveryLifecycleEventRecord>::new();

        if timestamp_is_expired(&entitlement.ends_at)
            && entitlement.status != DELIVERY_STATUS_EXPIRED
            && entitlement.status != DELIVERY_ENTITLEMENT_STATUS_REVOKED
        {
            entitlement.status = DELIVERY_STATUS_EXPIRED.to_string();
            entitlement.updated_at = now_rfc3339();
            entitlement.version = entitlement.version.saturating_add(1);
            new_events.push(postgres_lifecycle_event_record(
                &entitlement.entitlement_id,
                None,
                DELIVERY_LIFECYCLE_EVENT_EXPIRED,
                DELIVERY_STATUS_EXPIRED,
                Some("entitlement service window ended".to_string()),
                actor_id,
                BTreeMap::new(),
            ));
        } else if !matches!(
            entitlement.status.as_str(),
            DELIVERY_ENTITLEMENT_STATUS_REVOKED | DELIVERY_ENTITLEMENT_STATUS_SUSPENDED
        ) {
            let activation = sqlx::query(
                "SELECT payload
                   FROM delivery_activations
                  WHERE entitlement_id = $1 AND status = $2
                  ORDER BY activated_at
                  LIMIT 1
                  FOR UPDATE",
            )
            .bind(&entitlement.entitlement_id)
            .bind(DELIVERY_ACTIVATION_STATUS_ACTIVATED)
            .fetch_optional(&mut *tx)
            .await?
            .map(|row| row.get::<Json<DeliveryActivationRecord>, _>("payload").0);

            let segment_rows = sqlx::query(
                "SELECT payload
                   FROM delivery_service_segments
                  WHERE entitlement_id = $1
                  FOR UPDATE",
            )
            .bind(&entitlement.entitlement_id)
            .fetch_all(&mut *tx)
            .await?;
            let segments = segment_rows
                .into_iter()
                .map(|row| {
                    row.get::<Json<DeliveryServiceSegmentRecord>, _>("payload")
                        .0
                })
                .collect::<Vec<_>>();

            let Some(activation) = activation else {
                if entitlement.status != DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
                    entitlement.status =
                        DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY.to_string();
                    entitlement.updated_at = now_rfc3339();
                    entitlement.version = entitlement.version.saturating_add(1);
                    new_events.push(postgres_lifecycle_event_record(
                        &entitlement.entitlement_id,
                        None,
                        DELIVERY_LIFECYCLE_EVENT_NEEDS_MANUAL_SUPPLY,
                        DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY,
                        Some("activation_missing".to_string()),
                        actor_id,
                        BTreeMap::new(),
                    ));
                }
                update_postgres_entitlement(&mut tx, &entitlement).await?;
                insert_postgres_lifecycle_events(&mut tx, &new_events).await?;
                tx.commit().await?;
                return self
                    .delivery_lifecycle_response(&entitlement.entitlement_id)
                    .await
                    .map(|response| DeliveryLifecycleResult::Applied(Box::new(response)));
            };

            let coverage_until = segments
                .iter()
                .filter(|segment| segment.status != DELIVERY_SERVICE_SEGMENT_STATUS_REVOKED)
                .map(|segment| segment.effective_until.clone())
                .max()
                .unwrap_or_else(|| entitlement.starts_at.clone());
            if timestamp_after(&entitlement.ends_at, &coverage_until) {
                let used_artifact_ids = segments
                    .iter()
                    .map(|segment| segment.artifact_id.clone())
                    .collect::<HashSet<_>>();
                let artifact_rows = sqlx::query(
                    "SELECT payload
                       FROM delivery_artifacts
                      WHERE tenant_id = $1 AND project_id = $2 AND status = $3
                      FOR UPDATE",
                )
                .bind(activation.tenant_id.as_str())
                .bind(activation.project_id.as_str())
                .bind(DELIVERY_ARTIFACT_STATUS_ACTIVE)
                .fetch_all(&mut *tx)
                .await?;
                let candidate = artifact_rows
                    .into_iter()
                    .map(|row| row.get::<Json<DeliveryArtifactRecord>, _>("payload").0)
                    .filter(|artifact| {
                        artifact.provider == activation.provider
                            && !used_artifact_ids.contains(&artifact.artifact_id)
                            && artifact
                                .carrier_valid_until
                                .as_deref()
                                .is_some_and(|value| timestamp_after(value, &coverage_until))
                    })
                    .max_by(|left, right| left.version.cmp(&right.version));
                if let Some(artifact) = candidate {
                    if let Some(segment) = build_postgres_service_segment_record(
                        &segments,
                        &new_segments,
                        &entitlement,
                        &activation,
                        &artifact,
                        coverage_until,
                        actor_id,
                    ) {
                        new_events.push(postgres_lifecycle_event_record(
                            &entitlement.entitlement_id,
                            Some(segment.segment_id.clone()),
                            DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED,
                            segment.status.as_str(),
                            Some("continuation_artifact_matched".to_string()),
                            actor_id,
                            BTreeMap::from([(
                                "artifact_id".to_string(),
                                artifact.artifact_id.clone(),
                            )]),
                        ));
                        new_segments.push(segment);
                        if entitlement.status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
                            entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
                            entitlement.updated_at = now_rfc3339();
                            entitlement.version = entitlement.version.saturating_add(1);
                        }
                    }
                } else if entitlement.status != DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
                    entitlement.status =
                        DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY.to_string();
                    entitlement.updated_at = now_rfc3339();
                    entitlement.version = entitlement.version.saturating_add(1);
                    new_events.push(postgres_lifecycle_event_record(
                        &entitlement.entitlement_id,
                        None,
                        DELIVERY_LIFECYCLE_EVENT_NEEDS_MANUAL_SUPPLY,
                        DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY,
                        Some("continuation_artifact_missing".to_string()),
                        actor_id,
                        BTreeMap::new(),
                    ));
                }
            } else if entitlement.status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
                entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
                entitlement.updated_at = now_rfc3339();
                entitlement.version = entitlement.version.saturating_add(1);
            }
        }

        update_postgres_entitlement(&mut tx, &entitlement).await?;
        insert_postgres_service_segments(&mut tx, &new_segments).await?;
        insert_postgres_lifecycle_events(&mut tx, &new_events).await?;
        tx.commit().await?;
        self.delivery_lifecycle_response(&entitlement.entitlement_id)
            .await
            .map(|response| DeliveryLifecycleResult::Applied(Box::new(response)))
    }

    async fn extend_delivery_entitlement(
        &self,
        draft: DeliveryEntitlementExtendDraft,
    ) -> Result<ConcurrencyResult<DeliveryLifecycleResponse>> {
        let mut tx = self.pool.begin().await?;
        let Some(row) = sqlx::query(
            "SELECT payload
               FROM delivery_entitlements
              WHERE entitlement_id = $1
              FOR UPDATE",
        )
        .bind(&draft.entitlement_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            tx.commit().await?;
            return Ok(ConcurrencyResult::NotFound);
        };
        let mut entitlement = row.get::<Json<DeliveryEntitlementRecord>, _>("payload").0;
        if entitlement.version != draft.expected_version {
            tx.commit().await?;
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let old_ends_at = entitlement.ends_at.clone();
        entitlement.ends_at = timestamp_plus_days(&entitlement.ends_at, draft.extend_days)
            .unwrap_or_else(|| expires_at(u64::from(draft.extend_days) * 86_400));
        entitlement.service_days = entitlement.service_days.saturating_add(draft.extend_days);
        if entitlement.status != DELIVERY_ENTITLEMENT_STATUS_REVOKED
            && entitlement.status != DELIVERY_ENTITLEMENT_STATUS_SUSPENDED
            && !timestamp_is_expired(&entitlement.ends_at)
            && entitlement.activated_at.is_some()
        {
            entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
        }
        entitlement.updated_at = now_rfc3339();
        entitlement.version = entitlement.version.saturating_add(1);
        let event = postgres_lifecycle_event_record(
            &entitlement.entitlement_id,
            None,
            DELIVERY_LIFECYCLE_EVENT_RENEWED,
            entitlement.status.as_str(),
            draft.reason.clone(),
            &draft.actor_id,
            BTreeMap::from([
                ("old_service_ends_at".to_string(), old_ends_at),
                (
                    "new_service_ends_at".to_string(),
                    entitlement.ends_at.clone(),
                ),
                ("extend_days".to_string(), draft.extend_days.to_string()),
            ]),
        );
        update_postgres_entitlement(&mut tx, &entitlement).await?;
        insert_postgres_lifecycle_events(&mut tx, &[event]).await?;
        tx.commit().await?;
        match self
            .reconcile_delivery_lifecycle(&entitlement.entitlement_id, &draft.actor_id)
            .await?
        {
            DeliveryLifecycleResult::Applied(response) => Ok(ConcurrencyResult::Applied(*response)),
            DeliveryLifecycleResult::EntitlementNotFound => Ok(ConcurrencyResult::NotFound),
        }
    }

    async fn delivery_lifecycle_response(
        &self,
        entitlement_id: &str,
    ) -> Result<DeliveryLifecycleResponse> {
        let entitlement = sqlx::query(
            "SELECT payload
               FROM delivery_entitlements
              WHERE entitlement_id = $1",
        )
        .bind(entitlement_id)
        .fetch_one(&self.pool)
        .await?
        .get::<Json<DeliveryEntitlementRecord>, _>("payload")
        .0;
        let segments = self
            .list_delivery_service_segments(entitlement_id)
            .await?
            .data;
        let events = self
            .list_delivery_lifecycle_events(entitlement_id)
            .await?
            .data;
        Ok(DeliveryLifecycleResponse {
            entitlement: entitlement.public_view(),
            segments,
            events,
        })
    }

    async fn issue_delivery_download_grant(
        &self,
        draft: DeliveryDownloadGrantDraft,
    ) -> Result<DeliveryDownloadGrantIssueResult> {
        let mut tx = self.pool.begin().await?;
        let Some(activation_row) = sqlx::query(
            "SELECT payload
               FROM delivery_activations
              WHERE activation_id = $1
              FOR UPDATE",
        )
        .bind(&draft.activation_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadGrantIssueResult::ActivationNotFound);
        };
        let activation = activation_row
            .get::<Json<DeliveryActivationRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_activation_issue_result(&activation) {
            return Ok(result);
        }

        let Some(entitlement_row) = sqlx::query(
            "SELECT payload
               FROM delivery_entitlements
              WHERE entitlement_id = $1
              FOR UPDATE",
        )
        .bind(&activation.entitlement_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadGrantIssueResult::EntitlementNotFound);
        };
        let entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_entitlement_issue_result(&entitlement) {
            return Ok(result);
        }

        let segment_rows = sqlx::query(
            "SELECT payload
               FROM delivery_service_segments
              WHERE entitlement_id = $1 AND activation_id = $2
              FOR UPDATE",
        )
        .bind(&entitlement.entitlement_id)
        .bind(&activation.activation_id)
        .fetch_all(&mut *tx)
        .await?;
        let segments = segment_rows
            .into_iter()
            .map(|row| {
                row.get::<Json<DeliveryServiceSegmentRecord>, _>("payload")
                    .0
            })
            .collect::<Vec<_>>();
        let Some(segment) = current_segment_for_entitlement(
            &segments,
            &entitlement.entitlement_id,
            &activation.activation_id,
        )
        .cloned() else {
            return Ok(DeliveryDownloadGrantIssueResult::SegmentMissing);
        };

        let Some(artifact_row) = sqlx::query(
            "SELECT payload
               FROM delivery_artifacts
              WHERE delivery_id = $1 AND artifact_id = $2
              FOR UPDATE",
        )
        .bind(&activation.delivery_id)
        .bind(&segment.artifact_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadGrantIssueResult::ArtifactMissing);
        };
        let artifact = artifact_row
            .get::<Json<DeliveryArtifactRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_artifact_issue_result(&artifact) {
            return Ok(result);
        }

        let grant = build_delivery_download_grant_record(&draft, &activation, &artifact);
        sqlx::query(
            "INSERT INTO delivery_download_grants
                (grant_id, activation_id, delivery_id, artifact_id, entitlement_id, tenant_id, project_id, status, token_hash, expires_at, use_count, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(&grant.grant_id)
        .bind(&grant.activation_id)
        .bind(&grant.delivery_id)
        .bind(&grant.artifact_id)
        .bind(&grant.entitlement_id)
        .bind(grant.tenant_id.as_str())
        .bind(grant.project_id.as_str())
        .bind(&grant.status)
        .bind(&grant.token_hash)
        .bind(&grant.expires_at)
        .bind(i64::from(grant.use_count))
        .bind(Json(&grant))
        .bind(&grant.created_at)
        .bind(&grant.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(DeliveryDownloadGrantIssueResult::Issued(Box::new(
            DeliveryDownloadGrantIssueResponse {
                data: grant.public_view(),
                download_token: draft.token_plaintext,
            },
        )))
    }

    async fn get_delivery_download_grant(
        &self,
        grant_id: &str,
    ) -> Result<Option<DeliveryDownloadGrantResponse>> {
        let row = sqlx::query(
            "SELECT payload
               FROM delivery_download_grants
              WHERE grant_id = $1",
        )
        .bind(grant_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| DeliveryDownloadGrantResponse {
            data: row
                .get::<Json<DeliveryDownloadGrantRecord>, _>("payload")
                .0
                .public_view(),
        }))
    }

    async fn revoke_delivery_download_grant(
        &self,
        grant_id: &str,
        expected_version: u64,
        revoked_by: &str,
        revoke_reason: Option<String>,
    ) -> Result<ConcurrencyResult<DeliveryDownloadGrantResponse>> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT payload
               FROM delivery_download_grants
              WHERE grant_id = $1
              FOR UPDATE",
        )
        .bind(grant_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(ConcurrencyResult::NotFound);
        };
        let mut grant = row.get::<Json<DeliveryDownloadGrantRecord>, _>("payload").0;
        if grant.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let now = now_rfc3339();
        grant.status = DELIVERY_DOWNLOAD_GRANT_STATUS_REVOKED.to_string();
        grant.revoked_at = Some(now.clone());
        grant.revoked_by = Some(revoked_by.to_string());
        grant.revoke_reason = revoke_reason;
        grant.updated_at = now;
        grant.version = grant.version.saturating_add(1);
        sqlx::query(
            "UPDATE delivery_download_grants
                SET status = $2, payload = $3, updated_at = $4
              WHERE grant_id = $1",
        )
        .bind(&grant.grant_id)
        .bind(&grant.status)
        .bind(Json(&grant))
        .bind(&grant.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ConcurrencyResult::Applied(DeliveryDownloadGrantResponse {
            data: grant.public_view(),
        }))
    }

    async fn consume_delivery_download_grant(
        &self,
        token_hash: &str,
    ) -> Result<DeliveryDownloadConsumeResult> {
        let mut tx = self.pool.begin().await?;
        let Some(grant_row) = sqlx::query(
            "SELECT payload
               FROM delivery_download_grants
              WHERE token_hash = $1
              FOR UPDATE",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadConsumeResult::GrantNotFound);
        };
        let mut grant = grant_row
            .get::<Json<DeliveryDownloadGrantRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_grant_consume_result(&mut grant) {
            if grant.status == DELIVERY_DOWNLOAD_GRANT_STATUS_EXPIRED {
                sqlx::query(
                    "UPDATE delivery_download_grants
                        SET status = $2, payload = $3, updated_at = $4
                      WHERE grant_id = $1",
                )
                .bind(&grant.grant_id)
                .bind(&grant.status)
                .bind(Json(&grant))
                .bind(&grant.updated_at)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }
            return Ok(result);
        }

        let Some(activation_row) = sqlx::query(
            "SELECT payload
               FROM delivery_activations
              WHERE activation_id = $1
              FOR UPDATE",
        )
        .bind(&grant.activation_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadConsumeResult::ActivationNotFound);
        };
        let activation = activation_row
            .get::<Json<DeliveryActivationRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_activation_consume_result(&activation) {
            return Ok(result);
        }

        let Some(entitlement_row) = sqlx::query(
            "SELECT payload
               FROM delivery_entitlements
              WHERE entitlement_id = $1
              FOR UPDATE",
        )
        .bind(&grant.entitlement_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadConsumeResult::EntitlementNotFound);
        };
        let entitlement = entitlement_row
            .get::<Json<DeliveryEntitlementRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_entitlement_consume_result(&entitlement) {
            return Ok(result);
        }

        let segment_rows = sqlx::query(
            "SELECT payload
               FROM delivery_service_segments
              WHERE entitlement_id = $1 AND activation_id = $2
              FOR UPDATE",
        )
        .bind(&grant.entitlement_id)
        .bind(&grant.activation_id)
        .fetch_all(&mut *tx)
        .await?;
        let segments = segment_rows
            .into_iter()
            .map(|row| {
                row.get::<Json<DeliveryServiceSegmentRecord>, _>("payload")
                    .0
            })
            .collect::<Vec<_>>();
        if active_segment_for_artifact(
            &segments,
            &grant.entitlement_id,
            &grant.activation_id,
            &grant.artifact_id,
        )
        .is_none()
        {
            return Ok(DeliveryDownloadConsumeResult::SegmentMissing);
        }

        let Some(artifact_row) = sqlx::query(
            "SELECT payload, ciphertext
               FROM delivery_artifacts
              WHERE delivery_id = $1 AND artifact_id = $2
              FOR UPDATE",
        )
        .bind(&grant.delivery_id)
        .bind(&grant.artifact_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(DeliveryDownloadConsumeResult::ArtifactMissing);
        };
        let artifact = artifact_row
            .get::<Json<DeliveryArtifactRecord>, _>("payload")
            .0;
        if let Some(result) = blocked_download_artifact_consume_result(&artifact) {
            return Ok(result);
        }
        let ciphertext = artifact_row.get::<Vec<u8>, _>("ciphertext");
        let payload = mark_download_grant_used_and_payload(&mut grant, &artifact, ciphertext);
        sqlx::query(
            "UPDATE delivery_download_grants
                SET status = $2, use_count = $3, payload = $4, updated_at = $5
              WHERE grant_id = $1",
        )
        .bind(&grant.grant_id)
        .bind(&grant.status)
        .bind(i64::from(grant.use_count))
        .bind(Json(&grant))
        .bind(&grant.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(DeliveryDownloadConsumeResult::Retrieved(Box::new(payload)))
    }

    async fn list_codex_auth_accounts(&self) -> Result<CodexAuthAccountsResponse> {
        let rows = sqlx::query("SELECT payload FROM codex_auth_accounts ORDER BY codex_account_id")
            .fetch_all(&self.pool)
            .await?;
        let active_rows = sqlx::query(
            "SELECT account_id, provider, COUNT(*) AS active_runs
               FROM oauth_pool_runtime_leases
              WHERE status = 'active' AND expires_at > $1
              GROUP BY account_id, provider",
        )
        .bind(now_rfc3339())
        .fetch_all(&self.pool)
        .await?;
        let active_runs_by_account = active_rows
            .into_iter()
            .map(|row| {
                (
                    (
                        row.get::<String, _>("account_id"),
                        row.get::<String, _>("provider"),
                    ),
                    u32::try_from(row.get::<i64, _>("active_runs")).unwrap_or(u32::MAX),
                )
            })
            .collect::<HashMap<_, _>>();
        Ok(CodexAuthAccountsResponse {
            data: rows
                .into_iter()
                .map(|row| {
                    let account = row.get::<Json<CodexAuthAccountRecord>, _>("payload").0;
                    let active_runs = active_runs_by_account
                        .get(&(account.codex_account_id.clone(), account.provider.clone()))
                        .copied()
                        .unwrap_or(0);
                    account.public_view_with_active_runs(active_runs)
                })
                .collect(),
        })
    }

    async fn create_codex_auth_account(
        &self,
        record: &CodexAuthAccountRecord,
    ) -> Result<CodexAuthAccount> {
        sqlx::query(
            "INSERT INTO codex_auth_accounts
                (codex_account_id, tenant_id, project_id, provider_resource_id, display_name, status, auth_json_sha256, encrypted_auth_json, leased_until, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&record.codex_account_id)
        .bind(record.tenant_id.as_str())
        .bind(record.project_id.as_ref().map(ProjectId::as_str))
        .bind(record.provider_resource_id.as_str())
        .bind(&record.display_name)
        .bind(&record.status)
        .bind(&record.auth_json_sha256)
        .bind(Json(&record.encrypted_auth_json))
        .bind(record.leased_until.as_deref())
        .bind(Json(record))
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(record.public_view())
    }

    #[allow(dead_code)]
    async fn lease_codex_auth_account(
        &self,
        provider_resource_id: Option<&str>,
    ) -> Result<Option<CodexAuthAccountRecord>> {
        let row = if let Some(provider_resource_id) = provider_resource_id {
            sqlx::query(
                "SELECT payload FROM codex_auth_accounts
                 WHERE status = 'active' AND provider_resource_id = $1
                 ORDER BY updated_at ASC
                 LIMIT 1",
            )
            .bind(provider_resource_id)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT payload FROM codex_auth_accounts
                 WHERE status = 'active'
                 ORDER BY updated_at ASC
                 LIMIT 1",
            )
            .fetch_optional(&self.pool)
            .await?
        };

        let Some(row) = row else {
            return Ok(None);
        };
        let mut record = row.get::<Json<CodexAuthAccountRecord>, _>("payload").0;
        record.leased_until = Some(lease_until_rfc3339());
        record.updated_at = now_rfc3339();
        record.version = record.version.saturating_add(1);
        sqlx::query(
            "UPDATE codex_auth_accounts
                SET leased_until = $2, updated_at = $3, payload = $4
              WHERE codex_account_id = $1",
        )
        .bind(&record.codex_account_id)
        .bind(record.leased_until.as_deref())
        .bind(&record.updated_at)
        .bind(Json(&record))
        .execute(&self.pool)
        .await?;
        Ok(Some(record))
    }

    async fn select_oauth_pool_account(
        &self,
        request: &OAuthPoolSelectionRequest,
    ) -> Result<OAuthPoolSelection> {
        let mut tx = self.pool.begin().await?;
        let account_rows = sqlx::query("SELECT payload FROM codex_auth_accounts ORDER BY updated_at ASC FOR UPDATE SKIP LOCKED")
            .fetch_all(&mut *tx)
            .await?;
        let mut memory = MemoryStore {
            codex_auth_accounts: account_rows
                .into_iter()
                .map(|row| row.get::<Json<CodexAuthAccountRecord>, _>("payload").0)
                .collect(),
            oauth_sharing_leases: load_oauth_sharing_lease_records_from_tx(&mut tx).await?,
            oauth_carpools: load_oauth_carpool_records_from_tx(&mut tx).await?,
            oauth_pool_runtime_leases: load_oauth_pool_runtime_lease_records_from_tx(&mut tx)
                .await?,
            oauth_pool_session_bindings: load_oauth_pool_session_binding_records_from_tx(&mut tx)
                .await?,
            oauth_sharing_audit_events: load_oauth_sharing_audit_records_from_tx(&mut tx).await?,
            ..MemoryStore::default()
        };
        let audit_len = memory.oauth_sharing_audit_events.len();
        let selection = select_memory_oauth_pool_account(&mut memory, request);

        for account in &memory.codex_auth_accounts {
            persist_codex_auth_account_tx(&mut tx, account).await?;
        }

        for runtime_lease in &memory.oauth_pool_runtime_leases {
            upsert_oauth_pool_runtime_lease_tx(&mut tx, runtime_lease).await?;
        }

        for binding in &memory.oauth_pool_session_bindings {
            upsert_oauth_pool_session_binding_tx(&mut tx, binding).await?;
        }

        for event in memory.oauth_sharing_audit_events.iter().skip(audit_len) {
            insert_oauth_sharing_audit_event_tx(&mut tx, event).await?;
        }

        tx.commit().await?;
        Ok(selection)
    }

    async fn list_oauth_sharing_leases(&self) -> Result<OAuthSharingLeasesResponse> {
        Ok(OAuthSharingLeasesResponse {
            data: self.load_oauth_sharing_lease_records().await?,
        })
    }

    async fn upsert_oauth_sharing_lease(
        &self,
        record: &OAuthSharingLeaseRecord,
    ) -> Result<OAuthSharingLeaseRecord> {
        sqlx::query(
            "INSERT INTO oauth_sharing_leases
                (lease_id, provider, pool_id, borrower_workspace_id, status, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (lease_id) DO UPDATE SET
                provider = EXCLUDED.provider,
                pool_id = EXCLUDED.pool_id,
                borrower_workspace_id = EXCLUDED.borrower_workspace_id,
                status = EXCLUDED.status,
                payload = EXCLUDED.payload,
                updated_at = EXCLUDED.updated_at",
        )
        .bind(&record.lease_id)
        .bind(&record.provider)
        .bind(&record.pool_id)
        .bind(&record.borrower_workspace_id)
        .bind(&record.status)
        .bind(Json(record))
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;
        self.insert_oauth_sharing_audit_event(&OAuthSharingAuditEventRecord {
            audit_event_id: format!("oauthaud_{}", now_rfc3339().replace([':', '.', '-'], "")),
            event_type: "lease_upsert".to_string(),
            provider: record.provider.clone(),
            lease_id: Some(record.lease_id.clone()),
            carpool_id: None,
            workspace_id: Some(record.borrower_workspace_id.clone()),
            account_id: None,
            pool_id: Some(record.pool_id.clone()),
            reason: "lease upserted by control plane".to_string(),
            metadata: serde_json::json!({ "status": record.status }),
            created_at: now_rfc3339(),
        })
        .await?;
        Ok(record.clone())
    }

    async fn revoke_oauth_sharing_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<OAuthSharingLeaseRecord>> {
        let Some(mut record) = self
            .load_oauth_sharing_lease_records()
            .await?
            .into_iter()
            .find(|candidate| candidate.lease_id == lease_id)
        else {
            return Ok(None);
        };
        record.status = "revoked".to_string();
        record.updated_at = now_rfc3339();
        record.version = record.version.saturating_add(1);
        self.upsert_oauth_sharing_lease(&record).await?;
        Ok(Some(record))
    }

    async fn list_oauth_carpools(&self) -> Result<OAuthCarpoolsResponse> {
        Ok(OAuthCarpoolsResponse {
            data: self.load_oauth_carpool_records().await?,
        })
    }

    async fn upsert_oauth_carpool(
        &self,
        record: &OAuthCarpoolRecord,
    ) -> Result<OAuthCarpoolRecord> {
        sqlx::query(
            "INSERT INTO oauth_carpools
                (carpool_id, provider, enabled, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (carpool_id) DO UPDATE SET
                provider = EXCLUDED.provider,
                enabled = EXCLUDED.enabled,
                payload = EXCLUDED.payload,
                updated_at = EXCLUDED.updated_at",
        )
        .bind(&record.carpool_id)
        .bind(&record.provider)
        .bind(record.enabled)
        .bind(Json(record))
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;
        self.insert_oauth_sharing_audit_event(&OAuthSharingAuditEventRecord {
            audit_event_id: format!("oauthaud_{}", now_rfc3339().replace([':', '.', '-'], "")),
            event_type: "carpool_upsert".to_string(),
            provider: record.provider.clone(),
            lease_id: None,
            carpool_id: Some(record.carpool_id.clone()),
            workspace_id: None,
            account_id: None,
            pool_id: None,
            reason: "carpool upserted by control plane".to_string(),
            metadata: serde_json::json!({ "enabled": record.enabled }),
            created_at: now_rfc3339(),
        })
        .await?;
        Ok(record.clone())
    }

    async fn remove_oauth_carpool(&self, carpool_id: &str) -> Result<Option<OAuthCarpoolRecord>> {
        let Some(record) = self
            .load_oauth_carpool_records()
            .await?
            .into_iter()
            .find(|candidate| candidate.carpool_id == carpool_id)
        else {
            return Ok(None);
        };
        sqlx::query("DELETE FROM oauth_carpools WHERE carpool_id = $1")
            .bind(carpool_id)
            .execute(&self.pool)
            .await?;
        self.insert_oauth_sharing_audit_event(&OAuthSharingAuditEventRecord {
            audit_event_id: format!("oauthaud_{}", now_rfc3339().replace([':', '.', '-'], "")),
            event_type: "carpool_remove".to_string(),
            provider: record.provider.clone(),
            lease_id: None,
            carpool_id: Some(record.carpool_id.clone()),
            workspace_id: None,
            account_id: None,
            pool_id: None,
            reason: "carpool removed by control plane".to_string(),
            metadata: serde_json::json!({}),
            created_at: now_rfc3339(),
        })
        .await?;
        Ok(Some(record))
    }

    async fn read_oauth_sharing_usage(
        &self,
        filters: OAuthSharingUsageFilters<'_>,
    ) -> Result<OAuthSharingUsageResponse> {
        let memory = MemoryStore {
            oauth_sharing_audit_events: self.load_oauth_sharing_audit_records().await?,
            ..MemoryStore::default()
        };
        Ok(read_memory_oauth_sharing_usage(&memory, filters))
    }

    async fn heartbeat_oauth_pool_runtime_lease(
        &self,
        runtime_lease_id: &str,
    ) -> Result<Option<OAuthPoolRuntimeLeaseRecord>> {
        let mut tx = self.pool.begin().await?;
        let mut memory = MemoryStore {
            codex_auth_accounts: load_codex_auth_account_records_from_tx(&mut tx).await?,
            oauth_pool_runtime_leases: load_oauth_pool_runtime_lease_records_from_tx(&mut tx)
                .await?,
            oauth_pool_session_bindings: load_oauth_pool_session_binding_records_from_tx(&mut tx)
                .await?,
            oauth_sharing_audit_events: load_oauth_sharing_audit_records_from_tx(&mut tx).await?,
            ..MemoryStore::default()
        };
        let audit_len = memory.oauth_sharing_audit_events.len();
        let result = heartbeat_memory_runtime_lease(&mut memory, runtime_lease_id);
        for account in &memory.codex_auth_accounts {
            persist_codex_auth_account_tx(&mut tx, account).await?;
        }
        for runtime_lease in &memory.oauth_pool_runtime_leases {
            upsert_oauth_pool_runtime_lease_tx(&mut tx, runtime_lease).await?;
        }
        for binding in &memory.oauth_pool_session_bindings {
            upsert_oauth_pool_session_binding_tx(&mut tx, binding).await?;
        }
        for event in memory.oauth_sharing_audit_events.iter().skip(audit_len) {
            insert_oauth_sharing_audit_event_tx(&mut tx, event).await?;
        }
        tx.commit().await?;
        Ok(result)
    }

    async fn release_oauth_pool_runtime_lease(
        &self,
        runtime_lease_id: &str,
    ) -> Result<Option<OAuthPoolRuntimeLeaseRecord>> {
        let mut tx = self.pool.begin().await?;
        let mut memory = MemoryStore {
            codex_auth_accounts: load_codex_auth_account_records_from_tx(&mut tx).await?,
            oauth_pool_runtime_leases: load_oauth_pool_runtime_lease_records_from_tx(&mut tx)
                .await?,
            oauth_pool_session_bindings: load_oauth_pool_session_binding_records_from_tx(&mut tx)
                .await?,
            oauth_sharing_audit_events: load_oauth_sharing_audit_records_from_tx(&mut tx).await?,
            ..MemoryStore::default()
        };
        let audit_len = memory.oauth_sharing_audit_events.len();
        let result = release_memory_runtime_lease(&mut memory, runtime_lease_id);
        for account in &memory.codex_auth_accounts {
            persist_codex_auth_account_tx(&mut tx, account).await?;
        }
        for runtime_lease in &memory.oauth_pool_runtime_leases {
            upsert_oauth_pool_runtime_lease_tx(&mut tx, runtime_lease).await?;
        }
        for binding in &memory.oauth_pool_session_bindings {
            upsert_oauth_pool_session_binding_tx(&mut tx, binding).await?;
        }
        for event in memory.oauth_sharing_audit_events.iter().skip(audit_len) {
            insert_oauth_sharing_audit_event_tx(&mut tx, event).await?;
        }
        tx.commit().await?;
        Ok(result)
    }

    async fn record_oauth_pool_account_feedback(
        &self,
        account_id: &str,
        feedback: OAuthPoolAccountFeedback,
    ) -> Result<Option<CodexAuthAccount>> {
        let mut tx = self.pool.begin().await?;
        let mut memory = MemoryStore {
            codex_auth_accounts: load_codex_auth_account_records_from_tx(&mut tx).await?,
            oauth_sharing_audit_events: load_oauth_sharing_audit_records_from_tx(&mut tx).await?,
            ..MemoryStore::default()
        };
        let audit_len = memory.oauth_sharing_audit_events.len();
        let result = apply_memory_account_feedback(&mut memory, account_id, feedback);
        if let Some(account) = result.as_ref() {
            persist_codex_auth_account_tx(&mut tx, account).await?;
        }
        for event in memory.oauth_sharing_audit_events.iter().skip(audit_len) {
            insert_oauth_sharing_audit_event_tx(&mut tx, event).await?;
        }
        tx.commit().await?;
        Ok(result.map(|record| record.public_view()))
    }

    async fn load_oauth_sharing_lease_records(&self) -> Result<Vec<OAuthSharingLeaseRecord>> {
        let rows = sqlx::query("SELECT payload FROM oauth_sharing_leases ORDER BY lease_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<OAuthSharingLeaseRecord>, _>("payload").0)
            .collect())
    }

    async fn load_oauth_carpool_records(&self) -> Result<Vec<OAuthCarpoolRecord>> {
        let rows = sqlx::query("SELECT payload FROM oauth_carpools ORDER BY carpool_id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<OAuthCarpoolRecord>, _>("payload").0)
            .collect())
    }

    async fn load_oauth_sharing_audit_records(&self) -> Result<Vec<OAuthSharingAuditEventRecord>> {
        let rows =
            sqlx::query("SELECT payload FROM oauth_sharing_audit_events ORDER BY created_at")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                row.get::<Json<OAuthSharingAuditEventRecord>, _>("payload")
                    .0
            })
            .collect())
    }

    async fn insert_oauth_sharing_audit_event(
        &self,
        event: &OAuthSharingAuditEventRecord,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth_sharing_audit_events
                (audit_event_id, event_type, provider, lease_id, carpool_id, workspace_id, account_id, pool_id, payload, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (audit_event_id) DO NOTHING",
        )
        .bind(&event.audit_event_id)
        .bind(&event.event_type)
        .bind(&event.provider)
        .bind(event.lease_id.as_deref())
        .bind(event.carpool_id.as_deref())
        .bind(event.workspace_id.as_deref())
        .bind(event.account_id.as_deref())
        .bind(event.pool_id.as_deref())
        .bind(Json(event))
        .bind(&event.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_api_key(
        &self,
        api_key_id: &str,
        expected_version: u64,
    ) -> Result<ConcurrencyResult<ApiKey>> {
        let row = sqlx::query(
            "SELECT api_key_id, provider_resource_id, display_name, key_prefix, is_active, created_at, updated_at, version
               FROM api_keys
              WHERE api_key_id = $1",
        )
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(ConcurrencyResult::NotFound);
        };
        let current_version = row.get::<i64, _>("version") as u64;
        if current_version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let provider_resource_id = row.get::<String, _>("provider_resource_id");
        let new_version = current_version.saturating_add(1);
        let now = now_rfc3339();
        sqlx::query(
            "UPDATE api_keys
                SET is_active = false, updated_at = $2, version = $3
              WHERE api_key_id = $1",
        )
        .bind(api_key_id)
        .bind(&now)
        .bind(i64::try_from(new_version).unwrap_or(i64::MAX))
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(ApiKey {
            api_key_id: api_key_id.to_string(),
            provider_resource_id: ProviderResourceId::parse(provider_resource_id)
                .context("bad provider resource id in api key record")?,
            display_name: row.get::<String, _>("display_name"),
            key_prefix: row.get::<String, _>("key_prefix"),
            is_active: false,
            created_at: row.get::<String, _>("created_at"),
            updated_at: now,
            version: new_version,
        }))
    }

    async fn revoke_opening_grant(
        &self,
        grant_id: &str,
        expected_version: u64,
        revoked_by: &str,
    ) -> Result<ConcurrencyResult<OpeningGrant>> {
        let row = sqlx::query("SELECT payload FROM opening_grants WHERE grant_id = $1")
            .bind(grant_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(ConcurrencyResult::NotFound);
        };
        let mut record = row.get::<Json<OpeningGrantRecord>, _>("payload").0;
        if record.version != expected_version {
            return Ok(ConcurrencyResult::VersionConflict);
        }
        let now = now_rfc3339();
        record.status = "revoked".to_string();
        record.revoked_at = Some(now.clone());
        record.revoked_by = Some(revoked_by.to_string());
        record.updated_at = now;
        record.version = record.version.saturating_add(1);
        sqlx::query(
            "UPDATE opening_grants
                SET status = $2, payload = $3, updated_at = $4
              WHERE grant_id = $1",
        )
        .bind(&record.grant_id)
        .bind(&record.status)
        .bind(Json(&record))
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(ConcurrencyResult::Applied(record.public_view()))
    }

    async fn resolve_api_key(&self, hash: &str) -> Result<Option<ResolvedApiKey>> {
        let grant_row = sqlx::query(
            "SELECT payload
               FROM opening_grants
              WHERE credential_hash = $1
              LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(row) = grant_row {
            let grant = row.get::<Json<OpeningGrantRecord>, _>("payload").0;
            return Ok(Some(grant.to_resolved()));
        }

        let row = sqlx::query(
            "SELECT ak.api_key_id, ak.provider_resource_id, ak.is_active, ak.version, ak.created_at, ak.updated_at,
                    pr.tenant_id, pr.project_id
               FROM api_keys ak
               JOIN provider_resources pr ON pr.provider_resource_id = ak.provider_resource_id
              WHERE ak.hash = $1
              LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(ResolvedApiKey {
            api_key_id: row.get::<String, _>("api_key_id"),
            grant_id: None,
            owner_account_id: None,
            tenant_id: TenantId::parse(row.get::<String, _>("tenant_id"))
                .context("invalid tenant id for api key")?,
            project_id: row
                .get::<Option<String>, _>("project_id")
                .map(|project_id| ProjectId::parse(project_id))
                .transpose()
                .ok()
                .flatten(),
            is_active: row.get::<bool, _>("is_active"),
            status: if row.get::<bool, _>("is_active") {
                "active".to_string()
            } else {
                "revoked".to_string()
            },
            config_snapshot_id: None,
            route_policy_id: None,
            scopes: Vec::new(),
            expires_at: None,
        }))
    }

    async fn get_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        let resolved_id = if config_snapshot_id == ACTIVE_CONFIG_ALIAS {
            sqlx::query("SELECT config_snapshot_id FROM active_config_pointers WHERE pointer_key = 'default'")
                .fetch_optional(&self.pool)
                .await?
                .map(|row| row.get::<String, _>("config_snapshot_id"))
        } else {
            Some(config_snapshot_id.to_string())
        };
        let Some(snapshot_id) = resolved_id else {
            return Ok(None);
        };
        let row = sqlx::query("SELECT payload FROM config_snapshots WHERE config_snapshot_id = $1")
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| ConfigSnapshotResponse {
            config_snapshot: row.get::<Json<ConfigSnapshot>, _>("payload").0,
        }))
    }

    async fn activate_config_snapshot(
        &self,
        config_snapshot_id: &str,
    ) -> Result<Option<ConfigSnapshotResponse>> {
        let row = sqlx::query("SELECT payload FROM config_snapshots WHERE config_snapshot_id = $1")
            .bind(config_snapshot_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut snapshot = row.get::<Json<ConfigSnapshot>, _>("payload").0;
        let mut tx = self.pool.begin().await?;
        let sibling_rows = sqlx::query(
            "SELECT config_snapshot_id, payload
             FROM config_snapshots
             WHERE tenant_id = $1 AND project_id = $2",
        )
        .bind(snapshot.tenant_id.as_str())
        .bind(snapshot.project_id.as_str())
        .fetch_all(&mut *tx)
        .await?;

        for row in sibling_rows {
            let sibling_id = row.get::<String, _>("config_snapshot_id");
            if sibling_id == config_snapshot_id {
                continue;
            }
            let mut sibling = row.get::<Json<ConfigSnapshot>, _>("payload").0;
            if sibling.status == ConfigSnapshotStatus::Active {
                sibling.status = ConfigSnapshotStatus::Superseded;
                sqlx::query(
                    "UPDATE config_snapshots
                     SET status = 'superseded', payload = $2
                     WHERE config_snapshot_id = $1",
                )
                .bind(&sibling_id)
                .bind(Json(sibling))
                .execute(&mut *tx)
                .await?;
            }
        }

        snapshot.status = ConfigSnapshotStatus::Active;
        snapshot.activated_at = Some(now_rfc3339());
        sqlx::query(
            "UPDATE config_snapshots SET status = 'active', payload = $2 WHERE config_snapshot_id = $1",
        )
        .bind(config_snapshot_id)
        .bind(Json(snapshot.clone()))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO active_config_pointers (pointer_key, config_snapshot_id)
             VALUES ('default', $1)
             ON CONFLICT (pointer_key) DO UPDATE SET config_snapshot_id = EXCLUDED.config_snapshot_id",
        )
        .bind(config_snapshot_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(ConfigSnapshotResponse {
            config_snapshot: snapshot,
        }))
    }

    async fn simulate_route(
        &self,
        request: RouteSimulationRequest,
    ) -> Result<RouteSimulationResponse> {
        let provider_resources = self
            .list_provider_resources(&ProviderResourceFilters::default())
            .await?
            .data
            .into_iter()
            .filter(|resource| resource.tenant_id == request.tenant_id)
            .collect::<Vec<_>>();
        let policies = self
            .list_route_policies()
            .await?
            .data
            .into_iter()
            .filter(|policy| policy.tenant_id == request.tenant_id)
            .collect::<Vec<_>>();
        let active_snapshot = self
            .get_active_project_snapshot(&request.tenant_id, &request.project_id)
            .await?
            .context("active config snapshot missing for project")?;
        build_route_simulation_response(&provider_resources, &policies, &active_snapshot, &request)
    }

    async fn get_route_receipt(
        &self,
        route_receipt_id: &str,
    ) -> Result<Option<RouteReceiptResponse>> {
        let row = sqlx::query("SELECT payload FROM route_receipts WHERE route_receipt_id = $1")
            .bind(route_receipt_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| RouteReceiptResponse {
            route_receipt: row.get::<Json<RouteReceipt>, _>("payload").0,
        }))
    }

    async fn get_route_receipt_diagnostics(
        &self,
        route_receipt_id: &str,
    ) -> Result<Option<RouteReceiptDiagnosticsResponse>> {
        let row = sqlx::query(
            r"
            SELECT
                receipts.payload AS route_receipt,
                diagnostics.decision_timeline,
                diagnostics.policy_checks,
                diagnostics.provider_attempts,
                diagnostics.source_message_id,
                diagnostics.source_producer,
                diagnostics.source_request_id,
                diagnostics.source_trace_id,
                diagnostics.created_at::text AS diagnostics_created_at
            FROM route_receipts receipts
            LEFT JOIN route_receipt_diagnostics diagnostics
              ON diagnostics.route_receipt_id = receipts.route_receipt_id
            WHERE receipts.route_receipt_id = $1
            ",
        )
        .bind(route_receipt_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| {
            let route_receipt = row.get::<Json<RouteReceipt>, _>("route_receipt").0;
            let mut metadata = BTreeMap::new();
            for (key, value) in [
                (
                    "source_message_id",
                    row.get::<Option<String>, _>("source_message_id"),
                ),
                (
                    "source_producer",
                    row.get::<Option<String>, _>("source_producer"),
                ),
                (
                    "source_request_id",
                    row.get::<Option<String>, _>("source_request_id"),
                ),
                (
                    "source_trace_id",
                    row.get::<Option<String>, _>("source_trace_id"),
                ),
                (
                    "diagnostics_created_at",
                    row.get::<Option<String>, _>("diagnostics_created_at"),
                ),
            ] {
                if let Some(value) = value {
                    metadata.insert(key.to_string(), value);
                }
            }

            RouteReceiptDiagnosticsResponse {
                route_receipt,
                decision_timeline: row
                    .get::<Option<Json<Vec<RouteReceiptDecisionTraceStep>>>, _>("decision_timeline")
                    .map_or_else(Vec::new, |value| value.0),
                policy_checks: row
                    .get::<Option<Json<Vec<RouteReceiptPolicyCheck>>>, _>("policy_checks")
                    .map_or_else(Vec::new, |value| value.0),
                provider_attempts: row
                    .get::<Option<Json<Vec<RouteReceiptProviderAttempt>>>, _>("provider_attempts")
                    .map_or_else(Vec::new, |value| value.0),
                metadata,
            }
        }))
    }

    async fn list_route_receipts(
        &self,
        filters: &RouteReceiptFilters,
    ) -> Result<RouteReceiptsResponse> {
        let rows = sqlx::query("SELECT payload FROM route_receipts ORDER BY route_receipt_id")
            .fetch_all(&self.pool)
            .await?;
        let mut receipts = rows
            .into_iter()
            .map(|row| row.get::<Json<RouteReceipt>, _>("payload").0)
            .filter(|receipt| route_receipt_matches_filters(receipt, filters))
            .collect::<Vec<_>>();
        receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        if let Some(limit) = filters.limit {
            receipts.truncate(limit);
        }
        Ok(RouteReceiptsResponse { data: receipts })
    }

    async fn get_route_diagnostics(
        &self,
        route_policy_id: &str,
    ) -> Result<Option<RouteDiagnosticsResponse>> {
        let provider_resources = self
            .list_provider_resources(&ProviderResourceFilters::default())
            .await?
            .data;
        let route_policies = self.list_route_policies().await?.data;
        let config_snapshots = self.list_config_snapshots().await?.data;
        let route_receipts = self
            .list_route_receipts(&RouteReceiptFilters::default())
            .await?
            .data;
        Ok(build_route_diagnostics_response(
            &provider_resources,
            &route_policies,
            &config_snapshots,
            &route_receipts,
            route_policy_id,
            self.active_snapshot_id().await?.as_deref(),
        ))
    }

    async fn get_usage_summary(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        owner_account_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
    ) -> Result<UsageSummaryResponse> {
        let window_start_value = window_start.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let window_end_value = window_end.unwrap_or_else(now_rfc3339);
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(event_count), 0)::BIGINT AS event_count,
                COALESCE(SUM(input_tokens), 0)::BIGINT AS input_tokens,
                COALESCE(SUM(output_tokens), 0)::BIGINT AS output_tokens,
                COALESCE(SUM(cached_input_tokens), 0)::BIGINT AS cached_input_tokens,
                COALESCE(SUM(provider_cost_micros), 0)::BIGINT AS provider_cost_micros,
                COALESCE(SUM(billable_cost_micros), 0)::BIGINT AS billable_cost_micros,
                COALESCE(MIN(currency), 'USD') AS currency
            FROM usage_daily_projections
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR project_id = $2)
              AND ($3::text IS NULL OR owner_account_id = $3)
              AND usage_date >= DATE($4::timestamptz)
              AND usage_date <= DATE($5::timestamptz)
            "#,
        )
        .bind(tenant_id)
        .bind(project_id.as_deref())
        .bind(owner_account_id.as_deref())
        .bind(&window_start_value)
        .bind(&window_end_value)
        .fetch_one(&self.pool)
        .await?;

        let currency = row.get::<String, _>("currency");
        Ok(UsageSummaryResponse {
            data: UsageSummary {
                tenant_id: TenantId::parse(tenant_id.to_string())?,
                project_id: project_id.map(ProjectId::parse).transpose()?,
                owner_account_id,
                window_start: window_start_value,
                window_end: window_end_value,
                currency: currency.clone(),
                event_count: u64::try_from(row.get::<i64, _>("event_count")).unwrap_or_default(),
                input_tokens: u64::try_from(row.get::<i64, _>("input_tokens")).unwrap_or_default(),
                output_tokens: u64::try_from(row.get::<i64, _>("output_tokens"))
                    .unwrap_or_default(),
                cached_input_tokens: u64::try_from(row.get::<i64, _>("cached_input_tokens"))
                    .unwrap_or_default(),
                provider_cost: format_monetary_amount(
                    &currency,
                    row.get::<i64, _>("provider_cost_micros"),
                ),
                billable_price: format_monetary_amount(
                    &currency,
                    row.get::<i64, _>("billable_cost_micros"),
                ),
            },
        })
    }

    async fn get_usage_breakdown(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        owner_account_id: Option<String>,
        window_start: Option<String>,
        window_end: Option<String>,
        group_by: UsageBreakdownGroupBy,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<UsageBreakdownResponse> {
        let offset = parse_cursor_offset(cursor.as_deref());
        let limit = limit.unwrap_or(50).max(1);
        let window_start_value = window_start.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let window_end_value = window_end.unwrap_or_else(now_rfc3339);
        let (bucket_select, provider_select, model_select, group_expr) = match group_by {
            UsageBreakdownGroupBy::Provider => (
                "provider_id AS bucket",
                "provider_id AS provider_id",
                "NULL::text AS model_alias",
                "provider_id",
            ),
            UsageBreakdownGroupBy::Model => (
                "model_alias AS bucket",
                "NULL::text AS provider_id",
                "model_alias AS model_alias",
                "model_alias",
            ),
            UsageBreakdownGroupBy::Day => (
                "TO_CHAR(usage_date, 'YYYY-MM-DD') AS bucket",
                "NULL::text AS provider_id",
                "NULL::text AS model_alias",
                "usage_date",
            ),
        };
        let sql = format!(
            r#"
            SELECT
                {bucket_select},
                {provider_select},
                {model_select},
                COALESCE(SUM(input_tokens), 0)::BIGINT AS input_tokens,
                COALESCE(SUM(output_tokens), 0)::BIGINT AS output_tokens,
                COALESCE(SUM(cached_input_tokens), 0)::BIGINT AS cached_input_tokens,
                COALESCE(SUM(provider_cost_micros), 0)::BIGINT AS provider_cost_micros,
                COALESCE(SUM(billable_cost_micros), 0)::BIGINT AS billable_cost_micros,
                COALESCE(MIN(currency), 'USD') AS currency
            FROM usage_daily_projections
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR project_id = $2)
              AND ($3::text IS NULL OR owner_account_id = $3)
              AND usage_date >= DATE($4::timestamptz)
              AND usage_date <= DATE($5::timestamptz)
            GROUP BY {group_expr}
            ORDER BY {group_expr}
            OFFSET $6 LIMIT $7
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(tenant_id)
            .bind(project_id.as_deref())
            .bind(owner_account_id.as_deref())
            .bind(&window_start_value)
            .bind(&window_end_value)
            .bind(i64::try_from(offset).unwrap_or(i64::MAX))
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?;

        let data = rows
            .iter()
            .map(|row| {
                let currency = row.get::<String, _>("currency");
                UsageBreakdownRow {
                    bucket: row.get::<String, _>("bucket"),
                    provider_id: row.get::<Option<String>, _>("provider_id"),
                    model_alias: row.get::<Option<String>, _>("model_alias"),
                    input_tokens: u64::try_from(row.get::<i64, _>("input_tokens"))
                        .unwrap_or_default(),
                    output_tokens: u64::try_from(row.get::<i64, _>("output_tokens"))
                        .unwrap_or_default(),
                    cached_input_tokens: u64::try_from(row.get::<i64, _>("cached_input_tokens"))
                        .unwrap_or_default(),
                    provider_cost: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("provider_cost_micros"),
                    ),
                    billable_price: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("billable_cost_micros"),
                    ),
                }
            })
            .collect::<Vec<_>>();

        let next_cursor = if data.len() == usize::try_from(limit).unwrap_or(usize::MAX) {
            Some((offset + data.len()).to_string())
        } else {
            None
        };

        Ok(UsageBreakdownResponse { data, next_cursor })
    }

    async fn get_pricing_catalog(&self) -> Result<PricingCatalogResponse> {
        Ok(pricing_catalog_response(self.load_pricing_catalog().await?))
    }

    async fn create_pricing_simulation(
        &self,
        request: &PricingSimulationRequest,
    ) -> Result<PricingSimulationResponse> {
        let catalog = self.load_pricing_catalog().await?;
        Ok(simulate_pricing_with_catalog(&catalog, request))
    }

    async fn load_pricing_catalog(&self) -> Result<PricingCatalog> {
        load_pricing_catalog(&self.pool).await
    }

    async fn get_balance_projection(
        &self,
        tenant_id: &str,
        project_id: Option<String>,
        owner_account_id: Option<String>,
    ) -> Result<BalanceProjectionResponse> {
        let row = sqlx::query(
            r#"
            SELECT
                currency,
                COALESCE(SUM(provider_cost_micros), 0)::BIGINT AS provider_cost_micros,
                COALESCE(SUM(billable_cost_micros), 0)::BIGINT AS billable_cost_micros,
                COALESCE(SUM(configured_budget_micros), 0)::BIGINT AS configured_budget_micros,
                COALESCE(SUM(remaining_budget_micros), 0)::BIGINT AS remaining_budget_micros,
                CASE
                    WHEN BOOL_OR(threshold_status = 'exceeded') THEN 'exceeded'
                    WHEN BOOL_OR(threshold_status = 'warning') THEN 'warning'
                    ELSE 'ok'
                END AS threshold_status,
                to_char(
                    MAX(last_projected_at) AT TIME ZONE 'UTC',
                    'YYYY-MM-DD"T"HH24:MI:SS.US"Z"'
                ) AS last_projected_at
            FROM balance_projections
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR project_id = $2)
              AND ($3::text IS NULL OR owner_account_id = $3)
            GROUP BY currency
            ORDER BY MAX(last_projected_at) DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(project_id.as_deref())
        .bind(owner_account_id.as_deref())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let currency = row.get::<String, _>("currency");
            let last_projected_at = row.get::<String, _>("last_projected_at");
            let lag = projection_lag_seconds(&last_projected_at);
            Ok(BalanceProjectionResponse {
                data: BalanceProjection {
                    tenant_id: TenantId::parse(tenant_id.to_string())?,
                    project_id: project_id.map(ProjectId::parse).transpose()?,
                    owner_account_id,
                    currency: currency.clone(),
                    provider_cost_total: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("provider_cost_micros"),
                    ),
                    billable_total: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("billable_cost_micros"),
                    ),
                    configured_budget: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("configured_budget_micros"),
                    ),
                    remaining_budget: format_monetary_amount(
                        &currency,
                        row.get::<i64, _>("remaining_budget_micros"),
                    ),
                    threshold_status: row.get::<String, _>("threshold_status"),
                    last_projected_at,
                    projection_lag_seconds: lag,
                },
            })
        } else {
            let currency = "USD".to_string();
            let configured_budget_micros = default_budget_micros_for_scope(
                tenant_id,
                project_id.as_deref().unwrap_or("project"),
                owner_account_id.as_deref(),
            );
            Ok(BalanceProjectionResponse {
                data: BalanceProjection {
                    tenant_id: TenantId::parse(tenant_id.to_string())?,
                    project_id: project_id.map(ProjectId::parse).transpose()?,
                    owner_account_id,
                    currency: currency.clone(),
                    provider_cost_total: format_monetary_amount(&currency, 0),
                    billable_total: format_monetary_amount(&currency, 0),
                    configured_budget: format_monetary_amount(&currency, configured_budget_micros),
                    remaining_budget: format_monetary_amount(&currency, configured_budget_micros),
                    threshold_status: "ok".to_string(),
                    last_projected_at: now_rfc3339(),
                    projection_lag_seconds: 0,
                },
            })
        }
    }

    async fn create_billing_export(
        &self,
        request: BillingExportRequest,
    ) -> Result<BillingExportJobResponse> {
        let export_job_id = format!("export_{}", OffsetDateTime::now_utc().unix_timestamp());
        let requested_at = now_rfc3339();
        let export_content = render_billing_export_csv(
            self,
            request
                .tenant_id
                .as_ref()
                .map(core_domain::TenantId::as_str),
            request
                .project_id
                .as_ref()
                .map(core_domain::ProjectId::as_str),
            &request.window_start,
            &request.window_end,
        )
        .await?;
        sqlx::query(
            r#"
            INSERT INTO billing_export_jobs (
                export_job_id,
                tenant_id,
                project_id,
                window_start,
                window_end,
                format,
                status,
                requested_at,
                export_content,
                content_type
            ) VALUES ($1, $2, $3, $4, $5, $6, 'queued', $7, $8, 'text/csv')
            "#,
        )
        .bind(&export_job_id)
        .bind(
            request
                .tenant_id
                .as_ref()
                .map(core_domain::TenantId::as_str),
        )
        .bind(
            request
                .project_id
                .as_ref()
                .map(core_domain::ProjectId::as_str),
        )
        .bind(&request.window_start)
        .bind(&request.window_end)
        .bind(&request.format)
        .bind(&requested_at)
        .bind(&export_content)
        .execute(&self.pool)
        .await?;

        Ok(BillingExportJobResponse {
            data: BillingExportJob {
                export_job_id,
                status: "queued".to_string(),
                format: request.format,
                requested_at,
                completed_at: None,
                error_message: None,
                tenant_id: request.tenant_id,
                project_id: request.project_id,
            },
        })
    }

    async fn list_billing_exports(
        &self,
        tenant_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<BillingExportJobsResponse> {
        let rows = sqlx::query(
            r#"
            SELECT
                export_job_id,
                tenant_id,
                project_id,
                format,
                status,
                requested_at,
                completed_at,
                error_message,
                export_content
            FROM billing_export_jobs
            ORDER BY requested_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(maybe_complete_postgres_export_job(&self.pool, row).await?);
        }

        Ok(BillingExportJobsResponse {
            data: filter_billing_export_jobs(jobs, tenant_id, project_id),
        })
    }

    async fn get_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<BillingExportJobResponse>> {
        let row = sqlx::query(
            r#"
            SELECT
                export_job_id,
                tenant_id,
                project_id,
                format,
                status,
                requested_at,
                completed_at,
                error_message,
                export_content
            FROM billing_export_jobs
            WHERE export_job_id = $1
            "#,
        )
        .bind(export_job_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(BillingExportJobResponse {
            data: maybe_complete_postgres_export_job(&self.pool, row).await?,
        }))
    }

    async fn download_billing_export(
        &self,
        export_job_id: &str,
    ) -> Result<Option<(String, String)>> {
        let row = sqlx::query(
            r#"
            SELECT content_type, export_content
            FROM billing_export_jobs
            WHERE export_job_id = $1
            "#,
        )
        .bind(export_job_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let content = row.get::<Option<String>, _>("export_content");
        let content_type = row.get::<Option<String>, _>("content_type");
        Ok(content.map(|value| {
            (
                content_type.unwrap_or_else(|| "text/plain".to_string()),
                value,
            )
        }))
    }

    async fn create_wechat_payment_order(
        &self,
        record: WechatPaymentOrderRecord,
    ) -> Result<WechatPaymentOrderResponse> {
        sqlx::query(
            r#"
            INSERT INTO wechat_payment_orders (
                out_trade_no,
                tenant_id,
                project_id,
                amount_total,
                currency,
                channel,
                status,
                trade_state,
                code_url,
                prepay_id,
                transaction_id,
                notification_id,
                created_at,
                updated_at,
                expires_at,
                paid_at,
                payload
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17
            )
            "#,
        )
        .bind(&record.out_trade_no)
        .bind(&record.tenant_id)
        .bind(record.project_id.as_deref())
        .bind(i64::from(record.amount_total))
        .bind(&record.currency)
        .bind(&record.channel)
        .bind(&record.status)
        .bind(record.trade_state.as_deref())
        .bind(record.code_url.as_deref())
        .bind(record.prepay_id.as_deref())
        .bind(record.transaction_id.as_deref())
        .bind(record.notification_id.as_deref())
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.expires_at)
        .bind(record.paid_at.as_deref())
        .bind(Json(&record))
        .execute(&self.pool)
        .await?;

        Ok(WechatPaymentOrderResponse { data: record })
    }

    async fn get_wechat_payment_order(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<WechatPaymentOrderResponse>> {
        let row = sqlx::query("SELECT payload FROM wechat_payment_orders WHERE out_trade_no = $1")
            .bind(out_trade_no)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|row| WechatPaymentOrderResponse {
            data: row.get::<Json<WechatPaymentOrderRecord>, _>("payload").0,
        }))
    }

    async fn update_wechat_payment_order(
        &self,
        record: WechatPaymentOrderRecord,
    ) -> Result<WechatPaymentOrderResponse> {
        sqlx::query(
            r#"
            UPDATE wechat_payment_orders
               SET status = $2,
                   trade_state = $3,
                   code_url = $4,
                   prepay_id = $5,
                   transaction_id = $6,
                   notification_id = $7,
                   updated_at = $8,
                   paid_at = $9,
                   payload = $10
             WHERE out_trade_no = $1
            "#,
        )
        .bind(&record.out_trade_no)
        .bind(&record.status)
        .bind(record.trade_state.as_deref())
        .bind(record.code_url.as_deref())
        .bind(record.prepay_id.as_deref())
        .bind(record.transaction_id.as_deref())
        .bind(record.notification_id.as_deref())
        .bind(&record.updated_at)
        .bind(record.paid_at.as_deref())
        .bind(Json(&record))
        .execute(&self.pool)
        .await?;

        Ok(WechatPaymentOrderResponse { data: record })
    }

    async fn create_alipay_payment_order(
        &self,
        record: AlipayPaymentOrderRecord,
    ) -> Result<AlipayPaymentOrderResponse> {
        sqlx::query(
            r#"
            INSERT INTO alipay_payment_orders (
                out_trade_no, tenant_id, project_id, amount_total, currency, channel,
                status, trade_state, code_url, prepay_id, transaction_id, notification_id,
                created_at, updated_at, expires_at, paid_at, payload
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17
            )
            "#,
        )
        .bind(&record.out_trade_no)
        .bind(&record.tenant_id)
        .bind(record.project_id.as_deref())
        .bind(i64::from(record.amount_total))
        .bind(&record.currency)
        .bind(&record.channel)
        .bind(&record.status)
        .bind(record.trade_state.as_deref())
        .bind(record.code_url.as_deref())
        .bind(record.prepay_id.as_deref())
        .bind(record.transaction_id.as_deref())
        .bind(record.notification_id.as_deref())
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.expires_at)
        .bind(record.paid_at.as_deref())
        .bind(Json(&record))
        .execute(&self.pool)
        .await?;

        Ok(AlipayPaymentOrderResponse { data: record })
    }

    async fn get_alipay_payment_order(
        &self,
        out_trade_no: &str,
    ) -> Result<Option<AlipayPaymentOrderResponse>> {
        let row = sqlx::query("SELECT payload FROM alipay_payment_orders WHERE out_trade_no = $1")
            .bind(out_trade_no)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|row| AlipayPaymentOrderResponse {
            data: row.get::<Json<AlipayPaymentOrderRecord>, _>("payload").0,
        }))
    }

    async fn update_alipay_payment_order(
        &self,
        record: AlipayPaymentOrderRecord,
    ) -> Result<AlipayPaymentOrderResponse> {
        sqlx::query(
            r#"
            UPDATE alipay_payment_orders
               SET status = $2,
                   trade_state = $3,
                   code_url = $4,
                   prepay_id = $5,
                   transaction_id = $6,
                   notification_id = $7,
                   updated_at = $8,
                   paid_at = $9,
                   payload = $10
             WHERE out_trade_no = $1
            "#,
        )
        .bind(&record.out_trade_no)
        .bind(&record.status)
        .bind(record.trade_state.as_deref())
        .bind(record.code_url.as_deref())
        .bind(record.prepay_id.as_deref())
        .bind(record.transaction_id.as_deref())
        .bind(record.notification_id.as_deref())
        .bind(&record.updated_at)
        .bind(record.paid_at.as_deref())
        .bind(Json(&record))
        .execute(&self.pool)
        .await?;

        Ok(AlipayPaymentOrderResponse { data: record })
    }

    async fn create_renewal_intent(
        &self,
        draft: RenewalIntentDraft,
    ) -> Result<RenewalIntentCreateResult> {
        let mut tx = self.pool.begin().await?;
        let order_row = sqlx::query(
            "SELECT payload FROM wechat_payment_orders WHERE out_trade_no = $1 FOR UPDATE",
        )
        .bind(&draft.out_trade_no)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(order_row) = order_row else {
            tx.commit().await?;
            return Ok(RenewalIntentCreateResult::OrderNotFound);
        };
        let order = order_row
            .get::<Json<WechatPaymentOrderRecord>, _>("payload")
            .0;

        let grant_row =
            sqlx::query("SELECT payload FROM opening_grants WHERE grant_id = $1 FOR UPDATE")
                .bind(&draft.grant_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(grant_row) = grant_row else {
            tx.commit().await?;
            return Ok(RenewalIntentCreateResult::GrantNotFound);
        };
        let mut grant = grant_row.get::<Json<OpeningGrantRecord>, _>("payload").0;
        if !renewal_order_matches_grant(&order, &grant) {
            tx.commit().await?;
            return Ok(RenewalIntentCreateResult::GrantMismatch);
        }

        let intent = build_renewal_intent(&draft, &order, &mut grant);
        if intent.status == "renewed" {
            sqlx::query(
                "UPDATE opening_grants
                    SET status = $2, expires_at = $3, payload = $4, updated_at = $5
                  WHERE grant_id = $1",
            )
            .bind(&grant.grant_id)
            .bind(&grant.status)
            .bind(&grant.expires_at)
            .bind(Json(&grant))
            .bind(&grant.updated_at)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO billing_renewal_intents
                (renewal_intent_id, out_trade_no, grant_id, tenant_id, project_id, status, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&intent.renewal_intent_id)
        .bind(&intent.out_trade_no)
        .bind(&intent.grant_id)
        .bind(intent.tenant_id.as_str())
        .bind(intent.project_id.as_str())
        .bind(&intent.status)
        .bind(Json(&intent))
        .bind(&intent.created_at)
        .bind(&intent.updated_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(RenewalIntentCreateResult::Created(intent))
    }

    async fn list_renewal_intents(
        &self,
        filters: RenewalIntentFilters,
    ) -> Result<RenewalIntentsResponse> {
        let rows =
            sqlx::query("SELECT payload FROM billing_renewal_intents ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?;
        let intents = rows
            .into_iter()
            .map(|row| row.get::<Json<RenewalIntent>, _>("payload").0)
            .collect::<Vec<_>>();
        Ok(RenewalIntentsResponse {
            data: filter_renewal_intents(intents, &filters),
        })
    }

    async fn lookup_user(&self, identity_key: &IdentityLookup) -> Result<Option<UserIdentity>> {
        let row = match identity_key {
            IdentityLookup::Email(email) => {
                sqlx::query(
                    "SELECT payload FROM users WHERE lower(primary_email) = lower($1) LIMIT 1",
                )
                .bind(email)
                .fetch_optional(&self.pool)
                .await?
            }
            IdentityLookup::ProviderSubject(provider, subject) => {
                sqlx::query(
                    "SELECT u.payload AS payload
                     FROM auth_provider_links apl
                     JOIN users u ON u.user_id = apl.user_id
                     WHERE apl.provider = $1 AND apl.provider_subject = $2
                     LIMIT 1",
                )
                .bind(auth_provider_slug(*provider))
                .bind(subject)
                .fetch_optional(&self.pool)
                .await?
            }
        };
        Ok(row.map(|row| row.get::<Json<UserIdentity>, _>("payload").0))
    }

    async fn list_memberships(&self, user_id: &UserId) -> Result<Vec<TenantMembership>> {
        let rows = sqlx::query(
            "SELECT payload FROM tenant_memberships WHERE user_id = $1 ORDER BY tenant_id",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<TenantMembership>, _>("payload").0)
            .collect())
    }

    async fn list_links(&self, user_id: &UserId) -> Result<Vec<AuthProviderLink>> {
        let rows = sqlx::query(
            "SELECT payload FROM auth_provider_links WHERE user_id = $1 ORDER BY provider",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.get::<Json<AuthProviderLink>, _>("payload").0)
            .collect())
    }

    async fn load_user(&self, user_id: &UserId) -> Result<Option<UserIdentity>> {
        let row = sqlx::query("SELECT payload FROM users WHERE user_id = $1 LIMIT 1")
            .bind(user_id.as_str())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<UserIdentity>, _>("payload").0))
    }

    async fn revoke_sessions_for_user(&self, user_id: &UserId) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn upsert_federated_user(
        &self,
        provider: AuthProvider,
        subject: &str,
        email: Option<&str>,
        display_name: Option<&str>,
        workspace_slug: &str,
        role: TenantMembershipRole,
        now: &str,
    ) -> Result<UserIdentity> {
        let row = sqlx::query(
            "SELECT payload FROM tenants
             WHERE payload->>'slug' = $1 OR tenant_id = $1 OR ($1 = 'acme' AND tenant_id = 'tenant_acme')
             LIMIT 1",
        )
            .bind(workspace_slug)
            .fetch_optional(&self.pool)
            .await?
            .context("workspace not found for provider login")?;
        let tenant = row.get::<Json<Tenant>, _>("payload").0;

        let digest =
            Sha256::digest(format!("{}:{subject}", auth_provider_slug(provider)).as_bytes());
        let subject_hash = digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let user_id = format!("user_{subject_hash}");
        let primary_email = email.map_or_else(
            || format!("{subject}@{}.login.local", auth_provider_slug(provider)),
            str::to_string,
        );
        let user = UserIdentity {
            user_id: UserId::parse(user_id.clone()).unwrap(),
            primary_email: Some(primary_email.clone()),
            display_name: display_name.map_or_else(
                || format!("{} user", auth_provider_slug(provider)),
                str::to_string,
            ),
            avatar_url: None,
            created_at: now.to_string(),
            last_login_at: Some(now.to_string()),
        };

        sqlx::query(
            "INSERT INTO users (user_id, primary_email, payload) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET primary_email = EXCLUDED.primary_email, payload = EXCLUDED.payload",
        )
        .bind(&user_id)
        .bind(&primary_email)
        .bind(Json(user.clone()))
        .execute(&self.pool)
        .await?;

        let membership = membership(
            &format!("tmemb_{}_{}", auth_provider_slug(provider), subject_hash),
            &tenant,
            role,
        );
        sqlx::query(
            "INSERT INTO tenant_memberships (membership_id, user_id, tenant_id, payload)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (membership_id) DO UPDATE SET user_id = EXCLUDED.user_id, tenant_id = EXCLUDED.tenant_id, payload = EXCLUDED.payload",
        )
        .bind(membership.membership_id.as_str().to_string())
        .bind(&user_id)
        .bind(membership.tenant.id.as_str())
        .bind(Json(membership.clone()))
        .execute(&self.pool)
        .await?;

        let link = link(
            provider,
            subject,
            Some(&primary_email),
            provider != AuthProvider::Oidc,
        );
        sqlx::query(
            "INSERT INTO auth_provider_links (link_id, user_id, provider, provider_subject, email, can_unlink, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (link_id) DO UPDATE SET
               user_id = EXCLUDED.user_id,
               provider = EXCLUDED.provider,
               provider_subject = EXCLUDED.provider_subject,
               email = EXCLUDED.email,
               can_unlink = EXCLUDED.can_unlink,
               payload = EXCLUDED.payload",
        )
        .bind(link.link_id.as_str().to_string())
        .bind(&user_id)
        .bind(auth_provider_slug(provider))
        .bind(subject)
        .bind(Some(primary_email.as_str()))
        .bind(provider != AuthProvider::Oidc)
        .bind(Json(link.clone()))
        .execute(&self.pool)
        .await?;

        self.revoke_sessions_for_user(&UserId::parse(user_id).unwrap())
            .await?;

        Ok(user)
    }

    async fn get_active_project_snapshot(
        &self,
        tenant_id: &TenantId,
        project_id: &ProjectId,
    ) -> Result<Option<ConfigSnapshot>> {
        let row = sqlx::query(
            "SELECT payload
             FROM config_snapshots
             WHERE tenant_id = $1 AND project_id = $2 AND status = 'active'
             ORDER BY activated_at DESC NULLS LAST, config_snapshot_id DESC
             LIMIT 1",
        )
        .bind(tenant_id.as_str())
        .bind(project_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get::<Json<ConfigSnapshot>, _>("payload").0))
    }
}

#[derive(Debug, Clone)]
pub enum IdentityLookup {
    Email(String),
    ProviderSubject(AuthProvider, String),
}

fn issue_memory_session(
    store: &mut MemoryStore,
    session_id: &str,
    provider: AuthProvider,
    identity_key: &IdentityLookup,
    workspace_slug: &str,
    now: &str,
    expires_at: &str,
) -> Result<AuthLoginResult> {
    let user_id = match identity_key {
        IdentityLookup::Email(email) => store
            .email_identity_to_user_id
            .get(&email.to_lowercase())
            .cloned(),
        IdentityLookup::ProviderSubject(provider, subject) => store
            .provider_subject_to_user_id
            .get(&format!("{}:{}", auth_provider_slug(*provider), subject))
            .cloned(),
    };
    let user_id = match user_id {
        Some(user_id) => user_id,
        None if provider == AuthProvider::Oidc => match identity_key {
            IdentityLookup::ProviderSubject(AuthProvider::Oidc, subject) => {
                upsert_memory_oidc_user(
                    store,
                    subject,
                    None,
                    None,
                    workspace_slug,
                    if workspace_slug == "platform-admin" {
                        TenantMembershipRole::Admin
                    } else {
                        TenantMembershipRole::Member
                    },
                    now,
                )?
            }
            _ => anyhow::bail!("identity not found"),
        },
        None => anyhow::bail!("identity not found"),
    };

    let user = store.users.get(&user_id).cloned().context("missing user")?;
    let memberships = store
        .memberships_by_user
        .get(&user_id)
        .cloned()
        .unwrap_or_default();
    let active_membership = memberships
        .iter()
        .find(|membership| tenant_summary_matches_workspace(&membership.tenant, workspace_slug))
        .or_else(|| memberships.first())
        .cloned()
        .context("no tenant memberships")?;
    let links = store
        .provider_links_by_user
        .get(&user_id)
        .cloned()
        .unwrap_or_default();

    let session = AuthSession {
        session_id: AuthSessionId::parse(session_id.to_string()).unwrap(),
        state: AuthSessionState::Active,
        user,
        active_tenant_id: Some(active_membership.tenant.id),
        memberships,
        authenticated_by: provider,
        created_at: now.to_string(),
        expires_at: expires_at.to_string(),
        last_authenticated_at: now.to_string(),
    };
    store.sessions.insert(
        session_id.to_string(),
        StoredSession {
            session: session.clone(),
            links: links.clone(),
        },
    );

    Ok(AuthLoginResult {
        session,
        links,
        redirect_to: None,
    })
}

fn refresh_memory_session(store: &MemoryStore, stored: StoredSession) -> AuthLoginResult {
    let user_id = stored.session.user.user_id.as_str();
    let memberships = store
        .memberships_by_user
        .get(user_id)
        .cloned()
        .unwrap_or_else(|| stored.session.memberships.clone());
    let links = store
        .provider_links_by_user
        .get(user_id)
        .cloned()
        .unwrap_or_else(|| stored.links.clone());

    AuthLoginResult {
        session: AuthSession {
            active_tenant_id: resolve_active_tenant_id(
                stored.session.active_tenant_id.as_ref(),
                &memberships,
            ),
            memberships,
            ..stored.session
        },
        links,
        redirect_to: None,
    }
}

fn upsert_memory_oidc_user(
    store: &mut MemoryStore,
    subject: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    workspace_slug: &str,
    role: TenantMembershipRole,
    now: &str,
) -> Result<String> {
    let digest = Sha256::digest(subject.as_bytes());
    let subject_hash = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let user_id = format!("user_{subject_hash}");

    if store.users.contains_key(&user_id) {
        return Ok(user_id);
    }

    let tenant = store
        .tenants
        .iter()
        .find(|tenant| tenant_matches_workspace(tenant, workspace_slug))
        .cloned()
        .context("workspace not found for oidc login")?;
    let primary_email = email
        .map(str::to_string)
        .or_else(|| {
            std::env::var("CONTROL_PLANE_OIDC_EMAIL")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| format!("{subject}@enterprise.example"));
    let user = UserIdentity {
        user_id: UserId::parse(user_id.clone()).unwrap(),
        primary_email: Some(primary_email.clone()),
        display_name: display_name
            .map(str::to_string)
            .or_else(|| {
                std::env::var("CONTROL_PLANE_OIDC_DISPLAY_NAME")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| "Enterprise SSO User".to_string()),
        avatar_url: None,
        created_at: now.to_string(),
        last_login_at: Some(now.to_string()),
    };
    store.users.insert(user_id.clone(), user);
    store
        .email_identity_to_user_id
        .insert(primary_email.to_lowercase(), user_id.clone());
    store.provider_subject_to_user_id.insert(
        format!("{}:{}", auth_provider_slug(AuthProvider::Oidc), subject),
        user_id.clone(),
    );
    let membership = membership(&format!("tmemb_oidc_{subject_hash}"), &tenant, role);
    store
        .memberships_by_user
        .entry(user_id.clone())
        .or_default()
        .push(membership);
    store.provider_links_by_user.insert(
        user_id.clone(),
        vec![link(
            AuthProvider::Oidc,
            subject,
            Some(&primary_email),
            false,
        )],
    );

    Ok(user_id)
}

fn upsert_memory_provider_user(
    store: &mut MemoryStore,
    provider: AuthProvider,
    subject: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    workspace_slug: &str,
    role: TenantMembershipRole,
    now: &str,
) -> Result<String> {
    if provider == AuthProvider::Oidc {
        return upsert_memory_oidc_user(
            store,
            subject,
            email,
            display_name,
            workspace_slug,
            role,
            now,
        );
    }

    let digest = Sha256::digest(format!("{}:{subject}", auth_provider_slug(provider)).as_bytes());
    let subject_hash = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let user_id = format!("user_{subject_hash}");

    let tenant = store
        .tenants
        .iter()
        .find(|tenant| tenant_matches_workspace(tenant, workspace_slug))
        .cloned()
        .context("workspace not found for oauth login")?;
    let primary_email = email.map_or_else(
        || format!("{subject}@{}.login.local", auth_provider_slug(provider)),
        str::to_string,
    );
    let user = UserIdentity {
        user_id: UserId::parse(user_id.clone()).unwrap(),
        primary_email: Some(primary_email.clone()),
        display_name: display_name.map_or_else(
            || format!("{} user", auth_provider_slug(provider)),
            str::to_string,
        ),
        avatar_url: None,
        created_at: now.to_string(),
        last_login_at: Some(now.to_string()),
    };
    store.users.insert(user_id.clone(), user);
    store
        .email_identity_to_user_id
        .insert(primary_email.to_lowercase(), user_id.clone());
    store.provider_subject_to_user_id.insert(
        format!("{}:{}", auth_provider_slug(provider), subject),
        user_id.clone(),
    );

    let membership = membership(
        &format!("tmemb_{}_{}", auth_provider_slug(provider), subject_hash),
        &tenant,
        role,
    );
    let memberships = store
        .memberships_by_user
        .entry(user_id.clone())
        .or_default();
    if !memberships
        .iter()
        .any(|existing| existing.membership_id == membership.membership_id)
    {
        memberships.push(membership);
    }
    store.provider_links_by_user.insert(
        user_id.clone(),
        vec![link(provider, subject, Some(&primary_email), true)],
    );
    revoke_memory_sessions_for_user(store, &user_id);

    Ok(user_id)
}

fn revoke_memory_sessions_for_user(store: &mut MemoryStore, user_id: &str) {
    store
        .sessions
        .retain(|_, stored| stored.session.user.user_id.as_str() != user_id);
}

fn unlink_memory_provider(
    store: &mut MemoryStore,
    session_id: &str,
    provider: AuthProvider,
) -> Option<UnlinkAuthProviderResponse> {
    let user_id = store
        .sessions
        .get(session_id)?
        .session
        .user
        .user_id
        .as_str()
        .to_string();
    let removed = if provider == AuthProvider::Email {
        false
    } else {
        let before = store.sessions.get(session_id)?.links.len();
        if let Some(session) = store.sessions.get_mut(session_id) {
            session.links.retain(|link| link.provider != provider);
        }
        if let Some(user_links) = store.provider_links_by_user.get_mut(&user_id) {
            user_links.retain(|link| link.provider != provider);
        }
        let removed = store
            .sessions
            .get(session_id)
            .is_some_and(|session| session.links.len() != before);
        if removed {
            revoke_memory_sessions_for_user(store, &user_id);
        }
        removed
    };

    Some(UnlinkAuthProviderResponse { provider, removed })
}

fn update_memory_provider_resource(
    store: &mut MemoryStore,
    provider_resource: &ProviderResource,
    expected_version: u64,
) -> ConcurrencyResult<ProviderResource> {
    let index = store
        .provider_resources
        .iter()
        .position(|item| item.provider_resource_id == provider_resource.provider_resource_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .provider_resources
        .get(index)
        .expect("indexed provider resource should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut updated = provider_resource.clone();
    updated.version = expected_version.saturating_add(1);
    updated.created_at = current.created_at.clone();
    store.provider_resources[index] = updated.clone();
    ConcurrencyResult::Applied(updated)
}

fn disable_memory_provider_resource(
    store: &mut MemoryStore,
    provider_resource_id: &str,
    expected_version: u64,
) -> ConcurrencyResult<ProviderResource> {
    let index = store
        .provider_resources
        .iter()
        .position(|item| item.provider_resource_id.as_str() == provider_resource_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .provider_resources
        .get(index)
        .expect("indexed provider resource should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut disabled = current.clone();
    disabled.version = expected_version.saturating_add(1);
    disabled.status = ProviderResourceStatus::Disabled;
    disabled.updated_at = now_rfc3339();
    store.provider_resources[index] = disabled.clone();
    ConcurrencyResult::Applied(disabled)
}

fn fail_close_new_provider_resource(provider_resource: &mut ProviderResource) {
    if provider_resource.status != ProviderResourceStatus::Active {
        return;
    }

    if matches!(
        provider_resource.health_state,
        HealthState::Healthy | HealthState::Degraded
    ) {
        provider_resource.health_state = HealthState::Quarantined;
    }

    if provider_resource.health_state == HealthState::Quarantined {
        provider_resource
            .quarantine_reason
            .get_or_insert_with(|| PROVIDER_RESOURCE_INTAKE_PENDING_PROBE.to_string());
        provider_resource
            .health_message
            .get_or_insert_with(|| PROVIDER_RESOURCE_INTAKE_PENDING_MESSAGE.to_string());
    }
}

fn update_memory_route_policy(
    store: &mut MemoryStore,
    route_policy: &RoutePolicy,
    expected_version: u64,
) -> ConcurrencyResult<RoutePolicy> {
    let index = store
        .route_policies
        .iter()
        .position(|item| item.route_policy_id == route_policy.route_policy_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .route_policies
        .get(index)
        .expect("indexed route policy should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut updated = route_policy.clone();
    updated.version = expected_version.saturating_add(1);
    updated.created_at = current.created_at.clone();
    store.route_policies[index] = updated.clone();
    ConcurrencyResult::Applied(updated)
}

fn disable_memory_route_policy(
    store: &mut MemoryStore,
    route_policy_id: &str,
    expected_version: u64,
) -> ConcurrencyResult<RoutePolicy> {
    let index = store
        .route_policies
        .iter()
        .position(|item| item.route_policy_id.as_str() == route_policy_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .route_policies
        .get(index)
        .expect("indexed route policy should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let mut disabled = current.clone();
    disabled.version = expected_version.saturating_add(1);
    store.route_policies[index] = disabled.clone();
    store
        .route_policy_disabled_ids
        .insert(route_policy_id.to_string());
    ConcurrencyResult::Applied(disabled)
}

fn insert_memory_api_key(
    store: &mut MemoryStore,
    provider_resource_id: ProviderResourceId,
    display_name: &str,
    key_prefix: &str,
    hash: String,
    now: String,
) -> ApiKey {
    let api_key_id = format!("ak_{}", &hash[..16]);
    let record = ApiKeyRecord {
        api_key_id,
        provider_resource_id,
        display_name: display_name.to_string(),
        key_prefix: key_prefix.to_string(),
        hash,
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    store.api_keys.push(record.clone());
    record.public_view()
}

fn revoke_memory_api_key(
    store: &mut MemoryStore,
    api_key_id: &str,
    expected_version: u64,
) -> ConcurrencyResult<ApiKey> {
    let index = store
        .api_keys
        .iter()
        .position(|item| item.api_key_id == api_key_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .api_keys
        .get(index)
        .expect("indexed api key should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    if current.is_active {
        let mut updated = current.clone();
        updated.version = expected_version.saturating_add(1);
        updated.is_active = false;
        updated.updated_at = now_rfc3339();
        store.api_keys[index] = updated.clone();
        ConcurrencyResult::Applied(updated.public_view())
    } else {
        let mut updated = current.clone();
        updated.version = expected_version.saturating_add(1);
        updated.updated_at = now_rfc3339();
        store.api_keys[index] = updated.clone();
        ConcurrencyResult::Applied(updated.public_view())
    }
}

fn revoke_memory_opening_grant(
    store: &mut MemoryStore,
    grant_id: &str,
    expected_version: u64,
    revoked_by: &str,
) -> ConcurrencyResult<OpeningGrant> {
    let index = store
        .opening_grants
        .iter()
        .position(|item| item.grant_id == grant_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .opening_grants
        .get(index)
        .expect("indexed opening grant should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }

    let now = now_rfc3339();
    let mut updated = current.clone();
    updated.status = "revoked".to_string();
    updated.revoked_at = Some(now.clone());
    updated.revoked_by = Some(revoked_by.to_string());
    updated.updated_at = now;
    updated.version = expected_version.saturating_add(1);
    store.opening_grants[index] = updated.clone();
    ConcurrencyResult::Applied(updated.public_view())
}

fn prepare_delivery_records(
    draft: DeliveryPrepareDraft,
    redemption_code: &str,
    browser_file_unlock_code: &str,
) -> PreparedDeliveryRecords {
    let now = now_rfc3339();
    let delivery = DeliveryRecord {
        delivery_id: draft.delivery_id.clone(),
        tenant_id: draft.tenant_id,
        project_id: draft.project_id,
        owner_account_id: draft.owner_account_id,
        redemption_batch_id: draft.redemption_batch_id,
        provider: draft.provider,
        status: DELIVERY_STATUS_PREPARED.to_string(),
        operator_id: draft.operator_id,
        customer_label: draft.customer_label,
        source: DELIVERY_SOURCE_MANUAL_OPERATOR.to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        version: 1,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
    };
    let codes = vec![
        DeliveryCodeRecord {
            code_id: format!("dlvcode_{}_redemption", draft.delivery_id),
            delivery_id: draft.delivery_id.clone(),
            code_type: DELIVERY_CODE_TYPE_REDEMPTION.to_string(),
            code_hash: hash_api_key(redemption_code),
            code_prefix: delivery_code_prefix(redemption_code),
            code_last_four: credential_last_four(redemption_code),
            format_version: DELIVERY_CODE_FORMAT_REDEMPTION_V2.to_string(),
            status: "active".to_string(),
            expires_at: draft.code_expires_at.clone(),
            used_at: None,
            revoked_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            version: 1,
        },
        DeliveryCodeRecord {
            code_id: format!("dlvcode_{}_browser_unlock", draft.delivery_id),
            delivery_id: draft.delivery_id.clone(),
            code_type: DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK.to_string(),
            code_hash: hash_api_key(browser_file_unlock_code),
            code_prefix: delivery_code_prefix(browser_file_unlock_code),
            code_last_four: credential_last_four(browser_file_unlock_code),
            format_version: DELIVERY_CODE_FORMAT_BROWSER_FILE_UNLOCK_V2.to_string(),
            status: "active".to_string(),
            expires_at: draft.code_expires_at,
            used_at: None,
            revoked_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            version: 1,
        },
    ];
    let entitlement = DeliveryEntitlementRecord {
        entitlement_id: format!("dlvent_{}", draft.delivery_id),
        delivery_id: draft.delivery_id,
        service_kind: draft.service_kind,
        service_days: draft.service_days,
        starts_at: draft.starts_at,
        ends_at: draft.ends_at,
        status: DELIVERY_ENTITLEMENT_STATUS_PENDING_ACTIVATION.to_string(),
        activated_at: None,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    };
    PreparedDeliveryRecords {
        delivery,
        codes,
        entitlement,
        enforce_owner_redemption_capacity: draft.enforce_owner_redemption_capacity,
    }
}

pub fn build_prepared_delivery_records(
    draft: DeliveryPrepareDraft,
    redemption_code: &str,
    browser_file_unlock_code: &str,
) -> PreparedDeliveryRecords {
    prepare_delivery_records(draft, redemption_code, browser_file_unlock_code)
}

fn delivery_projection(
    delivery: &DeliveryRecord,
    codes: &[DeliveryCodeRecord],
    entitlement: &DeliveryEntitlementRecord,
) -> DeliveryProjection {
    let mut codes = codes
        .iter()
        .map(DeliveryCodeRecord::public_view)
        .collect::<Vec<_>>();
    codes.sort_by(|left, right| left.code_type.cmp(&right.code_type));
    DeliveryProjection {
        delivery: delivery.public_view(entitlement),
        codes,
        entitlement: entitlement.public_view(),
    }
}

fn memory_delivery_projection(
    store: &MemoryStore,
    delivery_id: &str,
) -> Option<DeliveryProjection> {
    let delivery = store
        .deliveries
        .iter()
        .find(|item| item.delivery_id == delivery_id)?;
    let entitlement = store
        .delivery_entitlements
        .iter()
        .find(|item| item.delivery_id == delivery_id)?;
    let codes = store
        .delivery_codes
        .iter()
        .filter(|item| item.delivery_id == delivery_id)
        .cloned()
        .collect::<Vec<_>>();
    Some(delivery_projection(delivery, &codes, entitlement))
}

#[derive(Debug, Clone, Default)]
struct DeliveryOperationsRecords {
    deliveries: Vec<DeliveryRecord>,
    codes: Vec<DeliveryCodeRecord>,
    entitlements: Vec<DeliveryEntitlementRecord>,
    artifacts: Vec<DeliveryArtifactRecord>,
    activations: Vec<DeliveryActivationRecord>,
    download_grants: Vec<DeliveryDownloadGrantRecord>,
    service_segments: Vec<DeliveryServiceSegmentRecord>,
    lifecycle_events: Vec<DeliveryLifecycleEventRecord>,
    upload_batches: Vec<DeliveryUploadBatchRecord>,
    upload_items: Vec<DeliveryUploadBatchItemRecord>,
}

fn delivery_operations_records_from_memory(store: &MemoryStore) -> DeliveryOperationsRecords {
    DeliveryOperationsRecords {
        deliveries: store.deliveries.clone(),
        codes: store.delivery_codes.clone(),
        entitlements: store.delivery_entitlements.clone(),
        artifacts: store.delivery_artifacts.clone(),
        activations: store.delivery_activations.clone(),
        download_grants: store.delivery_download_grants.clone(),
        service_segments: store.delivery_service_segments.clone(),
        lifecycle_events: store.delivery_lifecycle_events.clone(),
        upload_batches: store.delivery_upload_batches.clone(),
        upload_items: store.delivery_upload_batch_items.clone(),
    }
}

fn delivery_operations_overview(
    records: DeliveryOperationsRecords,
    scope: DeliveryOperationsScope,
) -> DeliveryOperationsOverviewResponse {
    let mut status_counts = BTreeMap::<(String, String), u64>::new();
    let deliveries = records
        .deliveries
        .iter()
        .filter(|delivery| delivery_matches_scope(delivery, &scope))
        .collect::<Vec<_>>();
    let window_delivery_ids = deliveries
        .iter()
        .filter(|delivery| timestamp_in_window(&delivery.created_at, &scope))
        .map(|delivery| delivery.delivery_id.clone())
        .collect::<HashSet<_>>();
    let scope_delivery_ids = deliveries
        .iter()
        .map(|delivery| delivery.delivery_id.clone())
        .collect::<HashSet<_>>();
    let scope_entitlement_ids = records
        .entitlements
        .iter()
        .filter(|entitlement| scope_delivery_ids.contains(&entitlement.delivery_id))
        .map(|entitlement| entitlement.entitlement_id.clone())
        .collect::<HashSet<_>>();

    let delivery_count = count_delivery_statuses(&records, &scope, &mut status_counts);
    let entitlement_count = count_entitlement_statuses(&records, &scope, &mut status_counts);
    let artifact_count = count_artifact_statuses(&records, &scope, &mut status_counts);
    let upload_batch_count = count_upload_batch_statuses(&records, &scope, &mut status_counts);
    let upload_item_count = count_upload_item_statuses(&records, &scope, &mut status_counts);
    let activation_count = count_activation_statuses(&records, &scope, &mut status_counts);
    let download_grant_count = count_download_grant_statuses(&records, &scope, &mut status_counts);
    let service_segment_count =
        count_service_segment_statuses(&records, &scope, &mut status_counts);
    let lifecycle_event_count = records
        .lifecycle_events
        .iter()
        .filter(|event| {
            scope_entitlement_ids.contains(&event.entitlement_id)
                && timestamp_in_window(&event.created_at, &scope)
        })
        .count() as u64;
    if lifecycle_event_count > 0 {
        status_counts.insert(
            ("lifecycle_event".to_string(), "recorded".to_string()),
            lifecycle_event_count,
        );
    }

    let mut recent_events = records
        .deliveries
        .iter()
        .filter(|delivery| scope_delivery_ids.contains(&delivery.delivery_id))
        .flat_map(|delivery| {
            delivery_operations_timeline_events_for_delivery(&records, &delivery.delivery_id)
        })
        .filter(|event| {
            window_delivery_ids.contains(event.delivery_id.as_deref().unwrap_or_default())
                || timestamp_in_window(&event.occurred_at, &scope)
        })
        .collect::<Vec<_>>();
    sort_timeline_events_desc(&mut recent_events);
    recent_events.truncate(scope.limit);

    let exceptions = delivery_operations_exception_list(
        &records,
        &DeliveryOperationsExceptionFilters {
            scope: scope.clone(),
            status: None,
            exception_type: None,
        },
    );

    let mut status_counts = status_counts
        .into_iter()
        .map(|((domain, status), count)| DeliveryOperationsStatusCount {
            domain,
            status,
            count,
        })
        .collect::<Vec<_>>();
    status_counts.sort_by(|left, right| {
        left.domain
            .cmp(&right.domain)
            .then_with(|| left.status.cmp(&right.status))
    });

    DeliveryOperationsOverviewResponse {
        data: DeliveryOperationsOverview {
            tenant_id: scope.tenant_id,
            project_id: scope.project_id,
            window_start: scope.window_start,
            window_end: scope.window_end,
            totals: DeliveryOperationsTotals {
                deliveries: delivery_count,
                artifacts: artifact_count,
                upload_batches: upload_batch_count,
                upload_items: upload_item_count,
                activations: activation_count,
                download_grants: download_grant_count,
                entitlements: entitlement_count,
                service_segments: service_segment_count,
                lifecycle_events: lifecycle_event_count,
                exceptions: exceptions.len() as u64,
            },
            status_counts,
            recent_events,
            exceptions,
        },
    }
}

fn delivery_operations_timeline(
    records: DeliveryOperationsRecords,
    filters: DeliveryOperationsObjectFilters,
) -> Option<DeliveryOperationsTimelineResponse> {
    let delivery_id = delivery_operations_target_delivery_id(&records, &filters)?;
    let mut data = delivery_operations_timeline_events_for_delivery(&records, &delivery_id)
        .into_iter()
        .filter(|event| timestamp_in_window(&event.occurred_at, &filters.scope))
        .collect::<Vec<_>>();
    sort_timeline_events(&mut data);
    data.truncate(filters.scope.limit);
    Some(DeliveryOperationsTimelineResponse { data })
}

fn delivery_operations_exceptions(
    records: DeliveryOperationsRecords,
    filters: DeliveryOperationsExceptionFilters,
) -> DeliveryOperationsExceptionsResponse {
    DeliveryOperationsExceptionsResponse {
        data: delivery_operations_exception_list(&records, &filters),
    }
}

fn delivery_operations_detail(
    records: DeliveryOperationsRecords,
    filters: DeliveryOperationsObjectFilters,
) -> Option<DeliveryOperationsDetailResponse> {
    let delivery_id = delivery_operations_target_delivery_id(&records, &filters)?;
    let delivery = records
        .deliveries
        .iter()
        .find(|delivery| delivery.delivery_id == delivery_id)?;
    let entitlement = records
        .entitlements
        .iter()
        .find(|entitlement| entitlement.delivery_id == delivery_id)?;
    let codes = records
        .codes
        .iter()
        .filter(|code| code.delivery_id == delivery_id)
        .cloned()
        .collect::<Vec<_>>();
    let delivery = delivery_projection(delivery, &codes, entitlement);
    let artifacts = records
        .artifacts
        .iter()
        .filter(|artifact| artifact.delivery_id == delivery_id)
        .map(DeliveryArtifactRecord::public_view)
        .collect::<Vec<_>>();
    let upload_items = records
        .upload_items
        .iter()
        .filter(|item| item.delivery_id == delivery_id)
        .map(DeliveryUploadBatchItemRecord::public_view)
        .collect::<Vec<_>>();
    let activations = records
        .activations
        .iter()
        .filter(|activation| activation.delivery_id == delivery_id)
        .map(DeliveryActivationRecord::public_view)
        .collect::<Vec<_>>();
    let download_grants = records
        .download_grants
        .iter()
        .filter(|grant| grant.delivery_id == delivery_id)
        .map(DeliveryDownloadGrantRecord::public_view)
        .collect::<Vec<_>>();
    let service_segments = records
        .service_segments
        .iter()
        .filter(|segment| segment.delivery_id == delivery_id)
        .map(DeliveryServiceSegmentRecord::public_view)
        .collect::<Vec<_>>();
    let entitlement_ids = records
        .entitlements
        .iter()
        .filter(|entitlement| entitlement.delivery_id == delivery_id)
        .map(|entitlement| entitlement.entitlement_id.as_str())
        .collect::<HashSet<_>>();
    let lifecycle_events = records
        .lifecycle_events
        .iter()
        .filter(|event| entitlement_ids.contains(event.entitlement_id.as_str()))
        .map(DeliveryLifecycleEventRecord::public_view)
        .collect::<Vec<_>>();
    let mut timeline = delivery_operations_timeline_events_for_delivery(&records, &delivery_id)
        .into_iter()
        .filter(|event| timestamp_in_window(&event.occurred_at, &filters.scope))
        .collect::<Vec<_>>();
    sort_timeline_events(&mut timeline);
    timeline.truncate(filters.scope.limit);
    let exceptions = delivery_operations_exception_list(
        &records,
        &DeliveryOperationsExceptionFilters {
            scope: filters.scope,
            status: None,
            exception_type: None,
        },
    )
    .into_iter()
    .filter(|exception| exception.delivery_id.as_deref() == Some(delivery_id.as_str()))
    .collect::<Vec<_>>();
    Some(DeliveryOperationsDetailResponse {
        data: DeliveryOperationsDetail {
            delivery,
            artifacts,
            upload_items,
            activations,
            download_grants,
            service_segments,
            lifecycle_events,
            timeline,
            exceptions,
        },
    })
}

fn delivery_operations_target_delivery_id(
    records: &DeliveryOperationsRecords,
    filters: &DeliveryOperationsObjectFilters,
) -> Option<String> {
    let delivery_id = match &filters.object_ref {
        DeliveryOperationsObjectRef::Delivery(delivery_id) => records
            .deliveries
            .iter()
            .find(|delivery| delivery.delivery_id == *delivery_id)
            .map(|delivery| delivery.delivery_id.clone()),
        DeliveryOperationsObjectRef::Entitlement(entitlement_id) => records
            .entitlements
            .iter()
            .find(|entitlement| entitlement.entitlement_id == *entitlement_id)
            .map(|entitlement| entitlement.delivery_id.clone()),
        DeliveryOperationsObjectRef::Activation(activation_id) => records
            .activations
            .iter()
            .find(|activation| activation.activation_id == *activation_id)
            .map(|activation| activation.delivery_id.clone()),
        DeliveryOperationsObjectRef::Artifact(artifact_id) => records
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == *artifact_id)
            .map(|artifact| artifact.delivery_id.clone()),
        DeliveryOperationsObjectRef::DownloadGrant(grant_id) => records
            .download_grants
            .iter()
            .find(|grant| grant.grant_id == *grant_id)
            .map(|grant| grant.delivery_id.clone()),
        DeliveryOperationsObjectRef::ServiceSegment(segment_id) => records
            .service_segments
            .iter()
            .find(|segment| segment.segment_id == *segment_id)
            .map(|segment| segment.delivery_id.clone()),
    }?;
    let delivery = records
        .deliveries
        .iter()
        .find(|delivery| delivery.delivery_id == delivery_id)?;
    if delivery_matches_scope(delivery, &filters.scope) {
        Some(delivery_id)
    } else {
        None
    }
}

fn delivery_operations_timeline_events_for_delivery(
    records: &DeliveryOperationsRecords,
    delivery_id: &str,
) -> Vec<DeliveryOperationsTimelineEvent> {
    let Some(delivery) = records
        .deliveries
        .iter()
        .find(|delivery| delivery.delivery_id == delivery_id)
    else {
        return Vec::new();
    };
    let mut events = vec![timeline_event(
        "delivery_created",
        "delivery",
        &delivery.delivery_id,
        &delivery.tenant_id,
        &delivery.project_id,
        Some(&delivery.delivery_id),
        None,
        None,
        None,
        None,
        None,
        &delivery.status,
        &delivery.created_at,
        "Delivery fact created",
    )];
    if let Some(revoked_at) = delivery.revoked_at.as_deref() {
        events.push(timeline_event(
            "delivery_revoked",
            "delivery",
            &delivery.delivery_id,
            &delivery.tenant_id,
            &delivery.project_id,
            Some(&delivery.delivery_id),
            None,
            None,
            None,
            None,
            None,
            DELIVERY_STATUS_REVOKED,
            revoked_at,
            "Delivery was revoked",
        ));
    }

    records
        .codes
        .iter()
        .filter(|code| code.delivery_id == delivery_id)
        .for_each(|code| {
            events.push(timeline_event(
                "delivery_code_created",
                "delivery_code",
                &code.code_id,
                &delivery.tenant_id,
                &delivery.project_id,
                Some(&code.delivery_id),
                None,
                None,
                None,
                None,
                None,
                &code.effective_status(),
                &code.created_at,
                &format!("{} code created", code.code_type),
            ));
            if let Some(used_at) = code.used_at.as_deref() {
                events.push(timeline_event(
                    "delivery_code_used",
                    "delivery_code",
                    &code.code_id,
                    &delivery.tenant_id,
                    &delivery.project_id,
                    Some(&code.delivery_id),
                    None,
                    None,
                    None,
                    None,
                    None,
                    DELIVERY_CODE_STATUS_USED,
                    used_at,
                    &format!("{} code used", code.code_type),
                ));
            }
        });

    records
        .entitlements
        .iter()
        .filter(|entitlement| entitlement.delivery_id == delivery_id)
        .for_each(|entitlement| {
            events.push(timeline_event(
                "delivery_entitlement_created",
                "delivery_entitlement",
                &entitlement.entitlement_id,
                &delivery.tenant_id,
                &delivery.project_id,
                Some(&entitlement.delivery_id),
                Some(&entitlement.entitlement_id),
                None,
                None,
                None,
                None,
                &entitlement.effective_status(),
                &entitlement.created_at,
                "Entitlement fact created",
            ));
            if let Some(activated_at) = entitlement.activated_at.as_deref() {
                events.push(timeline_event(
                    "delivery_entitlement_activated",
                    "delivery_entitlement",
                    &entitlement.entitlement_id,
                    &delivery.tenant_id,
                    &delivery.project_id,
                    Some(&entitlement.delivery_id),
                    Some(&entitlement.entitlement_id),
                    None,
                    None,
                    None,
                    None,
                    DELIVERY_ENTITLEMENT_STATUS_ACTIVE,
                    activated_at,
                    "Entitlement service window activated",
                ));
            }
        });

    records
        .artifacts
        .iter()
        .filter(|artifact| artifact.delivery_id == delivery_id)
        .for_each(|artifact| {
            events.push(timeline_event(
                "delivery_artifact_uploaded",
                "delivery_artifact",
                &artifact.artifact_id,
                &artifact.tenant_id,
                &artifact.project_id,
                Some(&artifact.delivery_id),
                None,
                None,
                Some(&artifact.artifact_id),
                None,
                None,
                &artifact.status,
                &artifact.created_at,
                "Encrypted delivery artifact uploaded",
            ));
            if let Some(superseded_at) = artifact.superseded_at.as_deref() {
                events.push(timeline_event(
                    "delivery_artifact_superseded",
                    "delivery_artifact",
                    &artifact.artifact_id,
                    &artifact.tenant_id,
                    &artifact.project_id,
                    Some(&artifact.delivery_id),
                    None,
                    None,
                    Some(&artifact.artifact_id),
                    None,
                    None,
                    DELIVERY_ARTIFACT_STATUS_SUPERSEDED,
                    superseded_at,
                    "Artifact was superseded",
                ));
            }
        });

    records
        .upload_items
        .iter()
        .filter(|item| item.delivery_id == delivery_id)
        .for_each(|item| {
            events.push(upload_timeline_event(
                "delivery_upload_item_recorded",
                item,
                &item.status,
                &item.created_at,
                "Delivery upload item staged",
            ));
            if item.status != DELIVERY_UPLOAD_ITEM_STATUS_PENDING {
                events.push(upload_timeline_event(
                    "delivery_upload_item_processed",
                    item,
                    &item.status,
                    &item.updated_at,
                    item.error_message
                        .as_deref()
                        .unwrap_or("Delivery upload item processed"),
                ));
            }
        });

    records
        .activations
        .iter()
        .filter(|activation| activation.delivery_id == delivery_id)
        .for_each(|activation| {
            events.push(timeline_event(
                "delivery_activated",
                "delivery_activation",
                &activation.activation_id,
                &activation.tenant_id,
                &activation.project_id,
                Some(&activation.delivery_id),
                Some(&activation.entitlement_id),
                Some(&activation.activation_id),
                Some(&activation.artifact_id),
                None,
                None,
                &activation.status,
                &activation.activated_at,
                "Delivery redeemed and activated",
            ));
        });

    records
        .download_grants
        .iter()
        .filter(|grant| grant.delivery_id == delivery_id)
        .for_each(|grant| {
            events.push(timeline_event(
                "delivery_download_grant_issued",
                "delivery_download_grant",
                &grant.grant_id,
                &grant.tenant_id,
                &grant.project_id,
                Some(&grant.delivery_id),
                Some(&grant.entitlement_id),
                Some(&grant.activation_id),
                Some(&grant.artifact_id),
                Some(&grant.grant_id),
                None,
                &grant.effective_status(),
                &grant.created_at,
                "Download grant issued",
            ));
            if let Some(used_at) = grant.used_at.as_deref() {
                events.push(timeline_event(
                    "delivery_download_grant_used",
                    "delivery_download_grant",
                    &grant.grant_id,
                    &grant.tenant_id,
                    &grant.project_id,
                    Some(&grant.delivery_id),
                    Some(&grant.entitlement_id),
                    Some(&grant.activation_id),
                    Some(&grant.artifact_id),
                    Some(&grant.grant_id),
                    None,
                    DELIVERY_DOWNLOAD_GRANT_STATUS_USED,
                    used_at,
                    "Download grant consumed",
                ));
            }
            if let Some(revoked_at) = grant.revoked_at.as_deref() {
                events.push(timeline_event(
                    "delivery_download_grant_revoked",
                    "delivery_download_grant",
                    &grant.grant_id,
                    &grant.tenant_id,
                    &grant.project_id,
                    Some(&grant.delivery_id),
                    Some(&grant.entitlement_id),
                    Some(&grant.activation_id),
                    Some(&grant.artifact_id),
                    Some(&grant.grant_id),
                    None,
                    DELIVERY_DOWNLOAD_GRANT_STATUS_REVOKED,
                    revoked_at,
                    "Download grant revoked",
                ));
            }
        });

    records
        .service_segments
        .iter()
        .filter(|segment| segment.delivery_id == delivery_id)
        .for_each(|segment| {
            events.push(timeline_event(
                "delivery_service_segment_created",
                "delivery_service_segment",
                &segment.segment_id,
                &segment.tenant_id,
                &segment.project_id,
                Some(&segment.delivery_id),
                Some(&segment.entitlement_id),
                Some(&segment.activation_id),
                Some(&segment.artifact_id),
                None,
                Some(&segment.segment_id),
                &segment.effective_status(),
                &segment.created_at,
                "Delivery service segment created",
            ));
        });

    let delivery_entitlement_ids = records
        .entitlements
        .iter()
        .filter(|entitlement| entitlement.delivery_id == delivery_id)
        .map(|entitlement| entitlement.entitlement_id.as_str())
        .collect::<HashSet<_>>();
    records
        .lifecycle_events
        .iter()
        .filter(|event| delivery_entitlement_ids.contains(event.entitlement_id.as_str()))
        .for_each(|event| {
            events.push(timeline_event(
                &event.event_type,
                "delivery_lifecycle_event",
                &event.event_id,
                &delivery.tenant_id,
                &delivery.project_id,
                Some(delivery_id),
                Some(&event.entitlement_id),
                None,
                event.payload.get("artifact_id").map(String::as_str),
                None,
                event.segment_id.as_deref(),
                &event.status,
                &event.created_at,
                event
                    .reason
                    .as_deref()
                    .unwrap_or("Lifecycle event recorded"),
            ));
        });

    sort_timeline_events(&mut events);
    events
}

fn upload_timeline_event(
    event_type: &str,
    item: &DeliveryUploadBatchItemRecord,
    status: &str,
    occurred_at: &str,
    summary: &str,
) -> DeliveryOperationsTimelineEvent {
    DeliveryOperationsTimelineEvent {
        event_id: format!("{event_type}:{}:{occurred_at}", item.item_id),
        event_type: event_type.to_string(),
        object_type: "delivery_upload_item".to_string(),
        object_id: item.item_id.clone(),
        tenant_id: item.tenant_id.clone(),
        project_id: item.project_id.clone(),
        delivery_id: Some(item.delivery_id.clone()),
        entitlement_id: None,
        activation_id: None,
        artifact_id: item.artifact_id.clone(),
        grant_id: None,
        segment_id: None,
        upload_batch_id: Some(item.batch_id.clone()),
        upload_item_id: Some(item.item_id.clone()),
        status: status.to_string(),
        occurred_at: occurred_at.to_string(),
        summary: summary.to_string(),
    }
}

fn timeline_event(
    event_type: &str,
    object_type: &str,
    object_id: &str,
    tenant_id: &TenantId,
    project_id: &ProjectId,
    delivery_id: Option<&str>,
    entitlement_id: Option<&str>,
    activation_id: Option<&str>,
    artifact_id: Option<&str>,
    grant_id: Option<&str>,
    segment_id: Option<&str>,
    status: &str,
    occurred_at: &str,
    summary: &str,
) -> DeliveryOperationsTimelineEvent {
    DeliveryOperationsTimelineEvent {
        event_id: format!("{event_type}:{object_id}:{occurred_at}"),
        event_type: event_type.to_string(),
        object_type: object_type.to_string(),
        object_id: object_id.to_string(),
        tenant_id: tenant_id.clone(),
        project_id: project_id.clone(),
        delivery_id: delivery_id.map(str::to_string),
        entitlement_id: entitlement_id.map(str::to_string),
        activation_id: activation_id.map(str::to_string),
        artifact_id: artifact_id.map(str::to_string),
        grant_id: grant_id.map(str::to_string),
        segment_id: segment_id.map(str::to_string),
        upload_batch_id: None,
        upload_item_id: None,
        status: status.to_string(),
        occurred_at: occurred_at.to_string(),
        summary: summary.to_string(),
    }
}

fn delivery_operations_exception_list(
    records: &DeliveryOperationsRecords,
    filters: &DeliveryOperationsExceptionFilters,
) -> Vec<DeliveryOperationsException> {
    let mut exceptions = Vec::new();
    records
        .deliveries
        .iter()
        .filter(|delivery| delivery_matches_scope(delivery, &filters.scope))
        .for_each(|delivery| {
            let has_delivery_artifact = records
                .artifacts
                .iter()
                .any(|artifact| artifact.delivery_id == delivery.delivery_id);
            if !has_delivery_artifact {
                exceptions.push(delivery_exception(
                    "missing_artifact",
                    "warning",
                    delivery,
                    None,
                    None,
                    None,
                    None,
                    None,
                    "missing_artifact",
                    Some("delivery has no uploaded artifact"),
                    &delivery.created_at,
                    "Delivery has no encrypted artifact uploaded",
                ));
            }
            if delivery.status == DELIVERY_STATUS_REVOKED {
                exceptions.push(delivery_exception(
                    "delivery_revoked",
                    "warning",
                    delivery,
                    None,
                    None,
                    None,
                    None,
                    None,
                    DELIVERY_STATUS_REVOKED,
                    delivery.revoke_reason.as_deref(),
                    delivery
                        .revoked_at
                        .as_deref()
                        .unwrap_or(&delivery.updated_at),
                    "Delivery was revoked",
                ));
            }
        });

    records
        .entitlements
        .iter()
        .filter_map(|entitlement| {
            let delivery = records
                .deliveries
                .iter()
                .find(|delivery| delivery.delivery_id == entitlement.delivery_id)?;
            if delivery_matches_scope(delivery, &filters.scope) {
                Some((delivery, entitlement))
            } else {
                None
            }
        })
        .for_each(|(delivery, entitlement)| {
            let status = entitlement.effective_status();
            if matches!(
                status.as_str(),
                DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY
                    | DELIVERY_STATUS_EXPIRED
                    | DELIVERY_ENTITLEMENT_STATUS_REVOKED
                    | DELIVERY_ENTITLEMENT_STATUS_SUSPENDED
            ) {
                exceptions.push(delivery_exception(
                    &format!("entitlement_{status}"),
                    if status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
                        "critical"
                    } else {
                        "warning"
                    },
                    delivery,
                    Some(&entitlement.entitlement_id),
                    None,
                    None,
                    None,
                    None,
                    &status,
                    None,
                    if status == DELIVERY_STATUS_EXPIRED {
                        &entitlement.ends_at
                    } else {
                        &entitlement.updated_at
                    },
                    "Delivery entitlement requires operator attention",
                ));
            }
        });

    records
        .artifacts
        .iter()
        .filter(|artifact| artifact_matches_scope(artifact, &filters.scope))
        .filter(|artifact| artifact.status == DELIVERY_ARTIFACT_STATUS_REVOKED)
        .for_each(|artifact| {
            exceptions.push(object_exception(
                "artifact_revoked",
                "warning",
                &artifact.tenant_id,
                &artifact.project_id,
                Some(&artifact.delivery_id),
                None,
                None,
                Some(&artifact.artifact_id),
                None,
                None,
                &artifact.status,
                artifact.revoke_reason.as_deref(),
                artifact
                    .revoked_at
                    .as_deref()
                    .unwrap_or(&artifact.updated_at),
                "Encrypted artifact was revoked",
            ));
        });

    records
        .download_grants
        .iter()
        .filter(|grant| grant_matches_scope(grant, &filters.scope))
        .for_each(|grant| {
            let status = grant.effective_status();
            if matches!(
                status.as_str(),
                DELIVERY_DOWNLOAD_GRANT_STATUS_EXPIRED | DELIVERY_DOWNLOAD_GRANT_STATUS_REVOKED
            ) {
                exceptions.push(object_exception(
                    &format!("download_grant_{status}"),
                    "info",
                    &grant.tenant_id,
                    &grant.project_id,
                    Some(&grant.delivery_id),
                    Some(&grant.entitlement_id),
                    Some(&grant.activation_id),
                    Some(&grant.artifact_id),
                    Some(&grant.grant_id),
                    None,
                    &status,
                    grant.revoke_reason.as_deref(),
                    grant.revoked_at.as_deref().unwrap_or(&grant.updated_at),
                    "Download grant is no longer usable",
                ));
            }
            if records
                .entitlements
                .iter()
                .find(|entitlement| entitlement.entitlement_id == grant.entitlement_id)
                .is_some_and(|entitlement| {
                    entitlement.effective_status() != DELIVERY_ENTITLEMENT_STATUS_ACTIVE
                })
                && status == DELIVERY_DOWNLOAD_GRANT_STATUS_ACTIVE
            {
                exceptions.push(object_exception(
                    "download_blocked_entitlement",
                    "critical",
                    &grant.tenant_id,
                    &grant.project_id,
                    Some(&grant.delivery_id),
                    Some(&grant.entitlement_id),
                    Some(&grant.activation_id),
                    Some(&grant.artifact_id),
                    Some(&grant.grant_id),
                    None,
                    "blocked",
                    Some("entitlement is not active"),
                    &grant.updated_at,
                    "Active download grant is blocked by entitlement state",
                ));
            }
        });

    records
        .service_segments
        .iter()
        .filter(|segment| segment_matches_scope(segment, &filters.scope))
        .for_each(|segment| {
            let status = segment.effective_status();
            if matches!(
                status.as_str(),
                DELIVERY_SERVICE_SEGMENT_STATUS_EXPIRED | DELIVERY_SERVICE_SEGMENT_STATUS_REVOKED
            ) {
                exceptions.push(object_exception(
                    &format!("service_segment_{status}"),
                    "warning",
                    &segment.tenant_id,
                    &segment.project_id,
                    Some(&segment.delivery_id),
                    Some(&segment.entitlement_id),
                    Some(&segment.activation_id),
                    Some(&segment.artifact_id),
                    None,
                    Some(&segment.segment_id),
                    &status,
                    None,
                    if status == DELIVERY_SERVICE_SEGMENT_STATUS_EXPIRED {
                        &segment.effective_until
                    } else {
                        &segment.updated_at
                    },
                    "Service segment is not currently active",
                ));
            }
        });

    records
        .lifecycle_events
        .iter()
        .filter_map(|event| {
            let entitlement = records
                .entitlements
                .iter()
                .find(|entitlement| entitlement.entitlement_id == event.entitlement_id)?;
            let delivery = records
                .deliveries
                .iter()
                .find(|delivery| delivery.delivery_id == entitlement.delivery_id)?;
            if delivery_matches_scope(delivery, &filters.scope) {
                Some((delivery, event))
            } else {
                None
            }
        })
        .for_each(|(delivery, event)| {
            let is_exception = event.status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY
                || event
                    .reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("missing"));
            if is_exception {
                exceptions.push(delivery_exception(
                    &format!("lifecycle_{}", event.event_type),
                    "critical",
                    delivery,
                    Some(&event.entitlement_id),
                    None,
                    event.payload.get("artifact_id").map(String::as_str),
                    None,
                    event.segment_id.as_deref(),
                    &event.status,
                    event.reason.as_deref(),
                    &event.created_at,
                    "Lifecycle event requires operator attention",
                ));
            }
        });

    records
        .upload_items
        .iter()
        .filter(|item| upload_item_matches_scope(item, &filters.scope))
        .filter(|item| {
            matches!(
                item.status.as_str(),
                DELIVERY_UPLOAD_ITEM_STATUS_REJECTED | DELIVERY_UPLOAD_ITEM_STATUS_FAILED
            )
        })
        .for_each(|item| {
            exceptions.push(upload_item_exception(
                item,
                if item.status == DELIVERY_UPLOAD_ITEM_STATUS_FAILED {
                    "critical"
                } else {
                    "warning"
                },
            ));
        });

    exceptions.retain(|exception| {
        timestamp_in_window(&exception.occurred_at, &filters.scope)
            && filters
                .status
                .as_deref()
                .is_none_or(|status| exception.status == status)
            && filters
                .exception_type
                .as_deref()
                .is_none_or(|exception_type| exception.exception_type == exception_type)
    });
    exceptions.sort_by(|left, right| {
        right
            .occurred_at
            .cmp(&left.occurred_at)
            .then_with(|| left.exception_id.cmp(&right.exception_id))
    });
    exceptions.truncate(filters.scope.limit);
    exceptions
}

fn upload_item_exception(
    item: &DeliveryUploadBatchItemRecord,
    severity: &str,
) -> DeliveryOperationsException {
    DeliveryOperationsException {
        exception_id: format!(
            "delivery_upload_item_{}:{}:{}",
            item.status, item.item_id, item.updated_at
        ),
        exception_type: format!("delivery_upload_item_{}", item.status),
        severity: severity.to_string(),
        tenant_id: item.tenant_id.clone(),
        project_id: item.project_id.clone(),
        delivery_id: Some(item.delivery_id.clone()),
        entitlement_id: None,
        activation_id: None,
        artifact_id: item.artifact_id.clone(),
        grant_id: None,
        segment_id: None,
        upload_batch_id: Some(item.batch_id.clone()),
        upload_item_id: Some(item.item_id.clone()),
        status: item.status.clone(),
        reason: item.error_code.clone(),
        occurred_at: item.updated_at.clone(),
        summary: item
            .error_message
            .clone()
            .unwrap_or_else(|| "Delivery upload item requires operator attention".to_string()),
    }
}

fn delivery_exception(
    exception_type: &str,
    severity: &str,
    delivery: &DeliveryRecord,
    entitlement_id: Option<&str>,
    activation_id: Option<&str>,
    artifact_id: Option<&str>,
    grant_id: Option<&str>,
    segment_id: Option<&str>,
    status: &str,
    reason: Option<&str>,
    occurred_at: &str,
    summary: &str,
) -> DeliveryOperationsException {
    object_exception(
        exception_type,
        severity,
        &delivery.tenant_id,
        &delivery.project_id,
        Some(&delivery.delivery_id),
        entitlement_id,
        activation_id,
        artifact_id,
        grant_id,
        segment_id,
        status,
        reason,
        occurred_at,
        summary,
    )
}

fn object_exception(
    exception_type: &str,
    severity: &str,
    tenant_id: &TenantId,
    project_id: &ProjectId,
    delivery_id: Option<&str>,
    entitlement_id: Option<&str>,
    activation_id: Option<&str>,
    artifact_id: Option<&str>,
    grant_id: Option<&str>,
    segment_id: Option<&str>,
    status: &str,
    reason: Option<&str>,
    occurred_at: &str,
    summary: &str,
) -> DeliveryOperationsException {
    let object_id = grant_id
        .or(segment_id)
        .or(activation_id)
        .or(artifact_id)
        .or(entitlement_id)
        .or(delivery_id)
        .unwrap_or("unknown");
    DeliveryOperationsException {
        exception_id: format!("{exception_type}:{object_id}:{occurred_at}"),
        exception_type: exception_type.to_string(),
        severity: severity.to_string(),
        tenant_id: tenant_id.clone(),
        project_id: project_id.clone(),
        delivery_id: delivery_id.map(str::to_string),
        entitlement_id: entitlement_id.map(str::to_string),
        activation_id: activation_id.map(str::to_string),
        artifact_id: artifact_id.map(str::to_string),
        grant_id: grant_id.map(str::to_string),
        segment_id: segment_id.map(str::to_string),
        upload_batch_id: None,
        upload_item_id: None,
        status: status.to_string(),
        reason: reason.map(str::to_string),
        occurred_at: occurred_at.to_string(),
        summary: summary.to_string(),
    }
}

fn count_delivery_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .deliveries
        .iter()
        .filter(|delivery| {
            delivery_matches_scope(delivery, scope)
                && timestamp_in_window(&delivery.created_at, scope)
        })
        .for_each(|delivery| {
            total = total.saturating_add(1);
            let status = records
                .entitlements
                .iter()
                .find(|entitlement| entitlement.delivery_id == delivery.delivery_id)
                .map_or_else(
                    || delivery.status.clone(),
                    |entitlement| delivery.effective_status(entitlement),
                );
            increment_status_count(counts, "delivery", &status);
        });
    total
}

fn count_entitlement_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .entitlements
        .iter()
        .filter_map(|entitlement| {
            let delivery = records
                .deliveries
                .iter()
                .find(|delivery| delivery.delivery_id == entitlement.delivery_id)?;
            Some((delivery, entitlement))
        })
        .filter(|(delivery, entitlement)| {
            delivery_matches_scope(delivery, scope)
                && timestamp_in_window(&entitlement.created_at, scope)
        })
        .for_each(|(_, entitlement)| {
            total = total.saturating_add(1);
            increment_status_count(counts, "entitlement", &entitlement.effective_status());
        });
    total
}

fn count_artifact_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact_matches_scope(artifact, scope)
                && timestamp_in_window(&artifact.created_at, scope)
        })
        .for_each(|artifact| {
            total = total.saturating_add(1);
            increment_status_count(counts, "artifact", &artifact.status);
        });
    total
}

fn count_upload_batch_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .upload_batches
        .iter()
        .filter(|batch| {
            upload_batch_matches_scope(batch, scope)
                && timestamp_in_window(&batch.created_at, scope)
        })
        .for_each(|batch| {
            total = total.saturating_add(1);
            increment_status_count(counts, "delivery_upload_batch", &batch.status);
        });
    total
}

fn count_upload_item_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .upload_items
        .iter()
        .filter(|item| {
            upload_item_matches_scope(item, scope) && timestamp_in_window(&item.created_at, scope)
        })
        .for_each(|item| {
            total = total.saturating_add(1);
            increment_status_count(counts, "delivery_upload_item", &item.status);
        });
    total
}

fn count_activation_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .activations
        .iter()
        .filter(|activation| {
            activation_matches_scope(activation, scope)
                && timestamp_in_window(&activation.created_at, scope)
        })
        .for_each(|activation| {
            total = total.saturating_add(1);
            increment_status_count(counts, "activation", &activation.status);
        });
    total
}

fn count_download_grant_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .download_grants
        .iter()
        .filter(|grant| {
            grant_matches_scope(grant, scope) && timestamp_in_window(&grant.created_at, scope)
        })
        .for_each(|grant| {
            total = total.saturating_add(1);
            increment_status_count(counts, "download_grant", &grant.effective_status());
        });
    total
}

fn count_service_segment_statuses(
    records: &DeliveryOperationsRecords,
    scope: &DeliveryOperationsScope,
    counts: &mut BTreeMap<(String, String), u64>,
) -> u64 {
    let mut total = 0_u64;
    records
        .service_segments
        .iter()
        .filter(|segment| {
            segment_matches_scope(segment, scope) && timestamp_in_window(&segment.created_at, scope)
        })
        .for_each(|segment| {
            total = total.saturating_add(1);
            increment_status_count(counts, "service_segment", &segment.effective_status());
        });
    total
}

fn increment_status_count(
    counts: &mut BTreeMap<(String, String), u64>,
    domain: &str,
    status: &str,
) {
    *counts
        .entry((domain.to_string(), status.to_string()))
        .or_insert(0) += 1;
}

fn delivery_matches_scope(delivery: &DeliveryRecord, scope: &DeliveryOperationsScope) -> bool {
    delivery.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| delivery.project_id == *project_id)
}

fn artifact_matches_scope(
    artifact: &DeliveryArtifactRecord,
    scope: &DeliveryOperationsScope,
) -> bool {
    artifact.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| artifact.project_id == *project_id)
}

fn upload_batch_matches_scope(
    batch: &DeliveryUploadBatchRecord,
    scope: &DeliveryOperationsScope,
) -> bool {
    batch.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| batch.project_id == *project_id)
}

fn upload_item_matches_scope(
    item: &DeliveryUploadBatchItemRecord,
    scope: &DeliveryOperationsScope,
) -> bool {
    item.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| item.project_id == *project_id)
}

fn activation_matches_scope(
    activation: &DeliveryActivationRecord,
    scope: &DeliveryOperationsScope,
) -> bool {
    activation.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| activation.project_id == *project_id)
}

fn grant_matches_scope(
    grant: &DeliveryDownloadGrantRecord,
    scope: &DeliveryOperationsScope,
) -> bool {
    grant.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| grant.project_id == *project_id)
}

fn segment_matches_scope(
    segment: &DeliveryServiceSegmentRecord,
    scope: &DeliveryOperationsScope,
) -> bool {
    segment.tenant_id == scope.tenant_id
        && scope
            .project_id
            .as_ref()
            .is_none_or(|project_id| segment.project_id == *project_id)
}

fn timestamp_in_window(value: &str, scope: &DeliveryOperationsScope) -> bool {
    let Ok(value) = OffsetDateTime::parse(value, &Rfc3339) else {
        return false;
    };
    let Ok(window_start) = OffsetDateTime::parse(&scope.window_start, &Rfc3339) else {
        return false;
    };
    let Ok(window_end) = OffsetDateTime::parse(&scope.window_end, &Rfc3339) else {
        return false;
    };
    value >= window_start && value <= window_end
}

fn sort_timeline_events(events: &mut [DeliveryOperationsTimelineEvent]) {
    events.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
}

fn sort_timeline_events_desc(events: &mut [DeliveryOperationsTimelineEvent]) {
    events.sort_by(|left, right| {
        right
            .occurred_at
            .cmp(&left.occurred_at)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
}

fn revoke_memory_delivery(
    store: &mut MemoryStore,
    delivery_id: &str,
    expected_version: u64,
    revoked_by: &str,
    revoke_reason: Option<String>,
) -> ConcurrencyResult<DeliveryResponse> {
    let index = store
        .deliveries
        .iter()
        .position(|item| item.delivery_id == delivery_id);
    let Some(index) = index else {
        return ConcurrencyResult::NotFound;
    };
    let current = store
        .deliveries
        .get(index)
        .expect("indexed delivery should exist");
    if current.version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }

    let now = now_rfc3339();
    let mut delivery = current.clone();
    delivery.status = DELIVERY_STATUS_REVOKED.to_string();
    delivery.revoked_at = Some(now.clone());
    delivery.revoked_by = Some(revoked_by.to_string());
    delivery.revoke_reason = revoke_reason;
    delivery.updated_at = now.clone();
    delivery.version = delivery.version.saturating_add(1);
    store.deliveries[index] = delivery;

    for code in store
        .delivery_codes
        .iter_mut()
        .filter(|code| code.delivery_id == delivery_id)
    {
        code.status = DELIVERY_STATUS_REVOKED.to_string();
        code.revoked_at = Some(now.clone());
        code.updated_at = now.clone();
        code.version = code.version.saturating_add(1);
    }

    for entitlement in store
        .delivery_entitlements
        .iter_mut()
        .filter(|entitlement| entitlement.delivery_id == delivery_id)
    {
        entitlement.status = DELIVERY_ENTITLEMENT_STATUS_REVOKED.to_string();
        entitlement.updated_at = now.clone();
    }

    let Some(data) = memory_delivery_projection(store, delivery_id) else {
        return ConcurrencyResult::NotFound;
    };
    ConcurrencyResult::Applied(DeliveryResponse { data })
}

fn delivery_code_prefix(code: &str) -> String {
    if code.len() <= 18 {
        code.to_string()
    } else {
        format!("{}...", &code[..18])
    }
}

fn delivery_secret_key(delivery_id: &str, code_type: &str) -> String {
    format!("{delivery_id}:{code_type}")
}

fn insert_memory_delivery_artifact(
    store: &mut MemoryStore,
    draft: DeliveryArtifactDraft,
) -> DeliveryArtifactRecord {
    let existing = store
        .delivery_artifacts
        .iter()
        .filter(|artifact| artifact.delivery_id == draft.delivery_id)
        .cloned()
        .collect::<Vec<_>>();
    let artifact = build_delivery_artifact_record(&draft, &existing);
    for item in store.delivery_artifacts.iter_mut().filter(|artifact| {
        artifact.delivery_id == draft.delivery_id
            && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
    }) {
        item.status = DELIVERY_ARTIFACT_STATUS_SUPERSEDED.to_string();
        item.superseded_at = Some(artifact.created_at.clone());
        item.superseded_by = Some(artifact.artifact_id.clone());
        item.updated_at = artifact.created_at.clone();
    }
    store.delivery_artifacts.push(artifact.clone());
    artifact
}

fn build_delivery_artifact_record(
    draft: &DeliveryArtifactDraft,
    existing: &[DeliveryArtifactRecord],
) -> DeliveryArtifactRecord {
    let now = now_rfc3339();
    let version = existing
        .iter()
        .filter(|artifact| artifact.delivery_id == draft.delivery_id)
        .map(|artifact| artifact.version)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    DeliveryArtifactRecord {
        artifact_id: draft.artifact_id.clone(),
        delivery_id: draft.delivery_id.clone(),
        tenant_id: draft.tenant_id.clone(),
        project_id: draft.project_id.clone(),
        artifact_kind: draft.artifact_kind.clone(),
        provider: draft.provider.clone(),
        status: DELIVERY_ARTIFACT_STATUS_ACTIVE.to_string(),
        version,
        file_name: draft.file_name.clone(),
        content_type: draft.content_type.clone(),
        size_bytes: u64::try_from(draft.ciphertext.len()).unwrap_or(u64::MAX),
        sha256: artifact_sha256(&draft.ciphertext),
        storage_backend: DELIVERY_ARTIFACT_STORAGE_BACKEND_DB_INLINE.to_string(),
        storage_ref: draft.artifact_id.clone(),
        carrier_valid_until: draft.carrier_valid_until.clone(),
        encryption_protocol: draft.encryption_protocol.clone(),
        encryption_version: draft.encryption_version.clone(),
        secret_kind: draft.secret_kind.clone(),
        created_by: draft.created_by.clone(),
        created_at: now.clone(),
        updated_at: now,
        superseded_at: None,
        superseded_by: None,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
        ciphertext: draft.ciphertext.clone(),
    }
}

fn artifact_sha256(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    format!("sha256:{:x}", hasher.finalize())
}

fn insert_memory_delivery_upload_batch(
    store: &mut MemoryStore,
    draft: DeliveryUploadBatchDraft,
) -> DeliveryUploadBatchRecord {
    if let Some(existing) = find_existing_upload_batch(
        &store.delivery_upload_batches,
        &draft.tenant_id,
        &draft.project_id,
        draft.idempotency_key.as_deref(),
        &draft.source_file_sha256,
    ) {
        return existing.clone();
    }
    let batch = build_delivery_upload_batch_record(&draft);
    let items = build_delivery_upload_batch_item_records(&draft, &batch);
    store.delivery_upload_batches.push(batch.clone());
    store.delivery_upload_batch_items.extend(items);
    batch
}

fn build_delivery_upload_batch_record(
    draft: &DeliveryUploadBatchDraft,
) -> DeliveryUploadBatchRecord {
    let now = now_rfc3339();
    DeliveryUploadBatchRecord {
        batch_id: draft.batch_id.clone(),
        tenant_id: draft.tenant_id.clone(),
        project_id: draft.project_id.clone(),
        provider: draft.provider.clone(),
        status: DELIVERY_UPLOAD_BATCH_STATUS_QUEUED.to_string(),
        source_file_name: draft.source_file_name.clone(),
        source_file_sha256: draft.source_file_sha256.clone(),
        idempotency_key: draft.idempotency_key.clone(),
        total_count: u32::try_from(draft.items.len()).unwrap_or(u32::MAX),
        success_count: 0,
        failed_count: 0,
        duplicate_count: 0,
        created_by: draft.created_by.clone(),
        created_at: now.clone(),
        updated_at: now,
        started_at: None,
        finished_at: None,
        error_summary: None,
        version: 1,
    }
}

fn build_delivery_upload_batch_item_records(
    draft: &DeliveryUploadBatchDraft,
    batch: &DeliveryUploadBatchRecord,
) -> Vec<DeliveryUploadBatchItemRecord> {
    let now = batch.created_at.clone();
    draft
        .items
        .iter()
        .map(|item| DeliveryUploadBatchItemRecord {
            item_id: item.item_id.clone(),
            batch_id: batch.batch_id.clone(),
            row_index: item.row_index,
            tenant_id: batch.tenant_id.clone(),
            project_id: batch.project_id.clone(),
            delivery_id: item.delivery_id.clone(),
            artifact_id: None,
            status: DELIVERY_UPLOAD_ITEM_STATUS_PENDING.to_string(),
            artifact_kind: item.artifact_kind.clone(),
            file_name: item.file_name.clone(),
            content_type: item.content_type.clone(),
            carrier_valid_until: item.carrier_valid_until.clone(),
            encryption_protocol: item.encryption_protocol.clone(),
            encryption_version: item.encryption_version.clone(),
            secret_kind: item.secret_kind.clone(),
            payload_sha256: artifact_sha256(&item.ciphertext),
            size_bytes: u64::try_from(item.ciphertext.len()).unwrap_or(u64::MAX),
            error_code: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            version: 1,
            ciphertext: item.ciphertext.clone(),
        })
        .collect()
}

fn find_existing_upload_batch<'a>(
    batches: &'a [DeliveryUploadBatchRecord],
    tenant_id: &TenantId,
    project_id: &ProjectId,
    idempotency_key: Option<&str>,
    source_file_sha256: &str,
) -> Option<&'a DeliveryUploadBatchRecord> {
    batches.iter().find(|batch| {
        batch.tenant_id == *tenant_id
            && batch.project_id == *project_id
            && (batch.source_file_sha256 == source_file_sha256
                || idempotency_key
                    .zip(batch.idempotency_key.as_deref())
                    .is_some_and(|(left, right)| left == right))
    })
}

fn process_memory_delivery_upload_batch(
    store: &mut MemoryStore,
    batch_id: &str,
    actor_id: &str,
) -> DeliveryUploadProcessResult {
    let Some(batch_index) = store
        .delivery_upload_batches
        .iter()
        .position(|batch| batch.batch_id == batch_id)
    else {
        return DeliveryUploadProcessResult::BatchNotFound;
    };
    let batch = store.delivery_upload_batches[batch_index].clone();
    if delivery_upload_batch_is_terminal(&batch.status) {
        return DeliveryUploadProcessResult::Processed(Box::new(batch.public_response()));
    }
    if batch.status == DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING {
        return DeliveryUploadProcessResult::BatchAlreadyProcessing;
    }
    if store.delivery_upload_batches.iter().any(|candidate| {
        candidate.batch_id != batch.batch_id
            && candidate.tenant_id == batch.tenant_id
            && candidate.project_id == batch.project_id
            && candidate.status == DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING
    }) {
        return DeliveryUploadProcessResult::ScopeBusy;
    }

    let now = now_rfc3339();
    {
        let batch = &mut store.delivery_upload_batches[batch_index];
        batch.status = DELIVERY_UPLOAD_BATCH_STATUS_PROCESSING.to_string();
        batch.started_at = Some(now.clone());
        batch.updated_at = now;
        batch.version = batch.version.saturating_add(1);
    }

    let item_indexes = store
        .delivery_upload_batch_items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.batch_id == batch_id).then_some(index))
        .collect::<Vec<_>>();
    for item_index in item_indexes {
        if store.delivery_upload_batch_items[item_index].status
            != DELIVERY_UPLOAD_ITEM_STATUS_PENDING
        {
            continue;
        }
        process_memory_delivery_upload_item(store, item_index, actor_id);
    }

    finalize_memory_delivery_upload_batch(store, batch_index);
    let batch = store.delivery_upload_batches[batch_index].clone();
    DeliveryUploadProcessResult::Processed(Box::new(batch.public_response()))
}

fn process_memory_delivery_upload_item(store: &mut MemoryStore, item_index: usize, actor_id: &str) {
    let item = store.delivery_upload_batch_items[item_index].clone();
    let outcome = delivery_upload_item_outcome_for_memory(store, &item, actor_id);
    apply_delivery_upload_item_outcome(&mut store.delivery_upload_batch_items[item_index], outcome);
}

fn delivery_upload_item_outcome_for_memory(
    store: &mut MemoryStore,
    item: &DeliveryUploadBatchItemRecord,
    actor_id: &str,
) -> DeliveryUploadItemOutcome {
    let Some(delivery) = store
        .deliveries
        .iter()
        .find(|delivery| delivery.delivery_id == item.delivery_id)
        .cloned()
    else {
        return DeliveryUploadItemOutcome::Rejected {
            code: "delivery_not_found",
            message: format!("delivery `{}` was not found", item.delivery_id),
        };
    };
    if delivery.tenant_id != item.tenant_id || delivery.project_id != item.project_id {
        return DeliveryUploadItemOutcome::Rejected {
            code: "delivery_scope_mismatch",
            message: "delivery does not belong to the upload batch tenant/project".to_string(),
        };
    }
    let Some(entitlement) = store
        .delivery_entitlements
        .iter()
        .find(|entitlement| entitlement.delivery_id == delivery.delivery_id)
        .cloned()
    else {
        return DeliveryUploadItemOutcome::Failed {
            code: "delivery_entitlement_missing",
            message: "delivery entitlement is missing".to_string(),
        };
    };
    if delivery.effective_status(&entitlement) != DELIVERY_STATUS_PREPARED {
        return DeliveryUploadItemOutcome::Rejected {
            code: "delivery_not_artifact_ready",
            message: format!(
                "delivery `{}` status `{}` cannot accept artifacts",
                delivery.delivery_id,
                delivery.effective_status(&entitlement)
            ),
        };
    }
    if delivery.provider != upload_batch_provider_for_item(store, item).unwrap_or_default() {
        return DeliveryUploadItemOutcome::Rejected {
            code: "delivery_provider_mismatch",
            message: "delivery provider does not match upload batch provider".to_string(),
        };
    }
    if let Some(existing) = store.delivery_artifacts.iter().find(|artifact| {
        artifact.delivery_id == item.delivery_id
            && upload_item_matches_existing_artifact_payload(item, artifact)
            && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
    }) {
        return DeliveryUploadItemOutcome::Duplicate {
            artifact_id: existing.artifact_id.clone(),
        };
    }

    let artifact = insert_memory_delivery_artifact(
        store,
        delivery_upload_item_artifact_draft(item, &delivery, actor_id),
    );
    DeliveryUploadItemOutcome::Accepted {
        artifact_id: artifact.artifact_id,
    }
}

fn upload_item_matches_existing_artifact_payload(
    item: &DeliveryUploadBatchItemRecord,
    artifact: &DeliveryArtifactRecord,
) -> bool {
    artifact.sha256 == item.payload_sha256
}

fn upload_batch_provider_for_item(
    store: &MemoryStore,
    item: &DeliveryUploadBatchItemRecord,
) -> Option<String> {
    store
        .delivery_upload_batches
        .iter()
        .find(|batch| batch.batch_id == item.batch_id)
        .map(|batch| batch.provider.clone())
}

fn delivery_upload_item_artifact_draft(
    item: &DeliveryUploadBatchItemRecord,
    delivery: &DeliveryRecord,
    actor_id: &str,
) -> DeliveryArtifactDraft {
    DeliveryArtifactDraft {
        artifact_id: format!("artifact_{}_{}", item.batch_id, item.row_index),
        delivery_id: item.delivery_id.clone(),
        tenant_id: item.tenant_id.clone(),
        project_id: item.project_id.clone(),
        artifact_kind: item.artifact_kind.clone(),
        provider: delivery.provider.clone(),
        file_name: item.file_name.clone(),
        content_type: item.content_type.clone(),
        carrier_valid_until: item.carrier_valid_until.clone(),
        encryption_protocol: item.encryption_protocol.clone(),
        encryption_version: item.encryption_version.clone(),
        secret_kind: item.secret_kind.clone(),
        ciphertext: item.ciphertext.clone(),
        created_by: actor_id.to_string(),
    }
}

#[derive(Debug, Clone)]
enum DeliveryUploadItemOutcome {
    Accepted { artifact_id: String },
    Duplicate { artifact_id: String },
    Rejected { code: &'static str, message: String },
    Failed { code: &'static str, message: String },
}

fn apply_delivery_upload_item_outcome(
    item: &mut DeliveryUploadBatchItemRecord,
    outcome: DeliveryUploadItemOutcome,
) {
    let now = now_rfc3339();
    match outcome {
        DeliveryUploadItemOutcome::Accepted { artifact_id } => {
            item.status = DELIVERY_UPLOAD_ITEM_STATUS_ACCEPTED.to_string();
            item.artifact_id = Some(artifact_id);
            item.error_code = None;
            item.error_message = None;
        }
        DeliveryUploadItemOutcome::Duplicate { artifact_id } => {
            item.status = DELIVERY_UPLOAD_ITEM_STATUS_DUPLICATE.to_string();
            item.artifact_id = Some(artifact_id);
            item.error_code = Some("artifact_duplicate".to_string());
            item.error_message =
                Some("an active artifact with the same payload already exists".to_string());
        }
        DeliveryUploadItemOutcome::Rejected { code, message } => {
            item.status = DELIVERY_UPLOAD_ITEM_STATUS_REJECTED.to_string();
            item.error_code = Some(code.to_string());
            item.error_message = Some(message);
        }
        DeliveryUploadItemOutcome::Failed { code, message } => {
            item.status = DELIVERY_UPLOAD_ITEM_STATUS_FAILED.to_string();
            item.error_code = Some(code.to_string());
            item.error_message = Some(message);
        }
    }
    item.updated_at = now;
    item.version = item.version.saturating_add(1);
}

fn finalize_memory_delivery_upload_batch(store: &mut MemoryStore, batch_index: usize) {
    let batch_id = store.delivery_upload_batches[batch_index].batch_id.clone();
    let items = store
        .delivery_upload_batch_items
        .iter()
        .filter(|item| item.batch_id == batch_id)
        .collect::<Vec<_>>();
    let total_count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let success_count = count_upload_items(&items, DELIVERY_UPLOAD_ITEM_STATUS_ACCEPTED);
    let duplicate_count = count_upload_items(&items, DELIVERY_UPLOAD_ITEM_STATUS_DUPLICATE);
    let failed_count = items
        .iter()
        .filter(|item| {
            matches!(
                item.status.as_str(),
                DELIVERY_UPLOAD_ITEM_STATUS_REJECTED | DELIVERY_UPLOAD_ITEM_STATUS_FAILED
            )
        })
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    let status =
        final_upload_batch_status(total_count, success_count, duplicate_count, failed_count);
    let error_summary = first_upload_error_summary(&items);
    let now = now_rfc3339();
    let batch = &mut store.delivery_upload_batches[batch_index];
    batch.status = status;
    batch.total_count = total_count;
    batch.success_count = success_count;
    batch.duplicate_count = duplicate_count;
    batch.failed_count = failed_count;
    batch.error_summary = error_summary;
    batch.finished_at = Some(now.clone());
    batch.updated_at = now;
    batch.version = batch.version.saturating_add(1);
}

fn count_upload_items(items: &[&DeliveryUploadBatchItemRecord], status: &str) -> u32 {
    items
        .iter()
        .filter(|item| item.status == status)
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn final_upload_batch_status(
    total_count: u32,
    success_count: u32,
    duplicate_count: u32,
    failed_count: u32,
) -> String {
    if failed_count == 0 && success_count.saturating_add(duplicate_count) == total_count {
        DELIVERY_UPLOAD_BATCH_STATUS_SUCCEEDED.to_string()
    } else if success_count > 0 || duplicate_count > 0 {
        DELIVERY_UPLOAD_BATCH_STATUS_PARTIALLY_FAILED.to_string()
    } else {
        DELIVERY_UPLOAD_BATCH_STATUS_FAILED.to_string()
    }
}

fn first_upload_error_summary(items: &[&DeliveryUploadBatchItemRecord]) -> Option<String> {
    items.iter().find_map(|item| {
        item.error_message
            .clone()
            .or_else(|| item.error_code.clone())
    })
}

fn delivery_upload_batch_is_terminal(status: &str) -> bool {
    matches!(
        status,
        DELIVERY_UPLOAD_BATCH_STATUS_SUCCEEDED
            | DELIVERY_UPLOAD_BATCH_STATUS_PARTIALLY_FAILED
            | DELIVERY_UPLOAD_BATCH_STATUS_FAILED
    )
}

fn activate_delivery_entitlement(
    entitlement: &mut DeliveryEntitlementRecord,
    activated_at: Option<String>,
) {
    let activated_at = activated_at.unwrap_or_else(now_rfc3339);
    entitlement.starts_at = activated_at.clone();
    entitlement.ends_at = timestamp_plus_days(&activated_at, entitlement.service_days)
        .unwrap_or_else(|| expires_at(u64::from(entitlement.service_days) * 86_400));
    entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
    entitlement.activated_at = Some(activated_at);
    entitlement.updated_at = now_rfc3339();
    entitlement.version = entitlement.version.saturating_add(1);
}

fn apply_segment_for_artifact(
    store: &mut MemoryStore,
    entitlement: &mut DeliveryEntitlementRecord,
    activation: &DeliveryActivationRecord,
    artifact: &DeliveryArtifactRecord,
    actor_id: &str,
    reason: &str,
) {
    match build_service_segment_record(
        store,
        entitlement,
        activation,
        artifact,
        activation.activated_at.clone(),
        actor_id,
    ) {
        Some(segment) => {
            let event = lifecycle_event_record(
                store,
                &entitlement.entitlement_id,
                Some(segment.segment_id.clone()),
                DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED,
                DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE,
                Some(reason.to_string()),
                actor_id,
                BTreeMap::from([
                    ("artifact_id".to_string(), artifact.artifact_id.clone()),
                    (
                        "effective_until".to_string(),
                        segment.effective_until.clone(),
                    ),
                ]),
            );
            store.delivery_service_segments.push(segment);
            store.delivery_lifecycle_events.push(event);
        }
        None => {
            mark_entitlement_needs_manual_supply(store, entitlement, actor_id, reason);
        }
    }
}

fn build_service_segment_record(
    store: &MemoryStore,
    entitlement: &DeliveryEntitlementRecord,
    activation: &DeliveryActivationRecord,
    artifact: &DeliveryArtifactRecord,
    effective_from: String,
    actor_id: &str,
) -> Option<DeliveryServiceSegmentRecord> {
    let carrier_valid_until = artifact.carrier_valid_until.clone()?;
    if !timestamp_after(&carrier_valid_until, &effective_from) {
        return None;
    }
    let effective_until = min_timestamp(&carrier_valid_until, &entitlement.ends_at)?;
    if !timestamp_after(&effective_until, &effective_from) {
        return None;
    }
    let segment_index = next_segment_index(
        &store.delivery_service_segments,
        &entitlement.entitlement_id,
    );
    let now = now_rfc3339();
    Some(DeliveryServiceSegmentRecord {
        segment_id: format!("dlvseg_{}_{}", entitlement.entitlement_id, segment_index),
        entitlement_id: entitlement.entitlement_id.clone(),
        activation_id: activation.activation_id.clone(),
        delivery_id: artifact.delivery_id.clone(),
        artifact_id: artifact.artifact_id.clone(),
        tenant_id: artifact.tenant_id.clone(),
        project_id: artifact.project_id.clone(),
        provider: artifact.provider.clone(),
        status: if timestamp_is_future(&effective_from) {
            DELIVERY_SERVICE_SEGMENT_STATUS_SCHEDULED.to_string()
        } else {
            DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE.to_string()
        },
        segment_index,
        effective_from,
        effective_until,
        carrier_valid_until,
        created_by: actor_id.to_string(),
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    })
}

fn next_segment_index(segments: &[DeliveryServiceSegmentRecord], entitlement_id: &str) -> u32 {
    segments
        .iter()
        .filter(|segment| segment.entitlement_id == entitlement_id)
        .map(|segment| segment.segment_index)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn mark_entitlement_needs_manual_supply(
    store: &mut MemoryStore,
    entitlement: &mut DeliveryEntitlementRecord,
    actor_id: &str,
    reason: &str,
) {
    let already_blocked = entitlement.status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY;
    entitlement.status = DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY.to_string();
    entitlement.updated_at = now_rfc3339();
    if !already_blocked {
        entitlement.version = entitlement.version.saturating_add(1);
    }
    if already_blocked {
        return;
    }
    let event = lifecycle_event_record(
        store,
        &entitlement.entitlement_id,
        None,
        DELIVERY_LIFECYCLE_EVENT_NEEDS_MANUAL_SUPPLY,
        DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY,
        Some(reason.to_string()),
        actor_id,
        BTreeMap::new(),
    );
    store.delivery_lifecycle_events.push(event);
}

fn lifecycle_event_record(
    store: &MemoryStore,
    entitlement_id: &str,
    segment_id: Option<String>,
    event_type: &str,
    status: &str,
    reason: Option<String>,
    actor_id: &str,
    payload: BTreeMap<String, String>,
) -> DeliveryLifecycleEventRecord {
    DeliveryLifecycleEventRecord {
        event_id: format!(
            "dlvevt_{}_{}",
            entitlement_id,
            store.delivery_lifecycle_events.len().saturating_add(1)
        ),
        entitlement_id: entitlement_id.to_string(),
        segment_id,
        event_type: event_type.to_string(),
        status: status.to_string(),
        reason,
        created_by: actor_id.to_string(),
        created_at: now_rfc3339(),
        payload,
    }
}

fn postgres_lifecycle_event_record(
    entitlement_id: &str,
    segment_id: Option<String>,
    event_type: &str,
    status: &str,
    reason: Option<String>,
    actor_id: &str,
    payload: BTreeMap<String, String>,
) -> DeliveryLifecycleEventRecord {
    DeliveryLifecycleEventRecord {
        event_id: format!("dlvevt_{}_{}", entitlement_id, next_id_suffix()),
        entitlement_id: entitlement_id.to_string(),
        segment_id,
        event_type: event_type.to_string(),
        status: status.to_string(),
        reason,
        created_by: actor_id.to_string(),
        created_at: now_rfc3339(),
        payload,
    }
}

fn build_postgres_service_segment_record(
    existing_segments: &[DeliveryServiceSegmentRecord],
    new_segments: &[DeliveryServiceSegmentRecord],
    entitlement: &DeliveryEntitlementRecord,
    activation: &DeliveryActivationRecord,
    artifact: &DeliveryArtifactRecord,
    effective_from: String,
    actor_id: &str,
) -> Option<DeliveryServiceSegmentRecord> {
    let carrier_valid_until = artifact.carrier_valid_until.clone()?;
    if !timestamp_after(&carrier_valid_until, &effective_from) {
        return None;
    }
    let effective_until = min_timestamp(&carrier_valid_until, &entitlement.ends_at)?;
    if !timestamp_after(&effective_until, &effective_from) {
        return None;
    }
    let segment_index = existing_segments
        .iter()
        .chain(new_segments.iter())
        .filter(|segment| segment.entitlement_id == entitlement.entitlement_id)
        .map(|segment| segment.segment_index)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let now = now_rfc3339();
    Some(DeliveryServiceSegmentRecord {
        segment_id: format!("dlvseg_{}_{}", entitlement.entitlement_id, segment_index),
        entitlement_id: entitlement.entitlement_id.clone(),
        activation_id: activation.activation_id.clone(),
        delivery_id: artifact.delivery_id.clone(),
        artifact_id: artifact.artifact_id.clone(),
        tenant_id: artifact.tenant_id.clone(),
        project_id: artifact.project_id.clone(),
        provider: artifact.provider.clone(),
        status: if timestamp_is_future(&effective_from) {
            DELIVERY_SERVICE_SEGMENT_STATUS_SCHEDULED.to_string()
        } else {
            DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE.to_string()
        },
        segment_index,
        effective_from,
        effective_until,
        carrier_valid_until,
        created_by: actor_id.to_string(),
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    })
}

fn active_segment_for_artifact<'a>(
    segments: &'a [DeliveryServiceSegmentRecord],
    entitlement_id: &str,
    activation_id: &str,
    artifact_id: &str,
) -> Option<&'a DeliveryServiceSegmentRecord> {
    segments
        .iter()
        .filter(|segment| {
            segment.entitlement_id == entitlement_id
                && segment.activation_id == activation_id
                && segment.artifact_id == artifact_id
                && segment.effective_status() == DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE
                && !timestamp_is_future(&segment.effective_from)
                && !timestamp_is_expired(&segment.effective_until)
        })
        .max_by(|left, right| left.segment_index.cmp(&right.segment_index))
}

fn current_segment_for_entitlement<'a>(
    segments: &'a [DeliveryServiceSegmentRecord],
    entitlement_id: &str,
    activation_id: &str,
) -> Option<&'a DeliveryServiceSegmentRecord> {
    segments
        .iter()
        .filter(|segment| {
            segment.entitlement_id == entitlement_id
                && segment.activation_id == activation_id
                && segment.effective_status() == DELIVERY_SERVICE_SEGMENT_STATUS_ACTIVE
                && !timestamp_is_future(&segment.effective_from)
                && !timestamp_is_expired(&segment.effective_until)
        })
        .max_by(|left, right| {
            left.effective_from
                .cmp(&right.effective_from)
                .then_with(|| left.segment_index.cmp(&right.segment_index))
        })
}

fn lifecycle_response_for_entitlement(
    store: &MemoryStore,
    entitlement: &DeliveryEntitlementRecord,
) -> DeliveryLifecycleResponse {
    let mut segments = store
        .delivery_service_segments
        .iter()
        .filter(|segment| segment.entitlement_id == entitlement.entitlement_id)
        .map(DeliveryServiceSegmentRecord::public_view)
        .collect::<Vec<_>>();
    segments.sort_by(|left, right| left.segment_index.cmp(&right.segment_index));
    let mut events = store
        .delivery_lifecycle_events
        .iter()
        .filter(|event| event.entitlement_id == entitlement.entitlement_id)
        .map(DeliveryLifecycleEventRecord::public_view)
        .collect::<Vec<_>>();
    events.sort_by(|left, right| left.created_at.cmp(&right.created_at));
    DeliveryLifecycleResponse {
        entitlement: entitlement.public_view(),
        segments,
        events,
    }
}

fn reconcile_memory_delivery_lifecycle(
    store: &mut MemoryStore,
    entitlement_id: &str,
    actor_id: &str,
) -> DeliveryLifecycleResult {
    let Some(entitlement_index) = store
        .delivery_entitlements
        .iter()
        .position(|entitlement| entitlement.entitlement_id == entitlement_id)
    else {
        return DeliveryLifecycleResult::EntitlementNotFound;
    };
    let mut entitlement = store.delivery_entitlements[entitlement_index].clone();
    if entitlement.status == DELIVERY_ENTITLEMENT_STATUS_REVOKED
        || entitlement.status == DELIVERY_ENTITLEMENT_STATUS_SUSPENDED
    {
        return DeliveryLifecycleResult::Applied(Box::new(lifecycle_response_for_entitlement(
            store,
            &entitlement,
        )));
    }

    if timestamp_is_expired(&entitlement.ends_at) {
        if entitlement.status != DELIVERY_STATUS_EXPIRED {
            entitlement.status = DELIVERY_STATUS_EXPIRED.to_string();
            entitlement.updated_at = now_rfc3339();
            entitlement.version = entitlement.version.saturating_add(1);
            let event = lifecycle_event_record(
                store,
                &entitlement.entitlement_id,
                None,
                DELIVERY_LIFECYCLE_EVENT_EXPIRED,
                DELIVERY_STATUS_EXPIRED,
                Some("entitlement service window ended".to_string()),
                actor_id,
                BTreeMap::new(),
            );
            store.delivery_lifecycle_events.push(event);
        }
        store.delivery_entitlements[entitlement_index] = entitlement.clone();
        return DeliveryLifecycleResult::Applied(Box::new(lifecycle_response_for_entitlement(
            store,
            &entitlement,
        )));
    }

    let Some(activation) = store
        .delivery_activations
        .iter()
        .find(|activation| {
            activation.entitlement_id == entitlement.entitlement_id
                && activation.status == DELIVERY_ACTIVATION_STATUS_ACTIVATED
        })
        .cloned()
    else {
        mark_entitlement_needs_manual_supply(
            store,
            &mut entitlement,
            actor_id,
            "activation_missing",
        );
        store.delivery_entitlements[entitlement_index] = entitlement.clone();
        return DeliveryLifecycleResult::Applied(Box::new(lifecycle_response_for_entitlement(
            store,
            &entitlement,
        )));
    };

    let coverage_until = store
        .delivery_service_segments
        .iter()
        .filter(|segment| {
            segment.entitlement_id == entitlement.entitlement_id
                && segment.status != DELIVERY_SERVICE_SEGMENT_STATUS_REVOKED
        })
        .map(|segment| segment.effective_until.clone())
        .max()
        .unwrap_or_else(|| entitlement.starts_at.clone());

    if !timestamp_after(&entitlement.ends_at, &coverage_until) {
        if entitlement.status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
            entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
            entitlement.updated_at = now_rfc3339();
            entitlement.version = entitlement.version.saturating_add(1);
        }
        store.delivery_entitlements[entitlement_index] = entitlement.clone();
        return DeliveryLifecycleResult::Applied(Box::new(lifecycle_response_for_entitlement(
            store,
            &entitlement,
        )));
    }

    let used_artifact_ids = store
        .delivery_service_segments
        .iter()
        .map(|segment| segment.artifact_id.clone())
        .collect::<HashSet<_>>();
    let candidate = store
        .delivery_artifacts
        .iter()
        .filter(|artifact| {
            artifact.tenant_id == activation.tenant_id
                && artifact.project_id == activation.project_id
                && artifact.provider == activation.provider
                && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
                && !used_artifact_ids.contains(&artifact.artifact_id)
                && artifact
                    .carrier_valid_until
                    .as_deref()
                    .is_some_and(|value| timestamp_after(value, &coverage_until))
        })
        .max_by(|left, right| left.version.cmp(&right.version))
        .cloned();
    let Some(artifact) = candidate else {
        mark_entitlement_needs_manual_supply(
            store,
            &mut entitlement,
            actor_id,
            "continuation_artifact_missing",
        );
        store.delivery_entitlements[entitlement_index] = entitlement.clone();
        return DeliveryLifecycleResult::Applied(Box::new(lifecycle_response_for_entitlement(
            store,
            &entitlement,
        )));
    };

    if let Some(segment) = build_service_segment_record(
        store,
        &entitlement,
        &activation,
        &artifact,
        coverage_until,
        actor_id,
    ) {
        let event = lifecycle_event_record(
            store,
            &entitlement.entitlement_id,
            Some(segment.segment_id.clone()),
            DELIVERY_LIFECYCLE_EVENT_SEGMENT_CREATED,
            segment.status.as_str(),
            Some("continuation_artifact_matched".to_string()),
            actor_id,
            BTreeMap::from([("artifact_id".to_string(), artifact.artifact_id.clone())]),
        );
        store.delivery_service_segments.push(segment);
        store.delivery_lifecycle_events.push(event);
        if entitlement.status == DELIVERY_ENTITLEMENT_STATUS_NEEDS_MANUAL_SUPPLY {
            entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
            entitlement.updated_at = now_rfc3339();
            entitlement.version = entitlement.version.saturating_add(1);
        }
    } else {
        mark_entitlement_needs_manual_supply(
            store,
            &mut entitlement,
            actor_id,
            "continuation_artifact_invalid_carrier_window",
        );
    }
    store.delivery_entitlements[entitlement_index] = entitlement.clone();
    DeliveryLifecycleResult::Applied(Box::new(lifecycle_response_for_entitlement(
        store,
        &entitlement,
    )))
}

fn extend_memory_delivery_entitlement(
    store: &mut MemoryStore,
    draft: DeliveryEntitlementExtendDraft,
) -> ConcurrencyResult<DeliveryLifecycleResponse> {
    let Some(entitlement_index) = store
        .delivery_entitlements
        .iter()
        .position(|entitlement| entitlement.entitlement_id == draft.entitlement_id)
    else {
        return ConcurrencyResult::NotFound;
    };
    let mut entitlement = store.delivery_entitlements[entitlement_index].clone();
    if entitlement.version != draft.expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let old_ends_at = entitlement.ends_at.clone();
    entitlement.ends_at = timestamp_plus_days(&entitlement.ends_at, draft.extend_days)
        .unwrap_or_else(|| expires_at(u64::from(draft.extend_days) * 86_400));
    entitlement.service_days = entitlement.service_days.saturating_add(draft.extend_days);
    if entitlement.status != DELIVERY_ENTITLEMENT_STATUS_REVOKED
        && entitlement.status != DELIVERY_ENTITLEMENT_STATUS_SUSPENDED
        && !timestamp_is_expired(&entitlement.ends_at)
        && entitlement.activated_at.is_some()
    {
        entitlement.status = DELIVERY_ENTITLEMENT_STATUS_ACTIVE.to_string();
    }
    entitlement.updated_at = now_rfc3339();
    entitlement.version = entitlement.version.saturating_add(1);
    let event = lifecycle_event_record(
        store,
        &entitlement.entitlement_id,
        None,
        DELIVERY_LIFECYCLE_EVENT_RENEWED,
        entitlement.status.as_str(),
        draft.reason.clone(),
        &draft.actor_id,
        BTreeMap::from([
            ("old_service_ends_at".to_string(), old_ends_at),
            (
                "new_service_ends_at".to_string(),
                entitlement.ends_at.clone(),
            ),
            ("extend_days".to_string(), draft.extend_days.to_string()),
        ]),
    );
    store.delivery_lifecycle_events.push(event);
    store.delivery_entitlements[entitlement_index] = entitlement.clone();
    let _ =
        reconcile_memory_delivery_lifecycle(store, &entitlement.entitlement_id, &draft.actor_id);
    let entitlement = store.delivery_entitlements[entitlement_index].clone();
    ConcurrencyResult::Applied(lifecycle_response_for_entitlement(store, &entitlement))
}

fn redeem_memory_delivery_activation(
    store: &mut MemoryStore,
    code_hash: &str,
    activation_id: String,
    activated_by: &str,
) -> DeliveryRedeemResult {
    let Some(code_index) = store.delivery_codes.iter().position(|code| {
        code.code_type == DELIVERY_CODE_TYPE_REDEMPTION && code.code_hash == code_hash
    }) else {
        return DeliveryRedeemResult::RedemptionCodeNotFound;
    };
    let mut code = store.delivery_codes[code_index].clone();
    if let Some(result) = blocked_redemption_code_result(&code) {
        return result;
    }

    let Some(delivery) = store
        .deliveries
        .iter()
        .find(|delivery| delivery.delivery_id == code.delivery_id)
        .cloned()
    else {
        return DeliveryRedeemResult::DeliveryNotFound;
    };
    let Some(entitlement_index) = store
        .delivery_entitlements
        .iter()
        .position(|entitlement| entitlement.delivery_id == code.delivery_id)
    else {
        return DeliveryRedeemResult::EntitlementNotFound;
    };
    let entitlement = store.delivery_entitlements[entitlement_index].clone();
    if let Some(result) = blocked_delivery_activation_result(&delivery, &entitlement) {
        return result;
    }

    let Some(artifact) = store
        .delivery_artifacts
        .iter()
        .filter(|artifact| {
            artifact.delivery_id == code.delivery_id
                && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
        })
        .max_by(|left, right| left.version.cmp(&right.version))
        .cloned()
    else {
        return DeliveryRedeemResult::ArtifactMissing;
    };

    let mut entitlement = entitlement;
    activate_delivery_entitlement(&mut entitlement, None);
    let activation = build_delivery_activation_record(
        &activation_id,
        &delivery,
        &mut code,
        &entitlement,
        &artifact,
    );
    apply_segment_for_artifact(
        store,
        &mut entitlement,
        &activation,
        &artifact,
        activated_by,
        "activation_segment",
    );
    store.delivery_codes[code_index] = code;
    store.delivery_entitlements[entitlement_index] = entitlement;
    store.delivery_activations.push(activation.clone());
    DeliveryRedeemResult::Activated(Box::new(DeliveryActivationResponse {
        data: activation.public_view(),
    }))
}

fn blocked_redemption_code_result(code: &DeliveryCodeRecord) -> Option<DeliveryRedeemResult> {
    if code.status == DELIVERY_CODE_STATUS_USED || code.used_at.is_some() {
        Some(DeliveryRedeemResult::RedemptionCodeUsed)
    } else if code.status == DELIVERY_STATUS_REVOKED {
        Some(DeliveryRedeemResult::RedemptionCodeRevoked)
    } else if code.status == DELIVERY_CODE_STATUS_ACTIVE && timestamp_is_expired(&code.expires_at) {
        Some(DeliveryRedeemResult::RedemptionCodeExpired)
    } else if code.status != DELIVERY_CODE_STATUS_ACTIVE {
        Some(DeliveryRedeemResult::RedemptionCodeNotActive(
            code.status.clone(),
        ))
    } else {
        None
    }
}

fn blocked_delivery_activation_result(
    delivery: &DeliveryRecord,
    entitlement: &DeliveryEntitlementRecord,
) -> Option<DeliveryRedeemResult> {
    let delivery_status = delivery.effective_status(entitlement);
    if delivery_status != DELIVERY_STATUS_PREPARED {
        return Some(DeliveryRedeemResult::DeliveryNotActive(delivery_status));
    }
    let entitlement_status = entitlement.effective_status();
    if !matches!(
        entitlement_status.as_str(),
        DELIVERY_ENTITLEMENT_STATUS_ACTIVE | DELIVERY_ENTITLEMENT_STATUS_PENDING_ACTIVATION
    ) {
        return Some(DeliveryRedeemResult::EntitlementNotActive(
            entitlement_status,
        ));
    }
    None
}

fn build_delivery_activation_record(
    activation_id: &str,
    delivery: &DeliveryRecord,
    code: &mut DeliveryCodeRecord,
    entitlement: &DeliveryEntitlementRecord,
    artifact: &DeliveryArtifactRecord,
) -> DeliveryActivationRecord {
    let now = entitlement.activated_at.clone().unwrap_or_else(now_rfc3339);
    code.status = DELIVERY_CODE_STATUS_USED.to_string();
    code.used_at = Some(now.clone());
    code.updated_at = now.clone();
    code.version = code.version.saturating_add(1);
    DeliveryActivationRecord {
        activation_id: activation_id.to_string(),
        delivery_id: delivery.delivery_id.clone(),
        code_id: code.code_id.clone(),
        entitlement_id: entitlement.entitlement_id.clone(),
        artifact_id: artifact.artifact_id.clone(),
        tenant_id: delivery.tenant_id.clone(),
        project_id: delivery.project_id.clone(),
        provider: delivery.provider.clone(),
        status: DELIVERY_ACTIVATION_STATUS_ACTIVATED.to_string(),
        activation_source: DELIVERY_ACTIVATION_SOURCE_REDEMPTION_CODE.to_string(),
        activated_at: now.clone(),
        entitlement_ends_at: entitlement.ends_at.clone(),
        artifact: artifact.public_view(),
        created_at: now.clone(),
        updated_at: now,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
    }
}

fn issue_memory_delivery_download_grant(
    store: &mut MemoryStore,
    draft: DeliveryDownloadGrantDraft,
) -> DeliveryDownloadGrantIssueResult {
    let Some(activation) = store
        .delivery_activations
        .iter()
        .find(|activation| activation.activation_id == draft.activation_id)
        .cloned()
    else {
        return DeliveryDownloadGrantIssueResult::ActivationNotFound;
    };
    if let Some(result) = blocked_download_activation_issue_result(&activation) {
        return result;
    }
    let Some(entitlement) = store
        .delivery_entitlements
        .iter()
        .find(|entitlement| entitlement.entitlement_id == activation.entitlement_id)
        .cloned()
    else {
        return DeliveryDownloadGrantIssueResult::EntitlementNotFound;
    };
    if let Some(result) = blocked_download_entitlement_issue_result(&entitlement) {
        return result;
    }
    let Some(segment) = current_segment_for_entitlement(
        &store.delivery_service_segments,
        &entitlement.entitlement_id,
        &activation.activation_id,
    )
    .cloned() else {
        return DeliveryDownloadGrantIssueResult::SegmentMissing;
    };
    let Some(artifact) = store
        .delivery_artifacts
        .iter()
        .find(|artifact| {
            artifact.delivery_id == activation.delivery_id
                && artifact.artifact_id == segment.artifact_id
        })
        .cloned()
    else {
        return DeliveryDownloadGrantIssueResult::ArtifactMissing;
    };
    if let Some(result) = blocked_download_artifact_issue_result(&artifact) {
        return result;
    }

    let grant = build_delivery_download_grant_record(&draft, &activation, &artifact);
    store.delivery_download_grants.push(grant.clone());
    DeliveryDownloadGrantIssueResult::Issued(Box::new(DeliveryDownloadGrantIssueResponse {
        data: grant.public_view(),
        download_token: draft.token_plaintext,
    }))
}

fn revoke_memory_delivery_download_grant(
    store: &mut MemoryStore,
    grant_id: &str,
    expected_version: u64,
    revoked_by: &str,
    revoke_reason: Option<String>,
) -> ConcurrencyResult<DeliveryDownloadGrantResponse> {
    let Some(index) = store
        .delivery_download_grants
        .iter()
        .position(|grant| grant.grant_id == grant_id)
    else {
        return ConcurrencyResult::NotFound;
    };
    if store.delivery_download_grants[index].version != expected_version {
        return ConcurrencyResult::VersionConflict;
    }
    let now = now_rfc3339();
    let grant = &mut store.delivery_download_grants[index];
    grant.status = DELIVERY_DOWNLOAD_GRANT_STATUS_REVOKED.to_string();
    grant.revoked_at = Some(now.clone());
    grant.revoked_by = Some(revoked_by.to_string());
    grant.revoke_reason = revoke_reason;
    grant.updated_at = now;
    grant.version = grant.version.saturating_add(1);
    ConcurrencyResult::Applied(DeliveryDownloadGrantResponse {
        data: grant.public_view(),
    })
}

fn consume_memory_delivery_download_grant(
    store: &mut MemoryStore,
    token_hash: &str,
) -> DeliveryDownloadConsumeResult {
    let Some(grant_index) = store
        .delivery_download_grants
        .iter()
        .position(|grant| grant.token_hash == token_hash)
    else {
        return DeliveryDownloadConsumeResult::GrantNotFound;
    };
    let mut grant = store.delivery_download_grants[grant_index].clone();
    if let Some(result) = blocked_download_grant_consume_result(&mut grant) {
        store.delivery_download_grants[grant_index] = grant;
        return result;
    }
    let Some(activation) = store
        .delivery_activations
        .iter()
        .find(|activation| activation.activation_id == grant.activation_id)
    else {
        return DeliveryDownloadConsumeResult::ActivationNotFound;
    };
    if let Some(result) = blocked_download_activation_consume_result(activation) {
        return result;
    }
    let Some(entitlement) = store
        .delivery_entitlements
        .iter()
        .find(|entitlement| entitlement.entitlement_id == grant.entitlement_id)
    else {
        return DeliveryDownloadConsumeResult::EntitlementNotFound;
    };
    if let Some(result) = blocked_download_entitlement_consume_result(entitlement) {
        return result;
    }
    if active_segment_for_artifact(
        &store.delivery_service_segments,
        &grant.entitlement_id,
        &grant.activation_id,
        &grant.artifact_id,
    )
    .is_none()
    {
        return DeliveryDownloadConsumeResult::SegmentMissing;
    }
    let Some(artifact) = store.delivery_artifacts.iter().find(|artifact| {
        artifact.delivery_id == grant.delivery_id && artifact.artifact_id == grant.artifact_id
    }) else {
        return DeliveryDownloadConsumeResult::ArtifactMissing;
    };
    if let Some(result) = blocked_download_artifact_consume_result(artifact) {
        return result;
    }
    let payload =
        mark_download_grant_used_and_payload(&mut grant, artifact, artifact.ciphertext.clone());
    store.delivery_download_grants[grant_index] = grant;
    DeliveryDownloadConsumeResult::Retrieved(Box::new(payload))
}

fn build_delivery_download_grant_record(
    draft: &DeliveryDownloadGrantDraft,
    activation: &DeliveryActivationRecord,
    artifact: &DeliveryArtifactRecord,
) -> DeliveryDownloadGrantRecord {
    let now = now_rfc3339();
    DeliveryDownloadGrantRecord {
        grant_id: draft.grant_id.clone(),
        activation_id: activation.activation_id.clone(),
        delivery_id: artifact.delivery_id.clone(),
        artifact_id: artifact.artifact_id.clone(),
        entitlement_id: activation.entitlement_id.clone(),
        tenant_id: activation.tenant_id.clone(),
        project_id: activation.project_id.clone(),
        provider: activation.provider.clone(),
        status: DELIVERY_DOWNLOAD_GRANT_STATUS_ACTIVE.to_string(),
        token_hash: hash_api_key(&draft.token_plaintext),
        token_prefix: download_token_prefix(&draft.token_plaintext),
        token_last_four: credential_last_four(&draft.token_plaintext),
        expires_at: expires_at(DELIVERY_DOWNLOAD_GRANT_TTL_SECONDS),
        max_uses: DELIVERY_DOWNLOAD_GRANT_MAX_USES,
        use_count: 0,
        artifact: artifact.public_view(),
        created_by: draft.created_by.clone(),
        created_at: now.clone(),
        updated_at: now,
        version: 1,
        used_at: None,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
    }
}

fn blocked_download_activation_issue_result(
    activation: &DeliveryActivationRecord,
) -> Option<DeliveryDownloadGrantIssueResult> {
    if activation.status != DELIVERY_ACTIVATION_STATUS_ACTIVATED || activation.revoked_at.is_some()
    {
        Some(DeliveryDownloadGrantIssueResult::ActivationNotActive(
            activation.status.clone(),
        ))
    } else {
        None
    }
}

fn blocked_download_activation_consume_result(
    activation: &DeliveryActivationRecord,
) -> Option<DeliveryDownloadConsumeResult> {
    if activation.status != DELIVERY_ACTIVATION_STATUS_ACTIVATED || activation.revoked_at.is_some()
    {
        Some(DeliveryDownloadConsumeResult::ActivationNotActive(
            activation.status.clone(),
        ))
    } else {
        None
    }
}

fn blocked_download_entitlement_issue_result(
    entitlement: &DeliveryEntitlementRecord,
) -> Option<DeliveryDownloadGrantIssueResult> {
    let status = entitlement.effective_status();
    if status == DELIVERY_ENTITLEMENT_STATUS_ACTIVE {
        None
    } else {
        Some(DeliveryDownloadGrantIssueResult::EntitlementNotActive(
            status,
        ))
    }
}

fn blocked_download_entitlement_consume_result(
    entitlement: &DeliveryEntitlementRecord,
) -> Option<DeliveryDownloadConsumeResult> {
    let status = entitlement.effective_status();
    if status == DELIVERY_ENTITLEMENT_STATUS_ACTIVE {
        None
    } else {
        Some(DeliveryDownloadConsumeResult::EntitlementNotActive(status))
    }
}

fn blocked_download_artifact_issue_result(
    artifact: &DeliveryArtifactRecord,
) -> Option<DeliveryDownloadGrantIssueResult> {
    if artifact.status == DELIVERY_ARTIFACT_STATUS_REVOKED {
        Some(DeliveryDownloadGrantIssueResult::ArtifactNotAvailable(
            artifact.status.clone(),
        ))
    } else if matches!(
        artifact.status.as_str(),
        DELIVERY_ARTIFACT_STATUS_ACTIVE | DELIVERY_ARTIFACT_STATUS_SUPERSEDED
    ) {
        None
    } else {
        Some(DeliveryDownloadGrantIssueResult::ArtifactNotAvailable(
            artifact.status.clone(),
        ))
    }
}

fn blocked_download_artifact_consume_result(
    artifact: &DeliveryArtifactRecord,
) -> Option<DeliveryDownloadConsumeResult> {
    if artifact.status == DELIVERY_ARTIFACT_STATUS_REVOKED {
        Some(DeliveryDownloadConsumeResult::ArtifactNotAvailable(
            artifact.status.clone(),
        ))
    } else if matches!(
        artifact.status.as_str(),
        DELIVERY_ARTIFACT_STATUS_ACTIVE | DELIVERY_ARTIFACT_STATUS_SUPERSEDED
    ) {
        None
    } else {
        Some(DeliveryDownloadConsumeResult::ArtifactNotAvailable(
            artifact.status.clone(),
        ))
    }
}

fn blocked_download_grant_consume_result(
    grant: &mut DeliveryDownloadGrantRecord,
) -> Option<DeliveryDownloadConsumeResult> {
    if grant.status == DELIVERY_DOWNLOAD_GRANT_STATUS_USED || grant.use_count >= grant.max_uses {
        Some(DeliveryDownloadConsumeResult::GrantUsed)
    } else if grant.status == DELIVERY_DOWNLOAD_GRANT_STATUS_REVOKED {
        Some(DeliveryDownloadConsumeResult::GrantRevoked)
    } else if grant.status == DELIVERY_DOWNLOAD_GRANT_STATUS_ACTIVE
        && timestamp_is_expired(&grant.expires_at)
    {
        grant.status = DELIVERY_DOWNLOAD_GRANT_STATUS_EXPIRED.to_string();
        grant.updated_at = now_rfc3339();
        grant.version = grant.version.saturating_add(1);
        Some(DeliveryDownloadConsumeResult::GrantExpired)
    } else if grant.status != DELIVERY_DOWNLOAD_GRANT_STATUS_ACTIVE {
        Some(DeliveryDownloadConsumeResult::GrantNotActive(
            grant.status.clone(),
        ))
    } else {
        None
    }
}

fn mark_download_grant_used_and_payload(
    grant: &mut DeliveryDownloadGrantRecord,
    artifact: &DeliveryArtifactRecord,
    ciphertext: Vec<u8>,
) -> DeliveryDownloadArtifactPayload {
    let now = now_rfc3339();
    grant.status = DELIVERY_DOWNLOAD_GRANT_STATUS_USED.to_string();
    grant.use_count = grant.use_count.saturating_add(1);
    grant.used_at = Some(now.clone());
    grant.updated_at = now;
    grant.version = grant.version.saturating_add(1);
    DeliveryDownloadArtifactPayload {
        grant: grant.public_view(),
        file_name: artifact_download_file_name(artifact),
        content_type: artifact.content_type.clone(),
        sha256: artifact.sha256.clone(),
        size_bytes: artifact.size_bytes,
        ciphertext,
    }
}

fn artifact_download_file_name(artifact: &DeliveryArtifactRecord) -> String {
    artifact
        .file_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("delivery-artifact.hcbrowser")
        .to_string()
}

fn download_token_prefix(token: &str) -> String {
    if token.len() <= 18 {
        token.to_string()
    } else {
        format!("{}...", &token[..18])
    }
}

fn normalize_oauth_sharing_lease(record: &mut OAuthSharingLeaseRecord) {
    let now = now_rfc3339();
    if record.created_at.is_empty() {
        record.created_at = now.clone();
    }
    record.updated_at = now;
    if record.version == 0 {
        record.version = 1;
    }
    if record.policy.is_empty() {
        record.policy = "fair_share".to_string();
    }
    if record.status.is_empty() {
        record.status = "active".to_string();
    }
    if record.max_concurrent_runs == 0 {
        record.max_concurrent_runs = 1;
    }
    if record.metadata.is_null() {
        record.metadata = serde_json::json!({});
    }
}

fn normalize_oauth_carpool(record: &mut OAuthCarpoolRecord) {
    let now = now_rfc3339();
    if record.created_at.is_empty() {
        record.created_at = now.clone();
    }
    record.updated_at = now;
    if record.version == 0 {
        record.version = 1;
    }
    if record.strategy.is_empty() {
        record.strategy = "fair_share".to_string();
    }
    if record.metadata.is_null() {
        record.metadata = serde_json::json!({});
    }
}

fn upsert_memory_oauth_sharing_lease(
    store: &mut MemoryStore,
    mut record: OAuthSharingLeaseRecord,
) -> OAuthSharingLeaseRecord {
    if let Some(existing) = store
        .oauth_sharing_leases
        .iter_mut()
        .find(|item| item.lease_id == record.lease_id)
    {
        record.created_at = existing.created_at.clone();
        record.version = existing.version.saturating_add(1);
        *existing = record.clone();
    } else {
        store.oauth_sharing_leases.push(record.clone());
    }
    push_oauth_audit_event(
        store,
        "lease_upsert",
        &record.provider,
        Some(&record.lease_id),
        None,
        Some(&record.borrower_workspace_id),
        None,
        Some(&record.pool_id),
        "lease upserted by control plane",
        serde_json::json!({ "status": record.status }),
    );
    record
}

fn revoke_memory_oauth_sharing_lease(
    store: &mut MemoryStore,
    lease_id: &str,
) -> Option<OAuthSharingLeaseRecord> {
    let index = store
        .oauth_sharing_leases
        .iter()
        .position(|item| item.lease_id == lease_id)?;
    let mut record = store.oauth_sharing_leases[index].clone();
    record.status = "revoked".to_string();
    record.updated_at = now_rfc3339();
    record.version = record.version.saturating_add(1);
    store.oauth_sharing_leases[index] = record.clone();
    push_oauth_audit_event(
        store,
        "lease_revoke",
        &record.provider,
        Some(&record.lease_id),
        None,
        Some(&record.borrower_workspace_id),
        None,
        Some(&record.pool_id),
        "lease revoked by control plane",
        serde_json::json!({}),
    );
    Some(record)
}

fn upsert_memory_oauth_carpool(
    store: &mut MemoryStore,
    mut record: OAuthCarpoolRecord,
) -> OAuthCarpoolRecord {
    if let Some(existing) = store
        .oauth_carpools
        .iter_mut()
        .find(|item| item.carpool_id == record.carpool_id)
    {
        record.created_at = existing.created_at.clone();
        record.version = existing.version.saturating_add(1);
        *existing = record.clone();
    } else {
        store.oauth_carpools.push(record.clone());
    }
    push_oauth_audit_event(
        store,
        "carpool_upsert",
        &record.provider,
        None,
        Some(&record.carpool_id),
        None,
        None,
        None,
        "carpool upserted by control plane",
        serde_json::json!({ "enabled": record.enabled }),
    );
    record
}

type OAuthSharingUsageKey = (
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

fn remove_memory_oauth_carpool(
    store: &mut MemoryStore,
    carpool_id: &str,
) -> Option<OAuthCarpoolRecord> {
    let index = store
        .oauth_carpools
        .iter()
        .position(|item| item.carpool_id == carpool_id)?;
    let mut record = store.oauth_carpools.remove(index);
    record.enabled = false;
    record.updated_at = now_rfc3339();
    record.version = record.version.saturating_add(1);
    push_oauth_audit_event(
        store,
        "carpool_remove",
        &record.provider,
        None,
        Some(&record.carpool_id),
        None,
        None,
        None,
        "carpool removed by control plane",
        serde_json::json!({}),
    );
    Some(record)
}

fn read_memory_oauth_sharing_usage(
    store: &MemoryStore,
    filters: OAuthSharingUsageFilters<'_>,
) -> OAuthSharingUsageResponse {
    let mut grouped: BTreeMap<OAuthSharingUsageKey, u64> = BTreeMap::new();
    let mut audit_events = Vec::new();

    for event in &store.oauth_sharing_audit_events {
        if !oauth_audit_event_matches(event, filters) {
            continue;
        }
        audit_events.push(event.clone());
        if matches!(event.event_type.as_str(), "select" | "bind") {
            let provider = event.provider.clone();
            let key = (
                event.lease_id.clone(),
                event.carpool_id.clone(),
                event.workspace_id.clone(),
                provider,
                event.account_id.clone(),
            );
            *grouped.entry(key).or_default() += 1;
        }
    }

    OAuthSharingUsageResponse {
        data: grouped
            .into_iter()
            .map(
                |((lease_id, carpool_id, workspace_id, provider, account_id), turns)| {
                    OAuthSharingUsageSummary {
                        lease_id,
                        carpool_id,
                        workspace_id,
                        provider,
                        account_id,
                        turns,
                    }
                },
            )
            .collect(),
        audit_events,
    }
}

fn oauth_audit_event_matches(
    event: &OAuthSharingAuditEventRecord,
    filters: OAuthSharingUsageFilters<'_>,
) -> bool {
    filters
        .lease_id
        .is_none_or(|value| event.lease_id.as_deref() == Some(value))
        && filters
            .carpool_id
            .is_none_or(|value| event.carpool_id.as_deref() == Some(value))
        && filters
            .workspace_id
            .is_none_or(|value| event.workspace_id.as_deref() == Some(value))
        && filters.provider.is_none_or(|value| event.provider == value)
        && filters
            .account_id
            .is_none_or(|value| event.account_id.as_deref() == Some(value))
}

fn runtime_lease_audit_metadata(runtime_lease: &OAuthPoolRuntimeLeaseRecord) -> serde_json::Value {
    serde_json::json!({
        "runtime_lease_id": runtime_lease.runtime_lease_id,
        "status": runtime_lease.status,
        "expires_at": runtime_lease.expires_at,
        "heartbeat_at": runtime_lease.heartbeat_at,
        "released_at": runtime_lease.released_at,
        "fencing_token": runtime_lease.fencing_token,
    })
}

fn push_runtime_lease_audit_event(
    store: &mut MemoryStore,
    event_type: &str,
    runtime_lease: &OAuthPoolRuntimeLeaseRecord,
    reason: &str,
) {
    push_oauth_audit_event(
        store,
        event_type,
        runtime_lease.provider.as_str(),
        runtime_lease.lease_id.as_deref(),
        runtime_lease.carpool_id.as_deref(),
        runtime_lease.workspace_id.as_deref(),
        Some(runtime_lease.account_id.as_str()),
        runtime_lease.pool_id.as_deref(),
        reason,
        runtime_lease_audit_metadata(runtime_lease),
    );
}

fn cleanup_memory_runtime_leases(store: &mut MemoryStore) {
    let now = now_rfc3339();
    let mut expired_account_ids = Vec::new();
    let mut expired_runtime_leases = Vec::new();
    for runtime_lease in &mut store.oauth_pool_runtime_leases {
        if runtime_lease.status == "active" && timestamp_is_expired(&runtime_lease.expires_at) {
            runtime_lease.status = "expired".to_string();
            runtime_lease.released_at = Some(now.clone());
            expired_account_ids.push(runtime_lease.account_id.clone());
            expired_runtime_leases.push(runtime_lease.clone());
        }
    }

    for account_id in expired_account_ids {
        if let Some(account) = store
            .codex_auth_accounts
            .iter_mut()
            .find(|account| account.codex_account_id == account_id)
        {
            account.active_runs = account.active_runs.saturating_sub(1);
            account.updated_at = now.clone();
            account.version = account.version.saturating_add(1);
        }
    }

    for runtime_lease in expired_runtime_leases {
        push_runtime_lease_audit_event(
            store,
            "runtime_lease_expire",
            &runtime_lease,
            "runtime lease expired",
        );
    }

    for binding in &mut store.oauth_pool_session_bindings {
        if binding.status == "active" && timestamp_is_expired(&binding.expires_at) {
            binding.status = "expired".to_string();
            binding.updated_at = now.clone();
        }
    }
}

fn active_runtime_lease_count(
    store: &MemoryStore,
    account_id: Option<&str>,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    workspace_id: Option<&str>,
    provider: &str,
) -> u64 {
    store
        .oauth_pool_runtime_leases
        .iter()
        .filter(|runtime_lease| {
            runtime_lease.status == "active"
                && !timestamp_is_expired(&runtime_lease.expires_at)
                && runtime_lease.provider == provider
                && account_id.is_none_or(|value| runtime_lease.account_id == value)
                && lease_id.is_none_or(|value| runtime_lease.lease_id.as_deref() == Some(value))
                && carpool_id.is_none_or(|value| runtime_lease.carpool_id.as_deref() == Some(value))
                && workspace_id
                    .is_none_or(|value| runtime_lease.workspace_id.as_deref() == Some(value))
        })
        .count()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn session_key_for_request(provider: &str, request: &OAuthPoolSelectionRequest) -> Option<String> {
    let session_id = request.session_id.as_deref()?.trim();
    if session_id.is_empty() {
        return None;
    }

    let mut hasher = Sha256::new();
    for part in [
        request.borrower_workspace_id.as_deref().unwrap_or(""),
        request.borrower_user_id.as_deref().unwrap_or(""),
        session_id,
        provider,
        request.model_id.as_deref().unwrap_or(""),
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    Some(format!("sessbind_{:x}", hasher.finalize()))
}

fn pool_binding_matches(
    binding: &OAuthPoolSessionBindingRecord,
    provider: &str,
    pool_ids: Option<&[String]>,
    workspace_id: Option<&str>,
    model_id: Option<&str>,
) -> bool {
    binding.provider == provider
        && binding.status == "active"
        && !timestamp_is_expired(&binding.expires_at)
        && pool_ids.is_none_or(|pool_ids| {
            binding
                .pool_id
                .as_deref()
                .is_some_and(|pool_id| pool_ids.iter().any(|candidate| candidate == pool_id))
        })
        && workspace_id.is_none_or(|value| binding.workspace_id.as_deref() == Some(value))
        && model_id.is_none_or(|value| binding.model_id.as_deref() == Some(value))
}

fn account_runtime_capacity_allows(
    store: &MemoryStore,
    account: &CodexAuthAccountRecord,
    provider: &str,
) -> bool {
    let limit = account.concurrency_limit.unwrap_or(1);
    active_runtime_lease_count(
        store,
        Some(account.codex_account_id.as_str()),
        None,
        None,
        None,
        provider,
    ) < u64::from(limit)
}

fn account_health_allows(account: &CodexAuthAccountRecord) -> bool {
    matches!(account.health_state.as_str(), "healthy" | "degraded")
}

fn find_bound_account_index(
    store: &MemoryStore,
    provider: &str,
    session_key: Option<&str>,
    pool_ids: Option<&[String]>,
    workspace_id: Option<&str>,
    model_id: Option<&str>,
) -> Option<(usize, OAuthPoolSessionBindingRecord)> {
    let session_key = session_key?;
    let binding = store
        .oauth_pool_session_bindings
        .iter()
        .find(|binding| {
            binding.session_key == session_key
                && pool_binding_matches(binding, provider, pool_ids, workspace_id, model_id)
        })?
        .clone();
    let index = store
        .codex_auth_accounts
        .iter()
        .position(|account| account.codex_account_id == binding.account_id)?;
    let account = &store.codex_auth_accounts[index];
    if oauth_account_is_schedulable(account, provider)
        && account_health_allows(account)
        && account_runtime_capacity_allows(store, account, provider)
    {
        Some((index, binding))
    } else {
        None
    }
}

fn next_runtime_lease_id(
    store: &MemoryStore,
    account_id: &str,
    operation_id: Option<&str>,
) -> String {
    let sequence = store.oauth_pool_runtime_leases.len().saturating_add(1);
    let mut hasher = Sha256::new();
    hasher.update(account_id.as_bytes());
    hasher.update(operation_id.unwrap_or("").as_bytes());
    hasher.update(sequence.to_string().as_bytes());
    format!("opoollease_{:x}", hasher.finalize())
}

fn next_binding_id(session_key: &str, provider: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_key.as_bytes());
    hasher.update(provider.as_bytes());
    format!("opoolbind_{:x}", hasher.finalize())
}

fn create_memory_runtime_lease(
    store: &mut MemoryStore,
    request: &OAuthPoolSelectionRequest,
    provider: &str,
    account: &CodexAuthAccountRecord,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    pool_id: Option<&str>,
    session_key: Option<&str>,
) -> OAuthPoolRuntimeLeaseRecord {
    let now = now_rfc3339();
    let ttl = request
        .lease_ttl_seconds
        .unwrap_or(DEFAULT_OAUTH_POOL_RUNTIME_LEASE_TTL_SECONDS)
        .clamp(1, DEFAULT_OAUTH_POOL_SESSION_BINDING_TTL_SECONDS);
    let fencing_token = store.oauth_pool_runtime_leases.len().saturating_add(1) as u64;
    let record = OAuthPoolRuntimeLeaseRecord {
        runtime_lease_id: next_runtime_lease_id(
            store,
            account.codex_account_id.as_str(),
            request.operation_id.as_deref(),
        ),
        account_id: account.codex_account_id.clone(),
        provider: provider.to_string(),
        pool_id: pool_id.map(str::to_string),
        lease_id: lease_id.map(str::to_string),
        carpool_id: carpool_id.map(str::to_string),
        workspace_id: request.borrower_workspace_id.clone(),
        session_key: session_key.map(str::to_string),
        holder_id: request.holder_id.clone(),
        operation_id: request.operation_id.clone(),
        status: "active".to_string(),
        expires_at: expires_at(ttl),
        heartbeat_at: now.clone(),
        released_at: None,
        fencing_token,
        created_at: now,
    };
    store.oauth_pool_runtime_leases.push(record.clone());
    record
}

fn upsert_memory_session_binding(
    store: &mut MemoryStore,
    request: &OAuthPoolSelectionRequest,
    provider: &str,
    account_id: &str,
    pool_id: Option<&str>,
    session_key: Option<&str>,
) -> Option<OAuthPoolSessionBindingRecord> {
    let session_key = session_key?;
    let now = now_rfc3339();
    let binding_policy = request
        .binding_policy
        .clone()
        .unwrap_or_else(|| "sticky_session".to_string());
    let binding = OAuthPoolSessionBindingRecord {
        binding_id: next_binding_id(session_key, provider),
        session_key: session_key.to_string(),
        provider: provider.to_string(),
        pool_id: pool_id.map(str::to_string),
        account_id: account_id.to_string(),
        workspace_id: request.borrower_workspace_id.clone(),
        model_id: request.model_id.clone(),
        binding_policy,
        expires_at: expires_at(DEFAULT_OAUTH_POOL_SESSION_BINDING_TTL_SECONDS),
        last_seen_at: now.clone(),
        rebind_count: 0,
        status: "active".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    if let Some(existing) = store
        .oauth_pool_session_bindings
        .iter_mut()
        .find(|existing| existing.session_key == session_key && existing.provider == provider)
    {
        let same_account = existing.account_id == account_id;
        existing.pool_id = binding.pool_id.clone();
        existing.account_id = binding.account_id.clone();
        existing.workspace_id = binding.workspace_id.clone();
        existing.model_id = binding.model_id.clone();
        existing.binding_policy = binding.binding_policy.clone();
        existing.expires_at = binding.expires_at.clone();
        existing.last_seen_at = binding.last_seen_at.clone();
        existing.status = "active".to_string();
        existing.updated_at = binding.updated_at.clone();
        if !same_account {
            existing.rebind_count = existing.rebind_count.saturating_add(1);
        }
        return Some(existing.clone());
    }

    store.oauth_pool_session_bindings.push(binding.clone());
    Some(binding)
}

fn select_memory_oauth_pool_account(
    store: &mut MemoryStore,
    request: &OAuthPoolSelectionRequest,
) -> OAuthPoolSelection {
    cleanup_memory_runtime_leases(store);
    let provider = request.provider.as_deref().unwrap_or("codex");
    let borrower_workspace_id = request.borrower_workspace_id.as_deref();
    let pool_id = request.pool_id.as_deref();

    let mut allowed_account_ids = None;
    let mut context_pool_ids = pool_id.map(|value| vec![value.to_string()]);
    let mut policy = "fair_share".to_string();
    let mut lease_id = None;
    let mut carpool_id = None;

    if let Some(requested_lease_id) = request.lease_id.as_deref() {
        let Some(lease) = store
            .oauth_sharing_leases
            .iter()
            .find(|candidate| candidate.lease_id == requested_lease_id)
            .cloned()
        else {
            return blocked_selection(
                store,
                provider,
                Some(requested_lease_id),
                None,
                borrower_workspace_id,
                pool_id,
                "lease_not_found",
            );
        };
        if let Some(reason) = lease_block_reason(&lease, borrower_workspace_id, pool_id, store) {
            return blocked_selection(
                store,
                provider,
                Some(&lease.lease_id),
                None,
                borrower_workspace_id,
                Some(&lease.pool_id),
                &reason,
            );
        }
        allowed_account_ids = lease.allowed_account_ids.clone();
        context_pool_ids = Some(vec![lease.pool_id.clone()]);
        policy = lease.policy;
        lease_id = Some(lease.lease_id);
    }

    if let Some(requested_carpool_id) = request.carpool_id.as_deref() {
        let Some(carpool) = store
            .oauth_carpools
            .iter()
            .find(|candidate| candidate.carpool_id == requested_carpool_id)
            .cloned()
        else {
            return blocked_selection(
                store,
                provider,
                lease_id.as_deref(),
                Some(requested_carpool_id),
                borrower_workspace_id,
                pool_id,
                "carpool_not_found",
            );
        };
        if let Some(reason) = carpool_block_reason(&carpool, borrower_workspace_id, pool_id, store)
        {
            return blocked_selection(
                store,
                provider,
                lease_id.as_deref(),
                Some(&carpool.carpool_id),
                borrower_workspace_id,
                pool_id,
                &reason,
            );
        }
        context_pool_ids = Some(carpool.pool_ids.clone());
        policy = carpool.strategy;
        carpool_id = Some(carpool.carpool_id);
    }

    let session_key = session_key_for_request(provider, request);
    let bound = find_bound_account_index(
        store,
        provider,
        session_key.as_deref(),
        context_pool_ids.as_deref(),
        borrower_workspace_id,
        request.model_id.as_deref(),
    );
    let selected_index = bound.as_ref().map(|(index, _)| *index).or_else(|| {
        select_schedulable_account_index(
            store,
            provider,
            context_pool_ids.as_deref(),
            allowed_account_ids.as_deref(),
            borrower_workspace_id,
            lease_id.as_deref(),
            carpool_id.as_deref(),
            &policy,
        )
    });
    let Some(selected_index) = selected_index else {
        return blocked_selection(
            store,
            provider,
            lease_id.as_deref(),
            carpool_id.as_deref(),
            borrower_workspace_id,
            pool_id,
            "no_schedulable_account",
        );
    };

    let selected_snapshot = store
        .codex_auth_accounts
        .get(selected_index)
        .expect("selected account index should exist")
        .clone();
    let effective_pool_id = context_pool_ids
        .as_deref()
        .and_then(|pool_ids| {
            pool_ids.iter().find(|candidate| {
                candidate.as_str() == selected_snapshot.provider_resource_id.as_str()
            })
        })
        .map(String::as_str)
        .or(pool_id)
        .or_else(|| Some(selected_snapshot.provider_resource_id.as_str()));
    let runtime_lease = create_memory_runtime_lease(
        store,
        request,
        provider,
        &selected_snapshot,
        lease_id.as_deref(),
        carpool_id.as_deref(),
        effective_pool_id,
        session_key.as_deref(),
    );
    let binding = upsert_memory_session_binding(
        store,
        request,
        provider,
        selected_snapshot.codex_account_id.as_str(),
        effective_pool_id,
        session_key.as_deref(),
    );
    if let Some(binding) = binding.as_ref() {
        push_oauth_audit_event(
            store,
            if bound.is_some() {
                "bind_reuse"
            } else {
                "bind"
            },
            provider,
            lease_id.as_deref(),
            carpool_id.as_deref(),
            borrower_workspace_id,
            Some(selected_snapshot.codex_account_id.as_str()),
            effective_pool_id,
            "session binding updated",
            serde_json::json!({
                "binding_id": binding.binding_id,
                "binding_policy": binding.binding_policy,
                "session_key": binding.session_key,
                "model_id": request.model_id,
            }),
        );
    }

    let now = now_rfc3339();
    let account_id;
    let selected;
    {
        let account = store
            .codex_auth_accounts
            .get_mut(selected_index)
            .expect("selected account index should exist");
        account.leased_until = Some(runtime_lease.expires_at.clone());
        account.active_runs = account.active_runs.saturating_add(1);
        account.updated_at = now;
        account.version = account.version.saturating_add(1);
        account_id = account.codex_account_id.clone();
        selected = account.clone();
    }
    let reason = if bound.is_some() {
        "sticky session binding selected".to_string()
    } else {
        selection_reason(lease_id.as_deref(), carpool_id.as_deref(), &policy)
    };
    push_oauth_audit_event(
        store,
        "select",
        provider,
        lease_id.as_deref(),
        carpool_id.as_deref(),
        borrower_workspace_id,
        Some(&account_id),
        Some(selected.provider_resource_id.as_str()),
        &reason,
        serde_json::json!({
            "runtime_lease_id": runtime_lease.runtime_lease_id,
            "binding_id": binding.as_ref().map(|binding| binding.binding_id.clone()),
            "model_id": request.model_id,
            "session_id": request.session_id,
        }),
    );
    OAuthPoolSelection {
        account: Some(selected),
        reason,
        blocked: false,
        lease_id,
        carpool_id,
        runtime_lease_id: Some(runtime_lease.runtime_lease_id),
        binding_id: binding.as_ref().map(|binding| binding.binding_id.clone()),
        binding_expires_at: binding.as_ref().map(|binding| binding.expires_at.clone()),
        fencing_token: Some(runtime_lease.fencing_token),
    }
}

fn select_schedulable_account_index(
    store: &MemoryStore,
    provider: &str,
    pool_ids: Option<&[String]>,
    allowed_account_ids: Option<&[String]>,
    borrower_workspace_id: Option<&str>,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    policy: &str,
) -> Option<usize> {
    let mut candidates = store
        .codex_auth_accounts
        .iter()
        .enumerate()
        .filter(|(_, account)| {
            oauth_account_is_schedulable(account, provider)
                && account_health_allows(account)
                && account_runtime_capacity_allows(store, account, provider)
                && pool_ids.is_none_or(|pool_ids| {
                    pool_ids
                        .iter()
                        .any(|pool_id| pool_id == account.provider_resource_id.as_str())
                })
                && allowed_account_ids.is_none_or(|allowed| {
                    allowed
                        .iter()
                        .any(|account_id| account_id == account.codex_account_id.as_str())
                })
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by_key(|(_, account)| {
        let usage = account_turn_usage(
            store,
            lease_id,
            carpool_id,
            borrower_workspace_id,
            Some(account.codex_account_id.as_str()),
            provider,
        );
        let active_runs = active_runtime_lease_count(
            store,
            Some(account.codex_account_id.as_str()),
            None,
            None,
            None,
            provider,
        );
        match policy {
            "owner_priority" => (active_runs, usage, account.updated_at.clone()),
            _ => (usage, active_runs, account.updated_at.clone()),
        }
    });

    candidates.first().map(|(index, _)| *index)
}

fn oauth_account_is_schedulable(account: &CodexAuthAccountRecord, provider: &str) -> bool {
    account.provider == provider
        && account.status == "active"
        && account_health_allows(account)
        && account.schedulable
        && account.credential_ready
        && time_gate_allows(account.rate_limited_until.as_deref())
        && time_gate_allows(account.overloaded_until.as_deref())
        && time_gate_allows(account.temp_unschedulable_until.as_deref())
}

fn lease_block_reason(
    lease: &OAuthSharingLeaseRecord,
    borrower_workspace_id: Option<&str>,
    pool_id: Option<&str>,
    store: &MemoryStore,
) -> Option<String> {
    if lease.status != "active" {
        return Some(format!("lease_{}", lease.status));
    }
    if borrower_workspace_id != Some(lease.borrower_workspace_id.as_str()) {
        return Some("borrower_not_authorized".to_string());
    }
    if pool_id.is_some_and(|pool_id| pool_id != lease.pool_id) {
        return Some("pool_not_authorized".to_string());
    }
    if !time_window_allows(&lease.starts_at, &lease.expires_at) {
        return Some("lease_expired".to_string());
    }
    let active_runs = active_context_runs(
        store,
        Some(&lease.lease_id),
        None,
        Some(&lease.borrower_workspace_id),
        &lease.provider,
    );
    if active_runs >= u64::from(lease.max_concurrent_runs) {
        return Some("lease_concurrency_exhausted".to_string());
    }
    if let Some(limit) = lease.usage_budget.turns {
        let used = account_turn_usage(
            store,
            Some(&lease.lease_id),
            None,
            Some(&lease.borrower_workspace_id),
            None,
            &lease.provider,
        );
        if used >= limit {
            return Some("lease_budget_exhausted".to_string());
        }
    }
    None
}

fn carpool_block_reason(
    carpool: &OAuthCarpoolRecord,
    borrower_workspace_id: Option<&str>,
    pool_id: Option<&str>,
    store: &MemoryStore,
) -> Option<String> {
    if !carpool.enabled {
        return Some("carpool_disabled".to_string());
    }
    let Some(borrower_workspace_id) = borrower_workspace_id else {
        return Some("borrower_workspace_required".to_string());
    };
    if !carpool
        .member_workspace_ids
        .iter()
        .any(|member_id| member_id == borrower_workspace_id)
    {
        return Some("borrower_not_authorized".to_string());
    }
    if pool_id.is_some_and(|pool_id| !carpool.pool_ids.iter().any(|item| item == pool_id)) {
        return Some("pool_not_authorized".to_string());
    }
    if let Some(limit) = carpool.per_member_concurrency_limit {
        let active_runs = active_context_runs(
            store,
            None,
            Some(&carpool.carpool_id),
            Some(borrower_workspace_id),
            &carpool.provider,
        );
        if active_runs >= u64::from(limit) {
            return Some("carpool_concurrency_exhausted".to_string());
        }
    }
    if let Some(limit) = carpool.per_member_turn_budget {
        let used = account_turn_usage(
            store,
            None,
            Some(&carpool.carpool_id),
            Some(borrower_workspace_id),
            None,
            &carpool.provider,
        );
        if used >= limit {
            return Some("carpool_budget_exhausted".to_string());
        }
    }
    None
}

fn blocked_selection(
    store: &mut MemoryStore,
    provider: &str,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    workspace_id: Option<&str>,
    pool_id: Option<&str>,
    reason: &str,
) -> OAuthPoolSelection {
    let event_type = if reason.contains("budget_exhausted") {
        "budget_exhausted"
    } else {
        "select_blocked"
    };
    push_oauth_audit_event(
        store,
        event_type,
        provider,
        lease_id,
        carpool_id,
        workspace_id,
        None,
        pool_id,
        reason,
        serde_json::json!({}),
    );
    OAuthPoolSelection {
        account: None,
        reason: reason.to_string(),
        blocked: true,
        lease_id: lease_id.map(str::to_string),
        carpool_id: carpool_id.map(str::to_string),
        runtime_lease_id: None,
        binding_id: None,
        binding_expires_at: None,
        fencing_token: None,
    }
}

fn heartbeat_memory_runtime_lease(
    store: &mut MemoryStore,
    runtime_lease_id: &str,
) -> Option<OAuthPoolRuntimeLeaseRecord> {
    cleanup_memory_runtime_leases(store);
    let index = store
        .oauth_pool_runtime_leases
        .iter()
        .position(|runtime_lease| runtime_lease.runtime_lease_id == runtime_lease_id)?;
    let now = now_rfc3339();
    let runtime_lease = store.oauth_pool_runtime_leases.get_mut(index)?;
    if runtime_lease.status != "active" || timestamp_is_expired(&runtime_lease.expires_at) {
        return Some(runtime_lease.clone());
    }
    runtime_lease.heartbeat_at = now;
    let runtime_lease = runtime_lease.clone();
    push_runtime_lease_audit_event(
        store,
        "runtime_lease_heartbeat",
        &runtime_lease,
        "runtime lease heartbeat recorded",
    );
    Some(runtime_lease)
}

fn release_memory_runtime_lease(
    store: &mut MemoryStore,
    runtime_lease_id: &str,
) -> Option<OAuthPoolRuntimeLeaseRecord> {
    cleanup_memory_runtime_leases(store);
    let index = store
        .oauth_pool_runtime_leases
        .iter()
        .position(|runtime_lease| runtime_lease.runtime_lease_id == runtime_lease_id)?;
    let now = now_rfc3339();
    let runtime_lease = store.oauth_pool_runtime_leases.get_mut(index)?;
    if runtime_lease.status == "active" {
        runtime_lease.status = "released".to_string();
        runtime_lease.released_at = Some(now.clone());
        if let Some(account) = store
            .codex_auth_accounts
            .iter_mut()
            .find(|account| account.codex_account_id == runtime_lease.account_id)
        {
            account.active_runs = account.active_runs.saturating_sub(1);
            account.updated_at = now.clone();
            account.version = account.version.saturating_add(1);
        }
        let runtime_lease = runtime_lease.clone();
        push_runtime_lease_audit_event(
            store,
            "runtime_lease_release",
            &runtime_lease,
            "runtime lease released",
        );
        return Some(runtime_lease);
    }
    Some(runtime_lease.clone())
}

fn feedback_error_code(feedback: &OAuthPoolAccountFeedback) -> Option<String> {
    feedback.error_code.clone().or_else(|| {
        feedback
            .status_code
            .map(|status_code| format!("http_{status_code}"))
    })
}

fn apply_memory_account_feedback(
    store: &mut MemoryStore,
    account_id: &str,
    feedback: OAuthPoolAccountFeedback,
) -> Option<CodexAuthAccountRecord> {
    let index = store
        .codex_auth_accounts
        .iter()
        .position(|account| account.codex_account_id == account_id)?;
    let now = now_rfc3339();
    let account = store.codex_auth_accounts.get_mut(index)?;
    let status_code = feedback.status_code;
    let outcome = feedback.outcome.as_deref().unwrap_or("error");
    let error_code = feedback_error_code(&feedback);

    if outcome == "success" || status_code.is_some_and(|status| (200..400).contains(&status)) {
        account.health_state = "healthy".to_string();
        account.health_score = (account.health_score + 0.05).min(1.0);
        account.last_error_code = None;
        account.consecutive_failures = 0;
        account.last_success_at = Some(now.clone());
    } else if status_code == Some(401) || error_code.as_deref() == Some("invalid_token") {
        account.health_state = "quarantined".to_string();
        account.health_score = 0.0;
        account.last_error_code = error_code;
        account.consecutive_failures = account.consecutive_failures.saturating_add(1);
    } else if status_code == Some(403) || matches!(error_code.as_deref(), Some("policy" | "risk")) {
        account.health_state = "cooldown".to_string();
        account.health_score = (account.health_score - 0.4).max(0.0);
        account.temp_unschedulable_until = Some(expires_at(OAUTH_POOL_COOLDOWN_SECONDS));
        account.last_error_code = error_code;
        account.consecutive_failures = account.consecutive_failures.saturating_add(1);
    } else if status_code == Some(429) {
        account.health_state = "degraded".to_string();
        let retry_after = feedback
            .retry_after_seconds
            .unwrap_or(OAUTH_POOL_COOLDOWN_SECONDS)
            .min(OAUTH_POOL_MAX_COOLDOWN_SECONDS);
        account.rate_limited_until = Some(expires_at(retry_after));
        account.health_score = (account.health_score - 0.2).max(0.1);
        account.last_error_code = error_code;
        account.consecutive_failures = account.consecutive_failures.saturating_add(1);
    } else {
        account.health_state = "degraded".to_string();
        account.health_score = (account.health_score - 0.2).max(0.1);
        account.last_error_code = error_code;
        account.consecutive_failures = account.consecutive_failures.saturating_add(1);
        if account.consecutive_failures >= 3 {
            account.overloaded_until = Some(expires_at(OAUTH_POOL_COOLDOWN_SECONDS));
        }
    }
    account.updated_at = now;
    account.version = account.version.saturating_add(1);
    let account_snapshot = account.clone();
    let event_type = if account_snapshot.health_state == "quarantined" {
        "quarantine"
    } else if account_snapshot.health_state == "cooldown" {
        "cooldown"
    } else {
        "feedback"
    };

    push_oauth_audit_event(
        store,
        event_type,
        account_snapshot.provider.as_str(),
        None,
        None,
        Some(account_snapshot.tenant_id.as_str()),
        Some(account_snapshot.codex_account_id.as_str()),
        Some(account_snapshot.provider_resource_id.as_str()),
        account_snapshot.health_state.as_str(),
        serde_json::json!({
            "runtime_lease_id": feedback.runtime_lease_id,
            "outcome": feedback.outcome,
            "status_code": status_code,
            "error_code": account_snapshot.last_error_code,
            "health_score": account_snapshot.health_score,
            "consecutive_failures": account_snapshot.consecutive_failures,
        }),
    );

    Some(account_snapshot)
}

#[allow(clippy::too_many_arguments)]
fn push_oauth_audit_event(
    store: &mut MemoryStore,
    event_type: &str,
    provider: &str,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    workspace_id: Option<&str>,
    account_id: Option<&str>,
    pool_id: Option<&str>,
    reason: &str,
    metadata: serde_json::Value,
) {
    let audit_event_id = format!(
        "oauthaud_{}",
        store.oauth_sharing_audit_events.len().saturating_add(1)
    );
    store
        .oauth_sharing_audit_events
        .push(OAuthSharingAuditEventRecord {
            audit_event_id,
            event_type: event_type.to_string(),
            provider: provider.to_string(),
            lease_id: lease_id.map(str::to_string),
            carpool_id: carpool_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            account_id: account_id.map(str::to_string),
            pool_id: pool_id.map(str::to_string),
            reason: reason.to_string(),
            metadata,
            created_at: now_rfc3339(),
        });
}

fn selection_reason(lease_id: Option<&str>, carpool_id: Option<&str>, policy: &str) -> String {
    match (lease_id, carpool_id) {
        (Some(lease_id), Some(carpool_id)) => {
            format!("authorized lease {lease_id} and carpool {carpool_id} selected by {policy}")
        }
        (Some(lease_id), None) => format!("authorized lease {lease_id} selected by {policy}"),
        (None, Some(carpool_id)) => {
            format!("authorized carpool {carpool_id} selected by {policy}")
        }
        (None, None) => "legacy pool selected by strict schedulable filter".to_string(),
    }
}

fn active_context_runs(
    store: &MemoryStore,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    workspace_id: Option<&str>,
    provider: &str,
) -> u64 {
    active_runtime_lease_count(store, None, lease_id, carpool_id, workspace_id, provider)
}

fn account_turn_usage(
    store: &MemoryStore,
    lease_id: Option<&str>,
    carpool_id: Option<&str>,
    workspace_id: Option<&str>,
    account_id: Option<&str>,
    provider: &str,
) -> u64 {
    store
        .oauth_sharing_audit_events
        .iter()
        .filter(|event| {
            event.event_type == "select"
                && event.provider == provider
                && lease_id.is_none_or(|value| event.lease_id.as_deref() == Some(value))
                && carpool_id.is_none_or(|value| event.carpool_id.as_deref() == Some(value))
                && workspace_id.is_none_or(|value| event.workspace_id.as_deref() == Some(value))
                && account_id.is_none_or(|value| event.account_id.as_deref() == Some(value))
        })
        .count()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn time_gate_allows(value: Option<&str>) -> bool {
    value.is_none_or(timestamp_is_expired)
}

fn time_window_allows(starts_at: &str, expires_at: &str) -> bool {
    !timestamp_is_future(starts_at) && !timestamp_is_expired(expires_at)
}

fn api_key_prefix(api_key: &str) -> String {
    if api_key.len() <= 6 {
        api_key.to_string()
    } else {
        format!("{}...", &api_key[..6])
    }
}

fn credential_last_four(credential: &str) -> String {
    let len = credential.len();
    if len <= 4 {
        credential.to_string()
    } else {
        credential[len - 4..].to_string()
    }
}

fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn activate_memory_config_snapshot(
    store: &mut MemoryStore,
    config_snapshot_id: &str,
) -> Option<ConfigSnapshotResponse> {
    let index = store
        .config_snapshots
        .iter()
        .position(|item| item.config_snapshot_id.as_str() == config_snapshot_id)?;
    let tenant_id = store.config_snapshots[index].tenant_id.clone();
    let project_id = store.config_snapshots[index].project_id.clone();
    for (candidate_index, snapshot) in store.config_snapshots.iter_mut().enumerate() {
        if candidate_index != index
            && snapshot.tenant_id == tenant_id
            && snapshot.project_id == project_id
            && snapshot.status == ConfigSnapshotStatus::Active
        {
            snapshot.status = ConfigSnapshotStatus::Superseded;
        }
    }
    let snapshot = &mut store.config_snapshots[index];
    snapshot.status = ConfigSnapshotStatus::Active;
    snapshot.activated_at = Some(now_rfc3339());
    store.active_config_snapshot_id = config_snapshot_id.to_string();
    Some(ConfigSnapshotResponse {
        config_snapshot: snapshot.clone(),
    })
}

fn simulate_memory_route(
    store: &MemoryStore,
    request: &RouteSimulationRequest,
) -> Result<RouteSimulationResponse> {
    let active_snapshot = store
        .config_snapshots
        .iter()
        .filter(|snapshot| {
            snapshot.status == ConfigSnapshotStatus::Active
                && snapshot.tenant_id == request.tenant_id
                && snapshot.project_id == request.project_id
        })
        .max_by(|left, right| {
            left.activated_at
                .cmp(&right.activated_at)
                .then_with(|| left.config_snapshot_id.cmp(&right.config_snapshot_id))
        })
        .cloned()
        .context("active config snapshot missing for project")?;
    let provider_resources = store
        .provider_resources
        .iter()
        .filter(|resource| resource.tenant_id == request.tenant_id)
        .cloned()
        .collect::<Vec<_>>();
    let route_policies = store
        .route_policies
        .iter()
        .filter(|policy| policy.tenant_id == request.tenant_id)
        .cloned()
        .collect::<Vec<_>>();
    build_route_simulation_response(
        &provider_resources,
        &route_policies,
        &active_snapshot,
        request,
    )
}

fn format_monetary_amount(currency: &str, micros: i64) -> MonetaryAmount {
    let sign = if micros < 0 { "-" } else { "" };
    let absolute = micros.abs();
    let whole = absolute / 1_000_000;
    let fractional = absolute % 1_000_000;
    MonetaryAmount {
        currency: currency.to_string(),
        amount: format!("{sign}{whole}.{fractional:06}"),
    }
}

fn projection_lag_seconds(timestamp: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .and_then(|parsed| {
            let duration =
                chrono::Utc::now().signed_duration_since(parsed.with_timezone(&chrono::Utc));
            duration.num_seconds().try_into().ok()
        })
        .unwrap_or_default()
}

fn parse_cursor_offset(cursor: Option<&str>) -> usize {
    cursor
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default()
}

fn sample_usage_summary_response(
    tenant_id: &str,
    project_id: Option<&str>,
    owner_account_id: Option<&str>,
    window_start: Option<&str>,
    window_end: Option<&str>,
) -> UsageSummaryResponse {
    UsageSummaryResponse {
        data: UsageSummary {
            tenant_id: TenantId::parse(tenant_id.to_string()).unwrap(),
            project_id: project_id.map(|value| ProjectId::parse(value.to_string()).unwrap()),
            owner_account_id: owner_account_id.map(str::to_string),
            window_start: window_start.unwrap_or("2026-04-01T00:00:00Z").to_string(),
            window_end: window_end.unwrap_or("2026-04-30T23:59:59Z").to_string(),
            currency: "USD".to_string(),
            event_count: 14,
            input_tokens: 18_420,
            output_tokens: 6_245,
            cached_input_tokens: 1_220,
            provider_cost: format_monetary_amount("USD", 124_500),
            billable_price: format_monetary_amount("USD", 152_025),
        },
    }
}

fn sample_usage_breakdown_response(
    _tenant_id: &str,
    _project_id: Option<&str>,
    _owner_account_id: Option<&str>,
    group_by: UsageBreakdownGroupBy,
    cursor: Option<String>,
    limit: Option<u32>,
) -> UsageBreakdownResponse {
    let rows = match group_by {
        UsageBreakdownGroupBy::Provider => vec![
            UsageBreakdownRow {
                bucket: "openai".to_string(),
                provider_id: Some("openai".to_string()),
                model_alias: None,
                input_tokens: 10_000,
                output_tokens: 4_000,
                cached_input_tokens: 500,
                provider_cost: format_monetary_amount("USD", 82_000),
                billable_price: format_monetary_amount("USD", 98_400),
            },
            UsageBreakdownRow {
                bucket: "anthropic".to_string(),
                provider_id: Some("anthropic".to_string()),
                model_alias: None,
                input_tokens: 8_420,
                output_tokens: 2_245,
                cached_input_tokens: 720,
                provider_cost: format_monetary_amount("USD", 42_500),
                billable_price: format_monetary_amount("USD", 53_625),
            },
        ],
        UsageBreakdownGroupBy::Model => vec![
            UsageBreakdownRow {
                bucket: "reasoning-fast".to_string(),
                provider_id: None,
                model_alias: Some("reasoning-fast".to_string()),
                input_tokens: 8_420,
                output_tokens: 2_245,
                cached_input_tokens: 720,
                provider_cost: format_monetary_amount("USD", 42_500),
                billable_price: format_monetary_amount("USD", 53_625),
            },
            UsageBreakdownRow {
                bucket: "support-safe".to_string(),
                provider_id: None,
                model_alias: Some("support-safe".to_string()),
                input_tokens: 10_000,
                output_tokens: 4_000,
                cached_input_tokens: 500,
                provider_cost: format_monetary_amount("USD", 82_000),
                billable_price: format_monetary_amount("USD", 98_400),
            },
        ],
        UsageBreakdownGroupBy::Day => vec![UsageBreakdownRow {
            bucket: "2026-04-21".to_string(),
            provider_id: None,
            model_alias: None,
            input_tokens: 18_420,
            output_tokens: 6_245,
            cached_input_tokens: 1_220,
            provider_cost: format_monetary_amount("USD", 124_500),
            billable_price: format_monetary_amount("USD", 152_025),
        }],
    };
    let offset = parse_cursor_offset(cursor.as_deref());
    let limit = usize::try_from(limit.unwrap_or(50)).unwrap_or(50);
    let paged = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = if paged.len() == limit {
        Some((offset + paged.len()).to_string())
    } else {
        None
    };
    UsageBreakdownResponse {
        data: paged,
        next_cursor,
    }
}

fn sample_balance_projection_response(
    tenant_id: &str,
    project_id: Option<&str>,
    owner_account_id: Option<&str>,
) -> BalanceProjectionResponse {
    let configured_budget = default_budget_micros_for_scope(
        tenant_id,
        project_id.unwrap_or("project"),
        owner_account_id,
    );
    let billable_total = 1_540_000;
    BalanceProjectionResponse {
        data: BalanceProjection {
            tenant_id: TenantId::parse(tenant_id.to_string()).unwrap(),
            project_id: project_id.map(|value| ProjectId::parse(value.to_string()).unwrap()),
            owner_account_id: owner_account_id.map(str::to_string),
            currency: "USD".to_string(),
            provider_cost_total: format_monetary_amount("USD", 1_244_000),
            billable_total: format_monetary_amount("USD", billable_total),
            configured_budget: format_monetary_amount("USD", configured_budget),
            remaining_budget: format_monetary_amount("USD", configured_budget - billable_total),
            threshold_status: "ok".to_string(),
            last_projected_at: now_rfc3339(),
            projection_lag_seconds: 0,
        },
    }
}

#[allow(dead_code)]
fn sample_billing_export_job_response(
    tenant_id: Option<&str>,
    project_id: Option<&str>,
    format: &str,
) -> BillingExportJobResponse {
    BillingExportJobResponse {
        data: BillingExportJob {
            export_job_id: "export_123".to_string(),
            status: "queued".to_string(),
            format: format.to_string(),
            requested_at: now_rfc3339(),
            completed_at: None,
            error_message: None,
            tenant_id: tenant_id.map(|value| TenantId::parse(value.to_string()).unwrap()),
            project_id: project_id.map(|value| ProjectId::parse(value.to_string()).unwrap()),
        },
    }
}

fn maybe_complete_memory_export_job(
    mut job: BillingExportJob,
    has_content: bool,
) -> BillingExportJob {
    if job.status == "queued" && has_content {
        job.status = "completed".to_string();
        job.completed_at = Some(now_rfc3339());
    }
    job
}

fn filter_billing_export_jobs(
    mut jobs: Vec<BillingExportJob>,
    tenant_id: Option<String>,
    project_id: Option<String>,
) -> Vec<BillingExportJob> {
    jobs.retain(|job| {
        if let Some(tenant_id) = tenant_id.as_deref()
            && job.tenant_id.as_ref().map(core_domain::TenantId::as_str) != Some(tenant_id)
        {
            return false;
        }
        if let Some(project_id) = project_id.as_deref()
            && job.project_id.as_ref().map(core_domain::ProjectId::as_str) != Some(project_id)
        {
            return false;
        }
        true
    });
    jobs.sort_by(|left, right| right.requested_at.cmp(&left.requested_at));
    jobs
}

fn create_memory_renewal_intent(
    store: &mut MemoryStore,
    draft: RenewalIntentDraft,
) -> RenewalIntentCreateResult {
    let Some(order) = store
        .wechat_payment_orders
        .get(&draft.out_trade_no)
        .cloned()
    else {
        return RenewalIntentCreateResult::OrderNotFound;
    };
    let Some(grant) = store
        .opening_grants
        .iter_mut()
        .find(|grant| grant.grant_id == draft.grant_id)
    else {
        return RenewalIntentCreateResult::GrantNotFound;
    };
    if !renewal_order_matches_grant(&order, grant) {
        return RenewalIntentCreateResult::GrantMismatch;
    }

    let intent = build_renewal_intent(&draft, &order, grant);
    store.renewal_intents.push(intent.clone());
    RenewalIntentCreateResult::Created(intent)
}

fn renewal_order_matches_grant(
    order: &WechatPaymentOrderRecord,
    grant: &OpeningGrantRecord,
) -> bool {
    order.tenant_id == grant.tenant_id.as_str()
        && order.project_id.as_deref() == Some(grant.project_id.as_str())
}

fn build_renewal_intent(
    draft: &RenewalIntentDraft,
    order: &WechatPaymentOrderRecord,
    grant: &mut OpeningGrantRecord,
) -> RenewalIntent {
    let now = now_rfc3339();
    let previous_grant_status = grant.effective_status();
    let previous_expires_at = grant.expires_at.clone();
    let requested_reason = draft
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (status, reason_code, reason, applied_at) = if order.status != "paid" {
        (
            "renewal_blocked",
            "payment_not_paid",
            "WeChat Pay order is not paid; future grant state was not restored.",
            None,
        )
    } else if !matches!(previous_grant_status.as_str(), "active" | "expired") {
        (
            "manual_review",
            "node6_state_missing_or_unrecoverable",
            "Opening grant state is not automatically recoverable; manual review is required.",
            None,
        )
    } else if !timestamp_is_future(&draft.renew_expires_at) {
        (
            "renewal_blocked",
            "renew_expires_at_not_future",
            "Renewal expiry is not in the future; future grant state was not restored.",
            None,
        )
    } else {
        grant.status = "active".to_string();
        grant.expires_at = draft.renew_expires_at.clone();
        grant.updated_at = now.clone();
        grant.version = grant.version.saturating_add(1);
        (
            "renewed",
            "payment_paid_grant_recovered",
            "Paid order matched the opening grant; only future grant state was restored.",
            Some(now.clone()),
        )
    };

    RenewalIntent {
        renewal_intent_id: draft.renewal_intent_id.clone(),
        out_trade_no: draft.out_trade_no.clone(),
        grant_id: draft.grant_id.clone(),
        tenant_id: grant.tenant_id.clone(),
        project_id: grant.project_id.clone(),
        payment_status: order.status.clone(),
        previous_grant_status,
        previous_expires_at,
        renew_expires_at: draft.renew_expires_at.clone(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        reason: requested_reason.unwrap_or(reason).to_string(),
        created_by: draft.created_by.clone(),
        created_at: now.clone(),
        updated_at: now,
        applied_at,
    }
}

fn filter_renewal_intents(
    mut intents: Vec<RenewalIntent>,
    filters: &RenewalIntentFilters,
) -> Vec<RenewalIntent> {
    intents.retain(|intent| {
        if let Some(tenant_id) = filters.tenant_id.as_deref()
            && intent.tenant_id.as_str() != tenant_id
        {
            return false;
        }
        if let Some(project_id) = filters.project_id.as_deref()
            && intent.project_id.as_str() != project_id
        {
            return false;
        }
        if let Some(grant_id) = filters.grant_id.as_deref()
            && intent.grant_id != grant_id
        {
            return false;
        }
        if let Some(out_trade_no) = filters.out_trade_no.as_deref()
            && intent.out_trade_no != out_trade_no
        {
            return false;
        }
        true
    });
    intents.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    intents
}

async fn render_billing_export_csv(
    store: &PostgresStore,
    tenant_id: Option<&str>,
    project_id: Option<&str>,
    window_start: &str,
    window_end: &str,
) -> Result<String> {
    let rows = store
        .get_usage_breakdown(
            tenant_id.unwrap_or("tenant_acme"),
            project_id.map(str::to_string),
            None,
            Some(window_start.to_string()),
            Some(window_end.to_string()),
            UsageBreakdownGroupBy::Day,
            None,
            Some(500),
        )
        .await?;

    let mut csv = String::from(
        "bucket,provider_id,model_alias,input_tokens,output_tokens,cached_input_tokens,provider_cost,billable_price\n",
    );
    for row in rows.data {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            row.bucket,
            row.provider_id.unwrap_or_default(),
            row.model_alias.unwrap_or_default(),
            row.input_tokens,
            row.output_tokens,
            row.cached_input_tokens,
            row.provider_cost.amount,
            row.billable_price.amount,
        ));
    }

    Ok(csv)
}

async fn maybe_complete_postgres_export_job(
    pool: &Pool<Postgres>,
    row: sqlx::postgres::PgRow,
) -> Result<BillingExportJob> {
    let export_job_id = row.get::<String, _>("export_job_id");
    let mut status = row.get::<String, _>("status");
    let mut completed_at = row.get::<Option<String>, _>("completed_at");
    let export_content = row.get::<Option<String>, _>("export_content");

    if status == "queued" && export_content.is_some() {
        completed_at = Some(now_rfc3339());
        status = "completed".to_string();
        sqlx::query(
            "UPDATE billing_export_jobs SET status = 'completed', completed_at = $2 WHERE export_job_id = $1",
        )
        .bind(&export_job_id)
        .bind(completed_at.as_deref())
        .execute(pool)
        .await?;
    }

    Ok(BillingExportJob {
        export_job_id,
        status,
        format: row.get::<String, _>("format"),
        requested_at: row.get::<String, _>("requested_at"),
        completed_at,
        error_message: row.get::<Option<String>, _>("error_message"),
        tenant_id: row
            .get::<Option<String>, _>("tenant_id")
            .map(TenantId::parse)
            .transpose()?,
        project_id: row
            .get::<Option<String>, _>("project_id")
            .map(ProjectId::parse)
            .transpose()?,
    })
}

fn sale_ready_route_simulation(
    snapshot: &ConfigSnapshot,
    route_policy: Option<&RoutePolicy>,
    provider_resources: &[ProviderResource],
) -> Option<RouteSimulationResponse> {
    let route_policy = route_policy?;
    let protocol_family = parse_protocol_family(&route_policy.protocol_family)?;
    let region = route_policy
        .preferred_regions
        .first()
        .cloned()
        .or_else(|| {
            provider_resources
                .first()
                .map(|resource| resource.region.clone())
        })
        .unwrap_or_else(|| "global".to_string());
    let request = RouteSimulationRequest {
        tenant_id: snapshot.tenant_id.clone(),
        project_id: snapshot.project_id.clone(),
        credential_scope: core_domain::CredentialId::parse("cred_sale_ready_diagnostics").unwrap(),
        protocol_family,
        model_alias: route_policy.model_alias.clone(),
        required_capabilities: route_policy.required_capabilities.clone(),
        region,
        expected_prompt_tokens: 1_000,
        expected_max_output_tokens: 1_000,
        traffic_class: "sale_ready_preview".to_string(),
    };

    build_route_simulation_response(
        provider_resources,
        std::slice::from_ref(route_policy),
        snapshot,
        &request,
    )
    .ok()
}

fn sale_ready_pricing_request(
    route_policy: Option<&RoutePolicy>,
    route_simulation: Option<&RouteSimulationResponse>,
    provider_resources: &[ProviderResource],
) -> Option<PricingSimulationRequest> {
    let route_policy = route_policy?;
    let selected_resource_id =
        route_simulation.and_then(|simulation| simulation.selected_target.as_ref());
    let provider_resource = selected_resource_id
        .and_then(|provider_resource_id| {
            provider_resources
                .iter()
                .find(|resource| resource.provider_resource_id == *provider_resource_id)
        })
        .or_else(|| provider_resources.first())?;

    Some(PricingSimulationRequest {
        provider_id: provider_resource.provider_id.clone(),
        model_alias: route_policy.model_alias.clone(),
        usage: UsageMetrics {
            input_tokens: 1_000,
            output_tokens: 1_000,
            cached_input_tokens: 0,
        },
        region: Some(provider_resource.region.clone()),
        image_generation_units: None,
        audio_seconds: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_sale_ready_package_response(
    snapshot: ConfigSnapshot,
    route_policy: Option<RoutePolicy>,
    provider_resources: Vec<ProviderResource>,
    missing_provider_resource_ids: Vec<String>,
    active_snapshot_id: Option<ConfigSnapshotId>,
    route_simulation: Option<RouteSimulationResponse>,
    pricing: Option<PricingSimulationResponse>,
    budget: Option<BalanceProjection>,
) -> SaleReadyPackageResponse {
    let carrier_check = match snapshot.status {
        ConfigSnapshotStatus::Draft | ConfigSnapshotStatus::Active => sale_ready_check(
            "carrier",
            "pass",
            "carrier_confirmed",
            "ConfigSnapshot is the sale-ready package carrier.",
        ),
        ConfigSnapshotStatus::Superseded => sale_ready_check(
            "carrier",
            "blocked",
            "snapshot_superseded",
            "Superseded config snapshots cannot be sold.",
        ),
    };
    let route_check = if route_policy.as_ref().is_some_and(|policy| {
        parse_protocol_family(&policy.protocol_family).is_some()
            && policy.required_capabilities.iter().all(|capability| {
                route_capability_supported_by_provider_capabilities(capability.as_str())
            })
    }) {
        sale_ready_check(
            "route",
            "pass",
            "route_policy_valid",
            "Route policy protocol and capabilities are supported.",
        )
    } else {
        sale_ready_check(
            "route",
            "blocked",
            "blocked_route_policy",
            "Route policy is missing or contains unsupported protocol/capability requirements.",
        )
    };
    let resource_check = if snapshot.provider_resource_ids.is_empty() {
        sale_ready_check(
            "resource",
            "blocked",
            "blocked_no_resource",
            "ConfigSnapshot does not reference any provider resources.",
        )
    } else if !missing_provider_resource_ids.is_empty() {
        sale_ready_check(
            "resource",
            "blocked",
            "blocked_missing_resource",
            format!(
                "Missing provider resources: {}.",
                missing_provider_resource_ids.join(", ")
            ),
        )
    } else if route_simulation
        .as_ref()
        .is_some_and(|simulation| simulation.admission_result == AdmissionResult::Admitted)
    {
        sale_ready_check(
            "resource",
            "pass",
            "resource_set_routeable",
            "Provider resource set produces at least one route candidate.",
        )
    } else {
        sale_ready_check(
            "resource",
            "blocked",
            "blocked_no_candidate",
            "Provider resource set cannot produce a route candidate.",
        )
    };
    let price_check = if pricing
        .as_ref()
        .is_some_and(|pricing| !pricing.line_items.is_empty())
    {
        sale_ready_check(
            "price",
            "pass",
            "pricing_available",
            "Pricing simulation produced a billable quote.",
        )
    } else {
        sale_ready_check(
            "price",
            "blocked",
            "blocked_no_price",
            "Pricing simulation could not produce a billable quote.",
        )
    };
    let budget_check = match budget.as_ref() {
        Some(budget) if budget.threshold_status != "exceeded" => sale_ready_check(
            "budget",
            "pass",
            "budget_available",
            "Balance projection can explain budget and remaining threshold.",
        ),
        Some(_) => sale_ready_check(
            "budget",
            "blocked",
            "blocked_budget",
            "Balance projection reports an exceeded threshold.",
        ),
        None => sale_ready_check(
            "budget",
            "blocked",
            "blocked_budget_unknown",
            "Balance projection is not available.",
        ),
    };
    let gateway_check = if active_snapshot_id.as_ref() == Some(&snapshot.config_snapshot_id) {
        sale_ready_check(
            "gateway",
            "pass",
            "gateway_active_config",
            "Gateway active config pointer references this snapshot.",
        )
    } else {
        sale_ready_check(
            "gateway",
            "blocked",
            "blocked_gateway_config",
            "Gateway active config pointer does not reference this snapshot.",
        )
    };
    let boundary_check = sale_ready_check(
        "boundary",
        "pass",
        "customer_key_deferred",
        "Node 2 does not create customer API keys; Node 3 handles opening.",
    );
    let checks = vec![
        carrier_check,
        route_check,
        resource_check,
        price_check,
        budget_check,
        gateway_check,
        boundary_check,
    ];
    let blocking_check = checks
        .iter()
        .find(|check| check.status == "blocked" && check.name != "boundary");
    let readiness = match blocking_check {
        None => SaleReadiness {
            status: "sale_ready".to_string(),
            reason_code: "sale_ready".to_string(),
            reason: "ConfigSnapshot is ready to hand off to customer opening.".to_string(),
            checks: checks.clone(),
        },
        Some(check) => SaleReadiness {
            status: match snapshot.status {
                ConfigSnapshotStatus::Draft => "draft_blocked",
                ConfigSnapshotStatus::Active | ConfigSnapshotStatus::Superseded => "suspended",
            }
            .to_string(),
            reason_code: check.reason_code.clone(),
            reason: check.reason.clone(),
            checks: checks.clone(),
        },
    };
    let handoff = SaleReadyHandoff {
        tenant_id: snapshot.tenant_id.clone(),
        project_id: snapshot.project_id.clone(),
        config_snapshot_id: snapshot.config_snapshot_id.clone(),
        route_policy_id: snapshot.route_policy_id.clone(),
        budget_policy_id: snapshot.budget_policy_id.clone(),
        provider_resource_ids: snapshot.provider_resource_ids.clone(),
        readiness_status: readiness.status.clone(),
    };

    SaleReadyPackageResponse {
        config_snapshot: snapshot,
        route_policy,
        provider_resources,
        route_simulation,
        pricing,
        budget,
        readiness,
        handoff,
        creates_customer_api_key: false,
    }
}

fn sale_ready_check(
    name: &str,
    status: &str,
    reason_code: &str,
    reason: impl Into<String>,
) -> SaleReadyCheck {
    SaleReadyCheck {
        name: name.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        reason: reason.into(),
    }
}

fn build_route_simulation_response(
    provider_resources: &[ProviderResource],
    route_policies: &[RoutePolicy],
    active_snapshot: &ConfigSnapshot,
    request: &RouteSimulationRequest,
) -> Result<RouteSimulationResponse> {
    let request_protocol_family = protocol_family_slug(&request.protocol_family);

    let route_policy = route_policies
        .iter()
        .find(|policy| policy.route_policy_id == active_snapshot.route_policy_id)
        .cloned()
        .context("route policy missing for snapshot")?;

    if route_policy.protocol_family != request_protocol_family {
        return Err(anyhow!(
            "route policy protocol family `{}` does not match request protocol family `{}`",
            route_policy.protocol_family,
            request_protocol_family,
        ));
    }

    if !route_policy
        .required_capabilities
        .iter()
        .all(|capability| route_capability_supported_by_provider_capabilities(capability.as_str()))
    {
        return Err(anyhow!(
            "route policy requires unsupported capability: {}",
            route_policy
                .required_capabilities
                .iter()
                .find(|capability| {
                    !route_capability_supported_by_provider_capabilities(capability.as_str())
                })
                .unwrap_or(&"unknown".to_string())
        ));
    }

    let route = routing_engine::evaluate_route(
        &RoutingConfig {
            config_snapshot: active_snapshot.clone(),
            route_policy,
            provider_targets: provider_resources
                .iter()
                .enumerate()
                .map(|(index, resource)| provider_resource_to_route_target(index, resource))
                .collect(),
        },
        &RoutingRequest {
            protocol_family: request_protocol_family.to_string(),
            model_alias: request.model_alias.clone(),
        },
    );

    let eligible_candidates = route
        .ranked_targets
        .iter()
        .map(|candidate| protocol_ir::EligibleCandidate {
            provider_resource_id: candidate.target.resource.provider_resource_id.clone(),
            score_breakdown: candidate.score_breakdown.clone(),
        })
        .collect::<Vec<_>>();
    let selected_target = eligible_candidates
        .first()
        .map(|candidate| candidate.provider_resource_id.clone());

    Ok(RouteSimulationResponse {
        simulation_id: format!("sim_{}", request.model_alias),
        config_snapshot_id: active_snapshot.config_snapshot_id.clone(),
        admission_result: route.admission_result,
        eligible_candidates,
        excluded_candidates: route.excluded_targets,
        selected_target,
        estimated_cost: MonetaryAmount {
            currency: "USD".to_string(),
            amount: "0.000210".to_string(),
        },
    })
}

const fn protocol_family_slug(protocol_family: &ProtocolFamily) -> &'static str {
    match protocol_family {
        ProtocolFamily::OpenAiChat => "openai_chat",
        ProtocolFamily::OpenAiResponses => "openai_responses",
        ProtocolFamily::OpenAiImages => "openai_images",
        ProtocolFamily::McpStreamableHttp => "mcp_streamable_http",
        ProtocolFamily::RealtimeWebRtc => "realtime_webrtc",
        ProtocolFamily::AnthropicMessages => "anthropic_messages",
        ProtocolFamily::GeminiGenerateContent => "gemini_generate_content",
    }
}

fn parse_protocol_family(protocol_family: &str) -> Option<ProtocolFamily> {
    match protocol_family {
        "openai_chat" => Some(ProtocolFamily::OpenAiChat),
        "openai_responses" => Some(ProtocolFamily::OpenAiResponses),
        "openai_images" => Some(ProtocolFamily::OpenAiImages),
        "mcp_streamable_http" => Some(ProtocolFamily::McpStreamableHttp),
        "realtime_webrtc" => Some(ProtocolFamily::RealtimeWebRtc),
        "anthropic_messages" => Some(ProtocolFamily::AnthropicMessages),
        "gemini_generate_content" => Some(ProtocolFamily::GeminiGenerateContent),
        _ => None,
    }
}

fn provider_resource_to_route_target(
    index: usize,
    resource: &ProviderResource,
) -> ProviderTargetRuntime {
    ProviderTargetRuntime {
        resource: resource.clone(),
        target_kind: if resource.is_transit_gateway {
            ProviderTargetKind::TransitGateway
        } else {
            ProviderTargetKind::Native
        },
        transit_metadata: None,
        priority: u32::try_from(index).unwrap_or(u32::MAX),
        upstream_model: None,
        api_key: String::new(),
        static_latency_score: 0.95,
        static_cost_score: if resource.deployment_scope == DeploymentScope::Shared {
            0.8
        } else {
            0.7
        },
        usd_per_1k_tokens: 0.0,
    }
}

const fn admission_result_slug(admission_result: AdmissionResult) -> &'static str {
    match admission_result {
        AdmissionResult::Admitted => "admitted",
        AdmissionResult::RejectedBudget => "rejected_budget",
        AdmissionResult::RejectedRateLimit => "rejected_rate_limit",
        AdmissionResult::RejectedConcurrency => "rejected_concurrency",
        AdmissionResult::RejectedPolicy => "rejected_policy",
        AdmissionResult::RejectedNoCandidate => "rejected_no_candidate",
    }
}

fn provider_matches_filters(
    resource: &ProviderResource,
    filters: &ProviderResourceFilters,
) -> bool {
    if let Some(tenant_id) = &filters.tenant_id {
        if resource.tenant_id.as_str() != tenant_id {
            return false;
        }
    }
    if let Some(health_state) = &filters.health_state {
        if health_state_slug(resource.health_state) != health_state {
            return false;
        }
    }
    if let Some(protocol_family) = &filters.protocol_family {
        if !provider_supports_protocol_family(resource, protocol_family) {
            return false;
        }
    }
    if let Some(capability) = &filters.capability {
        if !provider_supports_capability(resource, capability) {
            return false;
        }
    }
    if let Some(transit_gateway) = filters.transit_gateway {
        if resource.is_transit_gateway != transit_gateway {
            return false;
        }
    }
    if let Some(quarantined) = filters.quarantined {
        if matches!(resource.health_state, HealthState::Quarantined) != quarantined {
            return false;
        }
    }
    true
}

fn route_receipt_matches_filters(receipt: &RouteReceipt, filters: &RouteReceiptFilters) -> bool {
    if let Some(tenant_id) = &filters.tenant_id {
        if receipt.tenant_id.as_str() != tenant_id {
            return false;
        }
    }
    if let Some(project_id) = &filters.project_id {
        if receipt.project_id.as_str() != project_id {
            return false;
        }
    }
    if let Some(protocol_family) = &filters.protocol_family {
        if receipt.protocol_family != *protocol_family {
            return false;
        }
    }
    if let Some(route_policy_id) = &filters.route_policy_id {
        if receipt.route_policy_id.as_str() != route_policy_id {
            return false;
        }
    }
    if let Some(admission_result) = &filters.admission_result {
        if admission_result_slug(receipt.admission_result) != admission_result {
            return false;
        }
    }
    if let Some(provider_resource_id) = &filters.provider_resource_id {
        let selected = receipt
            .selected_target
            .as_ref()
            .is_some_and(|target| target.as_str() == provider_resource_id);
        let excluded = receipt
            .excluded_targets
            .iter()
            .any(|target| target.provider_resource_id.as_str() == provider_resource_id);
        if !selected && !excluded {
            return false;
        }
    }
    true
}

fn to_route_receipt_summary(receipt: &RouteReceipt) -> RouteReceiptSummary {
    RouteReceiptSummary {
        route_receipt_id: receipt.route_receipt_id.clone(),
        admission_result: receipt.admission_result,
        selected_target: receipt.selected_target.clone(),
        failure_reason: receipt.failure_reason.clone(),
        created_at: receipt.created_at.clone(),
    }
}

fn memory_route_receipt_diagnostics_response(
    route_receipt: RouteReceipt,
) -> RouteReceiptDiagnosticsResponse {
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "memory_store".to_string());
    metadata.insert("request_id".to_string(), route_receipt.request_id.clone());
    metadata.insert("trace_id".to_string(), route_receipt.trace_id.clone());

    RouteReceiptDiagnosticsResponse {
        policy_checks: vec![RouteReceiptPolicyCheck {
            policy_id: route_receipt.route_policy_id.clone(),
            status: match route_receipt.admission_result {
                AdmissionResult::Admitted => "passed".to_string(),
                _ => "failed".to_string(),
            },
            reason: route_receipt.failure_reason.clone(),
        }],
        decision_timeline: vec![RouteReceiptDecisionTraceStep {
            stage: "route_receipt".to_string(),
            status: if route_receipt.normalized_error.is_some() {
                "failed".to_string()
            } else {
                "recorded".to_string()
            },
            message: route_receipt.failure_reason.clone().unwrap_or_else(|| {
                "Route receipt was recorded without persisted worker diagnostics.".to_string()
            }),
            score: route_receipt.selected_target.as_ref().map(|_| {
                route_receipt.score_breakdown.latency
                    + route_receipt.score_breakdown.cost
                    + route_receipt.score_breakdown.health
                    + route_receipt.score_breakdown.trust
            }),
            notes: vec![
                format!("excluded_targets={}", route_receipt.excluded_targets.len()),
                format!(
                    "fallback_transitions={}",
                    route_receipt.fallback_transitions.len()
                ),
            ],
        }],
        provider_attempts: Vec::new(),
        metadata,
        route_receipt,
    }
}

fn build_memory_route_diagnostics(
    store: &MemoryStore,
    route_policy_id: &str,
) -> Option<RouteDiagnosticsResponse> {
    build_route_diagnostics_response(
        &store.provider_resources,
        &store.route_policies,
        &store.config_snapshots,
        &store.route_receipts.values().cloned().collect::<Vec<_>>(),
        route_policy_id,
        Some(store.active_config_snapshot_id.as_str()),
    )
}

fn build_route_diagnostics_response(
    provider_resources: &[ProviderResource],
    route_policies: &[RoutePolicy],
    config_snapshots: &[ConfigSnapshot],
    route_receipts: &[RouteReceipt],
    route_policy_id: &str,
    active_snapshot_id: Option<&str>,
) -> Option<RouteDiagnosticsResponse> {
    let route_policy = route_policies
        .iter()
        .find(|policy| policy.route_policy_id.as_str() == route_policy_id)
        .cloned()?;

    let active_snapshot = active_snapshot_id.and_then(|snapshot_id| {
        config_snapshots
            .iter()
            .find(|snapshot| snapshot.config_snapshot_id.as_str() == snapshot_id)
            .cloned()
    });
    let active_snapshot_matches_route_policy = active_snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.route_policy_id == route_policy.route_policy_id);

    let mut recent_receipts = route_receipts
        .iter()
        .filter(|receipt| receipt.route_policy_id == route_policy.route_policy_id)
        .cloned()
        .collect::<Vec<_>>();
    recent_receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    let recent_receipt_summaries = recent_receipts
        .iter()
        .take(5)
        .map(to_route_receipt_summary)
        .collect::<Vec<_>>();
    let last_route_receipt = recent_receipts.first().map(to_route_receipt_summary);

    let targets = provider_resources
        .iter()
        .filter(|resource| resource.tenant_id == route_policy.tenant_id)
        .cloned()
        .map(|provider_resource| {
            let recent_receipt = recent_receipts.iter().find(|receipt| {
                receipt
                    .selected_target
                    .as_ref()
                    .is_some_and(|selected| selected == &provider_resource.provider_resource_id)
                    || receipt.excluded_targets.iter().any(|excluded| {
                        excluded.provider_resource_id == provider_resource.provider_resource_id
                    })
            });
            let recent_exclusion = recent_receipt.and_then(|receipt| {
                receipt.excluded_targets.iter().find(|excluded| {
                    excluded.provider_resource_id == provider_resource.provider_resource_id
                })
            });
            let capability_gaps =
                provider_capability_gaps(&provider_resource, &route_policy.required_capabilities);
            let supports_protocol_family = provider_supports_protocol_family(
                &provider_resource,
                &route_policy.protocol_family,
            );
            let in_active_snapshot = active_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot
                    .provider_resource_ids
                    .iter()
                    .any(|id| id == &provider_resource.provider_resource_id)
            });

            let (decision, reason_code, reason, recent_receipt_reason) =
                if provider_resource.status != ProviderResourceStatus::Active {
                    (
                        RouteDiagnosticDecision::Excluded,
                        "provider_inactive".to_string(),
                        format!("Provider status is {:?}.", provider_resource.status),
                        None,
                    )
                } else if !supports_protocol_family {
                    (
                        RouteDiagnosticDecision::Excluded,
                        "protocol_family_unsupported".to_string(),
                        format!(
                            "Provider does not advertise protocol family `{}`.",
                            route_policy.protocol_family
                        ),
                        None,
                    )
                } else if !capability_gaps.is_empty() {
                    (
                        RouteDiagnosticDecision::Excluded,
                        format!("capability_gap_{}", capability_gaps[0]),
                        format!(
                            "Missing required capabilities: {}.",
                            capability_gaps.join(", ")
                        ),
                        None,
                    )
                } else if is_health_blocked(provider_resource.health_state) {
                    (
                        RouteDiagnosticDecision::Excluded,
                        format!(
                            "health_{}",
                            health_state_slug(provider_resource.health_state)
                        ),
                        provider_resource
                            .health_message
                            .clone()
                            .unwrap_or_else(|| "Provider health state blocks routing.".to_string()),
                        None,
                    )
                } else if recent_receipt
                    .and_then(|receipt| receipt.selected_target.as_ref())
                    .is_some_and(|selected| selected == &provider_resource.provider_resource_id)
                {
                    (
                        RouteDiagnosticDecision::Selected,
                        "selected_recent_receipt".to_string(),
                        "Selected by the most recent route receipt.".to_string(),
                        None,
                    )
                } else if let Some(excluded) = recent_exclusion {
                    (
                        RouteDiagnosticDecision::Excluded,
                        excluded.reason_code.clone(),
                        excluded.reason.clone(),
                        Some(excluded.reason.clone()),
                    )
                } else {
                    (
                        RouteDiagnosticDecision::Eligible,
                        "eligible".to_string(),
                        "Provider satisfies current protocol, capability, and health requirements."
                            .to_string(),
                        None,
                    )
                };

            RouteDiagnosticTarget {
                provider_resource,
                decision,
                in_active_snapshot,
                supports_protocol_family,
                capability_gaps,
                reason_code,
                reason,
                recent_receipt_id: recent_receipt.map(|receipt| receipt.route_receipt_id.clone()),
                recent_receipt_reason,
            }
        })
        .collect::<Vec<_>>();

    Some(RouteDiagnosticsResponse {
        route_policy,
        active_snapshot,
        active_snapshot_matches_route_policy,
        last_route_receipt,
        recent_receipts: recent_receipt_summaries,
        targets,
    })
}

async fn persist_codex_auth_account_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    account: &CodexAuthAccountRecord,
) -> Result<()> {
    sqlx::query(
        "UPDATE codex_auth_accounts
            SET leased_until = $2, updated_at = $3, payload = $4
          WHERE codex_account_id = $1",
    )
    .bind(&account.codex_account_id)
    .bind(account.leased_until.as_deref())
    .bind(&account.updated_at)
    .bind(Json(account))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_codex_auth_account_records_from_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<CodexAuthAccountRecord>> {
    let rows =
        sqlx::query("SELECT payload FROM codex_auth_accounts ORDER BY updated_at ASC FOR UPDATE")
            .fetch_all(&mut **tx)
            .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<Json<CodexAuthAccountRecord>, _>("payload").0)
        .collect())
}

async fn load_oauth_sharing_lease_records_from_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<OAuthSharingLeaseRecord>> {
    let rows = sqlx::query("SELECT payload FROM oauth_sharing_leases ORDER BY lease_id")
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<Json<OAuthSharingLeaseRecord>, _>("payload").0)
        .collect())
}

async fn load_oauth_carpool_records_from_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<OAuthCarpoolRecord>> {
    let rows = sqlx::query("SELECT payload FROM oauth_carpools ORDER BY carpool_id")
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<Json<OAuthCarpoolRecord>, _>("payload").0)
        .collect())
}

async fn load_oauth_pool_runtime_lease_records_from_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<OAuthPoolRuntimeLeaseRecord>> {
    let rows = sqlx::query(
        "SELECT runtime_lease_id, account_id, provider, pool_id, lease_id, carpool_id,
                workspace_id, session_key, holder_id, operation_id, status, expires_at,
                heartbeat_at, released_at, fencing_token, created_at
           FROM oauth_pool_runtime_leases
          ORDER BY created_at",
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| OAuthPoolRuntimeLeaseRecord {
            runtime_lease_id: row.get("runtime_lease_id"),
            account_id: row.get("account_id"),
            provider: row.get("provider"),
            pool_id: row.get("pool_id"),
            lease_id: row.get("lease_id"),
            carpool_id: row.get("carpool_id"),
            workspace_id: row.get("workspace_id"),
            session_key: row.get("session_key"),
            holder_id: row.get("holder_id"),
            operation_id: row.get("operation_id"),
            status: row.get("status"),
            expires_at: row.get("expires_at"),
            heartbeat_at: row.get("heartbeat_at"),
            released_at: row.get("released_at"),
            fencing_token: row.get::<i64, _>("fencing_token").try_into().unwrap_or(0),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn load_oauth_pool_session_binding_records_from_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<OAuthPoolSessionBindingRecord>> {
    let rows = sqlx::query(
        "SELECT binding_id, session_key, provider, pool_id, account_id, workspace_id,
                model_id, binding_policy, expires_at, last_seen_at, rebind_count,
                status, created_at, updated_at
           FROM oauth_pool_session_bindings
          ORDER BY updated_at",
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| OAuthPoolSessionBindingRecord {
            binding_id: row.get("binding_id"),
            session_key: row.get("session_key"),
            provider: row.get("provider"),
            pool_id: row.get("pool_id"),
            account_id: row.get("account_id"),
            workspace_id: row.get("workspace_id"),
            model_id: row.get("model_id"),
            binding_policy: row.get("binding_policy"),
            expires_at: row.get("expires_at"),
            last_seen_at: row.get("last_seen_at"),
            rebind_count: row.get::<i64, _>("rebind_count").try_into().unwrap_or(0),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_oauth_sharing_audit_records_from_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
) -> Result<Vec<OAuthSharingAuditEventRecord>> {
    let rows = sqlx::query("SELECT payload FROM oauth_sharing_audit_events ORDER BY created_at")
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            row.get::<Json<OAuthSharingAuditEventRecord>, _>("payload")
                .0
        })
        .collect())
}

async fn upsert_oauth_pool_runtime_lease_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    record: &OAuthPoolRuntimeLeaseRecord,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO oauth_pool_runtime_leases
            (runtime_lease_id, account_id, provider, pool_id, lease_id, carpool_id,
             workspace_id, session_key, holder_id, operation_id, status, expires_at,
             heartbeat_at, released_at, fencing_token, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
         ON CONFLICT (runtime_lease_id) DO UPDATE SET
            account_id = EXCLUDED.account_id,
            provider = EXCLUDED.provider,
            pool_id = EXCLUDED.pool_id,
            lease_id = EXCLUDED.lease_id,
            carpool_id = EXCLUDED.carpool_id,
            workspace_id = EXCLUDED.workspace_id,
            session_key = EXCLUDED.session_key,
            holder_id = EXCLUDED.holder_id,
            operation_id = EXCLUDED.operation_id,
            status = EXCLUDED.status,
            expires_at = EXCLUDED.expires_at,
            heartbeat_at = EXCLUDED.heartbeat_at,
            released_at = EXCLUDED.released_at,
            fencing_token = EXCLUDED.fencing_token",
    )
    .bind(&record.runtime_lease_id)
    .bind(&record.account_id)
    .bind(&record.provider)
    .bind(record.pool_id.as_deref())
    .bind(record.lease_id.as_deref())
    .bind(record.carpool_id.as_deref())
    .bind(record.workspace_id.as_deref())
    .bind(record.session_key.as_deref())
    .bind(record.holder_id.as_deref())
    .bind(record.operation_id.as_deref())
    .bind(&record.status)
    .bind(&record.expires_at)
    .bind(&record.heartbeat_at)
    .bind(record.released_at.as_deref())
    .bind(i64::try_from(record.fencing_token).unwrap_or(i64::MAX))
    .bind(&record.created_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_oauth_pool_session_binding_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    record: &OAuthPoolSessionBindingRecord,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO oauth_pool_session_bindings
            (binding_id, session_key, provider, pool_id, account_id, workspace_id,
             model_id, binding_policy, expires_at, last_seen_at, rebind_count,
             status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
         ON CONFLICT (session_key, provider) DO UPDATE SET
            binding_id = EXCLUDED.binding_id,
            pool_id = EXCLUDED.pool_id,
            account_id = EXCLUDED.account_id,
            workspace_id = EXCLUDED.workspace_id,
            model_id = EXCLUDED.model_id,
            binding_policy = EXCLUDED.binding_policy,
            expires_at = EXCLUDED.expires_at,
            last_seen_at = EXCLUDED.last_seen_at,
            rebind_count = EXCLUDED.rebind_count,
            status = EXCLUDED.status,
            updated_at = EXCLUDED.updated_at",
    )
    .bind(&record.binding_id)
    .bind(&record.session_key)
    .bind(&record.provider)
    .bind(record.pool_id.as_deref())
    .bind(&record.account_id)
    .bind(record.workspace_id.as_deref())
    .bind(record.model_id.as_deref())
    .bind(&record.binding_policy)
    .bind(&record.expires_at)
    .bind(&record.last_seen_at)
    .bind(i64::try_from(record.rebind_count).unwrap_or(i64::MAX))
    .bind(&record.status)
    .bind(&record.created_at)
    .bind(&record.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_oauth_sharing_audit_event_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    event: &OAuthSharingAuditEventRecord,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO oauth_sharing_audit_events
            (audit_event_id, event_type, provider, lease_id, carpool_id, workspace_id, account_id, pool_id, payload, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (audit_event_id) DO NOTHING",
    )
    .bind(&event.audit_event_id)
    .bind(&event.event_type)
    .bind(&event.provider)
    .bind(event.lease_id.as_deref())
    .bind(event.carpool_id.as_deref())
    .bind(event.workspace_id.as_deref())
    .bind(event.account_id.as_deref())
    .bind(event.pool_id.as_deref())
    .bind(Json(event))
    .bind(&event.created_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn membership(
    membership_id: &str,
    tenant: &Tenant,
    role: TenantMembershipRole,
) -> TenantMembership {
    TenantMembership {
        membership_id: TenantMembershipId::parse(membership_id.to_string()).unwrap(),
        tenant: TenantSummary {
            id: tenant.tenant_id.clone(),
            slug: tenant.slug.clone(),
            display_name: tenant.display_name.clone(),
        },
        role,
        status: TenantMembershipStatus::Active,
    }
}

fn tenant_matches_workspace(tenant: &Tenant, workspace: &str) -> bool {
    tenant.slug == workspace
        || tenant.tenant_id.as_str() == workspace
        || (workspace == "acme" && tenant.tenant_id.as_str() == "tenant_acme")
}

fn tenant_summary_matches_workspace(tenant: &TenantSummary, workspace: &str) -> bool {
    tenant.slug == workspace
        || tenant.id.as_str() == workspace
        || (workspace == "acme" && tenant.id.as_str() == "tenant_acme")
}

fn link(
    provider: AuthProvider,
    provider_subject: &str,
    email: Option<&str>,
    can_unlink: bool,
) -> AuthProviderLink {
    AuthProviderLink {
        link_id: auth_provider_link_id(provider, provider_subject),
        provider,
        provider_subject: provider_subject.to_string(),
        email: email.map(std::string::ToString::to_string),
        linked_at: "2026-04-22T00:00:00Z".to_string(),
        last_used_at: None,
        can_unlink,
    }
}

fn auth_provider_link_id(
    provider: AuthProvider,
    provider_subject: &str,
) -> core_domain::AuthProviderLinkId {
    let digest = Sha256::digest(format!(
        "{}:{provider_subject}",
        auth_provider_slug(provider)
    ));
    let suffix = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    core_domain::AuthProviderLinkId::parse(format!(
        "authlink_{}_{suffix}",
        auth_provider_slug(provider)
    ))
    .unwrap()
}

#[cfg(test)]
mod parity_tests;

#[cfg(test)]
mod tests {
    mod delivery_upload_tests;

    use super::{
        AdmissionResult, ConcurrencyResult, ConfigSnapshot, ConfigSnapshotId, HealthState,
        ProjectId, ProviderResource, ProviderResourceId, ProviderResourceStatus,
        RouteDiagnosticDecision, RoutePolicy, RoutePolicyId, RouteReceipt, RouteReceiptFilters,
        StoreMode, TenantId,
    };
    use core_domain::{ExcludedTarget, RouteReceiptId, ScoreBreakdown};

    fn sample_route_receipt(
        route_receipt_id: &str,
        tenant_id: &str,
        project_id: &str,
        protocol_family: &str,
        created_at: &str,
    ) -> RouteReceipt {
        RouteReceipt {
            route_receipt_id: RouteReceiptId::parse(route_receipt_id).unwrap(),
            tenant_id: TenantId::parse(tenant_id).unwrap(),
            project_id: ProjectId::parse(project_id).unwrap(),
            route_policy_id: RoutePolicyId::parse("routepol_openai_chat_default").unwrap(),
            request_id: format!("{route_receipt_id}_request"),
            trace_id: format!("{route_receipt_id}_trace"),
            protocol_family: protocol_family.to_string(),
            model_alias: "reasoning-fast".to_string(),
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_default").unwrap(),
            admission_result: AdmissionResult::Admitted,
            selected_target: Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()),
            excluded_targets: vec![ExcludedTarget {
                provider_resource_id: ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
                reason_code: "sample".to_string(),
                reason: "sample".to_string(),
            }],
            failure_reason: None,
            score_breakdown: ScoreBreakdown {
                latency: 0.8,
                cost: 0.6,
                health: 1.0,
                trust: 1.0,
            },
            fallback_transitions: Vec::new(),
            normalized_error: None,
            created_at: created_at.to_string(),
        }
    }

    fn sample_provider_resource() -> ProviderResource {
        ProviderResource {
            provider_resource_id: ProviderResourceId::parse("prvrsrc_cp_test_store").unwrap(),
            tenant_id: core_domain::TenantId::parse("tenant_acme").unwrap(),
            project_id: Some(core_domain::ProjectId::parse("proj_core").unwrap()),
            provider_id: "openai".to_string(),
            name: "cp-store-provider".to_string(),
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
            budget_policy_id: None,
            capabilities: core_domain::ProviderCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_json_mode: true,
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

    fn sample_route_policy() -> RoutePolicy {
        RoutePolicy {
            route_policy_id: RoutePolicyId::parse("routepol_cp_store".to_string()).unwrap(),
            tenant_id: core_domain::TenantId::parse("tenant_acme").unwrap(),
            display_name: "store-route-policy".to_string(),
            protocol_family: "openai_chat".to_string(),
            model_alias: "reasoning-fast".to_string(),
            required_capabilities: vec!["json_mode".to_string()],
            preferred_regions: vec!["us-east-1".to_string()],
            version: 1,
            created_at: "2026-04-22T00:00:00Z".to_string(),
            updated_at: "2026-04-22T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn auth_provider_link_ids_match_shared_schema_character_set() {
        let link = super::link(
            core_domain::AuthProvider::Email,
            "ops@huge-router.dev",
            Some("ops@huge-router.dev"),
            false,
        );

        assert!(link.link_id.as_str().starts_with("authlink_email_"));
        assert!(
            link.link_id
                .as_str()
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || character == '_'
                    || character == '-')
        );
    }

    #[test]
    fn cleanup_runtime_leases_records_expiration_audit() {
        let mut store = super::MemoryStore {
            oauth_pool_runtime_leases: vec![super::OAuthPoolRuntimeLeaseRecord {
                runtime_lease_id: "opoollease_expired".to_string(),
                account_id: "codexacct_expired".to_string(),
                provider: "codex".to_string(),
                pool_id: Some("prvrsrc_codex_pool".to_string()),
                lease_id: Some("lease_codex_share".to_string()),
                carpool_id: None,
                workspace_id: Some("tenant_acme".to_string()),
                session_key: Some("session-expired".to_string()),
                holder_id: Some("gateway-a".to_string()),
                operation_id: Some("op-expired".to_string()),
                status: "active".to_string(),
                expires_at: "2000-01-01T00:00:00Z".to_string(),
                heartbeat_at: "2000-01-01T00:00:00Z".to_string(),
                released_at: None,
                fencing_token: 7,
                created_at: "2000-01-01T00:00:00Z".to_string(),
            }],
            ..super::MemoryStore::default()
        };

        super::cleanup_memory_runtime_leases(&mut store);

        assert_eq!(store.oauth_pool_runtime_leases[0].status, "expired");
        let event = store
            .oauth_sharing_audit_events
            .iter()
            .find(|event| event.event_type == "runtime_lease_expire")
            .expect("runtime lease expiration should be audited");
        assert_eq!(
            event.metadata["runtime_lease_id"].as_str(),
            Some("opoollease_expired")
        );
        assert_eq!(event.lease_id.as_deref(), Some("lease_codex_share"));
    }

    #[tokio::test]
    async fn oauth_login_flow_preserves_redirect_target() {
        let store = StoreMode::memory();
        let flow = store
            .create_oauth_flow(
                "oauth_state_wechat",
                core_domain::OAuthProvider::Wechat,
                "acme-retail",
                Some("/app/providers"),
                "2099-01-01T00:00:00Z",
            )
            .await
            .unwrap();

        assert_eq!(flow.redirect_to.as_deref(), Some("/app/providers"));

        let consumed = store
            .consume_login_flow("oauth_state_wechat")
            .await
            .unwrap()
            .expect("oauth flow should be consumable");

        assert_eq!(consumed.provider, Some(core_domain::OAuthProvider::Wechat));
        assert_eq!(consumed.redirect_to.as_deref(), Some("/app/providers"));
    }

    #[tokio::test]
    async fn memory_store_enforces_version_for_provider_resource_updates() {
        let store = StoreMode::memory();
        let created = store
            .create_provider_resource(sample_provider_resource())
            .await
            .unwrap();
        assert_eq!(created.version, 1);

        let mut stale_update = created.clone();
        stale_update.name = "stale-update".to_string();
        let stale = store
            .update_provider_resource(stale_update, 0)
            .await
            .unwrap();
        assert!(matches!(stale, ConcurrencyResult::VersionConflict));

        let mut update = created.clone();
        update.name = "updated-name".to_string();
        let applied = store.update_provider_resource(update, 1).await.unwrap();
        match applied {
            ConcurrencyResult::Applied(resource) => assert_eq!(resource.version, 2),
            _ => panic!("expected applied update"),
        }
    }

    #[tokio::test]
    async fn memory_store_fail_closes_new_provider_resources_until_probe() {
        let store = StoreMode::memory();
        let mut resource = sample_provider_resource();
        resource.provider_resource_id =
            ProviderResourceId::parse("prvrsrc_cp_store_intake").unwrap();
        resource.health_state = HealthState::Healthy;
        resource.health_message = None;
        resource.quarantine_reason = None;

        let created = store.create_provider_resource(resource).await.unwrap();

        assert_eq!(created.status, ProviderResourceStatus::Active);
        assert_eq!(created.health_state, HealthState::Quarantined);
        assert_eq!(
            created.quarantine_reason.as_deref(),
            Some(super::PROVIDER_RESOURCE_INTAKE_PENDING_PROBE)
        );
        assert_eq!(
            created.health_message.as_deref(),
            Some(super::PROVIDER_RESOURCE_INTAKE_PENDING_MESSAGE)
        );
    }

    #[test]
    fn route_diagnostics_excludes_non_active_provider_status() {
        let mut resource = sample_provider_resource();
        resource.status = ProviderResourceStatus::Disabled;
        resource.health_state = HealthState::Healthy;
        let provider_resource_id = resource.provider_resource_id.clone();
        let route_policy = sample_route_policy();
        let snapshot = ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_cp_store").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            revision: 1,
            budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap(),
            route_policy_id: route_policy.route_policy_id.clone(),
            provider_resource_ids: vec![provider_resource_id],
            status: core_domain::ConfigSnapshotStatus::Active,
            activated_at: Some("2026-04-22T00:00:00Z".to_string()),
        };

        let diagnostics = super::build_route_diagnostics_response(
            &[resource],
            &[route_policy],
            &[snapshot],
            &[],
            "routepol_cp_store",
            Some("cfgsnap_cp_store"),
        )
        .expect("route diagnostics");

        assert_eq!(diagnostics.targets.len(), 1);
        assert!(matches!(
            diagnostics.targets[0].decision,
            RouteDiagnosticDecision::Excluded
        ));
        assert_eq!(diagnostics.targets[0].reason_code, "provider_inactive");
        assert!(diagnostics.targets[0].reason.contains("Disabled"));
    }

    #[tokio::test]
    async fn memory_store_route_policy_disable_is_filterable() {
        let store = StoreMode::memory();
        let created = store
            .create_route_policy(sample_route_policy())
            .await
            .unwrap();
        let before = store.list_route_policies().await.unwrap().data;
        let before_len = before.len();
        let applied = store
            .disable_route_policy(&created.route_policy_id.to_string(), 1)
            .await;
        let applied = match applied.unwrap() {
            ConcurrencyResult::Applied(route_policy) => route_policy,
            other => panic!("expected applied route policy: {other:?}"),
        };
        assert_eq!(applied.version, 2);
        let after = store.list_route_policies().await.unwrap().data;
        assert_eq!(after.len(), before_len - 1);
        assert!(
            !after
                .iter()
                .any(|policy| policy.route_policy_id == created.route_policy_id)
        );
    }

    #[tokio::test]
    async fn memory_store_only_persists_api_key_hash() {
        let store = StoreMode::memory();
        let api_key = "akp_store_secret_value_123";
        let created = store
            .create_api_key(
                ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
                "store-api-key",
                api_key,
            )
            .await
            .unwrap();
        assert_eq!(created.key_prefix, "akp_st...");
        assert_eq!(created.version, 1);

        let keys = store.list_api_keys().await.unwrap().data;
        assert_eq!(keys.len(), 1);

        let store = &store;
        if let StoreMode::Memory(memory) = store {
            let (hash, api_key_id) = {
                let memory = memory.read().unwrap();
                let keys: Vec<_> = memory.api_keys.iter().collect();
                assert_eq!(keys.len(), 1);
                let stored = &keys[0];
                assert_ne!(stored.hash, api_key);
                assert_eq!(stored.hash.len(), 64);
                assert_eq!(
                    stored.provider_resource_id,
                    ProviderResourceId::parse("prvrsrc_openai_primary").unwrap()
                );
                assert_eq!(stored.display_name, "store-api-key");
                assert_eq!(stored.key_prefix, "akp_st...");
                (stored.hash.clone(), stored.api_key_id.clone())
            };
            let revoked = store.revoke_api_key(&api_key_id, 1).await.unwrap();
            assert!(matches!(revoked, ConcurrencyResult::Applied(_)));
            assert_ne!(hash, api_key);
        } else {
            panic!("expected memory store");
        }
    }

    #[tokio::test]
    async fn memory_store_list_route_receipts_supports_filters_and_recent_order() {
        let store = StoreMode::memory();
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_a",
            "tenant_acme",
            "proj_core",
            "openai_chat",
            "2026-04-22T00:01:00Z",
        ));
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_b",
            "tenant_acme",
            "proj_core",
            "openai_chat",
            "2026-04-22T00:03:00Z",
        ));
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_c",
            "tenant_platform",
            "proj_core",
            "openai_responses",
            "2026-04-22T00:02:00Z",
        ));

        let all = store
            .list_route_receipts(&RouteReceiptFilters::default())
            .await
            .unwrap()
            .data;
        assert_eq!(all.len(), 4);
        assert_eq!(all[0].route_receipt_id.as_str(), "routercpt_store_b");
        assert_eq!(all[1].route_receipt_id.as_str(), "routercpt_store_c");
        assert_eq!(all[2].route_receipt_id.as_str(), "routercpt_store_a");
        assert_eq!(
            all[3].route_receipt_id.as_str(),
            "routercpt_acme_relay_eval"
        );

        let tenant_filtered = store
            .list_route_receipts(&RouteReceiptFilters {
                tenant_id: Some("tenant_acme".to_string()),
                ..RouteReceiptFilters::default()
            })
            .await
            .unwrap()
            .data;
        assert_eq!(tenant_filtered.len(), 3);
        assert_eq!(
            tenant_filtered[0].route_receipt_id.as_str(),
            "routercpt_store_b"
        );

        let protocol_filtered = store
            .list_route_receipts(&RouteReceiptFilters {
                protocol_family: Some("openai_chat".to_string()),
                ..RouteReceiptFilters::default()
            })
            .await
            .unwrap()
            .data;
        assert_eq!(protocol_filtered.len(), 3);
    }

    #[tokio::test]
    async fn memory_store_bootstrap_replay_capsule_links_to_seeded_route_receipt() {
        let store = StoreMode::memory();
        let tenant_id = TenantId::parse("tenant_acme").unwrap();

        let replay_capsule = store
            .get_replay_capsule(&tenant_id, "replay_acme_relay_eval")
            .await
            .unwrap()
            .expect("seeded replay capsule should exist")
            .replay_capsule;
        let route_receipt = store
            .get_route_receipt(replay_capsule.route_receipt_id.as_str())
            .await
            .unwrap()
            .expect("seeded route receipt should exist")
            .route_receipt;

        assert_eq!(
            route_receipt.route_receipt_id,
            replay_capsule.route_receipt_id
        );
        assert_eq!(route_receipt.failure_reason, None);
        assert_eq!(
            route_receipt.selected_target,
            Some(ProviderResourceId::parse("prvrsrc_openai_primary").unwrap())
        );
    }

    #[tokio::test]
    async fn memory_store_create_relay_evaluation_records_successful_route_receipt() {
        let store = StoreMode::memory();
        let tenant_id = TenantId::parse("tenant_acme").unwrap();

        let evaluation = store
            .create_relay_evaluation(&tenant_id, "trialconn_acme_relay")
            .await
            .unwrap();
        let replay_capsule = store
            .get_replay_capsule(&tenant_id, evaluation.replay_capsule_id.as_str())
            .await
            .unwrap()
            .expect("created replay capsule should exist")
            .replay_capsule;
        let route_receipt = store
            .get_route_receipt(replay_capsule.route_receipt_id.as_str())
            .await
            .unwrap()
            .expect("created route receipt should exist")
            .route_receipt;

        assert_eq!(
            route_receipt.route_receipt_id,
            replay_capsule.route_receipt_id
        );
        assert_eq!(route_receipt.failure_reason, None);
        assert_eq!(route_receipt.admission_result, AdmissionResult::Admitted);
    }

    #[tokio::test]
    async fn memory_store_get_route_receipt_diagnostics_returns_trace_context() {
        let store = StoreMode::memory();
        store.insert_route_receipt_for_tests(sample_route_receipt(
            "routercpt_store_diag",
            "tenant_acme",
            "proj_core",
            "openai_chat",
            "2026-04-22T00:01:00Z",
        ));

        let diagnostics = store
            .get_route_receipt_diagnostics("routercpt_store_diag")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            diagnostics.route_receipt.route_receipt_id.as_str(),
            "routercpt_store_diag"
        );
        assert_eq!(
            diagnostics.metadata.get("source").map(String::as_str),
            Some("memory_store")
        );
        assert_eq!(diagnostics.policy_checks.len(), 1);
        assert_eq!(diagnostics.decision_timeline[0].stage, "route_receipt");
    }

    #[tokio::test]
    async fn memory_store_activation_supersedes_previous_project_snapshot() {
        let store = StoreMode::memory();
        let newer = core_domain::ConfigSnapshot {
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_gateway_v2").unwrap(),
            tenant_id: TenantId::parse("tenant_acme").unwrap(),
            project_id: ProjectId::parse("proj_core").unwrap(),
            revision: 2,
            status: core_domain::ConfigSnapshotStatus::Draft,
            activated_at: None,
            provider_resource_ids: vec![
                ProviderResourceId::parse("prvrsrc_openai_backup").unwrap(),
            ],
            route_policy_id: RoutePolicyId::parse("routepol_acme_support").unwrap(),
            budget_policy_id: core_domain::BudgetPolicyId::parse("budgetpol_default").unwrap(),
        };
        let created = store.create_config_snapshot(newer).await.unwrap();
        assert_eq!(created.config_snapshot_id.as_str(), "cfgsnap_gateway_v2");

        let activated = store
            .activate_config_snapshot("cfgsnap_gateway_v2")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            activated.config_snapshot.config_snapshot_id.as_str(),
            "cfgsnap_gateway_v2"
        );

        let snapshots = store.list_config_snapshots().await.unwrap().data;
        let gateway_v1 = snapshots
            .iter()
            .find(|snapshot| snapshot.config_snapshot_id.as_str() == "cfgsnap_gateway_v1")
            .unwrap();
        let gateway_v2 = snapshots
            .iter()
            .find(|snapshot| snapshot.config_snapshot_id.as_str() == "cfgsnap_gateway_v2")
            .unwrap();
        assert_eq!(
            gateway_v1.status,
            core_domain::ConfigSnapshotStatus::Superseded
        );
        assert_eq!(gateway_v2.status, core_domain::ConfigSnapshotStatus::Active);
    }
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn expires_at(seconds: u64) -> String {
    let seconds_i64 = i64::try_from(seconds).unwrap_or(i64::MAX);
    let timestamp = OffsetDateTime::now_utc() + time::Duration::seconds(seconds_i64);
    timestamp
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn lease_until_rfc3339() -> String {
    expires_at(60 * 5)
}

fn timestamp_is_expired(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|timestamp| timestamp <= OffsetDateTime::now_utc())
        .unwrap_or(true)
}

fn timestamp_is_future(value: &str) -> bool {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|timestamp| timestamp > OffsetDateTime::now_utc())
        .unwrap_or(true)
}

fn timestamp_after(left: &str, right: &str) -> bool {
    let Ok(left) = OffsetDateTime::parse(left, &Rfc3339) else {
        return false;
    };
    let Ok(right) = OffsetDateTime::parse(right, &Rfc3339) else {
        return false;
    };
    left > right
}

fn min_timestamp(left: &str, right: &str) -> Option<String> {
    let left = OffsetDateTime::parse(left, &Rfc3339).ok()?;
    let right = OffsetDateTime::parse(right, &Rfc3339).ok()?;
    let value = if left <= right { left } else { right };
    value.format(&Rfc3339).ok()
}

fn timestamp_plus_days(value: &str, days: u32) -> Option<String> {
    let value = OffsetDateTime::parse(value, &Rfc3339).ok()?;
    let value = value.checked_add(time::Duration::days(i64::from(days)))?;
    value.format(&Rfc3339).ok()
}

const fn default_record_version() -> u64 {
    1
}

fn default_delivery_owner_account_id() -> String {
    DEFAULT_OWNER_ACCOUNT_ID.to_string()
}

fn default_delivery_account_bundle_encryption_protocol() -> String {
    DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_PROTOCOL_V2.to_string()
}

fn default_delivery_account_bundle_encryption_version() -> String {
    DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_VERSION_V2.to_string()
}

fn default_delivery_artifact_secret_kind() -> String {
    DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK.to_string()
}

fn hydrate_upload_batch_item_ciphertext(
    mut item: DeliveryUploadBatchItemRecord,
    ciphertext: Vec<u8>,
) -> DeliveryUploadBatchItemRecord {
    item.ciphertext = ciphertext;
    item
}

async fn postgres_delivery_upload_item_outcome(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    item: &DeliveryUploadBatchItemRecord,
    batch: &DeliveryUploadBatchRecord,
    actor_id: &str,
) -> Result<DeliveryUploadItemOutcome> {
    let Some(delivery_row) = sqlx::query(
        "SELECT payload
           FROM deliveries
          WHERE delivery_id = $1
          FOR UPDATE",
    )
    .bind(&item.delivery_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(DeliveryUploadItemOutcome::Rejected {
            code: "delivery_not_found",
            message: format!("delivery `{}` was not found", item.delivery_id),
        });
    };
    let delivery = delivery_row.get::<Json<DeliveryRecord>, _>("payload").0;
    if delivery.tenant_id != batch.tenant_id || delivery.project_id != batch.project_id {
        return Ok(DeliveryUploadItemOutcome::Rejected {
            code: "delivery_scope_mismatch",
            message: "delivery does not belong to the upload batch tenant/project".to_string(),
        });
    }
    if delivery.provider != batch.provider {
        return Ok(DeliveryUploadItemOutcome::Rejected {
            code: "delivery_provider_mismatch",
            message: "delivery provider does not match upload batch provider".to_string(),
        });
    }
    let Some(entitlement_row) = sqlx::query(
        "SELECT payload
           FROM delivery_entitlements
          WHERE delivery_id = $1
          FOR UPDATE",
    )
    .bind(&delivery.delivery_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(DeliveryUploadItemOutcome::Failed {
            code: "delivery_entitlement_missing",
            message: "delivery entitlement is missing".to_string(),
        });
    };
    let entitlement = entitlement_row
        .get::<Json<DeliveryEntitlementRecord>, _>("payload")
        .0;
    if delivery.effective_status(&entitlement) != DELIVERY_STATUS_PREPARED {
        return Ok(DeliveryUploadItemOutcome::Rejected {
            code: "delivery_not_artifact_ready",
            message: format!(
                "delivery `{}` status `{}` cannot accept artifacts",
                delivery.delivery_id,
                delivery.effective_status(&entitlement)
            ),
        });
    }

    let existing_rows = sqlx::query(
        "SELECT payload
           FROM delivery_artifacts
          WHERE delivery_id = $1
          FOR UPDATE",
    )
    .bind(&delivery.delivery_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut existing = existing_rows
        .into_iter()
        .map(|row| row.get::<Json<DeliveryArtifactRecord>, _>("payload").0)
        .collect::<Vec<_>>();
    if let Some(artifact) = existing.iter().find(|artifact| {
        artifact.sha256 == item.payload_sha256 && artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE
    }) {
        return Ok(DeliveryUploadItemOutcome::Duplicate {
            artifact_id: artifact.artifact_id.clone(),
        });
    }

    let artifact = build_delivery_artifact_record(
        &delivery_upload_item_artifact_draft(item, &delivery, actor_id),
        &existing,
    );
    for existing_artifact in existing
        .iter_mut()
        .filter(|artifact| artifact.status == DELIVERY_ARTIFACT_STATUS_ACTIVE)
    {
        existing_artifact.status = DELIVERY_ARTIFACT_STATUS_SUPERSEDED.to_string();
        existing_artifact.superseded_at = Some(artifact.created_at.clone());
        existing_artifact.superseded_by = Some(artifact.artifact_id.clone());
        existing_artifact.updated_at = artifact.created_at.clone();
        sqlx::query(
            "UPDATE delivery_artifacts
                SET status = $2, payload = $3, updated_at = $4
              WHERE artifact_id = $1",
        )
        .bind(&existing_artifact.artifact_id)
        .bind(&existing_artifact.status)
        .bind(Json(&existing_artifact))
        .bind(&existing_artifact.updated_at)
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO delivery_artifacts
            (artifact_id, delivery_id, tenant_id, project_id, status, version, sha256, size_bytes, storage_backend, storage_ref, ciphertext, payload, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(&artifact.artifact_id)
    .bind(&artifact.delivery_id)
    .bind(artifact.tenant_id.as_str())
    .bind(artifact.project_id.as_str())
    .bind(&artifact.status)
    .bind(i64::try_from(artifact.version).unwrap_or(i64::MAX))
    .bind(&artifact.sha256)
    .bind(i64::try_from(artifact.size_bytes).unwrap_or(i64::MAX))
    .bind(&artifact.storage_backend)
    .bind(&artifact.storage_ref)
    .bind(&artifact.ciphertext)
    .bind(Json(&artifact))
    .bind(&artifact.created_at)
    .bind(&artifact.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(DeliveryUploadItemOutcome::Accepted {
        artifact_id: artifact.artifact_id,
    })
}

async fn update_postgres_upload_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    batch: &DeliveryUploadBatchRecord,
) -> Result<()> {
    sqlx::query(
        "UPDATE delivery_upload_batches
            SET status = $2,
                success_count = $3,
                failed_count = $4,
                duplicate_count = $5,
                payload = $6,
                updated_at = $7
          WHERE batch_id = $1",
    )
    .bind(&batch.batch_id)
    .bind(&batch.status)
    .bind(i64::from(batch.success_count))
    .bind(i64::from(batch.failed_count))
    .bind(i64::from(batch.duplicate_count))
    .bind(Json(batch))
    .bind(&batch.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn update_postgres_upload_item(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    item: &DeliveryUploadBatchItemRecord,
) -> Result<()> {
    sqlx::query(
        "UPDATE delivery_upload_batch_items
            SET status = $2,
                artifact_id = $3,
                payload = $4,
                updated_at = $5
          WHERE item_id = $1",
    )
    .bind(&item.item_id)
    .bind(&item.status)
    .bind(item.artifact_id.as_deref())
    .bind(Json(item))
    .bind(&item.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn finalize_postgres_delivery_upload_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    batch: &mut DeliveryUploadBatchRecord,
    items: &[DeliveryUploadBatchItemRecord],
) -> Result<()> {
    let item_refs = items.iter().collect::<Vec<_>>();
    let total_count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let success_count = count_upload_items(&item_refs, DELIVERY_UPLOAD_ITEM_STATUS_ACCEPTED);
    let duplicate_count = count_upload_items(&item_refs, DELIVERY_UPLOAD_ITEM_STATUS_DUPLICATE);
    let failed_count = items
        .iter()
        .filter(|item| {
            matches!(
                item.status.as_str(),
                DELIVERY_UPLOAD_ITEM_STATUS_REJECTED | DELIVERY_UPLOAD_ITEM_STATUS_FAILED
            )
        })
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    batch.status =
        final_upload_batch_status(total_count, success_count, duplicate_count, failed_count);
    batch.total_count = total_count;
    batch.success_count = success_count;
    batch.duplicate_count = duplicate_count;
    batch.failed_count = failed_count;
    batch.error_summary = first_upload_error_summary(&item_refs);
    let now = now_rfc3339();
    batch.finished_at = Some(now.clone());
    batch.updated_at = now;
    batch.version = batch.version.saturating_add(1);
    update_postgres_upload_batch(tx, batch).await
}

async fn update_postgres_entitlement(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    entitlement: &DeliveryEntitlementRecord,
) -> Result<()> {
    sqlx::query(
        "UPDATE delivery_entitlements
            SET status = $2, ends_at = $3, payload = $4, updated_at = $5
          WHERE entitlement_id = $1",
    )
    .bind(&entitlement.entitlement_id)
    .bind(&entitlement.status)
    .bind(&entitlement.ends_at)
    .bind(Json(entitlement))
    .bind(&entitlement.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_postgres_service_segments(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    segments: &[DeliveryServiceSegmentRecord],
) -> Result<()> {
    for segment in segments {
        sqlx::query(
            "INSERT INTO delivery_service_segments
                (segment_id, entitlement_id, activation_id, delivery_id, artifact_id, tenant_id, project_id, status, effective_from, effective_until, carrier_valid_until, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(&segment.segment_id)
        .bind(&segment.entitlement_id)
        .bind(&segment.activation_id)
        .bind(&segment.delivery_id)
        .bind(&segment.artifact_id)
        .bind(segment.tenant_id.as_str())
        .bind(segment.project_id.as_str())
        .bind(&segment.status)
        .bind(&segment.effective_from)
        .bind(&segment.effective_until)
        .bind(&segment.carrier_valid_until)
        .bind(Json(segment))
        .bind(&segment.created_at)
        .bind(&segment.updated_at)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn insert_postgres_lifecycle_events(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    events: &[DeliveryLifecycleEventRecord],
) -> Result<()> {
    for event in events {
        sqlx::query(
            "INSERT INTO delivery_lifecycle_events
                (event_id, entitlement_id, segment_id, event_type, status, payload, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&event.event_id)
        .bind(&event.entitlement_id)
        .bind(event.segment_id.as_deref())
        .bind(&event.event_type)
        .bind(&event.status)
        .bind(Json(event))
        .bind(&event.created_at)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn resolve_active_tenant_id(
    existing: Option<&TenantId>,
    memberships: &[TenantMembership],
) -> Option<TenantId> {
    existing
        .filter(|tenant_id| {
            memberships
                .iter()
                .any(|membership| membership.tenant.id == **tenant_id)
        })
        .cloned()
        .or_else(|| {
            memberships
                .iter()
                .find(|membership| membership.status == TenantMembershipStatus::Active)
                .map(|membership| membership.tenant.id.clone())
        })
}

fn env_flag_enabled(name: &str, default: bool) -> bool {
    std::env::var(name).ok().map_or(default, |value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn env_flag_not_disabled(name: &str) -> bool {
    std::env::var(name).ok().is_none_or(|value| {
        !matches!(
            value.to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        )
    })
}

fn env_present(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
}

fn oauth_client_configured(slug: &str) -> bool {
    env_present(&format!("CONTROL_PLANE_OAUTH_{slug}_CLIENT_ID"))
        && env_present(&format!("CONTROL_PLANE_OAUTH_{slug}_CLIENT_SECRET"))
}

pub fn mock_auth_enabled() -> bool {
    env_flag_enabled("CONTROL_PLANE_ALLOW_MOCK_AUTH", false)
}

pub fn auth_provider_enabled(provider: AuthProvider) -> bool {
    match provider {
        AuthProvider::Email => env_flag_enabled("CONTROL_PLANE_AUTH_EMAIL_ENABLED", true),
        AuthProvider::Github => {
            mock_auth_enabled()
                || (env_flag_not_disabled("CONTROL_PLANE_AUTH_GITHUB_ENABLED")
                    && oauth_client_configured("GITHUB"))
        }
        AuthProvider::Google => {
            mock_auth_enabled()
                || (env_flag_not_disabled("CONTROL_PLANE_AUTH_GOOGLE_ENABLED")
                    && oauth_client_configured("GOOGLE"))
        }
        AuthProvider::Wechat => {
            mock_auth_enabled()
                || (env_flag_not_disabled("CONTROL_PLANE_AUTH_WECHAT_ENABLED")
                    && oauth_client_configured("WECHAT"))
        }
        AuthProvider::Oidc => {
            mock_auth_enabled()
                || (env_flag_not_disabled("CONTROL_PLANE_AUTH_OIDC_ENABLED") && oidc_enabled())
        }
    }
}

pub const fn oauth_provider_slug(provider: OAuthProvider) -> &'static str {
    match provider {
        OAuthProvider::Github => "github",
        OAuthProvider::Google => "google",
        OAuthProvider::Wechat => "wechat",
        OAuthProvider::Oidc => "oidc",
    }
}

pub const fn auth_provider_slug(provider: AuthProvider) -> &'static str {
    match provider {
        AuthProvider::Email => "email",
        AuthProvider::Github => "github",
        AuthProvider::Google => "google",
        AuthProvider::Wechat => "wechat",
        AuthProvider::Oidc => "oidc",
    }
}

pub fn oidc_enabled() -> bool {
    env_present("CONTROL_PLANE_OIDC_AUTHORIZATION_URL")
        && env_present("CONTROL_PLANE_OIDC_TOKEN_URL")
        && env_present("CONTROL_PLANE_OIDC_CLIENT_ID")
}

const fn login_flow_kind_slug(kind: LoginFlowKind) -> &'static str {
    match kind {
        LoginFlowKind::Email => "email",
        LoginFlowKind::OAuth => "oauth",
    }
}

const fn config_snapshot_status_slug(status: ConfigSnapshotStatus) -> &'static str {
    match status {
        ConfigSnapshotStatus::Draft => "draft",
        ConfigSnapshotStatus::Active => "active",
        ConfigSnapshotStatus::Superseded => "superseded",
    }
}
