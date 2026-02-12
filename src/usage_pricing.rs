use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::db::{BillingLinkRow, CreditRow, PriceRow, SubscriptionRow};
use crate::error::{LibError, Result};
use crate::models::stripe_client_from_env;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageInterval {
    Day,
    Week,
    Month,
    Year,
}

impl UsageInterval {
    fn to_stripe(self) -> stripe::CreatePriceRecurringInterval {
        match self {
            UsageInterval::Day => stripe::CreatePriceRecurringInterval::Day,
            UsageInterval::Week => stripe::CreatePriceRecurringInterval::Week,
            UsageInterval::Month => stripe::CreatePriceRecurringInterval::Month,
            UsageInterval::Year => stripe::CreatePriceRecurringInterval::Year,
        }
    }

    fn matches_recurring(self, recurring: stripe::RecurringInterval) -> bool {
        matches!(
            (self, recurring),
            (UsageInterval::Day, stripe::RecurringInterval::Day)
                | (UsageInterval::Week, stripe::RecurringInterval::Week)
                | (UsageInterval::Month, stripe::RecurringInterval::Month)
                | (UsageInterval::Year, stripe::RecurringInterval::Year)
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageTierType {
    Credits,
    Metered,
}

impl UsageTierType {
    fn as_db(self) -> &'static str {
        match self {
            UsageTierType::Credits => "credits",
            UsageTierType::Metered => "metered",
        }
    }

    fn from_db(value: &str) -> Result<Self> {
        match value {
            "credits" => Ok(UsageTierType::Credits),
            "metered" => Ok(UsageTierType::Metered),
            _ => Err(LibError::database(
                "Invalid usage tier type",
                anyhow!("unknown usage tier type: {value}"),
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCreditPackConfig {
    pub key: String,
    pub name: String,
    pub description: String,
    pub currency: stripe::Currency,
    pub unit_amount_cents: u64,
    pub credits_grant: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_price_lookup_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMeteredTierConfig {
    pub key: String,
    pub name: String,
    pub description: String,
    pub currency: stripe::Currency,
    pub unit_amount_cents: u64,
    pub interval: UsageInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_cap: Option<u64>,
    pub meter_display_name: String,
    pub meter_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_price_lookup_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_meter_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCreditZoneConfig {
    pub packs: Vec<UsageCreditPackConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_pack_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePricingConfig {
    pub untrusted_zone: UsageCreditZoneConfig,
    pub metered_gates: Vec<UsageMeteredTierConfig>,
    pub uncapped_tier: UsageMeteredTierConfig,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTierSummary {
    pub key: String,
    pub tier_type: UsageTierType,
    pub gate_order: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_cap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits_grant: Option<i32>,
    pub price_id: String,
    pub product_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_event_name: Option<String>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePricingCatalog {
    pub tiers: Vec<UsageTierSummary>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReportRequest {
    pub quantity: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurred_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReportOutcome {
    pub mode: UsageTierType,
    pub tier_key: String,
    pub quantity: u64,
    pub idempotency_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_credits: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_remaining: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_identifier: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAccessState {
    pub mode: UsageTierType,
    pub tier_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_cap: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_in_period: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_price_id: Option<String>,
}

#[derive(Clone, Debug, FromRow)]
struct UsageTierRow {
    key: String,
    product_id: String,
    price_id: String,
    price_lookup_key: String,
    meter_id: Option<String>,
    meter_event_name: Option<String>,
    tier_type: String,
    gate_order: i32,
    usage_cap: Option<i64>,
    credits_grant: Option<i32>,
    is_default: bool,
    is_active: bool,
    #[allow(dead_code)]
    created: NaiveDateTime,
    #[allow(dead_code)]
    updated: NaiveDateTime,
}

impl UsageTierRow {
    fn table_name() -> &'static str {
        "stripe.usage_tiers"
    }

    async fn upsert(pool: &PgPool, row: &Self) -> Result<(), sqlx::Error> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {} (
                key,
                product_id,
                price_id,
                price_lookup_key,
                meter_id,
                meter_event_name,
                tier_type,
                gate_order,
                usage_cap,
                credits_grant,
                is_default,
                is_active,
                created,
                updated
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            ON CONFLICT (key) DO UPDATE SET
                product_id = EXCLUDED.product_id,
                price_id = EXCLUDED.price_id,
                price_lookup_key = EXCLUDED.price_lookup_key,
                meter_id = EXCLUDED.meter_id,
                meter_event_name = EXCLUDED.meter_event_name,
                tier_type = EXCLUDED.tier_type,
                gate_order = EXCLUDED.gate_order,
                usage_cap = EXCLUDED.usage_cap,
                credits_grant = EXCLUDED.credits_grant,
                is_default = EXCLUDED.is_default,
                is_active = EXCLUDED.is_active,
                updated = EXCLUDED.updated
            "#,
            Self::table_name()
        ))
        .bind(&row.key)
        .bind(&row.product_id)
        .bind(&row.price_id)
        .bind(&row.price_lookup_key)
        .bind(&row.meter_id)
        .bind(&row.meter_event_name)
        .bind(&row.tier_type)
        .bind(row.gate_order)
        .bind(row.usage_cap)
        .bind(row.credits_grant)
        .bind(row.is_default)
        .bind(row.is_active)
        .bind(row.created)
        .bind(row.updated)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn clear_default_flags(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query(&format!(
            r#"
            UPDATE {} SET is_default = FALSE, updated = $1
            WHERE is_default = TRUE
            "#,
            Self::table_name()
        ))
        .bind(Utc::now().naive_utc())
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_by_key(pool: &PgPool, key: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(&format!(
            r#"
            SELECT
                key,
                product_id,
                price_id,
                price_lookup_key,
                meter_id,
                meter_event_name,
                tier_type,
                gate_order,
                usage_cap,
                credits_grant,
                is_default,
                is_active,
                created,
                updated
            FROM {}
            WHERE key = $1
            LIMIT 1
            "#,
            Self::table_name()
        ))
        .bind(key)
        .fetch_optional(pool)
        .await
    }

    async fn get_all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(&format!(
            r#"
            SELECT
                key,
                product_id,
                price_id,
                price_lookup_key,
                meter_id,
                meter_event_name,
                tier_type,
                gate_order,
                usage_cap,
                credits_grant,
                is_default,
                is_active,
                created,
                updated
            FROM {}
            ORDER BY gate_order ASC, key ASC
            "#,
            Self::table_name()
        ))
        .fetch_all(pool)
        .await
    }

    async fn get_by_price_id(pool: &PgPool, price_id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(&format!(
            r#"
            SELECT
                key,
                product_id,
                price_id,
                price_lookup_key,
                meter_id,
                meter_event_name,
                tier_type,
                gate_order,
                usage_cap,
                credits_grant,
                is_default,
                is_active,
                created,
                updated
            FROM {}
            WHERE price_id = $1
            LIMIT 1
            "#,
            Self::table_name()
        ))
        .bind(price_id)
        .fetch_optional(pool)
        .await
    }

    async fn get_default(pool: &PgPool) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(&format!(
            r#"
            SELECT
                key,
                product_id,
                price_id,
                price_lookup_key,
                meter_id,
                meter_event_name,
                tier_type,
                gate_order,
                usage_cap,
                credits_grant,
                is_default,
                is_active,
                created,
                updated
            FROM {}
            WHERE is_default = TRUE
            LIMIT 1
            "#,
            Self::table_name()
        ))
        .fetch_optional(pool)
        .await
    }
}

#[derive(Clone, Debug, FromRow)]
struct UsageEventRow {
    event_id: Uuid,
    idempotency_key: String,
    internal_id: Uuid,
    tier_key: Option<String>,
    #[allow(dead_code)]
    meter_event_name: Option<String>,
    quantity: i64,
    used_credits: bool,
    stripe_identifier: Option<String>,
    occurred_at: NaiveDateTime,
    #[allow(dead_code)]
    created: NaiveDateTime,
    delivered_at: Option<NaiveDateTime>,
}

impl UsageEventRow {
    fn table_name() -> &'static str {
        "stripe.usage_events"
    }

    async fn get_by_idempotency(
        pool: &PgPool,
        idempotency_key: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(&format!(
            r#"
            SELECT
                event_id,
                idempotency_key,
                internal_id,
                tier_key,
                meter_event_name,
                quantity,
                used_credits,
                stripe_identifier,
                occurred_at,
                created,
                delivered_at
            FROM {}
            WHERE idempotency_key = $1
            LIMIT 1
            "#,
            Self::table_name()
        ))
        .bind(idempotency_key)
        .fetch_optional(pool)
        .await
    }

    async fn insert_pending(
        pool: &PgPool,
        event_id: Uuid,
        idempotency_key: &str,
        internal_id: Uuid,
        quantity: i64,
        occurred_at: NaiveDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(&format!(
            r#"
            INSERT INTO {} (event_id, idempotency_key, internal_id, quantity, occurred_at, created)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Self::table_name()
        ))
        .bind(event_id)
        .bind(idempotency_key)
        .bind(internal_id)
        .bind(quantity)
        .bind(occurred_at)
        .bind(Utc::now().naive_utc())
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn mark_delivered(
        pool: &PgPool,
        event_id: Uuid,
        tier_key: &str,
        meter_event_name: Option<&str>,
        used_credits: bool,
        stripe_identifier: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(&format!(
            r#"
            UPDATE {} SET
                tier_key = $2,
                meter_event_name = $3,
                used_credits = $4,
                stripe_identifier = $5,
                delivered_at = $6
            WHERE event_id = $1
            "#,
            Self::table_name()
        ))
        .bind(event_id)
        .bind(tier_key)
        .bind(meter_event_name)
        .bind(used_credits)
        .bind(stripe_identifier)
        .bind(Utc::now().naive_utc())
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn sum_usage_in_window(
        pool: &PgPool,
        internal_id: Uuid,
        tier_key: &str,
        window_start: NaiveDateTime,
        window_end: NaiveDateTime,
    ) -> Result<i64, sqlx::Error> {
        #[derive(FromRow)]
        struct UsageTotal {
            total: i64,
        }

        let row = sqlx::query_as::<_, UsageTotal>(&format!(
            r#"
            SELECT COALESCE(SUM(quantity), 0)::BIGINT AS total
            FROM {}
            WHERE
                internal_id = $1
                AND tier_key = $2
                AND used_credits = FALSE
                AND delivered_at IS NOT NULL
                AND occurred_at >= $3
                AND occurred_at < $4
            "#,
            Self::table_name()
        ))
        .bind(internal_id)
        .bind(tier_key)
        .bind(window_start)
        .bind(window_end)
        .fetch_one(pool)
        .await?;

        Ok(row.total)
    }
}

#[derive(Clone, Debug, Serialize)]
struct CreateStripeMeterRequest<'a> {
    display_name: &'a str,
    event_name: &'a str,
    default_aggregation: StripeMeterDefaultAggregation<'a>,
    customer_mapping: StripeMeterCustomerMapping<'a>,
    value_settings: StripeMeterValueSettings<'a>,
}

#[derive(Clone, Debug, Serialize)]
struct StripeMeterDefaultAggregation<'a> {
    formula: &'a str,
}

#[derive(Clone, Debug, Serialize)]
struct StripeMeterCustomerMapping<'a> {
    event_payload_key: &'a str,
}

#[derive(Clone, Debug, Serialize)]
struct StripeMeterValueSettings<'a> {
    event_payload_key: &'a str,
}

#[derive(Clone, Debug, Serialize)]
struct CreateStripeMeterEventRequest<'a> {
    event_name: &'a str,
    identifier: &'a str,
    payload: StripeMeterEventPayload<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
struct StripeMeterEventPayload<'a> {
    stripe_customer_id: &'a str,
    value: &'a str,
}

pub async fn upsert_usage_pricing(
    pool: Arc<PgPool>,
    config: UsagePricingConfig,
) -> Result<UsagePricingCatalog> {
    let default_pack_key = validate_usage_pricing_config(&config)?;
    let client = stripe_client_from_env()?;

    UsageTierRow::clear_default_flags(&pool)
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to reset usage defaults",
                anyhow!("usage_pricing failed to reset defaults: {e}"),
            )
        })?;

    for pack in &config.untrusted_zone.packs {
        let existing = UsageTierRow::get_by_key(&pool, &pack.key)
            .await
            .map_err(|e| {
                LibError::database(
                    "Failed to load usage tier",
                    anyhow!(
                        "usage_pricing failed to load existing credit tier {}: {e}",
                        pack.key
                    ),
                )
            })?;

        let metadata = usage_tier_metadata(&pack.key, UsageTierType::Credits);
        let product_id = upsert_product(
            &client,
            &pack.name,
            &pack.description,
            pack.stripe_product_id.as_deref(),
            existing.as_ref().map(|r| r.product_id.as_str()),
            &metadata,
        )
        .await?;

        let lookup_key = pack
            .stripe_price_lookup_key
            .clone()
            .unwrap_or_else(|| format!("subseq:{}:credits", pack.key));
        let nickname = format!("{} credits", pack.name);
        let price_id = upsert_price(
            &client,
            DesiredPrice {
                product_id: &product_id,
                lookup_key: &lookup_key,
                nickname: &nickname,
                currency: pack.currency,
                unit_amount_cents: pack.unit_amount_cents,
                metadata: usage_tier_metadata(&pack.key, UsageTierType::Credits),
                kind: DesiredPriceKind::OneTime,
            },
        )
        .await?;

        PriceRow::insert(&pool, &PriceRow::new(&pack.key, &price_id))
            .await
            .map_err(|e| {
                LibError::database(
                    "Failed to upsert pricing plan",
                    anyhow!("usage_pricing failed to upsert price row {}: {e}", pack.key),
                )
            })?;

        let row = UsageTierRow {
            key: pack.key.clone(),
            product_id,
            price_id,
            price_lookup_key: lookup_key,
            meter_id: None,
            meter_event_name: None,
            tier_type: UsageTierType::Credits.as_db().to_string(),
            gate_order: 0,
            usage_cap: None,
            credits_grant: Some(pack.credits_grant),
            is_default: pack.key == default_pack_key,
            is_active: true,
            created: Utc::now().naive_utc(),
            updated: Utc::now().naive_utc(),
        };
        UsageTierRow::upsert(&pool, &row).await.map_err(|e| {
            LibError::database(
                "Failed to upsert usage tier",
                anyhow!(
                    "usage_pricing failed to upsert credit tier {}: {e}",
                    pack.key
                ),
            )
        })?;
    }

    for (idx, gate) in config.metered_gates.iter().enumerate() {
        upsert_metered_tier(&pool, &client, gate, idx as i32 + 1).await?;
    }
    upsert_metered_tier(
        &pool,
        &client,
        &config.uncapped_tier,
        config.metered_gates.len() as i32 + 1,
    )
    .await?;

    list_usage_tiers(pool).await
}

pub async fn list_usage_tiers(pool: Arc<PgPool>) -> Result<UsagePricingCatalog> {
    let rows = UsageTierRow::get_all(&pool).await.map_err(|e| {
        LibError::database(
            "Failed to list usage tiers",
            anyhow!("usage_pricing failed to list tiers: {e}"),
        )
    })?;

    let mut tiers = Vec::with_capacity(rows.len());
    for row in rows {
        tiers.push(UsageTierSummary {
            key: row.key,
            tier_type: UsageTierType::from_db(&row.tier_type)?,
            gate_order: row.gate_order,
            usage_cap: row.usage_cap.and_then(|v| u64::try_from(v).ok()),
            credits_grant: row.credits_grant,
            price_id: row.price_id,
            product_id: row.product_id,
            meter_id: row.meter_id,
            meter_event_name: row.meter_event_name,
            is_default: row.is_default,
        });
    }

    Ok(UsagePricingCatalog { tiers })
}

pub async fn usage_access_state(pool: Arc<PgPool>, internal_id: Uuid) -> Result<UsageAccessState> {
    let sub = SubscriptionRow::get_by_internal_id(&pool, internal_id)
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to load subscription",
                anyhow!("usage_pricing failed to load subscription: {e}"),
            )
        })?;
    let tier = resolve_usage_tier(&pool, sub.as_ref()).await?;
    let tier_type = UsageTierType::from_db(&tier.tier_type)?;

    let credits = match tier_type {
        UsageTierType::Credits => {
            CreditRow::get_credits(&pool, internal_id)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to load credits",
                        anyhow!("usage_pricing failed to load credits: {e}"),
                    )
                })?
        }
        UsageTierType::Metered => None,
    };

    let (usage_cap, used_in_period) = match tier_type {
        UsageTierType::Credits => (None, None),
        UsageTierType::Metered => {
            let cap = tier.usage_cap.and_then(|v| u64::try_from(v).ok());
            let used = match (cap, sub.as_ref()) {
                (Some(_), Some(sub)) => {
                    let (start, end) = metered_window(sub);
                    let total = UsageEventRow::sum_usage_in_window(
                        &pool,
                        internal_id,
                        &tier.key,
                        start,
                        end,
                    )
                    .await
                    .map_err(|e| {
                        LibError::database(
                            "Failed to load usage totals",
                            anyhow!("usage_pricing failed to load usage totals: {e}"),
                        )
                    })?;
                    Some(total.max(0) as u64)
                }
                _ => None,
            };
            (cap, used)
        }
    };

    Ok(UsageAccessState {
        mode: tier_type,
        tier_key: tier.key,
        credits,
        usage_cap,
        used_in_period,
        subscription_price_id: sub.and_then(|s| s.price_id),
    })
}

