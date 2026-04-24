use crate::openai::OpenAiAdapter;
use provider_anthropic::AnthropicAdapter;
use provider_bedrock::BedrockConverseAdapter;
use provider_gateway::GatewayAdapter;
use provider_gemini::GeminiAdapter;
use provider_traits::{ProviderAdapter, ProviderAdapterRegistry, ProviderRegistryError};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt::{Display, Formatter},
    sync::Arc,
};

const ENABLED_PROVIDER_ENV_KEYS: &[&str] =
    &["GATEWAY_PROVIDER_ADAPTERS", "GATEWAY_ENABLED_PROVIDERS"];
const DISABLED_PROVIDER_ENV_KEYS: &[&str] = &[
    "GATEWAY_DISABLED_PROVIDER_ADAPTERS",
    "GATEWAY_DISABLED_PROVIDERS",
];

#[derive(Clone, Copy)]
struct ProviderAdapterPlugin {
    provider_kind: &'static str,
    build: fn() -> Arc<dyn ProviderAdapter>,
}

const BUILTIN_PROVIDER_PLUGINS: &[ProviderAdapterPlugin] = &[
    ProviderAdapterPlugin {
        provider_kind: "openai",
        build: build_openai_adapter,
    },
    ProviderAdapterPlugin {
        provider_kind: "anthropic",
        build: build_anthropic_adapter,
    },
    ProviderAdapterPlugin {
        provider_kind: "bedrock",
        build: build_bedrock_adapter,
    },
    ProviderAdapterPlugin {
        provider_kind: "gateway",
        build: build_gateway_adapter,
    },
    ProviderAdapterPlugin {
        provider_kind: "gemini",
        build: build_gemini_adapter,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderRegistryConfig {
    enabled_provider_kinds: Option<BTreeSet<String>>,
    disabled_provider_kinds: BTreeSet<String>,
}

impl ProviderRegistryConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            enabled_provider_kinds: read_first_csv_env(ENABLED_PROVIDER_ENV_KEYS),
            disabled_provider_kinds: read_all_csv_env(DISABLED_PROVIDER_ENV_KEYS),
        }
    }

    #[cfg(test)]
    pub(crate) fn all_enabled() -> Self {
        Self {
            enabled_provider_kinds: None,
            disabled_provider_kinds: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    fn only(provider_kinds: &[&str]) -> Self {
        Self {
            enabled_provider_kinds: Some(
                provider_kinds
                    .iter()
                    .map(|provider_kind| normalize_provider_kind(provider_kind))
                    .collect(),
            ),
            disabled_provider_kinds: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    fn with_disabled(provider_kinds: &[&str]) -> Self {
        Self {
            enabled_provider_kinds: None,
            disabled_provider_kinds: provider_kinds
                .iter()
                .map(|provider_kind| normalize_provider_kind(provider_kind))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderCompositionError {
    UnknownProviderKind {
        provider_kind: String,
        available_provider_kinds: Vec<String>,
    },
    NoProviderAdaptersEnabled,
    Registry(ProviderRegistryError),
}

impl Display for ProviderCompositionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownProviderKind {
                provider_kind,
                available_provider_kinds,
            } => write!(
                formatter,
                "unknown provider adapter `{provider_kind}`; available adapters: {}",
                available_provider_kinds.join(", ")
            ),
            Self::NoProviderAdaptersEnabled => {
                write!(
                    formatter,
                    "provider adapter composition enabled no adapters"
                )
            }
            Self::Registry(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ProviderCompositionError {}

impl From<ProviderRegistryError> for ProviderCompositionError {
    fn from(error: ProviderRegistryError) -> Self {
        Self::Registry(error)
    }
}

type ProviderCompositionResult<T> = Result<T, ProviderCompositionError>;

pub(crate) fn default_provider_registry() -> ProviderCompositionResult<ProviderAdapterRegistry> {
    provider_registry_from_config(&ProviderRegistryConfig::from_env())
}

pub(crate) fn provider_registry_from_config(
    config: &ProviderRegistryConfig,
) -> ProviderCompositionResult<ProviderAdapterRegistry> {
    validate_provider_config(config)?;

    let mut registry = ProviderAdapterRegistry::new();
    for plugin in BUILTIN_PROVIDER_PLUGINS {
        if provider_enabled(plugin.provider_kind, config) {
            registry.register((plugin.build)())?;
        }
    }

    if registry.is_empty() {
        return Err(ProviderCompositionError::NoProviderAdaptersEnabled);
    }

    Ok(registry)
}

fn validate_provider_config(config: &ProviderRegistryConfig) -> ProviderCompositionResult<()> {
    let available = available_provider_kinds();
    let requested = config
        .enabled_provider_kinds
        .iter()
        .flatten()
        .chain(config.disabled_provider_kinds.iter());

    for provider_kind in requested {
        if !available.contains(provider_kind.as_str()) {
            return Err(ProviderCompositionError::UnknownProviderKind {
                provider_kind: provider_kind.clone(),
                available_provider_kinds: available
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            });
        }
    }

    Ok(())
}

fn available_provider_kinds() -> BTreeSet<&'static str> {
    BUILTIN_PROVIDER_PLUGINS
        .iter()
        .map(|plugin| plugin.provider_kind)
        .collect()
}

fn provider_enabled(provider_kind: &str, config: &ProviderRegistryConfig) -> bool {
    let explicitly_enabled = config
        .enabled_provider_kinds
        .as_ref()
        .is_none_or(|enabled| enabled.contains(provider_kind));

    explicitly_enabled && !config.disabled_provider_kinds.contains(provider_kind)
}

fn read_first_csv_env(keys: &[&str]) -> Option<BTreeSet<String>> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .and_then(|value| parse_csv_set(&value))
    })
}

fn read_all_csv_env(keys: &[&str]) -> BTreeSet<String> {
    keys.iter()
        .filter_map(|key| std::env::var(key).ok())
        .filter_map(|value| parse_csv_set(&value))
        .flatten()
        .collect()
}

fn parse_csv_set(value: &str) -> Option<BTreeSet<String>> {
    let values = value
        .split(',')
        .map(normalize_provider_kind)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();

    (!values.is_empty()).then_some(values)
}

fn normalize_provider_kind(provider_kind: &str) -> String {
    provider_kind.trim().to_ascii_lowercase()
}

fn build_openai_adapter() -> Arc<dyn ProviderAdapter> {
    Arc::new(OpenAiAdapter::default())
}

fn build_anthropic_adapter() -> Arc<dyn ProviderAdapter> {
    Arc::new(AnthropicAdapter::default())
}

fn build_bedrock_adapter() -> Arc<dyn ProviderAdapter> {
    Arc::new(BedrockConverseAdapter::default())
}

fn build_gateway_adapter() -> Arc<dyn ProviderAdapter> {
    Arc::new(GatewayAdapter::default())
}

fn build_gemini_adapter() -> Arc<dyn ProviderAdapter> {
    Arc::new(GeminiAdapter::default())
}

#[cfg(test)]
mod tests {
    use super::{
        ProviderCompositionError, ProviderRegistryConfig, parse_csv_set,
        provider_registry_from_config,
    };

    #[test]
    fn default_provider_config_registers_all_builtin_adapters() {
        let registry = provider_registry_from_config(&ProviderRegistryConfig::all_enabled())
            .expect("default provider catalog should compose");

        assert_eq!(registry.len(), 5);
        assert!(registry.resolve("openai").is_some());
        assert!(registry.resolve("anthropic").is_some());
        assert!(registry.resolve("bedrock").is_some());
        assert!(registry.resolve("gateway").is_some());
        assert!(registry.resolve("gemini").is_some());
    }

    #[test]
    fn provider_config_can_select_a_subset() {
        let registry =
            provider_registry_from_config(&ProviderRegistryConfig::only(&["openai", "gateway"]))
                .expect("selected provider catalog should compose");

        assert_eq!(registry.provider_kinds(), vec!["gateway", "openai"]);
        assert!(registry.resolve("anthropic").is_none());
    }

    #[test]
    fn provider_config_can_disable_a_builtin_adapter() {
        let registry =
            provider_registry_from_config(&ProviderRegistryConfig::with_disabled(&["bedrock"]))
                .expect("provider catalog should compose with disabled adapters");

        assert_eq!(registry.len(), 4);
        assert!(registry.resolve("bedrock").is_none());
    }

    #[test]
    fn provider_config_rejects_unknown_provider_kind() {
        let Err(error) = provider_registry_from_config(&ProviderRegistryConfig::only(&["made-up"]))
        else {
            panic!("unknown provider should fail composition");
        };

        assert!(matches!(
            error,
            ProviderCompositionError::UnknownProviderKind { .. }
        ));
    }

    #[test]
    fn provider_config_rejects_empty_composition() {
        let Err(error) = provider_registry_from_config(&ProviderRegistryConfig::with_disabled(&[
            "openai",
            "anthropic",
            "bedrock",
            "gateway",
            "gemini",
        ])) else {
            panic!("empty provider composition should fail");
        };

        assert_eq!(error, ProviderCompositionError::NoProviderAdaptersEnabled);
    }

    #[test]
    fn provider_csv_parser_normalizes_values() {
        let values =
            parse_csv_set(" OpenAI, gateway,,GEMINI ").expect("non-empty CSV should produce a set");

        assert_eq!(
            values.into_iter().collect::<Vec<_>>(),
            vec!["gateway", "gemini", "openai"]
        );
    }
}
