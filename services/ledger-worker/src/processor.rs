#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::missing_const_for_fn,
    clippy::needless_raw_string_hashes,
    clippy::trivially_copy_pass_by_ref,
    clippy::useless_conversion
)]

mod entry;
mod ingestion;
mod projections;
mod schema;

pub use entry::BudgetThresholdEventRecord;
pub use ingestion::{IngestionOutcome, UsagePersistenceOutcome, handle_usage_event_recorded};
pub use projections::ensure_projection_tables;
pub use schema::ensure_ledger_table;

#[cfg(test)]
mod tests {
    use super::entry::{
        build_pending_entry_with_provider_id, ledger_entry_id, parse_estimated_cost_micros,
    };
    use super::*;
    use core_domain::UsagePhase;
    use protocol_ir::UsageEventRecordedMessage;

    fn usage_event_recorded_message(owner_account_id: Option<&str>) -> UsageEventRecordedMessage {
        let usage_event = core_domain::UsageEvent {
            usage_event_id: core_domain::UsageEventId::parse("usageevt_owner_scope").unwrap(),
            route_receipt_id: core_domain::RouteReceiptId::parse("routercpt_owner_scope").unwrap(),
            grant_id: Some("grant_owner_scope".to_string()),
            owner_account_id: owner_account_id.map(str::to_string),
            tenant_id: core_domain::TenantId::parse("tenant_acme").unwrap(),
            project_id: core_domain::ProjectId::parse("proj_core").unwrap(),
            provider_resource_id: core_domain::ProviderResourceId::parse("prvrsrc_openai_primary")
                .unwrap(),
            model_alias: "reasoning-fast".to_string(),
            phase: UsagePhase::Final,
            idempotency_key: "routercpt_owner_scope:final".to_string(),
            usage: core_domain::UsageMetrics {
                input_tokens: 120,
                output_tokens: 80,
                cached_input_tokens: 0,
            },
            estimated_cost: core_domain::MonetaryAmount {
                currency: "USD".to_string(),
                amount: "0.000600".to_string(),
            },
            recorded_at: "2026-04-22T00:00:00Z".to_string(),
        };
        UsageEventRecordedMessage {
            message_id: "msg_usageevt_owner_scope".to_string(),
            message_type: protocol_ir::MessageType::UsageEventRecorded,
            schema_version: 1,
            occurred_at: "2026-04-22T00:00:00Z".to_string(),
            producer: core_domain::ServiceName::parse("gateway-api").unwrap(),
            trace_id: Some("trace_owner_scope".to_string()),
            request_id: Some("req_owner_scope".to_string()),
            idempotency_key: usage_event.idempotency_key.clone(),
            payload: protocol_ir::UsageEventRecorded { usage_event },
        }
    }

    #[test]
    fn parse_amount_micros_supports_fractional_digits_and_padding() {
        assert_eq!(
            parse_estimated_cost_micros("0.1420").expect("parse"),
            142_000
        );
        assert_eq!(parse_estimated_cost_micros("2").expect("parse"), 2_000_000);
        assert_eq!(
            parse_estimated_cost_micros("1.5").expect("parse"),
            1_500_000
        );
        assert_eq!(
            parse_estimated_cost_micros("1.000001").expect("parse"),
            1_000_001
        );
        assert_eq!(
            parse_estimated_cost_micros("1.0000019").expect("parse"),
            1_000_001
        );
    }

    #[test]
    fn parse_amount_micros_rejects_invalid_values() {
        assert!(parse_estimated_cost_micros("abc").is_err());
        assert!(parse_estimated_cost_micros("1.2.3").is_err());
        assert!(parse_estimated_cost_micros("").is_err());
        assert!(parse_estimated_cost_micros("-1").is_err());
    }

    #[test]
    fn generated_ledger_entry_id_is_deterministic() {
        let first = ledger_entry_id("usageevt_123:final");
        let second = ledger_entry_id("usageevt_123:final");
        assert_eq!(first, second);
        assert!(first.starts_with("ledger_"));
    }

    #[test]
    fn ledger_entry_id_changes_with_idempotency_key() {
        let first = ledger_entry_id("routercpt_instance_a_42:final");
        let repeated = ledger_entry_id("routercpt_instance_a_42:final");
        let second = ledger_entry_id("routercpt_instance_b_42:final");

        assert_eq!(first, repeated);
        assert_ne!(first, second);
        assert!(first.starts_with("ledger_"));
        assert!(second.starts_with("ledger_"));
    }

    #[test]
    fn build_pending_entry_carries_owner_account_id() {
        let event = usage_event_recorded_message(Some("acct_acme_owner"));
        let entry = build_pending_entry_with_provider_id(&event, "openai".to_string())
            .expect("pending entry should build");

        assert_eq!(entry.grant_id.as_deref(), Some("grant_owner_scope"));
        assert_eq!(entry.owner_account_id.as_deref(), Some("acct_acme_owner"));
        assert_eq!(
            projections::projection_owner_account_id(&entry),
            "acct_acme_owner"
        );
    }

    #[test]
    fn legacy_usage_event_uses_default_owner_projection_key() {
        let event = usage_event_recorded_message(None);
        let entry = build_pending_entry_with_provider_id(&event, "openai".to_string())
            .expect("pending entry should build");

        assert_eq!(entry.owner_account_id, None);
        assert_eq!(
            projections::projection_owner_account_id(&entry),
            core_domain::DEFAULT_OWNER_ACCOUNT_ID
        );
    }

    #[test]
    fn budget_threshold_events_are_owner_scoped() {
        let first = build_pending_entry_with_provider_id(
            &usage_event_recorded_message(Some("acct_owner_a")),
            "openai".to_string(),
        )
        .expect("pending entry should build");
        let second = build_pending_entry_with_provider_id(
            &usage_event_recorded_message(Some("acct_owner_b")),
            "openai".to_string(),
        )
        .expect("pending entry should build");

        assert_ne!(
            projections::budget_threshold_event_id(&first, "warning"),
            projections::budget_threshold_event_id(&second, "warning")
        );
    }

    #[test]
    fn threshold_status_tracks_budget_crossings() {
        assert_eq!(metering::threshold_status(10_000_000, 75_000_000), "ok");
        assert_eq!(
            metering::threshold_status(60_000_000, 75_000_000),
            "warning"
        );
        assert_eq!(
            metering::threshold_status(80_000_000, 75_000_000),
            "exceeded"
        );
    }
}