pub async fn report_usage(
    pool: Arc<PgPool>,
    internal_id: Uuid,
    request: UsageReportRequest,
) -> Result<UsageReportOutcome> {
    if request.quantity == 0 {
        return Err(LibError::invalid(
            "Invalid usage quantity",
            anyhow!("usage quantity must be > 0"),
        ));
    }

    let quantity_i64 = i64::try_from(request.quantity).map_err(|_| {
        LibError::invalid(
            "Invalid usage quantity",
            anyhow!("usage quantity exceeds i64"),
        )
    })?;

    let occurred_at = request.occurred_at.unwrap_or_else(Utc::now).naive_utc();
    let idempotency_key = request
        .idempotency_key
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let existing = UsageEventRow::get_by_idempotency(&pool, &idempotency_key)
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to load usage event",
                anyhow!("usage_pricing failed to load idempotency event: {e}"),
            )
        })?;

    let event = if let Some(row) = existing {
        if row.internal_id != internal_id {
            return Err(LibError::forbidden(
                "Idempotency key already used",
                anyhow!("usage idempotency key reused across internal IDs"),
            ));
        }
        if row.quantity != quantity_i64 {
            return Err(LibError::invalid(
                "Idempotency key conflict",
                anyhow!("idempotency key reused with a different quantity"),
            ));
        }
        if row.delivered_at.is_some() {
            return Ok(UsageReportOutcome {
                mode: if row.used_credits {
                    UsageTierType::Credits
                } else {
                    UsageTierType::Metered
                },
                tier_key: row.tier_key.unwrap_or_else(|| "unknown".to_string()),
                quantity: request.quantity,
                idempotency_key: row.idempotency_key,
                remaining_credits: None,
                cap_limit: None,
                cap_remaining: None,
                stripe_identifier: row.stripe_identifier,
            });
        }
        row
    } else {
        let event_id = Uuid::new_v4();
        UsageEventRow::insert_pending(
            &pool,
            event_id,
            &idempotency_key,
            internal_id,
            quantity_i64,
            occurred_at,
        )
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to persist usage event",
                anyhow!("usage_pricing failed to insert usage event: {e}"),
            )
        })?;
        UsageEventRow {
            event_id,
            idempotency_key,
            internal_id,
            tier_key: None,
            meter_event_name: None,
            quantity: quantity_i64,
            used_credits: false,
            stripe_identifier: None,
            occurred_at,
            created: Utc::now().naive_utc(),
            delivered_at: None,
        }
    };

    let sub = SubscriptionRow::get_by_internal_id(&pool, internal_id)
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to load subscription",
                anyhow!("usage_pricing failed to load subscription: {e}"),
            )
        })?;
    let tier = resolve_usage_tier(&pool, sub.as_ref()).await?;
    let tier_type = UsageTierType::from_db(&tier.tier_type)?;

    match tier_type {
        UsageTierType::Credits => {
            let quantity_i32 = i32::try_from(request.quantity).map_err(|_| {
                LibError::invalid(
                    "Invalid usage quantity",
                    anyhow!("usage quantity exceeds i32 for credits path"),
                )
            })?;

            let debited = try_sub_credits(&pool, internal_id, quantity_i32).await?;
            if !debited {
                return Err(LibError::forbidden(
                    "Not enough credits",
                    anyhow!("usage_pricing insufficient credits for internal_id={internal_id}"),
                ));
            }

            UsageEventRow::mark_delivered(&pool, event.event_id, &tier.key, None, true, None)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to finalize usage event",
                        anyhow!("usage_pricing failed to finalize credits event: {e}"),
                    )
                })?;

            let credits = CreditRow::get_credits(&pool, internal_id)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to load credits",
                        anyhow!("usage_pricing failed to read remaining credits: {e}"),
                    )
                })?;

            Ok(UsageReportOutcome {
                mode: UsageTierType::Credits,
                tier_key: tier.key,
                quantity: request.quantity,
                idempotency_key: event.idempotency_key,
                remaining_credits: credits,
                cap_limit: None,
                cap_remaining: None,
                stripe_identifier: None,
            })
        }
        UsageTierType::Metered => {
            let meter_event_name = tier.meter_event_name.clone().ok_or_else(|| {
                LibError::database(
                    "Invalid usage tier configuration",
                    anyhow!("metered tier missing meter event name for key={}", tier.key),
                )
            })?;

            let mut used_after = None;
            if let Some(cap) = tier.usage_cap.and_then(|v| u64::try_from(v).ok()) {
                let sub = sub.as_ref().ok_or_else(|| {
                    LibError::forbidden(
                        "Usage tier unavailable",
                        anyhow!("metered tier selected without subscription"),
                    )
                })?;
                let (start, end) = metered_window(sub);
                let used =
                    UsageEventRow::sum_usage_in_window(&pool, internal_id, &tier.key, start, end)
                        .await
                        .map_err(|e| {
                            LibError::database(
                                "Failed to load usage totals",
                                anyhow!("usage_pricing failed to calculate usage totals: {e}"),
                            )
                        })?;
                let used = used.max(0) as u64;
                let next = used.saturating_add(request.quantity);
                if next > cap {
                    return Err(LibError::forbidden(
                        "Usage cap exceeded",
                        anyhow!(
                            "usage_pricing cap exceeded for internal_id={} tier={} used={} quantity={} cap={}",
                            internal_id,
                            tier.key,
                            used,
                            request.quantity,
                            cap
                        ),
                    ));
                }
                used_after = Some(next);
            }

            let customer = BillingLinkRow::get_by_internal_id(&pool, internal_id)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to load customer link",
                        anyhow!("usage_pricing failed to load customer link: {e}"),
                    )
                })?
                .ok_or_else(|| {
                    LibError::not_found(
                        "Stripe customer not found",
                        anyhow!("no billing link found for internal_id={internal_id}"),
                    )
                })?;

            let stripe_identifier = post_meter_event(
                &meter_event_name,
                &customer.customer_id,
                request.quantity,
                &event.idempotency_key,
                Some(event.occurred_at.and_utc()),
            )
            .await?;

            UsageEventRow::mark_delivered(
                &pool,
                event.event_id,
                &tier.key,
                Some(&meter_event_name),
                false,
                Some(&stripe_identifier),
            )
            .await
            .map_err(|e| {
                LibError::database(
                    "Failed to finalize usage event",
                    anyhow!("usage_pricing failed to finalize metered event: {e}"),
                )
            })?;

            let cap_limit = tier.usage_cap.and_then(|v| u64::try_from(v).ok());
            let cap_remaining = match (cap_limit, used_after) {
                (Some(limit), Some(used)) => Some(limit.saturating_sub(used)),
                _ => None,
            };

            Ok(UsageReportOutcome {
                mode: UsageTierType::Metered,
                tier_key: tier.key,
                quantity: request.quantity,
                idempotency_key: event.idempotency_key,
                remaining_credits: None,
                cap_limit,
                cap_remaining,
                stripe_identifier: Some(stripe_identifier),
            })
        }
    }
}

