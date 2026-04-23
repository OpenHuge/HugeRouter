use core_domain::UsageMetrics;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingDimension {
    InputTokens,
    OutputTokens,
    CachedInputTokens,
    ImageGenerations,
    AudioSeconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingSource {
    PlatformCatalog,
    ProviderNative,
    ContractOverride,
    TenantOverride,
    Promotional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingCatalogEntry {
    pub catalog_id: String,
    pub catalog_version: u32,
    pub dimension: PricingDimension,
    pub provider_id: String,
    pub model_alias: Option<String>,
    pub region: Option<String>,
    pub micros_per_unit: i64,
    pub unit_denominator: u64,
    pub source: PricingSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingCatalog {
    pub catalog_id: String,
    pub catalog_version: u32,
    pub currency: String,
    pub entries: Vec<PricingCatalogEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdditionalUsageDimensions {
    pub image_generation_units: u32,
    pub audio_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingLineItem {
    pub dimension: PricingDimension,
    pub units: u64,
    pub provider_cost_micros: i64,
    pub billable_cost_micros: i64,
    pub rate_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceQuote {
    pub catalog_id: String,
    pub catalog_version: u32,
    pub currency: String,
    pub provider_cost_micros: i64,
    pub billable_cost_micros: i64,
    pub line_items: Vec<PricingLineItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProviderRateCard {
    provider_id: &'static str,
    input_micros_per_1k: i64,
    output_micros_per_1k: i64,
    cached_input_micros_per_1k: i64,
    image_generation_micros_per_unit: i64,
    audio_seconds_micros_per_unit: i64,
    billable_markup_bps: i64,
    source: PricingSource,
}

const DEFAULT_CATALOG_ID: &str = "pricing_catalog_default";
const DEFAULT_CATALOG_VERSION: u32 = 1;
const DEFAULT_CURRENCY: &str = "USD";

const DEFAULT_RATE_CARD: ProviderRateCard = ProviderRateCard {
    provider_id: "default",
    input_micros_per_1k: 2_000,
    output_micros_per_1k: 6_000,
    cached_input_micros_per_1k: 500,
    image_generation_micros_per_unit: 15_000,
    audio_seconds_micros_per_unit: 1_200,
    billable_markup_bps: 2_500,
    source: PricingSource::PlatformCatalog,
};

const RATE_CARDS: &[ProviderRateCard] = &[
    ProviderRateCard {
        provider_id: "openai",
        input_micros_per_1k: 2_500,
        output_micros_per_1k: 8_500,
        cached_input_micros_per_1k: 750,
        image_generation_micros_per_unit: 18_000,
        audio_seconds_micros_per_unit: 1_500,
        billable_markup_bps: 2_000,
        source: PricingSource::ProviderNative,
    },
    ProviderRateCard {
        provider_id: "anthropic",
        input_micros_per_1k: 3_000,
        output_micros_per_1k: 9_000,
        cached_input_micros_per_1k: 900,
        image_generation_micros_per_unit: 21_000,
        audio_seconds_micros_per_unit: 1_800,
        billable_markup_bps: 2_200,
        source: PricingSource::ProviderNative,
    },
    ProviderRateCard {
        provider_id: "gemini",
        input_micros_per_1k: 1_800,
        output_micros_per_1k: 5_500,
        cached_input_micros_per_1k: 450,
        image_generation_micros_per_unit: 14_000,
        audio_seconds_micros_per_unit: 1_000,
        billable_markup_bps: 1_800,
        source: PricingSource::ProviderNative,
    },
];

fn lookup_rate_card(provider_id: &str) -> ProviderRateCard {
    RATE_CARDS
        .iter()
        .find(|rate_card| rate_card.provider_id == provider_id)
        .copied()
        .unwrap_or(DEFAULT_RATE_CARD)
}

fn micros_for_units(units: u32, micros_per_1k: i64) -> i64 {
    (i64::from(units) * micros_per_1k) / 1_000
}

const fn billable_from_provider_cost(provider_cost_micros: i64, billable_markup_bps: i64) -> i64 {
    provider_cost_micros + ((provider_cost_micros * billable_markup_bps) / 10_000)
}

fn micros_for_dimension(
    dimension: PricingDimension,
    units: u32,
    rate_card: ProviderRateCard,
) -> i64 {
    match dimension {
        PricingDimension::InputTokens => micros_for_units(units, rate_card.input_micros_per_1k),
        PricingDimension::OutputTokens => micros_for_units(units, rate_card.output_micros_per_1k),
        PricingDimension::CachedInputTokens => {
            micros_for_units(units, rate_card.cached_input_micros_per_1k)
        }
        PricingDimension::ImageGenerations => {
            i64::from(units) * rate_card.image_generation_micros_per_unit
        }
        PricingDimension::AudioSeconds => {
            i64::from(units) * rate_card.audio_seconds_micros_per_unit
        }
    }
}

#[must_use]
pub fn default_catalog() -> PricingCatalog {
    let mut entries = Vec::new();
    for rate_card in RATE_CARDS {
        for (dimension, micros_per_unit, denominator) in [
            (
                PricingDimension::InputTokens,
                rate_card.input_micros_per_1k,
                1_000_u64,
            ),
            (
                PricingDimension::OutputTokens,
                rate_card.output_micros_per_1k,
                1_000_u64,
            ),
            (
                PricingDimension::CachedInputTokens,
                rate_card.cached_input_micros_per_1k,
                1_000_u64,
            ),
            (
                PricingDimension::ImageGenerations,
                rate_card.image_generation_micros_per_unit,
                1_u64,
            ),
            (
                PricingDimension::AudioSeconds,
                rate_card.audio_seconds_micros_per_unit,
                1_u64,
            ),
        ] {
            entries.push(PricingCatalogEntry {
                catalog_id: DEFAULT_CATALOG_ID.to_string(),
                catalog_version: DEFAULT_CATALOG_VERSION,
                dimension,
                provider_id: rate_card.provider_id.to_string(),
                model_alias: None,
                region: Some("global".to_string()),
                micros_per_unit,
                unit_denominator: denominator,
                source: rate_card.source,
            });
        }
    }

    PricingCatalog {
        catalog_id: DEFAULT_CATALOG_ID.to_string(),
        catalog_version: DEFAULT_CATALOG_VERSION,
        currency: DEFAULT_CURRENCY.to_string(),
        entries,
    }
}

/// Returns a deterministic price quote for the current built-in catalog.
#[must_use]
pub fn quote_usage(provider_id: &str, usage: &UsageMetrics) -> PriceQuote {
    quote_usage_with_additions(provider_id, usage, AdditionalUsageDimensions::default())
}

#[must_use]
pub fn quote_usage_with_additions(
    provider_id: &str,
    usage: &UsageMetrics,
    additions: AdditionalUsageDimensions,
) -> PriceQuote {
    let rate_card = lookup_rate_card(provider_id);

    let mut line_items = Vec::with_capacity(5);
    let dimensions = [
        (PricingDimension::InputTokens, usage.input_tokens),
        (PricingDimension::OutputTokens, usage.output_tokens),
        (
            PricingDimension::CachedInputTokens,
            usage.cached_input_tokens,
        ),
        (
            PricingDimension::ImageGenerations,
            additions.image_generation_units,
        ),
        (PricingDimension::AudioSeconds, additions.audio_seconds),
    ];

    let mut provider_cost_micros = 0_i64;
    let mut billable_cost_micros = 0_i64;

    for (dimension, units) in dimensions {
        if units == 0 {
            continue;
        }
        let line_provider_cost = micros_for_dimension(dimension, units, rate_card);
        let line_billable_cost =
            billable_from_provider_cost(line_provider_cost, rate_card.billable_markup_bps);

        provider_cost_micros += line_provider_cost;
        billable_cost_micros += line_billable_cost;

        line_items.push(PricingLineItem {
            dimension,
            units: u64::from(units),
            provider_cost_micros: line_provider_cost,
            billable_cost_micros: line_billable_cost,
            rate_source: format!("{:?}", rate_card.source).to_ascii_lowercase(),
        });
    }

    PriceQuote {
        catalog_id: DEFAULT_CATALOG_ID.to_string(),
        catalog_version: DEFAULT_CATALOG_VERSION,
        currency: DEFAULT_CURRENCY.to_string(),
        provider_cost_micros,
        billable_cost_micros,
        line_items,
    }
}

#[must_use]
pub fn default_budget_micros(tenant_id: &str, project_id: &str) -> i64 {
    match (tenant_id, project_id) {
        ("tenant_acme", "proj_core") => 75_000_000,
        ("tenant_acme", _) => 40_000_000,
        ("tenant_northstar", _) => 25_000_000,
        _ => 50_000_000,
    }
}

#[must_use]
pub const fn threshold_status(
    billable_cost_micros: i64,
    configured_budget_micros: i64,
) -> &'static str {
    if configured_budget_micros <= 0 {
        return "unbounded";
    }

    if billable_cost_micros >= configured_budget_micros {
        return "exceeded";
    }

    if billable_cost_micros * 100 >= configured_budget_micros * 80 {
        return "warning";
    }

    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_provider_cost_and_billable_cost_for_known_provider() {
        let quote = quote_usage(
            "openai",
            &UsageMetrics {
                input_tokens: 1_000,
                output_tokens: 500,
                cached_input_tokens: 250,
            },
        );

        assert_eq!(quote.provider_cost_micros, 6_937);
        assert_eq!(quote.billable_cost_micros, 8_324);
        assert_eq!(quote.line_items.len(), 3);
        assert_eq!(quote.catalog_version, DEFAULT_CATALOG_VERSION);
    }

    #[test]
    fn exposes_catalog_entries_for_modal_dimensions() {
        let catalog = default_catalog();
        assert!(
            catalog
                .entries
                .iter()
                .any(|entry| entry.dimension == PricingDimension::ImageGenerations)
        );
        assert!(
            catalog
                .entries
                .iter()
                .any(|entry| entry.dimension == PricingDimension::AudioSeconds)
        );
    }

    #[test]
    fn falls_back_to_default_catalog_for_unknown_provider() {
        let quote = quote_usage(
            "custom-provider",
            &UsageMetrics {
                input_tokens: 500,
                output_tokens: 500,
                cached_input_tokens: 0,
            },
        );

        assert_eq!(quote.provider_cost_micros, 4_000);
        assert_eq!(quote.billable_cost_micros, 5_000);
    }

    #[test]
    fn quotes_image_and_audio_placeholders_when_requested() {
        let quote = quote_usage_with_additions(
            "openai",
            &UsageMetrics {
                input_tokens: 0,
                output_tokens: 0,
                cached_input_tokens: 0,
            },
            AdditionalUsageDimensions {
                image_generation_units: 2,
                audio_seconds: 5,
            },
        );

        assert!(quote.provider_cost_micros > 0);
        assert_eq!(quote.line_items.len(), 2);
    }

    #[test]
    fn threshold_status_tracks_budget_crossing() {
        assert_eq!(threshold_status(10, 100), "ok");
        assert_eq!(threshold_status(85, 100), "warning");
        assert_eq!(threshold_status(100, 100), "exceeded");
    }
}
