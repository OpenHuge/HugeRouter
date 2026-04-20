use core_domain::ServiceName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub protocol: &'static str,
    pub source_service: ServiceName,
}

impl RequestEnvelope {
    #[must_use]
    pub const fn bootstrap() -> Self {
        Self {
            protocol: "bootstrap-placeholder",
            source_service: ServiceName("gateway-api"),
        }
    }
}