async fn upsert_metered_tier(
    pool: &PgPool,
    client: &stripe::Client,
    gate: &UsageMeteredTierConfig,
    gate_order: i32,
) -> Result<()> {
    let existing = UsageTierRow::get_by_key(pool, &gate.key)
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to load usage tier",
                anyhow!(
                    "usage_pricing failed to load existing metered tier {}: {e}",
                    gate.key
                ),
            )
        })?;

    let product_id = upsert_product(
        client,
        &gate.name,
        &gate.description,
        gate.stripe_product_id.as_deref(),
        existing.as_ref().map(|r| r.product_id.as_str()),
        &usage_tier_metadata(&gate.key, UsageTierType::Metered),
    )
    .await?;

    let meter_id = upsert_meter(
        client,
        &gate.meter_display_name,
        &gate.meter_event_name,
        gate.stripe_meter_id.as_deref(),
        existing.as_ref().and_then(|r| r.meter_id.as_deref()),
    )
    .await?;

    let lookup_key = gate
        .stripe_price_lookup_key
        .clone()
        .unwrap_or_else(|| format!("subseq:{}:metered", gate.key));
    let nickname = format!("{} metered", gate.name);
    let price_id = upsert_price(
        client,
        DesiredPrice {
            product_id: &product_id,
            lookup_key: &lookup_key,
            nickname: &nickname,
            currency: gate.currency,
            unit_amount_cents: gate.unit_amount_cents,
            metadata: usage_tier_metadata(&gate.key, UsageTierType::Metered),
            kind: DesiredPriceKind::Metered {
                interval: gate.interval,
            },
        },
    )
    .await?;

    PriceRow::insert(pool, &PriceRow::new(&gate.key, &price_id))
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to upsert pricing plan",
                anyhow!("usage_pricing failed to upsert price row {}: {e}", gate.key),
            )
        })?;

    let row = UsageTierRow {
        key: gate.key.clone(),
        product_id,
        price_id,
        price_lookup_key: lookup_key,
        meter_id: Some(meter_id),
        meter_event_name: Some(gate.meter_event_name.clone()),
        tier_type: UsageTierType::Metered.as_db().to_string(),
        gate_order,
        usage_cap: gate.usage_cap.map(i64::try_from).transpose().map_err(|_| {
            LibError::invalid(
                "Invalid usage cap",
                anyhow!("usage_cap exceeds i64 for {}", gate.key),
            )
        })?,
        credits_grant: None,
        is_default: false,
        is_active: true,
        created: Utc::now().naive_utc(),
        updated: Utc::now().naive_utc(),
    };
    UsageTierRow::upsert(pool, &row).await.map_err(|e| {
        LibError::database(
            "Failed to upsert usage tier",
            anyhow!(
                "usage_pricing failed to upsert metered tier {}: {e}",
                gate.key
            ),
        )
    })?;

    Ok(())
}

