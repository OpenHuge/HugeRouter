use anyhow::{Result, anyhow};
use core_domain::MonetaryAmount;
use metering::{
    AdditionalUsageDimensions, PricingCatalog, PricingSource, default_catalog,
    quote_usage_for_catalog, quote_usage_with_additions,
};
use protocol_ir::{
    PricingCatalogEntry, PricingCatalogResponse, PricingSimulationLineItem,
    PricingSimulationRequest, PricingSimulationResponse,
};
use sqlx::{Pool, Postgres, Row};

pub fn default_pricing_catalog_response() -> PricingCatalogResponse {
    pricing_catalog_response(default_catalog())
}

pub fn pricing_catalog_response(catalog: PricingCatalog) -> PricingCatalogResponse {
    PricingCatalogResponse {
        catalog_id: catalog.catalog_id,
        catalog_version: catalog.catalog_version,
        currency: catalog.currency,
        entries: catalog
            .entries
            .into_iter()
            .map(|entry| PricingCatalogEntry {
                dimension: pricing_dimension_slug(entry.dimension),
                provider_id: entry.provider_id,
                model_alias: entry.model_alias,
                region: entry.region,
                micros_per_unit: entry.micros_per_unit,
                unit_denominator: entry.unit_denominator,
                source: pricing_source_slug(entry.source),
            })
            .collect(),
    }
}

pub fn simulate_pricing(request: &PricingSimulationRequest) -> PricingSimulationResponse {
    let quote = quote_usage_with_additions(
        &request.provider_id,
        &request.usage,
        AdditionalUsageDimensions {
            image_generation_units: request.image_generation_units.unwrap_or_default(),
            audio_seconds: request.audio_seconds.unwrap_or_default(),
        },
    );
    pricing_simulation_response_from_quote(quote)
}

pub fn simulate_pricing_with_catalog(
    catalog: &PricingCatalog,
    request: &PricingSimulationRequest,
) -> PricingSimulationResponse {
    let quote = quote_usage_for_catalog(
        catalog,
        &request.provider_id,
        request.model_alias.as_str().into(),
        request.region.as_deref(),
        &request.usage,
        AdditionalUsageDimensions {
            image_generation_units: request.image_generation_units.unwrap_or_default(),
            audio_seconds: request.audio_seconds.unwrap_or_default(),
        },
    );
    pricing_simulation_response_from_quote(quote)
}

pub async fn load_pricing_catalog(pool: &Pool<Postgres>) -> Result<PricingCatalog> {
    let rows = sqlx::query(
        r"
        SELECT
            catalog_id,
            catalog_version,
            currency,
            dimension,
            provider_id,
            model_alias,
            region,
            micros_per_unit,
            billable_micros_per_unit,
            unit_denominator,
            source
        FROM pricing_catalog_entries
        ORDER BY provider_id, model_alias NULLS FIRST, region NULLS FIRST, dimension
        ",
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(default_catalog());
    }

    let catalog_id = rows[0].get::<String, _>("catalog_id");
    let catalog_version = u32::try_from(rows[0].get::<i32, _>("catalog_version"))?;
    let currency = rows[0].get::<String, _>("currency");
    let mut entries = Vec::with_capacity(rows.len());

    for row in rows {
        entries.push(metering::PricingCatalogEntry {
            catalog_id: row.get("catalog_id"),
            catalog_version: u32::try_from(row.get::<i32, _>("catalog_version"))?,
            dimension: parse_pricing_dimension(row.get::<String, _>("dimension").as_str())?,
            provider_id: row.get("provider_id"),
            model_alias: row.get("model_alias"),
            region: row.get("region"),
            micros_per_unit: row.get("micros_per_unit"),
            billable_micros_per_unit: row.get("billable_micros_per_unit"),
            unit_denominator: u64::try_from(row.get::<i64, _>("unit_denominator"))?,
            source: parse_pricing_source(row.get::<String, _>("source").as_str())?,
        });
    }

    Ok(PricingCatalog {
        catalog_id,
        catalog_version,
        currency,
        entries,
    })
}

pub async fn seed_default_pricing_catalog(pool: &Pool<Postgres>) -> Result<()> {
    seed_pricing_catalog(pool, &default_catalog()).await
}

