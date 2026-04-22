use anyhow::Result;
use protocol_ir::RequestEnvelope;

pub trait ProviderAdapter: Send + Sync {
    fn provider_kind(&self) -> &'static str;

    /// Returns a provider-specific placeholder payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider adapter cannot handle the request.
    fn handle(&self, request: &RequestEnvelope) -> Result<String>;
}