struct DesiredPrice<'a> {
    product_id: &'a str,
    lookup_key: &'a str,
    nickname: &'a str,
    currency: stripe::Currency,
    unit_amount_cents: u64,
    metadata: stripe::Metadata,
    kind: DesiredPriceKind,
}

enum DesiredPriceKind {
    OneTime,
    Metered { interval: UsageInterval },
}

async fn upsert_price(client: &stripe::Client, desired: DesiredPrice<'_>) -> Result<String> {
    let mut params = stripe::ListPrices::new();
    params.lookup_keys = Some(vec![desired.lookup_key.to_string()]);
    params.limit = Some(10);
    let existing = stripe::Price::list(client, &params).await.map_err(|e| {
        LibError::upstream(
            "Failed to list Stripe prices",
            anyhow!("usage_pricing failed to list prices by lookup_key: {e}"),
        )
    })?;

    let candidate = existing.data.into_iter().next();
    if let Some(price) = candidate {
        if price_matches(&price, &desired)? {
            let mut update = stripe::UpdatePrice::new();
            update.nickname = Some(desired.nickname);
            update.metadata = Some(desired.metadata.clone());
            update.active = Some(true);
            stripe::Price::update(client, &price.id, update)
                .await
                .map_err(|e| {
                    LibError::upstream(
                        "Failed to update Stripe price",
                        anyhow!(
                            "usage_pricing failed to update existing price {}: {e}",
                            price.id
                        ),
                    )
                })?;
            return Ok(price.id.to_string());
        }
    }

    let mut create = stripe::CreatePrice::new(desired.currency);
    create.lookup_key = Some(desired.lookup_key);
    create.transfer_lookup_key = Some(true);
    create.nickname = Some(desired.nickname);
    create.metadata = Some(desired.metadata);
    create.product = Some(stripe::IdOrCreate::Id(desired.product_id));
    create.unit_amount = Some(i64::try_from(desired.unit_amount_cents).map_err(|_| {
        LibError::invalid(
            "Invalid price amount",
            anyhow!("usage_pricing amount overflow for {}", desired.lookup_key),
        )
    })?);

    if let DesiredPriceKind::Metered { interval } = desired.kind {
        create.recurring = Some(stripe::CreatePriceRecurring {
            aggregate_usage: Some(stripe::CreatePriceRecurringAggregateUsage::Sum),
            interval: interval.to_stripe(),
            interval_count: Some(1),
            trial_period_days: None,
            usage_type: Some(stripe::CreatePriceRecurringUsageType::Metered),
        });
    }

    let created = stripe::Price::create(client, create).await.map_err(|e| {
        LibError::upstream(
            "Failed to create Stripe price",
            anyhow!(
                "usage_pricing failed to create price {}: {e}",
                desired.lookup_key
            ),
        )
    })?;
    Ok(created.id.to_string())
}

