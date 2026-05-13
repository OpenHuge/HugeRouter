pub const DEFAULT_REDEMPTION_UNIT_COUNT: usize = 8;
pub const UNBOUNDED_ACTIVE_REDEMPTION_UNITS_PER_OWNER: usize = usize::MAX;
pub const REDEMPTION_CODE_KIND: &str = "red";
pub const BROWSER_FILE_UNLOCK_CODE_KIND: &str = "brw";
pub const DELIVERY_CODE_TYPE_REDEMPTION: &str = "redemption_code";
pub const DELIVERY_CODE_TYPE_BROWSER_FILE_UNLOCK: &str = "browser_file_unlock_code";
pub const DELIVERY_CODE_FORMAT_REDEMPTION_V2: &str = "ku0-red-v2";
pub const DELIVERY_CODE_FORMAT_BROWSER_FILE_UNLOCK_V2: &str = "ku0-brw-v2";
pub const DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_PROTOCOL_V2: &str = "delivery_account_bundle_v2";
pub const DELIVERY_ACCOUNT_BUNDLE_ENCRYPTION_VERSION_V2: &str = "2";

pub const fn inventory_status(active_count: usize) -> &'static str {
    if active_count == 0 { "empty" } else { "issued" }
}
