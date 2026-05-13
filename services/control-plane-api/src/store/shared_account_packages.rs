use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedAccountPackageHeadRecord {
    pub account_id: String,
    pub current_version: u64,
    pub file_name: String,
    pub file_hash: String,
    pub import_secret: String,
    pub serialized: String,
    pub updated_at: String,
    pub updated_by_client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedAccountPackageVersionRecord {
    pub account_id: String,
    pub version: u64,
    pub file_name: String,
    pub file_hash: String,
    pub import_secret: String,
    pub serialized: String,
    pub base_version: u64,
    pub created_at: String,
    pub updated_by_client_id: String,
}

#[derive(Debug, Clone)]
pub struct PublishSharedAccountPackageVersionDraft {
    pub account_id: String,
    pub file_name: String,
    pub file_hash: String,
    pub import_secret: String,
    pub serialized: String,
    pub base_version: u64,
    pub created_at: String,
    pub updated_by_client_id: String,
}
