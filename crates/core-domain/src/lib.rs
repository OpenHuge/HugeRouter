use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceName(pub &'static str);

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("placeholder domain error: {0}")]
    Placeholder(&'static str),
}

