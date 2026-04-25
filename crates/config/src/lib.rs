use thiserror::Error;

pub const DEFAULT_GATEWAY_API_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_CONTROL_PLANE_INTERNAL_TOKEN: &str = "change-me";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    Development,
    Test,
    Production,
}

impl AppEnvironment {
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidEnvironment`] when the value is not a
    /// supported `HugeRouter` runtime environment.
    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "development" | "dev" | "local" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(ConfigError::InvalidEnvironment {
                value: value.to_string(),
            }),
        }
    }

    #[must_use]
    pub const fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayApiConfig {
    pub bind_address: String,
    pub environment: AppEnvironment,
}

impl GatewayApiConfig {
    #[must_use]
    pub fn development_defaults() -> Self {
        Self {
            bind_address: DEFAULT_GATEWAY_API_ADDR.to_string(),
            environment: AppEnvironment::Development,
        }
    }

    /// # Errors
    ///
    /// Returns [`ConfigError`] when environment values are invalid or when the
    /// production gateway is missing required control-plane credentials.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// # Errors
    ///
    /// Returns [`ConfigError`] when supplied values are invalid or when the
    /// production gateway is missing required control-plane credentials.
    pub fn from_lookup(
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let environment = lookup("HUGE_ROUTER_ENV")
            .or_else(|| lookup("HUGEROUTER_ENV"))
            .as_deref()
            .map_or(Ok(AppEnvironment::Development), AppEnvironment::parse)?;

        let bind_address = match lookup("GATEWAY_API_ADDR") {
            Some(value) if value.trim().is_empty() => {
                return Err(ConfigError::EmptyEnvVar {
                    name: "GATEWAY_API_ADDR",
                });
            }
            Some(value) => value,
            None => DEFAULT_GATEWAY_API_ADDR.to_string(),
        };

        validate_gateway_production_secrets(environment, |name| lookup(name))?;

        Ok(Self {
            bind_address,
            environment,
        })
    }
}

fn validate_gateway_production_secrets(
    environment: AppEnvironment,
    mut lookup: impl FnMut(&str) -> Option<String>,
) -> Result<(), ConfigError> {
    if !environment.is_production() {
        return Ok(());
    }

    let token = lookup("CONTROL_PLANE_INTERNAL_TOKEN")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::MissingRequiredEnv {
            name: "CONTROL_PLANE_INTERNAL_TOKEN",
        })?;

    if token == DEFAULT_CONTROL_PLANE_INTERNAL_TOKEN {
        return Err(ConfigError::InsecureDefaultSecret {
            name: "CONTROL_PLANE_INTERNAL_TOKEN",
        });
    }

    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    #[error("environment variable `{name}` must not be empty")]
    EmptyEnvVar { name: &'static str },
    #[error("environment variable `{name}` must not use the documented placeholder secret")]
    InsecureDefaultSecret { name: &'static str },
    #[error("unsupported HUGE_ROUTER_ENV value `{value}`")]
    InvalidEnvironment { value: String },
    #[error("missing required environment variable `{name}`")]
    MissingRequiredEnv { name: &'static str },
}

#[cfg(test)]
mod tests {
    use super::{AppEnvironment, ConfigError, DEFAULT_GATEWAY_API_ADDR, GatewayApiConfig};
    use std::collections::HashMap;

    fn config_from(entries: &[(&str, &str)]) -> Result<GatewayApiConfig, ConfigError> {
        let values = entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();

        GatewayApiConfig::from_lookup(|name| values.get(name).cloned())
    }

    #[test]
    fn development_config_uses_safe_local_defaults() {
        let config = config_from(&[]).unwrap();

        assert_eq!(config.environment, AppEnvironment::Development);
        assert_eq!(config.bind_address, DEFAULT_GATEWAY_API_ADDR);
    }

    #[test]
    fn production_requires_control_plane_internal_token() {
        let error = config_from(&[("HUGEROUTER_ENV", "production")]).unwrap_err();

        assert_eq!(
            error,
            ConfigError::MissingRequiredEnv {
                name: "CONTROL_PLANE_INTERNAL_TOKEN"
            }
        );
    }

    #[test]
    fn production_rejects_documented_placeholder_token() {
        let error = config_from(&[
            ("HUGE_ROUTER_ENV", "production"),
            ("CONTROL_PLANE_INTERNAL_TOKEN", "change-me"),
        ])
        .unwrap_err();

        assert_eq!(
            error,
            ConfigError::InsecureDefaultSecret {
                name: "CONTROL_PLANE_INTERNAL_TOKEN"
            }
        );
    }

    #[test]
    fn production_accepts_explicit_control_plane_internal_token() {
        let config = config_from(&[
            ("HUGE_ROUTER_ENV", "prod"),
            ("GATEWAY_API_ADDR", "0.0.0.0:8080"),
            ("CONTROL_PLANE_INTERNAL_TOKEN", "prod-internal-token"),
        ])
        .unwrap();

        assert_eq!(config.environment, AppEnvironment::Production);
        assert_eq!(config.bind_address, "0.0.0.0:8080");
    }
}