fn price_matches(price: &stripe::Price, desired: &DesiredPrice<'_>) -> Result<bool> {
    let same_product = price_product_id(price)
        .map(|pid| pid == desired.product_id)
        .unwrap_or(false);
    let same_currency = price.currency == Some(desired.currency);
    let same_unit_amount = price.unit_amount
        == Some(i64::try_from(desired.unit_amount_cents).map_err(|_| {
            LibError::invalid(
                "Invalid price amount",
                anyhow!("usage_pricing amount overflow for {}", desired.lookup_key),
            )
        })?);

    let recurring_matches = match desired.kind {
        DesiredPriceKind::OneTime => price.recurring.is_none(),
        DesiredPriceKind::Metered { interval } => {
            let recurring = match &price.recurring {
                Some(r) => r,
                None => return Ok(false),
            };
            interval.matches_recurring(recurring.interval)
                && recurring.usage_type == stripe::RecurringUsageType::Metered
        }
    };

    Ok(same_product && same_currency && same_unit_amount && recurring_matches)
}

fn price_product_id(price: &stripe::Price) -> Option<String> {
    match price.product.as_ref() {
        Some(stripe::Expandable::Id(id)) => Some(id.to_string()),
        Some(stripe::Expandable::Object(product)) => Some(product.id.to_string()),
        None => None,
    }
}

