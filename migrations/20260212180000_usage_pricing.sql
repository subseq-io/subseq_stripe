ALTER TABLE stripe.subscriptions
    ADD COLUMN IF NOT EXISTS price_id TEXT,
    ADD COLUMN IF NOT EXISTS current_period_start TIMESTAMP;

CREATE TABLE IF NOT EXISTS stripe.usage_tiers (
    key TEXT PRIMARY KEY,
    product_id TEXT NOT NULL,
    price_id TEXT NOT NULL UNIQUE,
    price_lookup_key TEXT NOT NULL UNIQUE,
    meter_id TEXT,
    meter_event_name TEXT,
    tier_type TEXT NOT NULL,
    gate_order INTEGER NOT NULL DEFAULT 0,
    usage_cap BIGINT,
    credits_grant INTEGER,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CHECK (tier_type IN ('credits', 'metered'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_stripe_usage_tiers_default
    ON stripe.usage_tiers (is_default)
    WHERE is_default = TRUE;

CREATE INDEX IF NOT EXISTS idx_stripe_usage_tiers_price_id
    ON stripe.usage_tiers (price_id);

CREATE TABLE IF NOT EXISTS stripe.usage_events (
    event_id UUID PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    internal_id UUID NOT NULL,
    tier_key TEXT,
    meter_event_name TEXT,
    quantity BIGINT NOT NULL,
    used_credits BOOLEAN NOT NULL DEFAULT FALSE,
    stripe_identifier TEXT,
    occurred_at TIMESTAMP NOT NULL,
    created TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    delivered_at TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stripe_usage_events_internal_time
    ON stripe.usage_events (internal_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_stripe_usage_events_tier_time
    ON stripe.usage_events (tier_key, occurred_at DESC);
