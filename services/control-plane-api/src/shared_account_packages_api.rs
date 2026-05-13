use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    ApiError, ControlPlaneState, next_request_context,
};

#[derive(Debug, Clone, Serialize)]
pub struct SharedAccountPackageHeadResponse {
    pub data: crate::store::SharedAccountPackageHeadRecord,
}

#[derive(Debug, Clone, Serialize)]
pub struct SharedAccountPackageVersionResponse {
    pub data: crate::store::SharedAccountPackageVersionRecord,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishSharedAccountPackageVersionRequest {
    pub file_name: String,
    pub file_hash: String,
    pub import_secret: String,
    pub serialized: String,
    pub base_version: u64,
    pub updated_by_client_id: String,
}

pub async fn get_shared_account_package_head(
    State(state): State<ControlPlaneState>,
    Path(account_id): Path<String>,
) -> Result<Json<SharedAccountPackageHeadResponse>, ApiError> {
    let context = next_request_context();
    let head = state
        .store
        .get_shared_account_package_head(&account_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "shared_account_package_head_unavailable",
                format!("failed to load shared account package head: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "shared_account_package_head_not_found",
                format!("shared account package `{account_id}` was not found"),
                &context,
            )
        })?;
    Ok(Json(SharedAccountPackageHeadResponse { data: head }))
}

pub async fn publish_shared_account_package_version(
    State(state): State<ControlPlaneState>,
    Path(account_id): Path<String>,
    Json(request): Json<PublishSharedAccountPackageVersionRequest>,
) -> Result<Json<SharedAccountPackageVersionResponse>, ApiError> {
    let context = next_request_context();
    if request.import_secret.trim().len() < 8 {
        return Err(ApiError::bad_request(
            "shared_account_package_import_secret_invalid",
            "import_secret must be at least 8 characters".to_string(),
            &context,
        ));
    }
    let (_head, version) = state
        .store
        .publish_shared_account_package_version(crate::store::PublishSharedAccountPackageVersionDraft {
            account_id,
            file_name: request.file_name,
            file_hash: request.file_hash,
            import_secret: request.import_secret,
            serialized: request.serialized,
            base_version: request.base_version,
            created_at: crate::store::now_rfc3339(),
            updated_by_client_id: request.updated_by_client_id,
        })
        .await
        .map_err(|error| {
            ApiError::internal(
                "shared_account_package_publish_failed",
                format!("failed to publish shared account package version: {error}"),
                &context,
            )
        })?;
    Ok(Json(SharedAccountPackageVersionResponse { data: version }))
}