async fn upsert_product(
    client: &stripe::Client,
    name: &str,
    description: &str,
    configured_product_id: Option<&str>,
    existing_product_id: Option<&str>,
    metadata: &stripe::Metadata,
) -> Result<String> {
    let product_id = configured_product_id.or(existing_product_id);
    if let Some(product_id) = product_id {
        let product_id_t = stripe::ProductId::from_str(product_id).map_err(|_| {
            LibError::invalid(
                "Invalid Stripe product ID",
                anyhow!("usage_pricing invalid Stripe product ID {}", product_id),
            )
        })?;

        let product_exists = stripe::Product::retrieve(client, &product_id_t, &[])
            .await
            .is_ok();
        if product_exists {
            let mut update = stripe::UpdateProduct::new();
            update.name = Some(name);
            update.description = Some(description.to_string());
            update.metadata = Some(metadata.clone());
            stripe::Product::update(client, &product_id_t, update)
                .await
                .map_err(|e| {
                    LibError::upstream(
                        "Failed to update Stripe product",
                        anyhow!("usage_pricing failed to update Stripe product {product_id}: {e}"),
                    )
                })?;
            return Ok(product_id.to_string());
        }

        let mut create = stripe::CreateProduct::new(name);
        create.id = Some(product_id);
        if !description.is_empty() {
            create.description = Some(description);
        }
        create.metadata = Some(metadata.clone());
        let created = stripe::Product::create(client, create).await.map_err(|e| {
            LibError::upstream(
                "Failed to create Stripe product",
                anyhow!("usage_pricing failed to create Stripe product {product_id}: {e}"),
            )
        })?;
        return Ok(created.id.to_string());
    }

    let mut create = stripe::CreateProduct::new(name);
    if !description.is_empty() {
        create.description = Some(description);
    }
    create.metadata = Some(metadata.clone());
    let created = stripe::Product::create(client, create).await.map_err(|e| {
        LibError::upstream(
            "Failed to create Stripe product",
            anyhow!("usage_pricing failed to create Stripe product: {e}"),
        )
    })?;
    Ok(created.id.to_string())
}

