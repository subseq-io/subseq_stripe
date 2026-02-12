# Usage Pricing App Configuration

This document defines what an app must provide to configure usage-based pricing in `subseq_stripe` for:
- untrusted upfront credits
- capped metered upgrade gates
- an uncapped metered terminal tier

## Required Runtime Prerequisites

- Postgres migrations applied (including `20260212180000_usage_pricing.sql`).
- `STRIPE_API_KEY` set in the service environment.
- App has authenticated `internal_id` values for users/orgs.
- For metered reporting, the user must have a Stripe customer link in `stripe.customers` (created during checkout finalization).

## Library API Surface

App code should use:
- `upsert_usage_pricing(pool, config)` to create/update Stripe products/prices/meters and local tier mappings.
- `usage_access_state(pool, internal_id)` to inspect active access mode (credits vs metered).
- `report_usage(pool, internal_id, request)` to submit billable usage with idempotency.

Exports are available from:
- `subseq_stripe::prelude::*`

## Configuration Contract

Main type:

```rust
UsagePricingConfig {
  untrusted_zone: UsageCreditZoneConfig,
  metered_gates: Vec<UsageMeteredTierConfig>,
  uncapped_tier: UsageMeteredTierConfig,
}
```

### `UsageCreditZoneConfig`

- `packs`: one or more upfront credit products.
- `default_pack_key`: optional key of the fallback/default tier.

Each `UsageCreditPackConfig`:
- `key`: stable internal plan key (used for checkout + mapping in DB).
- `name`, `description`
- `currency`
- `unit_amount_cents`
- `credits_grant` (must be `> 0`)
- optional Stripe pinning fields:
  - `stripe_product_id`
  - `stripe_price_lookup_key`

### `UsageMeteredTierConfig`

For both capped gates and uncapped tier:
- `key`: stable internal plan key
- `name`, `description`
- `currency`
- `unit_amount_cents`
- `interval`: `Day | Week | Month | Year`
- `meter_display_name`
- `meter_event_name` (required, non-empty)
- optional Stripe pinning fields:
  - `stripe_product_id`
  - `stripe_price_lookup_key`
  - `stripe_meter_id`

Additional rule for gated tiers:
- `usage_cap` is required and must be `> 0`.

Additional rule for terminal uncapped tier:
- `usage_cap` must be `None`.

## Validation Rules Enforced by `upsert_usage_pricing`

- `untrusted_zone.packs` cannot be empty.
- `metered_gates` cannot be empty.
- tier `key` values must be globally unique across packs + gates + uncapped tier.
- `default_pack_key` must match one of the pack keys if set.
- uncapped tier cannot set `usage_cap`.
- all metered tiers require non-empty `meter_event_name`.

## Example Static Configuration

```rust
use subseq_stripe::prelude::*;

fn usage_pricing_config() -> UsagePricingConfig {
    UsagePricingConfig {
        untrusted_zone: UsageCreditZoneConfig {
            packs: vec![
                UsageCreditPackConfig {
                    key: "credits-starter".into(),
                    name: "Starter Credits".into(),
                    description: "Entry tier with prepaid credits".into(),
                    currency: stripe::Currency::USD,
                    unit_amount_cents: 1000,
                    credits_grant: 1_000,
                    stripe_product_id: None,
                    stripe_price_lookup_key: Some("app:credits:starter".into()),
                },
                UsageCreditPackConfig {
                    key: "credits-growth".into(),
                    name: "Growth Credits".into(),
                    description: "Larger prepaid credit bundle".into(),
                    currency: stripe::Currency::USD,
                    unit_amount_cents: 4000,
                    credits_grant: 5_000,
                    stripe_product_id: None,
                    stripe_price_lookup_key: Some("app:credits:growth".into()),
                },
            ],
            default_pack_key: Some("credits-starter".into()),
        },
        metered_gates: vec![
            UsageMeteredTierConfig {
                key: "metered-cap-50k".into(),
                name: "Metered 50k".into(),
                description: "Capped metered plan (50k units/month)".into(),
                currency: stripe::Currency::USD,
                unit_amount_cents: 1,
                interval: UsageInterval::Month,
                usage_cap: Some(50_000),
                meter_display_name: "API Units".into(),
                meter_event_name: "api_units".into(),
                stripe_product_id: None,
                stripe_price_lookup_key: Some("app:metered:50k".into()),
                stripe_meter_id: None,
            },
            UsageMeteredTierConfig {
                key: "metered-cap-250k".into(),
                name: "Metered 250k".into(),
                description: "Capped metered plan (250k units/month)".into(),
                currency: stripe::Currency::USD,
                unit_amount_cents: 1,
                interval: UsageInterval::Month,
                usage_cap: Some(250_000),
                meter_display_name: "API Units".into(),
                meter_event_name: "api_units".into(),
                stripe_product_id: None,
                stripe_price_lookup_key: Some("app:metered:250k".into()),
                stripe_meter_id: None,
            },
        ],
        uncapped_tier: UsageMeteredTierConfig {
            key: "metered-uncapped".into(),
            name: "Metered Uncapped".into(),
            description: "Uncapped metered plan".into(),
            currency: stripe::Currency::USD,
            unit_amount_cents: 1,
            interval: UsageInterval::Month,
            usage_cap: None,
            meter_display_name: "API Units".into(),
            meter_event_name: "api_units".into(),
            stripe_product_id: None,
            stripe_price_lookup_key: Some("app:metered:uncapped".into()),
            stripe_meter_id: None,
        },
    }
}
```

## Startup Flow (App Responsibility)

1. Build a static `UsagePricingConfig` in app code.
2. Run:
   - `create_stripe_tables(pool)` (if not already run in migrations bootstrap)
   - `upsert_usage_pricing(pool, usage_pricing_config())`
3. Optionally inspect `UsagePricingCatalog` response and assert expected keys.

This keeps Stripe product/price/meter state aligned with app config without per-app manual Stripe admin steps.

## Checkout and Tier Selection

- Tier keys are also checkout keys via `stripe.prices` mapping.
- App chooses which key to send to `/stripe/checkout`.
- After successful subscription checkout, `subseq_stripe` persists active `price_id` and billing period bounds on `stripe.subscriptions`.

## Usage Reporting Contract

Call on every billable event:

```rust
let outcome = report_usage(
    pool.clone(),
    internal_id,
    UsageReportRequest {
        quantity: 42,
        idempotency_key: Some("event-uuid-from-your-system".into()),
        occurred_at: None,
    },
).await?;
```

Behavior:
- Credits tier:
  - Debits `stripe.credits` atomically.
  - Returns `remaining_credits`.
- Metered tier:
  - Enforces capped tiers per active subscription period.
  - Posts Stripe meter event using configured `meter_event_name`.
  - Returns `stripe_identifier`.
- Idempotency:
  - Same `idempotency_key` + same payload is replay-safe.
  - Reusing key with different quantity/internal_id returns an error.

## Error Semantics to Handle in App

- `Forbidden`:
  - insufficient credits
  - capped tier exceeded
  - idempotency key conflict across identities
- `NotFound`:
  - no Stripe customer mapping for metered reporting
  - no default usage tier configured
- `InvalidInput`:
  - malformed configuration
  - invalid usage quantity

## Operational Recommendations

- Treat tier `key` as immutable once in production.
- Keep `meter_event_name` stable per metric stream.
- Generate deterministic idempotency keys from your usage event IDs.
- Call `report_usage` only from trusted backend paths, never directly from clients.
