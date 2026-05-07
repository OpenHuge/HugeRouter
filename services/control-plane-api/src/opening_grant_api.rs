use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD};
use protocol_ir::{CreateOpeningGrantRequest, SaleReadyPackageResponse};
use ring::rand;

use crate::{ApiError, RequestContext};

pub const OPENING_CREDENTIAL_KIND_API_KEY: &str = "api_key";
const OPENING_SCOPE_ROUTE_CODEX: &str = "route:codex";
const OPENING_SCOPE_PROVIDER_COMMERCIAL: &str = "provider:hugerouter-commercial";

pub fn validate_opening_grantee(
    request: &CreateOpeningGrantRequest,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let grantee_kind = request.grantee_kind.trim();
    if !matches!(grantee_kind, "customer" | "agent" | "internal_test") {
        return Err(ApiError::bad_request(
            "opening_grantee_kind_invalid",
            "grantee_kind must be customer, agent, or internal_test".to_string(),
            context,
        ));
    }
    if request.grantee_id.trim().is_empty() {
        return Err(ApiError::bad_request(
            "opening_grantee_required",
            "grantee_id is required".to_string(),
            context,
        ));
    }
    Ok(())
}

pub fn validate_opening_owner_account_id(
    owner_account_id: &str,
    context: &RequestContext,
) -> Result<String, ApiError> {
    let owner_account_id = owner_account_id.trim();
    if owner_account_id.is_empty()
        || owner_account_id.len() > 128
        || !owner_account_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(ApiError::bad_request(
            "opening_owner_account_id_invalid",
            "owner_account_id must be 1-128 ascii letters, digits, underscore, hyphen, or dot"
                .to_string(),
            context,
        ));
    }
    Ok(owner_account_id.to_string())
}

pub fn opening_scopes(
    requested_scopes: &[String],
    package: &SaleReadyPackageResponse,
    context: &RequestContext,
) -> Result<Vec<String>, ApiError> {
    let mut scopes = if requested_scopes.is_empty() {
        let mut scopes = vec![
            OPENING_SCOPE_ROUTE_CODEX.to_string(),
            OPENING_SCOPE_PROVIDER_COMMERCIAL.to_string(),
        ];
        if let Some(route_policy) = &package.route_policy {
            scopes.push(format!("protocol:{}", route_policy.protocol_family));
            scopes.push(format!("model:{}", route_policy.model_alias));
        }
        scopes
    } else {
        requested_scopes
            .iter()
            .map(|scope| scope.trim())
            .filter(|scope| !scope.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    scopes.sort();
    scopes.dedup();

    let has_route_scope = scopes
        .iter()
        .any(|scope| scope == OPENING_SCOPE_ROUTE_CODEX);
    let has_provider_scope = scopes
        .iter()
        .any(|scope| scope == OPENING_SCOPE_PROVIDER_COMMERCIAL);
    if !has_route_scope || !has_provider_scope {
        return Err(ApiError::bad_request(
            "opening_scope_invalid",
            format!(
                "scopes must include `{OPENING_SCOPE_ROUTE_CODEX}` and `{OPENING_SCOPE_PROVIDER_COMMERCIAL}`"
            ),
            context,
        ));
    }
    Ok(scopes)
}

pub fn generate_opening_api_key(context: &RequestContext) -> Result<String, ApiError> {
    let rng = rand::SystemRandom::new();
    let mut token_bytes = [0_u8; 24];
    rand::SecureRandom::fill(&rng, &mut token_bytes).map_err(|_| {
        ApiError::internal(
            "credential_generation_failed",
            "failed to generate opening credential".to_string(),
            context,
        )
    })?;
    Ok(format!(
        "akp_{}",
        BASE64_URL_SAFE_NO_PAD.encode(token_bytes)
    ))
}