async fn seed_pricing_catalog(pool: &Pool<Postgres>, catalog: &PricingCatalog) -> Result<()> {
    for entry in &catalog.entries {
        sqlx::query(
            r"
            INSERT INTO pricing_catalog_entries (
                catalog_id,
                catalog_version,
                currency,
                dimension,
                provider_id,
                model_alias,
                region,
                micros_per_unit,
                billable_micros_per_unit,
                unit_denominator,
                source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (catalog_id, catalog_version, dimension, provider_id, model_alias_key, region_key)
            DO UPDATE SET
                currency = EXCLUDED.currency,
                micros_per_unit = EXCLUDED.micros_per_unit,
                billable_micros_per_unit = EXCLUDED.billable_micros_per_unit,
                unit_denominator = EXCLUDED.unit_denominator,
                source = EXCLUDED.source,
                updated_at = NOW()
            ",
        )
        .bind(&entry.catalog_id)
        .bind(i32::try_from(entry.catalog_version)?)
        .bind(&catalog.currency)
        .bind(pricing_dimension_slug(entry.dimension))
        .bind(&entry.provider_id)
        .bind(entry.model_alias.as_deref())
        .bind(entry.region.as_deref())
        .bind(entry.micros_per_unit)
        .bind(entry.billable_micros_per_unit)
        .bind(i64::try_from(entry.unit_denominator)?)
        .bind(pricing_source_slug(entry.source))
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn pricing_simulation_response_from_quote(
    quote: metering::PriceQuote,
) -> PricingSimulationResponse {
    PricingSimulationResponse {
        catalog_id: quote.catalog_id,
        catalog_version: quote.catalog_version,
        currency: quote.currency.clone(),
        provider_cost: format_monetary_amount(&quote.currency, quote.provider_cost_micros),
        billable_price: format_monetary_amount(&quote.currency, quote.billable_cost_micros),
        line_items: quote
            .line_items
            .into_iter()
            .map(|line_item| PricingSimulationLineItem {
                dimension: pricing_dimension_slug(line_item.dimension),
                units: line_item.units,
                provider_cost: format_monetary_amount(
                    &quote.currency,
                    line_item.provider_cost_micros,
                ),
                billable_price: format_monetary_amount(
                    &quote.currency,
                    line_item.billable_cost_micros,
                ),
                rate_source: line_item.rate_source,
            })
            .collect(),
    }
}

fn pricing_dimension_slug(dimension: metering::PricingDimension) -> String {
    match dimension {
        metering::PricingDimension::InputTokens => "input_tokens".to_string(),
        metering::PricingDimension::OutputTokens => "output_tokens".to_string(),
        metering::PricingDimension::CachedInputTokens => "cached_input_tokens".to_string(),
        metering::PricingDimension::ImageGenerations => "image_generations".to_string(),
        metering::PricingDimension::AudioSeconds => "audio_seconds".to_string(),
    }
}

fn pricing_source_slug(source: PricingSource) -> String {
    match source {
        PricingSource::PlatformCatalog => "platform_catalog".to_string(),
        PricingSource::ProviderNative => "provider_native".to_string(),
        PricingSource::ContractOverride => "contract_override".to_string(),
        PricingSource::TenantOverride => "tenant_override".to_string(),
        PricingSource::Promotional => "promotional".to_string(),
    }
}

fn parse_pricing_dimension(value: &str) -> Result<metering::PricingDimension> {
    match value {
        "input_tokens" => Ok(metering::PricingDimension::InputTokens),
        "output_tokens" => Ok(metering::PricingDimension::OutputTokens),
        "cached_input_tokens" => Ok(metering::PricingDimension::CachedInputTokens),
        "image_generations" => Ok(metering::PricingDimension::ImageGenerations),
        "audio_seconds" => Ok(metering::PricingDimension::AudioSeconds),
        _ => Err(anyhow!("unknown pricing dimension `{value}`")),
    }
}

fn parse_pricing_source(value: &str) -> Result<PricingSource> {
    match value {
        "platform_catalog" => Ok(PricingSource::PlatformCatalog),
        "provider_native" => Ok(PricingSource::ProviderNative),
        "contract_override" => Ok(PricingSource::ContractOverride),
        "tenant_override" => Ok(PricingSource::TenantOverride),
        "promotional" => Ok(PricingSource::Promotional),
        _ => Err(anyhow!("unknown pricing source `{value}`")),
    }
}

fn format_monetary_amount(currency: &str, micros: i64) -> MonetaryAmount {
    let sign = if micros < 0 { "-" } else { "" };
    let absolute = micros.abs();
    let whole = absolute / 1_000_000;
    let fractional = absolute % 1_000_000;
    MonetaryAmount {
        currency: currency.to_string(),
        amount: format!("{sign}{whole}.{fractional:06}"),
    }
}
