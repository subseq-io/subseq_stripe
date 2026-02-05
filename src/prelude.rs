//! Convenience re-exports for implementing this library.
//!
//! Environment variables:
//! - `STRIPE_API_KEY` (required): Secret key used to call the Stripe API.
//! - `STRIPE_WEBHOOK_SECRET` (optional): If set, enables the `/stripe/webhook` route and is used
//!   to verify incoming webhook signatures.

#[cfg(feature = "api")]
pub use crate::api::{routes, HasPool, StripeApp};

#[cfg(feature = "api")]
pub use crate::stripe_events::HandlesStripeEvents;

#[cfg(feature = "sqlx")]
pub use crate::db::create_stripe_tables;

pub use crate::error::{ErrorKind, LibError, Result};
