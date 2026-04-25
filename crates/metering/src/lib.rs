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
    pub billable_micros_per_unit: i64,
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
        provider_id: "bedrock",
        input_micros_per_1k: 6_000,
        output_micros_per_1k: 30_000,
        cached_input_micros_per_1k: 600,
        image_generation_micros_per_unit: 20_000,
        audio_seconds_micros_per_unit: 1_600,
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

fn micros_for_catalog_rate(units: u32, micros_per_unit: i64, unit_denominator: u64) -> i64 {
    if unit_denominator == 0 {
        return 0;
    }

    let micros = (i128::from(units) * i128::from(micros_per_unit)) / i128::from(unit_denominator);
    i64::try_from(micros).unwrap_or_else(|_| {
        if micros.is_negative() {
            i64::MIN
        } else {
            i64::MAX
        }
    })
}

#[must_use]
pub fn default_catalog() -> PricingCatalog {
    let mut entries = Vec::new();
    for rate_card in RATE_CARDS.iter().chain(std::iter::once(&DEFAULT_RATE_CARD)) {
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
                billable_micros_per_unit: billable_from_provider_cost(
                    micros_per_unit,
                    rate_card.billable_markup_bps,
                ),
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
pub fn quote_usage_for_catalog(
    catalog: &PricingCatalog,
    provider_id: &str,
    model_alias: Option<&str>,
    region: Option<&str>,
    usage: &UsageMetrics,
    additions: AdditionalUsageDimensions,
) -> PriceQuote {
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

        let Some(rate) = select_catalog_rate(catalog, provider_id, model_alias, region, dimension)
        else {
            continue;
        };
        let line_provider_cost =
            micros_for_catalog_rate(units, rate.micros_per_unit, rate.unit_denominator);
        let line_billable_cost =
            micros_for_catalog_rate(units, rate.billable_micros_per_unit, rate.unit_denominator);

        provider_cost_micros += line_provider_cost;
        billable_cost_micros += line_billable_cost;

        line_items.push(PricingLineItem {
            dimension,
            units: u64::from(units),
            provider_cost_micros: line_provider_cost,
            billable_cost_micros: line_billable_cost,
            rate_source: format!("{:?}", rate.source).to_ascii_lowercase(),
        });
    }

    PriceQuote {
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        currency: catalog.currency.clone(),
        provider_cost_micros,
        billable_cost_micros,
        line_items,
    }
}

fn select_catalog_rate<'a>(
    catalog: &'a PricingCatalog,
    provider_id: &str,
    model_alias: Option<&str>,
    region: Option<&str>,
    dimension: PricingDimension,
) -> Option<&'a PricingCatalogEntry> {
    catalog
        .entries
        .iter()
        .filter(|entry| entry.provider_id == provider_id && entry.dimension == dimension)
        .filter(|entry| match (model_alias, entry.model_alias.as_deref()) {
            (Some(requested), Some(entry_model)) => requested == entry_model,
            (_, None) => true,
            (None, Some(_)) => false,
        })
        .filter(|entry| match (region, entry.region.as_deref()) {
            (Some(requested), Some(entry_region)) => {
                requested == entry_region || entry_region == "global"
            }
            (_, Some("global") | None) => true,
            (None, Some(_)) => false,
        })
        .max_by_key(|entry| {
            let model_score = match (model_alias, entry.model_alias.as_deref()) {
                (Some(requested), Some(entry_model)) if requested == entry_model => 2,
                (_, None) => 1,
                _ => 0,
            };
            let region_score = match (region, entry.region.as_deref()) {
                (Some(requested), Some(entry_region)) if requested == entry_region => 2,
                (_, Some("global") | None) => 1,
                _ => 0,
            };

            model_score + region_score
        })
        .or_else(|| {
            catalog
                .entries
                .iter()
                .find(|entry| entry.provider_id == "default" && entry.dimension == dimension)
        })
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
    fn quotes_provider_cost_and_billable_cost_for_bedrock() {
        let quote = quote_usage(
            "bedrock",
            &UsageMetrics {
                input_tokens: 1_000,
                output_tokens: 500,
                cached_input_tokens: 250,
            },
        );

        assert_eq!(quote.provider_cost_micros, 21_150);
        assert_eq!(quote.billable_cost_micros, 25_803);
        assert_eq!(quote.line_items.len(), 3);
        assert!(
            quote
                .line_items
                .iter()
                .all(|line_item| line_item.rate_source == "providernative")
        );
    }

    #[test]
    fn exposes_catalog_entries_for_bedrock() {
        let catalog = default_catalog();
        let bedrock_entries = catalog
            .entries
            .iter()
            .filter(|entry| entry.provider_id == "bedrock")
            .collect::<Vec<_>>();

        assert_eq!(bedrock_entries.len(), 5);
        assert!(
            bedrock_entries
                .iter()
                .any(|entry| entry.dimension == PricingDimension::InputTokens)
        );
        assert!(
            bedrock_entries
                .iter()
                .any(|entry| entry.dimension == PricingDimension::OutputTokens)
        );
        assert!(
            bedrock_entries
                .iter()
                .all(|entry| entry.source == PricingSource::ProviderNative)
        );
        assert!(
            bedrock_entries
                .iter()
                .all(|entry| entry.region.as_deref() == Some("global"))
        );
    }

    #[test]
    fn quote_usage_for_catalog_uses_model_and_region_specific_rates() {
        let catalog = PricingCatalog {
            catalog_id: "pricing_catalog_test".to_string(),
            catalog_version: 7,
            currency: "USD".to_string(),
            entries: vec![
                PricingCatalogEntry {
                    catalog_id: "pricing_catalog_test".to_string(),
                    catalog_version: 7,
                    dimension: PricingDimension::InputTokens,
                    provider_id: "openai".to_string(),
                    model_alias: None,
                    region: Some("global".to_string()),
                    micros_per_unit: 1_000,
                    billable_micros_per_unit: 1_100,
                    unit_denominator: 1_000,
                    source: PricingSource::PlatformCatalog,
                },
                PricingCatalogEntry {
                    catalog_id: "pricing_catalog_test".to_string(),
                    catalog_version: 7,
                    dimension: PricingDimension::InputTokens,
                    provider_id: "openai".to_string(),
                    model_alias: Some("reasoning-fast".to_string()),
                    region: Some("us-east-1".to_string()),
                    micros_per_unit: 2_000,
                    billable_micros_per_unit: 3_000,
                    unit_denominator: 1_000,
                    source: PricingSource::ContractOverride,
                },
            ],
        };
        let quote = quote_usage_for_catalog(
            &catalog,
            "openai",
            Some("reasoning-fast"),
            Some("us-east-1"),
            &UsageMetrics {
                input_tokens: 1_500,
                output_tokens: 0,
                cached_input_tokens: 0,
            },
            AdditionalUsageDimensions::default(),
        );

        assert_eq!(quote.catalog_id, "pricing_catalog_test");
        assert_eq!(quote.catalog_version, 7);
        assert_eq!(quote.provider_cost_micros, 3_000);
        assert_eq!(quote.billable_cost_micros, 4_500);
        assert_eq!(quote.line_items[0].rate_source, "contractoverride");
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