async fn upsert_meter(
    client: &stripe::Client,
    meter_display_name: &str,
    meter_event_name: &str,
    configured_meter_id: Option<&str>,
    existing_meter_id: Option<&str>,
) -> Result<String> {
    if meter_event_name.trim().is_empty() {
        return Err(LibError::invalid(
            "Invalid meter event name",
            anyhow!("meter event name cannot be empty"),
        ));
    }

    let meter_id = configured_meter_id.or(existing_meter_id);
    if let Some(meter_id) = meter_id {
        let meter: std::result::Result<Value, stripe::StripeError> =
            client.get(&format!("/billing/meters/{meter_id}")).await;
        if let Ok(meter) = meter {
            let id = value_string(&meter, "id").unwrap_or_else(|| meter_id.to_string());
            return Ok(id);
        }
    }

    let req = CreateStripeMeterRequest {
        display_name: meter_display_name,
        event_name: meter_event_name,
        default_aggregation: StripeMeterDefaultAggregation { formula: "sum" },
        customer_mapping: StripeMeterCustomerMapping {
            event_payload_key: "stripe_customer_id",
        },
        value_settings: StripeMeterValueSettings {
            event_payload_key: "value",
        },
    };

    let meter: Value = client
        .post_form("/billing/meters", req)
        .await
        .map_err(|e| {
            LibError::upstream(
                "Failed to create Stripe meter",
                anyhow!(
                    "usage_pricing failed to create Stripe meter for event {meter_event_name}: {e}"
                ),
            )
        })?;

    value_string(&meter, "id").ok_or_else(|| {
        LibError::upstream(
            "Failed to parse Stripe meter",
            anyhow!("usage_pricing Stripe meter response missing id"),
        )
    })
}

async fn post_meter_event(
    event_name: &str,
    customer_id: &str,
    quantity: u64,
    idempotency_key: &str,
    occurred_at: Option<DateTime<Utc>>,
) -> Result<String> {
    let client = stripe_client_from_env()?;
    let value = quantity.to_string();
    let req = CreateStripeMeterEventRequest {
        event_name,
        identifier: idempotency_key,
        payload: StripeMeterEventPayload {
            stripe_customer_id: customer_id,
            value: &value,
        },
        timestamp: occurred_at.map(|ts| ts.timestamp()),
    };

    let response: Value = client
        .post_form("/billing/meter_events", req)
        .await
        .map_err(|e| {
            LibError::upstream(
                "Failed to report meter usage",
                anyhow!("usage_pricing failed to post Stripe meter event: {e}"),
            )
        })?;
    Ok(value_string(&response, "identifier").unwrap_or_else(|| idempotency_key.to_string()))
}

async fn resolve_usage_tier(pool: &PgPool, sub: Option<&SubscriptionRow>) -> Result<UsageTierRow> {
    if let Some(sub) = sub {
        if sub.is_active {
            if let Some(price_id) = sub.price_id.as_deref() {
                if let Some(tier) = UsageTierRow::get_by_price_id(pool, price_id)
                    .await
                    .map_err(|e| {
                        LibError::database(
                            "Failed to load usage tier",
                            anyhow!("usage_pricing failed to load tier by price_id: {e}"),
                        )
                    })?
                {
                    if tier.is_active {
                        return Ok(tier);
                    }
                }
            }
        }
    }

    UsageTierRow::get_default(pool)
        .await
        .map_err(|e| {
            LibError::database(
                "Failed to load default usage tier",
                anyhow!("usage_pricing failed to load default tier: {e}"),
            )
        })?
        .ok_or_else(|| {
            LibError::not_found(
                "Default usage tier not configured",
                anyhow!("usage_pricing default tier missing"),
            )
        })
}

async fn try_sub_credits(pool: &PgPool, internal_id: Uuid, quantity: i32) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE stripe.credits
        SET credits = credits - $2, updated = $3
        WHERE internal_id = $1 AND credits >= $2
        "#,
    )
    .bind(internal_id)
    .bind(quantity)
    .bind(Utc::now().naive_utc())
    .execute(pool)
    .await
    .map_err(|e| {
        LibError::database(
            "Failed to debit credits",
            anyhow!("usage_pricing failed to debit credits: {e}"),
        )
    })?;

    Ok(result.rows_affected() == 1)
}

fn metered_window(sub: &SubscriptionRow) -> (NaiveDateTime, NaiveDateTime) {
    let end = sub
        .current_period_timestamp
        .unwrap_or_else(|| Utc::now().naive_utc() + Duration::days(1));
    let start = sub
        .current_period_start
        .unwrap_or_else(|| end - Duration::days(31));
    (start, end)
}

