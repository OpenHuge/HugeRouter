use routing_engine::{RoutingConfig, RoutingRequest};
use tracing::info;

use crate::{ActiveGatewayConfig, NormalizedChatRequest, RequestContext};

pub use routing_engine::{
    ProviderTargetRuntime, RankedTarget, RouteEvaluation, evaluate_route as evaluate_shared_route,
};

pub fn evaluate_route(
    active_config: &ActiveGatewayConfig,
    request: &NormalizedChatRequest,
    context: &RequestContext,
) -> RouteEvaluation {
    let route = evaluate_shared_route(
        &RoutingConfig {
            config_snapshot: active_config.config_snapshot.clone(),
            route_policy: active_config.route_policy.clone(),
            provider_targets: active_config.provider_targets.clone(),
        },
        &RoutingRequest {
            protocol_family: request.protocol_family.clone(),
            model_alias: request.model_alias.clone(),
        },
    );

    info!(
        request_id = context.request_id,
        trace_id = context.trace_id,
        candidate_count = route.ranked_targets.len(),
        excluded_count = route.excluded_targets.len(),
        "route evaluation completed"
    );

    route
}
