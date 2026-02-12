mod common;

use std::sync::Arc;

use anyhow::{Context, Result as AnyResult};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use subseq_stripe::db::{CreditRow, SubscriptionRow};
use subseq_stripe::tables::{SubscriptionStateUpdate, SubscriptionType};
use subseq_stripe::usage_pricing::{UsageTierType, report_usage, usage_access_state};
use uuid::Uuid;

fn run_db_test<F, Fut>(run: F)
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = AnyResult<()>>,
{
    sqlx::test_block_on(async move {
        let test_db = common::TestDb::new().await?;
        test_db.prepare().await?;

        let pool = test_db.pool.clone();
        let run_result = run(pool).await;
        let teardown_result = test_db.teardown().await;

        teardown_result?;
        run_result
    })
    .expect("integration test failed");
}

async fn insert_credit_default_tier(pool: &PgPool, key: &str) -> AnyResult<()> {
    let now = Utc::now().naive_utc();
    sqlx::query(
        r#"
        INSERT INTO stripe.usage_tiers (
            key, product_id, price_id, price_lookup_key, meter_id, meter_event_name,
            tier_type, gate_order, usage_cap, credits_grant, is_default, is_active, created, updated
        )
        VALUES ($1, $2, $3, $4, NULL, NULL, 'credits', 0, NULL, $5, TRUE, TRUE, $6, $6)
        "#,
    )
    .bind(key)
    .bind(format!("prod_{key}"))
    .bind(format!("price_{key}"))
    .bind(format!("lookup_{key}"))
    .bind(100_i32)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_metered_tier(
    pool: &PgPool,
    key: &str,
    price_id: &str,
    usage_cap: i64,
) -> AnyResult<()> {
    let now = Utc::now().naive_utc();
    sqlx::query(
        r#"
        INSERT INTO stripe.usage_tiers (
            key, product_id, price_id, price_lookup_key, meter_id, meter_event_name,
            tier_type, gate_order, usage_cap, credits_grant, is_default, is_active, created, updated
        )
        VALUES ($1, $2, $3, $4, 'meter_1', 'meter_evt_1', 'metered', 1, $5, NULL, FALSE, TRUE, $6, $6)
        "#,
    )
    .bind(key)
    .bind(format!("prod_{key}"))
    .bind(price_id)
    .bind(format!("lookup_{key}"))
    .bind(usage_cap)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

#[test]
fn subscription_row_tracks_price_and_period_start_end() {
    run_db_test(|pool| async move {
        let internal_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        let row = SubscriptionRow {
            internal_id,
            created: now,
            updated: now,
            subscription_id: Some("sub_123".to_string()),
            price_id: Some("price_basic".to_string()),
            subscription_type: serde_json::to_string(&SubscriptionType::Paid)?,
            seats: 1,
            is_active: true,
            current_period_start: Some(now),
            current_period_timestamp: Some(now + Duration::days(30)),
            cancel_at_period_end: false,
            last_payment_failed: false,
            is_auto_billing: true,
        };
        SubscriptionRow::insert(&pool, &row)
            .await
            .context("insert subscription row")?;

        let fetched = SubscriptionRow::get_by_internal_id(&pool, internal_id)
            .await?
            .expect("subscription row");
        assert_eq!(fetched.price_id.as_deref(), Some("price_basic"));
        assert_eq!(fetched.current_period_start, row.current_period_start);

        let update = SubscriptionStateUpdate {
            subscription_id: Some("sub_123".to_string()),
            price_id: Some("price_metered_cap".to_string()),
            subscription_type: SubscriptionType::Paid,
            cancel_at_period_end: false,
            current_period_start: Some(now + Duration::days(1)),
            current_period_end: Some(now + Duration::days(31)),
            is_auto_billing: true,
            seats: Some(3),
        };
        SubscriptionRow::update_by_internal_id(&pool, internal_id, update)
            .await
            .context("update subscription row by internal_id")?;

        let updated = SubscriptionRow::get_by_internal_id(&pool, internal_id)
            .await?
            .expect("subscription row after update");
        assert_eq!(updated.price_id.as_deref(), Some("price_metered_cap"));
        assert_eq!(updated.current_period_start, Some(now + Duration::days(1)));
        assert_eq!(
            updated.current_period_timestamp,
            Some(now + Duration::days(31))
        );
        assert_eq!(updated.seats, 3);

        Ok(())
    });
}

#[test]
fn usage_access_state_defaults_to_credit_tier_without_subscription() {
    run_db_test(|pool| async move {
        let internal_id = Uuid::new_v4();
        insert_credit_default_tier(&pool, "credits_default").await?;
        CreditRow::add_credits(&pool, internal_id, 75).await?;

        let state = usage_access_state(Arc::new(pool.clone()), internal_id).await?;
        assert_eq!(state.mode, UsageTierType::Credits);
        assert_eq!(state.tier_key, "credits_default");
        assert_eq!(state.credits, Some(75));
        assert_eq!(state.usage_cap, None);
        assert_eq!(state.used_in_period, None);

        Ok(())
    });
}

#[test]
fn report_usage_debits_credits_once_per_idempotency_key() {
    run_db_test(|pool| async move {
        let internal_id = Uuid::new_v4();
        insert_credit_default_tier(&pool, "credits_default").await?;
        CreditRow::add_credits(&pool, internal_id, 10).await?;

        let first = report_usage(
            Arc::new(pool.clone()),
            internal_id,
            subseq_stripe::usage_pricing::UsageReportRequest {
                quantity: 3,
                idempotency_key: Some("usage-event-1".to_string()),
                occurred_at: None,
            },
        )
        .await?;
        assert_eq!(first.mode, UsageTierType::Credits);
        assert_eq!(first.tier_key, "credits_default");
        assert_eq!(first.remaining_credits, Some(7));

        let second = report_usage(
            Arc::new(pool.clone()),
            internal_id,
            subseq_stripe::usage_pricing::UsageReportRequest {
                quantity: 3,
                idempotency_key: Some("usage-event-1".to_string()),
                occurred_at: None,
            },
        )
        .await?;
        assert_eq!(second.mode, UsageTierType::Credits);
        assert_eq!(second.tier_key, "credits_default");

        let credits_after = CreditRow::get_credits(&pool, internal_id).await?;
        assert_eq!(credits_after, Some(7));

        let usage_event_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM stripe.usage_events")
                .fetch_one(&pool)
                .await?;
        assert_eq!(usage_event_count, 1);

        Ok(())
    });
}

#[test]
fn usage_access_state_reports_metered_cap_usage_for_active_subscription() {
    run_db_test(|pool| async move {
        let internal_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        insert_metered_tier(&pool, "metered_cap_1", "price_metered_cap_1", 1_000).await?;

        let sub = SubscriptionRow {
            internal_id,
            created: now,
            updated: now,
            subscription_id: Some("sub_abc".to_string()),
            price_id: Some("price_metered_cap_1".to_string()),
            subscription_type: serde_json::to_string(&SubscriptionType::Paid)?,
            seats: 1,
            is_active: true,
            current_period_start: Some(now - Duration::days(1)),
            current_period_timestamp: Some(now + Duration::days(29)),
            cancel_at_period_end: false,
            last_payment_failed: false,
            is_auto_billing: true,
        };
        SubscriptionRow::insert(&pool, &sub).await?;

        sqlx::query(
            r#"
            INSERT INTO stripe.usage_events (
                event_id, idempotency_key, internal_id, tier_key, meter_event_name, quantity,
                used_credits, stripe_identifier, occurred_at, created, delivered_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, FALSE, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind("evt-metered-1")
        .bind(internal_id)
        .bind("metered_cap_1")
        .bind("meter_evt_1")
        .bind(120_i64)
        .bind("evt-metered-1")
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO stripe.usage_events (
                event_id, idempotency_key, internal_id, tier_key, meter_event_name, quantity,
                used_credits, stripe_identifier, occurred_at, created, delivered_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, FALSE, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind("evt-metered-2")
        .bind(internal_id)
        .bind("metered_cap_1")
        .bind("meter_evt_1")
        .bind(80_i64)
        .bind("evt-metered-2")
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await?;

        let state = usage_access_state(Arc::new(pool.clone()), internal_id).await?;
        assert_eq!(state.mode, UsageTierType::Metered);
        assert_eq!(state.tier_key, "metered_cap_1");
        assert_eq!(state.usage_cap, Some(1_000));
        assert_eq!(state.used_in_period, Some(200));
        assert_eq!(
            state.subscription_price_id.as_deref(),
            Some("price_metered_cap_1")
        );

        Ok(())
    });
}