fn usage_tier_metadata(key: &str, tier_type: UsageTierType) -> stripe::Metadata {
    let mut metadata: HashMap<String, String> = HashMap::new();
    metadata.insert("subseq_usage_tier_key".to_string(), key.to_string());
    metadata.insert(
        "subseq_usage_tier_type".to_string(),
        tier_type.as_db().to_string(),
    );
    metadata
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

fn validate_usage_pricing_config(config: &UsagePricingConfig) -> Result<String> {
    if config.untrusted_zone.packs.is_empty() {
        return Err(LibError::invalid(
            "Missing credit packs",
            anyhow!("usage_pricing requires at least one untrusted credit pack"),
        ));
    }
    if config.metered_gates.is_empty() {
        return Err(LibError::invalid(
            "Missing metered gates",
            anyhow!("usage_pricing requires at least one capped metered gate"),
        ));
    }
    if config.uncapped_tier.usage_cap.is_some() {
        return Err(LibError::invalid(
            "Invalid uncapped tier",
            anyhow!("usage_pricing uncapped_tier must not set usage_cap"),
        ));
    }

    let mut seen = HashMap::<String, ()>::new();
    for pack in &config.untrusted_zone.packs {
        if pack.credits_grant <= 0 {
            return Err(LibError::invalid(
                "Invalid credit grant",
                anyhow!("credits_grant must be > 0 for {}", pack.key),
            ));
        }
        if seen.insert(pack.key.clone(), ()).is_some() {
            return Err(LibError::invalid(
                "Duplicate tier key",
                anyhow!("duplicate usage tier key {}", pack.key),
            ));
        }
    }
    for gate in &config.metered_gates {
        if gate.usage_cap.map(|cap| cap == 0).unwrap_or(true) {
            return Err(LibError::invalid(
                "Invalid metered gate cap",
                anyhow!("metered gate {} must set usage_cap > 0", gate.key),
            ));
        }
        if gate.meter_event_name.trim().is_empty() {
            return Err(LibError::invalid(
                "Invalid meter event name",
                anyhow!("metered gate {} has empty meter_event_name", gate.key),
            ));
        }
        if seen.insert(gate.key.clone(), ()).is_some() {
            return Err(LibError::invalid(
                "Duplicate tier key",
                anyhow!("duplicate usage tier key {}", gate.key),
            ));
        }
    }
    if config.uncapped_tier.meter_event_name.trim().is_empty() {
        return Err(LibError::invalid(
            "Invalid meter event name",
            anyhow!(
                "uncapped tier {} has empty meter_event_name",
                config.uncapped_tier.key
            ),
        ));
    }
    if seen.insert(config.uncapped_tier.key.clone(), ()).is_some() {
        return Err(LibError::invalid(
            "Duplicate tier key",
            anyhow!("duplicate usage tier key {}", config.uncapped_tier.key),
        ));
    }

    if let Some(default_key) = config.untrusted_zone.default_pack_key.as_deref() {
        if !config
            .untrusted_zone
            .packs
            .iter()
            .any(|pack| pack.key == default_key)
        {
            return Err(LibError::invalid(
                "Invalid default credit pack",
                anyhow!("default credit pack {} not found in packs", default_key),
            ));
        }
        return Ok(default_key.to_string());
    }

    Ok(config.untrusted_zone.packs[0].key.clone())
}

#[cfg(test)]
mod test {
    use super::*;

    fn sample_config() -> UsagePricingConfig {
        UsagePricingConfig {
            untrusted_zone: UsageCreditZoneConfig {
                packs: vec![UsageCreditPackConfig {
                    key: "credits-small".to_string(),
                    name: "Starter Credits".to_string(),
                    description: "Starter pack".to_string(),
                    currency: stripe::Currency::USD,
                    unit_amount_cents: 500,
                    credits_grant: 1000,
                    stripe_product_id: None,
                    stripe_price_lookup_key: None,
                }],
                default_pack_key: None,
            },
            metered_gates: vec![UsageMeteredTierConfig {
                key: "metered-cap-1".to_string(),
                name: "Gate 1".to_string(),
                description: "Capped gate".to_string(),
                currency: stripe::Currency::USD,
                unit_amount_cents: 1,
                interval: UsageInterval::Month,
                usage_cap: Some(10_000),
                meter_display_name: "Gate 1 usage".to_string(),
                meter_event_name: "gate_1_usage".to_string(),
                stripe_product_id: None,
                stripe_price_lookup_key: None,
                stripe_meter_id: None,
            }],
            uncapped_tier: UsageMeteredTierConfig {
                key: "metered-uncapped".to_string(),
                name: "Uncapped".to_string(),
                description: "Uncapped gate".to_string(),
                currency: stripe::Currency::USD,
                unit_amount_cents: 1,
                interval: UsageInterval::Month,
                usage_cap: None,
                meter_display_name: "Uncapped usage".to_string(),
                meter_event_name: "uncapped_usage".to_string(),
                stripe_product_id: None,
                stripe_price_lookup_key: None,
                stripe_meter_id: None,
            },
        }
    }

    #[test]
    fn validate_usage_config_picks_first_pack_as_default() {
        let config = sample_config();
        let default_key = validate_usage_pricing_config(&config).expect("valid config");
        assert_eq!(default_key, "credits-small");
    }

    #[test]
    fn validate_usage_config_rejects_capped_uncapped_tier() {
        let mut config = sample_config();
        config.uncapped_tier.usage_cap = Some(1);
        let err = validate_usage_pricing_config(&config).expect_err("must reject invalid uncapped");
        assert!(matches!(err.kind, crate::error::ErrorKind::InvalidInput));
    }
}
