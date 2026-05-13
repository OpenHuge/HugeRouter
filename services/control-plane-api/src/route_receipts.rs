use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use protocol_ir::{RouteReceiptDiagnosticsResponse, RouteReceiptResponse};

use crate::store::{RouteReceiptFilters, RouteReceiptsResponse};
use crate::{
    ApiError, ControlPlaneState, authorize_v1_request, ensure_project_matches_tenant, load_project,
    next_request_context,
};

pub async fn get(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(route_receipt_id): Path<String>,
) -> Result<Json<RouteReceiptResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let receipt = state
        .store
        .get_route_receipt(&route_receipt_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route receipt: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "route_receipt_not_found",
                format!("route receipt `{route_receipt_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(receipt.route_receipt.tenant_id.as_str(), &context)?;
    Ok(Json(receipt))
}

pub async fn get_diagnostics(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Path(route_receipt_id): Path<String>,
) -> Result<Json<RouteReceiptDiagnosticsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    let diagnostics = state
        .store
        .get_route_receipt_diagnostics(&route_receipt_id)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route receipt diagnostics: {error}"),
                &context,
            )
        })?
        .ok_or_else(|| {
            ApiError::not_found(
                "route_receipt_not_found",
                format!("route receipt `{route_receipt_id}` was not found"),
                &context,
            )
        })?;
    authz.ensure_read_tenant(diagnostics.route_receipt.tenant_id.as_str(), &context)?;
    Ok(Json(diagnostics))
}

pub async fn list(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
    Query(filters): Query<RouteReceiptFilters>,
) -> Result<Json<RouteReceiptsResponse>, ApiError> {
    let context = next_request_context();
    let authz = authorize_v1_request(&state, &headers, &context).await?;
    if let Some(project_id) = filters.project_id.as_deref() {
        let project = load_project(&state, project_id, &context).await?;
        if let Some(tenant_id) = filters.tenant_id.as_deref() {
            ensure_project_matches_tenant(&project, tenant_id, &context)?;
            authz.ensure_read_tenant(tenant_id, &context)?;
        }
        authz.ensure_read_project(&project, &context)?;
    } else if let Some(tenant_id) = filters.tenant_id.as_deref() {
        authz.ensure_read_tenant(tenant_id, &context)?;
    }
    let mut response = state
        .store
        .list_route_receipts(&filters)
        .await
        .map_err(|error| {
            ApiError::internal(
                "storage_unavailable",
                format!("failed to load route receipts: {error}"),
                &context,
            )
        })?;
    response.data = authz.filter_route_receipts(response.data);
    Ok(Json(response))
}
