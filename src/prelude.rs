//! Convenience re-exports for implementing this library.
//!
//! Environment variables:
//! - `STRIPE_API_KEY` (required): Secret key used to call the Stripe API.
//! - `STRIPE_WEBHOOK_SECRET` (optional): If set, enables the `/stripe/webhook` route and is used
//!   to verify incoming webhook signatures.

#[cfg(feature = "api")]
pub use crate::api::{HasPool, StripeApp, routes};

#[cfg(feature = "api")]
pub use crate::stripe_events::HandlesStripeEvents;

#[cfg(feature = "sqlx")]
pub use crate::db::create_stripe_tables;

#[cfg(feature = "sqlx")]
pub use crate::usage_pricing::{
    UsageAccessState, UsageCreditPackConfig, UsageCreditZoneConfig, UsageInterval,
    UsageMeteredTierConfig, UsagePricingCatalog, UsagePricingConfig, UsageReportOutcome,
    UsageReportRequest, UsageTierSummary, UsageTierType, list_usage_tiers, report_usage,
    upsert_usage_pricing, usage_access_state,
};

pub use crate::error::{ErrorKind, LibError, Result};
