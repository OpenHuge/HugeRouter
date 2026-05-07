use super::{ResolvedApiKey, timestamp_is_expired};
use core_domain::{
    BudgetPolicyId, ConfigSnapshotId, DEFAULT_OWNER_ACCOUNT_ID, ProjectId, ProviderResourceId,
    RoutePolicyId, TenantId,
};
use protocol_ir::OpeningGrant;
use serde::{Deserialize, Serialize};

pub const OPENING_GRANT_MAX_ACTIVE_CHILD_KEYS: usize = 8;

pub(super) const OPENING_GRANT_OWNER_LOCK_INSERT_SQL: &str = "INSERT INTO opening_grant_owner_locks
    (tenant_id, project_id, owner_account_id, created_at)
    VALUES ($1, $2, $3, $4)
    ON CONFLICT (tenant_id, project_id, owner_account_id) DO NOTHING";

pub(super) const OPENING_GRANT_OWNER_LOCK_SELECT_SQL: &str = "SELECT owner_account_id
    FROM opening_grant_owner_locks
    WHERE tenant_id = $1 AND project_id = $2 AND owner_account_id = $3
    FOR UPDATE";

#[derive(Debug, Clone)]
pub struct OpeningGrantDraft {
    pub grant_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub owner_account_id: String,
    pub grantee_kind: String,
    pub grantee_id: String,
    pub grantee_label: Option<String>,
    pub config_snapshot_id: ConfigSnapshotId,
    pub route_policy_id: RoutePolicyId,
    pub budget_policy_id: BudgetPolicyId,
    pub provider_resource_ids: Vec<ProviderResourceId>,
    pub credential_kind: String,
    pub scopes: Vec<String>,
    pub expires_at: String,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningGrantRecord {
    pub grant_id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    #[serde(default = "default_owner_account_id")]
    pub owner_account_id: String,
    pub grantee_kind: String,
    pub grantee_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grantee_label: Option<String>,
    pub config_snapshot_id: ConfigSnapshotId,
    pub route_policy_id: RoutePolicyId,
    pub budget_policy_id: BudgetPolicyId,
    pub provider_resource_ids: Vec<ProviderResourceId>,
    pub credential_kind: String,
    pub credential_id: String,
    pub credential_key_prefix: String,
    pub credential_last_four: String,
    pub credential_hash: String,
    pub scopes: Vec<String>,
    pub expires_at: String,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpeningGrantCreateResult {
    Created(Box<OpeningGrant>),
    OwnerLimitReached { active_count: usize, limit: usize },
    ActiveGranteeExists { grant_id: String },
}

impl OpeningGrantRecord {
    pub(super) fn effective_status(&self) -> String {
        if self.status == "active" && timestamp_is_expired(&self.expires_at) {
            "expired".to_string()
        } else {
            self.status.clone()
        }
    }

    pub(super) fn public_view(&self) -> OpeningGrant {
        OpeningGrant {
            grant_id: self.grant_id.clone(),
            tenant_id: self.tenant_id.clone(),
            project_id: self.project_id.clone(),
            owner_account_id: self.owner_account_id.clone(),
            grantee_kind: self.grantee_kind.clone(),
            grantee_id: self.grantee_id.clone(),
            grantee_label: self.grantee_label.clone(),
            config_snapshot_id: self.config_snapshot_id.clone(),
            route_policy_id: self.route_policy_id.clone(),
            budget_policy_id: self.budget_policy_id.clone(),
            provider_resource_ids: self.provider_resource_ids.clone(),
            credential_kind: self.credential_kind.clone(),
            credential_id: self.credential_id.clone(),
            credential_key_prefix: self.credential_key_prefix.clone(),
            credential_last_four: self.credential_last_four.clone(),
            scopes: self.scopes.clone(),
            expires_at: self.expires_at.clone(),
            status: self.effective_status(),
            created_by: self.created_by.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            version: self.version,
            revoked_at: self.revoked_at.clone(),
            revoked_by: self.revoked_by.clone(),
        }
    }

    pub(super) fn to_resolved(&self) -> ResolvedApiKey {
        let status = self.effective_status();
        ResolvedApiKey {
            api_key_id: self.credential_id.clone(),
            grant_id: Some(self.grant_id.clone()),
            owner_account_id: Some(self.owner_account_id.clone()),
            tenant_id: self.tenant_id.clone(),
            project_id: Some(self.project_id.clone()),
            is_active: status == "active",
            status,
            config_snapshot_id: Some(self.config_snapshot_id.clone()),
            route_policy_id: Some(self.route_policy_id.clone()),
            scopes: self.scopes.clone(),
            expires_at: Some(self.expires_at.clone()),
        }
    }
}

pub(super) fn opening_grant_create_blocker(
    existing: &[OpeningGrantRecord],
    candidate: &OpeningGrantRecord,
) -> Option<OpeningGrantCreateResult> {
    let active_for_owner = |record: &&OpeningGrantRecord| {
        record.tenant_id == candidate.tenant_id
            && record.project_id == candidate.project_id
            && record.owner_account_id == candidate.owner_account_id
            && opening_grant_is_active(record)
    };

    if let Some(existing_grant) = existing.iter().filter(active_for_owner).find(|record| {
        record.grantee_kind == candidate.grantee_kind && record.grantee_id == candidate.grantee_id
    }) {
        return Some(OpeningGrantCreateResult::ActiveGranteeExists {
            grant_id: existing_grant.grant_id.clone(),
        });
    }

    let active_count = existing.iter().filter(active_for_owner).count();

    if active_count >= OPENING_GRANT_MAX_ACTIVE_CHILD_KEYS {
        return Some(OpeningGrantCreateResult::OwnerLimitReached {
            active_count,
            limit: OPENING_GRANT_MAX_ACTIVE_CHILD_KEYS,
        });
    }

    None
}

fn default_owner_account_id() -> String {
    DEFAULT_OWNER_ACCOUNT_ID.to_string()
}

fn opening_grant_is_active(record: &OpeningGrantRecord) -> bool {
    record.effective_status() == "active"
}

#[cfg(test)]
fn opening_grant_owner_lock_key(record: &OpeningGrantRecord) -> String {
    format!(
        "opening_grants:{}:{}:{}",
        record.tenant_id, record.project_id, record.owner_account_id
    )
}

#[cfg(test)]
mod tests {
    use super::{
        OPENING_GRANT_OWNER_LOCK_INSERT_SQL, OPENING_GRANT_OWNER_LOCK_SELECT_SQL,
        OpeningGrantRecord, opening_grant_owner_lock_key,
    };
    use core_domain::{
        BudgetPolicyId, ConfigSnapshotId, ProjectId, ProviderResourceId, RoutePolicyId, TenantId,
    };

    fn sample_opening_grant_record(
        tenant_id: &str,
        project_id: &str,
        owner_account_id: &str,
        grantee_id: &str,
    ) -> OpeningGrantRecord {
        OpeningGrantRecord {
            grant_id: format!("grant_{grantee_id}"),
            tenant_id: TenantId::parse(tenant_id).unwrap(),
            project_id: ProjectId::parse(project_id).unwrap(),
            owner_account_id: owner_account_id.to_string(),
            grantee_kind: "customer".to_string(),
            grantee_id: grantee_id.to_string(),
            grantee_label: None,
            config_snapshot_id: ConfigSnapshotId::parse("cfgsnap_gateway_v1").unwrap(),
            route_policy_id: RoutePolicyId::parse("routepol_openai_chat_default").unwrap(),
            budget_policy_id: BudgetPolicyId::parse("budgetpol_default").unwrap(),
            provider_resource_ids: vec![
                ProviderResourceId::parse("prvrsrc_openai_primary").unwrap(),
            ],
            credential_kind: "api_key".to_string(),
            credential_id: format!("cred_{grantee_id}"),
            credential_key_prefix: "akp".to_string(),
            credential_last_four: "test".to_string(),
            credential_hash: format!("hash_{grantee_id}"),
            scopes: vec!["route:codex".to_string()],
            expires_at: "2999-01-01T00:00:00Z".to_string(),
            status: "active".to_string(),
            created_by: "test".to_string(),
            created_at: "2026-05-07T00:00:00Z".to_string(),
            updated_at: "2026-05-07T00:00:00Z".to_string(),
            version: 1,
            revoked_at: None,
            revoked_by: None,
        }
    }

    #[test]
    fn opening_grant_owner_lock_key_is_scoped_to_owner_capacity_bucket() {
        let record =
            sample_opening_grant_record("tenant_acme", "proj_core", "acct_owner", "cust_one");
        let same_bucket =
            sample_opening_grant_record("tenant_acme", "proj_core", "acct_owner", "cust_two");
        let other_owner =
            sample_opening_grant_record("tenant_acme", "proj_core", "acct_other", "cust_one");

        assert_eq!(
            opening_grant_owner_lock_key(&record),
            "opening_grants:tenant_acme:proj_core:acct_owner"
        );
        assert_eq!(
            opening_grant_owner_lock_key(&record),
            opening_grant_owner_lock_key(&same_bucket)
        );
        assert_ne!(
            opening_grant_owner_lock_key(&record),
            opening_grant_owner_lock_key(&other_owner)
        );
        assert!(OPENING_GRANT_OWNER_LOCK_INSERT_SQL.contains("ON CONFLICT"));
        assert!(OPENING_GRANT_OWNER_LOCK_SELECT_SQL.contains("FOR UPDATE"));
    }
}
