//! Stripe webhook event hooks.
//!
//! This trait provides one method per Stripe event type. Override the hooks you need; the
//! defaults call `on_unhandled_event` which returns `Ok(())` so unhandled events do not fail
//! webhook delivery.

use anyhow::anyhow;
use stripe::{Event, EventObject, EventType};
use uuid::Uuid;

use crate::api::HasPool;
use crate::db;
use crate::error::{ErrorKind, LibError, Result as LibResult};
use crate::models::BoxFut;
use crate::tables::SubscriptionStateUpdate;

/// Hook surface for Stripe webhooks.
///
/// Implementors should override the relevant `on_event_*` methods.
pub trait HandlesStripeEvents: HasPool {
    /// Dispatches to the appropriate `on_event_*` method for the incoming Stripe event.
    fn handle_stripe_event(&self, event: Event) -> BoxFut<LibResult<()>> {
        match event.type_ {
            EventType::AccountApplicationAuthorized => {
                self.on_event_account_application_authorized(event)
            }
            EventType::AccountApplicationDeauthorized => {
                self.on_event_account_application_deauthorized(event)
            }
            EventType::AccountExternalAccountCreated => {
                self.on_event_account_external_account_created(event)
            }
            EventType::AccountExternalAccountDeleted => {
                self.on_event_account_external_account_deleted(event)
            }
            EventType::AccountExternalAccountUpdated => {
                self.on_event_account_external_account_updated(event)
            }
            EventType::AccountUpdated => self.on_event_account_updated(event),
            EventType::ApplicationFeeCreated => self.on_event_application_fee_created(event),
            EventType::ApplicationFeeRefundUpdated => {
                self.on_event_application_fee_refund_updated(event)
            }
            EventType::ApplicationFeeRefunded => self.on_event_application_fee_refunded(event),
            EventType::BalanceAvailable => self.on_event_balance_available(event),
            EventType::BillingPortalConfigurationCreated => {
                self.on_event_billing_portal_configuration_created(event)
            }
            EventType::BillingPortalConfigurationUpdated => {
                self.on_event_billing_portal_configuration_updated(event)
            }
            EventType::CapabilityUpdated => self.on_event_capability_updated(event),
            EventType::CashBalanceFundsAvailable => {
                self.on_event_cash_balance_funds_available(event)
            }
            EventType::ChargeCaptured => self.on_event_charge_captured(event),
            EventType::ChargeDisputeClosed => self.on_event_charge_dispute_closed(event),
            EventType::ChargeDisputeCreated => self.on_event_charge_dispute_created(event),
            EventType::ChargeDisputeFundsReinstated => {
                self.on_event_charge_dispute_funds_reinstated(event)
            }
            EventType::ChargeDisputeFundsWithdrawn => {
                self.on_event_charge_dispute_funds_withdrawn(event)
            }
            EventType::ChargeDisputeUpdated => self.on_event_charge_dispute_updated(event),
            EventType::ChargeExpired => self.on_event_charge_expired(event),
            EventType::ChargeFailed => self.on_event_charge_failed(event),
            EventType::ChargePending => self.on_event_charge_pending(event),
            EventType::ChargeRefundUpdated => self.on_event_charge_refund_updated(event),
            EventType::ChargeRefunded => self.on_event_charge_refunded(event),
            EventType::ChargeSucceeded => self.on_event_charge_succeeded(event),
            EventType::ChargeUpdated => self.on_event_charge_updated(event),
            EventType::CheckoutSessionAsyncPaymentFailed => {
                self.on_event_checkout_session_async_payment_failed(event)
            }
            EventType::CheckoutSessionAsyncPaymentSucceeded => {
                self.on_event_checkout_session_async_payment_succeeded(event)
            }
            EventType::CheckoutSessionCompleted => self.on_event_checkout_session_completed(event),
            EventType::CheckoutSessionExpired => self.on_event_checkout_session_expired(event),
            EventType::CouponCreated => self.on_event_coupon_created(event),
            EventType::CouponDeleted => self.on_event_coupon_deleted(event),
            EventType::CouponUpdated => self.on_event_coupon_updated(event),
            EventType::CreditNoteCreated => self.on_event_credit_note_created(event),
            EventType::CreditNoteUpdated => self.on_event_credit_note_updated(event),
            EventType::CreditNoteVoided => self.on_event_credit_note_voided(event),
            EventType::CustomerCreated => self.on_event_customer_created(event),
            EventType::CustomerDeleted => self.on_event_customer_deleted(event),
            EventType::CustomerDiscountCreated => self.on_event_customer_discount_created(event),
            EventType::CustomerDiscountDeleted => self.on_event_customer_discount_deleted(event),
            EventType::CustomerDiscountUpdated => self.on_event_customer_discount_updated(event),
            EventType::CustomerSourceCreated => self.on_event_customer_source_created(event),
            EventType::CustomerSourceDeleted => self.on_event_customer_source_deleted(event),
            EventType::CustomerSourceExpiring => self.on_event_customer_source_expiring(event),
            EventType::CustomerSourceUpdated => self.on_event_customer_source_updated(event),
            EventType::CustomerSubscriptionCreated => {
                self.on_event_customer_subscription_created(event)
            }
            EventType::CustomerSubscriptionDeleted => {
                self.on_event_customer_subscription_deleted(event)
            }
            EventType::CustomerSubscriptionPaused => {
                self.on_event_customer_subscription_paused(event)
            }
            EventType::CustomerSubscriptionPendingUpdateApplied => {
                self.on_event_customer_subscription_pending_update_applied(event)
            }
            EventType::CustomerSubscriptionPendingUpdateExpired => {
                self.on_event_customer_subscription_pending_update_expired(event)
            }
            EventType::CustomerSubscriptionResumed => {
                self.on_event_customer_subscription_resumed(event)
            }
            EventType::CustomerSubscriptionTrialWillEnd => {
                self.on_event_customer_subscription_trial_will_end(event)
            }
            EventType::CustomerSubscriptionUpdated => {
                self.on_event_customer_subscription_updated(event)
            }
            EventType::CustomerTaxIdCreated => self.on_event_customer_tax_id_created(event),
            EventType::CustomerTaxIdDeleted => self.on_event_customer_tax_id_deleted(event),
            EventType::CustomerTaxIdUpdated => self.on_event_customer_tax_id_updated(event),
            EventType::CustomerUpdated => self.on_event_customer_updated(event),
            EventType::FileCreated => self.on_event_file_created(event),
            EventType::IdentityVerificationSessionCanceled => {
                self.on_event_identity_verification_session_canceled(event)
            }
            EventType::IdentityVerificationSessionCreated => {
                self.on_event_identity_verification_session_created(event)
            }
            EventType::IdentityVerificationSessionProcessing => {
                self.on_event_identity_verification_session_processing(event)
            }
            EventType::IdentityVerificationSessionRedacted => {
                self.on_event_identity_verification_session_redacted(event)
            }
            EventType::IdentityVerificationSessionRequiresInput => {
                self.on_event_identity_verification_session_requires_input(event)
            }
            EventType::IdentityVerificationSessionVerified => {
                self.on_event_identity_verification_session_verified(event)
            }
            EventType::InvoiceCreated => self.on_event_invoice_created(event),
            EventType::InvoiceDeleted => self.on_event_invoice_deleted(event),
            EventType::InvoiceFinalizationFailed => {
                self.on_event_invoice_finalization_failed(event)
            }
            EventType::InvoiceFinalized => self.on_event_invoice_finalized(event),
            EventType::InvoiceMarkedUncollectible => {
                self.on_event_invoice_marked_uncollectible(event)
            }
            EventType::InvoicePaid => self.on_event_invoice_paid(event),
            EventType::InvoicePaymentActionRequired => {
                self.on_event_invoice_payment_action_required(event)
            }
            EventType::InvoicePaymentFailed => self.on_event_invoice_payment_failed(event),
            EventType::InvoicePaymentSucceeded => self.on_event_invoice_payment_succeeded(event),
            EventType::InvoiceSent => self.on_event_invoice_sent(event),
            EventType::InvoiceUpcoming => self.on_event_invoice_upcoming(event),
            EventType::InvoiceUpdated => self.on_event_invoice_updated(event),
            EventType::InvoiceVoided => self.on_event_invoice_voided(event),
            EventType::InvoiceItemCreated => self.on_event_invoice_item_created(event),
            EventType::InvoiceItemDeleted => self.on_event_invoice_item_deleted(event),
            EventType::InvoiceItemUpdated => self.on_event_invoice_item_updated(event),
            EventType::IssuingAuthorizationCreated => {
                self.on_event_issuing_authorization_created(event)
            }
            EventType::IssuingAuthorizationRequest => {
                self.on_event_issuing_authorization_request(event)
            }
            EventType::IssuingAuthorizationUpdated => {
                self.on_event_issuing_authorization_updated(event)
            }
            EventType::IssuingCardCreated => self.on_event_issuing_card_created(event),
            EventType::IssuingCardUpdated => self.on_event_issuing_card_updated(event),
            EventType::IssuingCardholderCreated => self.on_event_issuing_cardholder_created(event),
            EventType::IssuingCardholderUpdated => self.on_event_issuing_cardholder_updated(event),
            EventType::IssuingDisputeClosed => self.on_event_issuing_dispute_closed(event),
            EventType::IssuingDisputeCreated => self.on_event_issuing_dispute_created(event),
            EventType::IssuingDisputeFundsReinstated => {
                self.on_event_issuing_dispute_funds_reinstated(event)
            }
            EventType::IssuingDisputeSubmitted => self.on_event_issuing_dispute_submitted(event),
            EventType::IssuingDisputeUpdated => self.on_event_issuing_dispute_updated(event),
            EventType::IssuingTransactionCreated => {
                self.on_event_issuing_transaction_created(event)
            }
            EventType::IssuingTransactionUpdated => {
                self.on_event_issuing_transaction_updated(event)
            }
            EventType::MandateUpdated => self.on_event_mandate_updated(event),
            EventType::OrderCreated => self.on_event_order_created(event),
            EventType::OrderPaymentFailed => self.on_event_order_payment_failed(event),
            EventType::OrderPaymentSucceeded => self.on_event_order_payment_succeeded(event),
            EventType::OrderUpdated => self.on_event_order_updated(event),
            EventType::OrderReturnCreated => self.on_event_order_return_created(event),
            EventType::OrderReturnUpdated => self.on_event_order_return_updated(event),
            EventType::PaymentIntentAmountCapturableUpdated => {
                self.on_event_payment_intent_amount_capturable_updated(event)
            }
            EventType::PaymentIntentCanceled => self.on_event_payment_intent_canceled(event),
            EventType::PaymentIntentCreated => self.on_event_payment_intent_created(event),
            EventType::PaymentIntentPartiallyFunded => {
                self.on_event_payment_intent_partially_funded(event)
            }
            EventType::PaymentIntentPaymentFailed => {
                self.on_event_payment_intent_payment_failed(event)
            }
            EventType::PaymentIntentProcessing => self.on_event_payment_intent_processing(event),
            EventType::PaymentIntentRequiresAction => {
                self.on_event_payment_intent_requires_action(event)
            }
            EventType::PaymentIntentRequiresCapture => {
                self.on_event_payment_intent_requires_capture(event)
            }
            EventType::PaymentIntentSucceeded => self.on_event_payment_intent_succeeded(event),
            EventType::PaymentLinkCreated => self.on_event_payment_link_created(event),
            EventType::PaymentLinkUpdated => self.on_event_payment_link_updated(event),
            EventType::PaymentMethodAttached => self.on_event_payment_method_attached(event),
            EventType::PaymentMethodAutomaticallyUpdated => {
                self.on_event_payment_method_automatically_updated(event)
            }
            EventType::PaymentMethodDetached => self.on_event_payment_method_detached(event),
            EventType::PaymentMethodUpdated => self.on_event_payment_method_updated(event),
            EventType::PayoutCanceled => self.on_event_payout_canceled(event),
            EventType::PayoutCreated => self.on_event_payout_created(event),
            EventType::PayoutFailed => self.on_event_payout_failed(event),
            EventType::PayoutPaid => self.on_event_payout_paid(event),
            EventType::PayoutUpdated => self.on_event_payout_updated(event),
            EventType::PersonCreated => self.on_event_person_created(event),
            EventType::PersonDeleted => self.on_event_person_deleted(event),
            EventType::PersonUpdated => self.on_event_person_updated(event),
            EventType::PlanCreated => self.on_event_plan_created(event),
            EventType::PlanDeleted => self.on_event_plan_deleted(event),
            EventType::PlanUpdated => self.on_event_plan_updated(event),
            EventType::PriceCreated => self.on_event_price_created(event),
            EventType::PriceDeleted => self.on_event_price_deleted(event),
            EventType::PriceUpdated => self.on_event_price_updated(event),
            EventType::ProductCreated => self.on_event_product_created(event),
            EventType::ProductDeleted => self.on_event_product_deleted(event),
            EventType::ProductUpdated => self.on_event_product_updated(event),
            EventType::PromotionCodeCreated => self.on_event_promotion_code_created(event),
            EventType::PromotionCodeUpdated => self.on_event_promotion_code_updated(event),
            EventType::QuoteAccepted => self.on_event_quote_accepted(event),
            EventType::QuoteCanceled => self.on_event_quote_canceled(event),
            EventType::QuoteCreated => self.on_event_quote_created(event),
            EventType::QuoteFinalized => self.on_event_quote_finalized(event),
            EventType::RadarEarlyFraudWarningCreated => {
                self.on_event_radar_early_fraud_warning_created(event)
            }
            EventType::RadarEarlyFraudWarningUpdated => {
                self.on_event_radar_early_fraud_warning_updated(event)
            }
            EventType::RecipientCreated => self.on_event_recipient_created(event),
            EventType::RecipientDeleted => self.on_event_recipient_deleted(event),
            EventType::RecipientUpdated => self.on_event_recipient_updated(event),
            EventType::ReportingReportRunFailed => self.on_event_reporting_report_run_failed(event),
            EventType::ReportingReportRunSucceeded => {
                self.on_event_reporting_report_run_succeeded(event)
            }
            EventType::ReportingReportTypeUpdated => {
                self.on_event_reporting_report_type_updated(event)
            }
            EventType::ReviewClosed => self.on_event_review_closed(event),
            EventType::ReviewOpened => self.on_event_review_opened(event),
            EventType::SetupIntentCanceled => self.on_event_setup_intent_canceled(event),
            EventType::SetupIntentCreated => self.on_event_setup_intent_created(event),
            EventType::SetupIntentRequiresAction => {
                self.on_event_setup_intent_requires_action(event)
            }
            EventType::SetupIntentSetupFailed => self.on_event_setup_intent_setup_failed(event),
            EventType::SetupIntentSucceeded => self.on_event_setup_intent_succeeded(event),
            EventType::SigmaScheduledQueryRunCreated => {
                self.on_event_sigma_scheduled_query_run_created(event)
            }
            EventType::SkuCreated => self.on_event_sku_created(event),
            EventType::SkuDeleted => self.on_event_sku_deleted(event),
            EventType::SkuUpdated => self.on_event_sku_updated(event),
            EventType::SourceCanceled => self.on_event_source_canceled(event),
            EventType::SourceChargeable => self.on_event_source_chargeable(event),
            EventType::SourceFailed => self.on_event_source_failed(event),
            EventType::SourceMandateNotification => {
                self.on_event_source_mandate_notification(event)
            }
            EventType::SourceRefundAttributesRequired => {
                self.on_event_source_refund_attributes_required(event)
            }
            EventType::SourceTransactionCreated => self.on_event_source_transaction_created(event),
            EventType::SourceTransactionUpdated => self.on_event_source_transaction_updated(event),
            EventType::SubscriptionScheduleAborted => {
                self.on_event_subscription_schedule_aborted(event)
            }
            EventType::SubscriptionScheduleCanceled => {
                self.on_event_subscription_schedule_canceled(event)
            }
            EventType::SubscriptionScheduleCompleted => {
                self.on_event_subscription_schedule_completed(event)
            }
            EventType::SubscriptionScheduleCreated => {
                self.on_event_subscription_schedule_created(event)
            }
            EventType::SubscriptionScheduleExpiring => {
                self.on_event_subscription_schedule_expiring(event)
            }
            EventType::SubscriptionScheduleReleased => {
                self.on_event_subscription_schedule_released(event)
            }
            EventType::SubscriptionScheduleUpdated => {
                self.on_event_subscription_schedule_updated(event)
            }
            EventType::TaxRateCreated => self.on_event_tax_rate_created(event),
            EventType::TaxRateUpdated => self.on_event_tax_rate_updated(event),
            EventType::TerminalReaderActionFailed => {
                self.on_event_terminal_reader_action_failed(event)
            }
            EventType::TerminalReaderActionSucceeded => {
                self.on_event_terminal_reader_action_succeeded(event)
            }
            EventType::TestHelpersTestClockAdvancing => {
                self.on_event_test_helpers_test_clock_advancing(event)
            }
            EventType::TestHelpersTestClockCreated => {
                self.on_event_test_helpers_test_clock_created(event)
            }
            EventType::TestHelpersTestClockDeleted => {
                self.on_event_test_helpers_test_clock_deleted(event)
            }
            EventType::TestHelpersTestClockInternalFailure => {
                self.on_event_test_helpers_test_clock_internal_failure(event)
            }
            EventType::TestHelpersTestClockReady => {
                self.on_event_test_helpers_test_clock_ready(event)
            }
            EventType::TopupCanceled => self.on_event_topup_canceled(event),
            EventType::TopupCreated => self.on_event_topup_created(event),
            EventType::TopupFailed => self.on_event_topup_failed(event),
            EventType::TopupReversed => self.on_event_topup_reversed(event),
            EventType::TopupSucceeded => self.on_event_topup_succeeded(event),
            EventType::TransferCreated => self.on_event_transfer_created(event),
            EventType::TransferFailed => self.on_event_transfer_failed(event),
            EventType::TransferPaid => self.on_event_transfer_paid(event),
            EventType::TransferReversed => self.on_event_transfer_reversed(event),
            EventType::TransferUpdated => self.on_event_transfer_updated(event),
            EventType::Unknown => self.on_event_unknown(event),
        }
    }

    /// Stripe event `subscription_schedule.aborted`.
    fn on_event_subscription_schedule_aborted(&self, event: Event) -> BoxFut<LibResult<()>> {
        let schedule = match event.data.object {
            EventObject::SubscriptionSchedule(sub) => sub,
            _ => return self.on_unhandled_event(event),
        };
        let pool = self.pool();
        Box::pin(async move {
            let sub_id = schedule.id.as_str().to_owned();
            db::SubscriptionRow::inactivate_by_sub_id(&pool, &sub_id, true)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to update subscription",
                        anyhow!("stripe event failed to inactivate subscription: {e}"),
                    )
                })?;
            Ok(())
        })
    }

    /// Stripe event `subscription_schedule.completed`.
    fn on_event_subscription_schedule_completed(&self, event: Event) -> BoxFut<LibResult<()>> {
        let schedule = match event.data.object {
            EventObject::SubscriptionSchedule(sub) => sub,
            _ => return self.on_unhandled_event(event),
        };
        let pool = self.pool();
        Box::pin(async move {
            let sub_id = schedule.id.as_str().to_owned();
            db::SubscriptionRow::inactivate_by_sub_id(&pool, &sub_id, true)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to update subscription",
                        anyhow!("stripe event failed to inactivate subscription: {e}"),
                    )
                })?;
            Ok(())
        })
    }

    /// Stripe event `subscription_schedule.canceled`.
    fn on_event_subscription_schedule_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        let schedule = match event.data.object {
            EventObject::SubscriptionSchedule(sub) => sub,
            _ => return self.on_unhandled_event(event),
        };
        let pool = self.pool();
        Box::pin(async move {
            let customer_id = schedule.customer.id().as_str().to_owned();
            let row = db::BillingLinkRow::get_by_customer_id(&pool, &customer_id)
                .await
                .map_err(|e| {
                    LibError::database(
                        "Failed to get billing link",
                        anyhow!("stripe event failed to get billing link: {e}"),
                    )
                })?;

            let internal_id = match row {
                Some(row) => row.internal_id,
                None => {
                    tracing::warn!(
                        "Stripe webhook: no billing link found for customer {}",
                        customer_id
                    );
                    return Ok(());
                }
            };

            if let Err(err) = db::deactivate_subscription(pool, internal_id).await {
                if matches!(err.kind, ErrorKind::NotFound) {
                    tracing::warn!(
                        "Stripe webhook: subscription not found for internal id {}",
                        internal_id
                    );
                    return Ok(());
                }
                return Err(err);
            }

            Ok(())
        })
    }

    /// Stripe event `subscription_schedule.created`.
    fn on_event_subscription_schedule_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        let schedule = match event.data.object {
            EventObject::SubscriptionSchedule(sub) => sub,
            _ => return self.on_unhandled_event(event),
        };
        let pool = self.pool();
        Box::pin(async move {
            let get_billing_link = |customer_id: String| -> BoxFut<Option<Uuid>> {
                let pool = pool.clone();
                Box::pin(async move {
                    let row = db::BillingLinkRow::get_by_customer_id(&pool, &customer_id)
                        .await
                        .ok()
                        .flatten()?;
                    Some(row.internal_id)
                })
            };

            let update_subscription =
                |internal_id: Uuid, update: SubscriptionStateUpdate| -> BoxFut<LibResult<()>> {
                    let pool = pool.clone();
                    Box::pin(async move {
                        db::SubscriptionRow::update_by_internal_id(&pool, internal_id, update)
                            .await
                            .map_err(|e| {
                                LibError::database(
                                    "Failed to update subscription",
                                    anyhow!("stripe event failed to update subscription: {e}"),
                                )
                            })
                    })
                };

            crate::models::subscription_updated(schedule, get_billing_link, update_subscription)
                .await
        })
    }

    /// Stripe event `subscription_schedule.updated`.
    fn on_event_subscription_schedule_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_event_subscription_schedule_created(event)
    }

    /// Stripe event `account.application.authorized`.
    fn on_event_account_application_authorized(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `account.application.deauthorized`.
    fn on_event_account_application_deauthorized(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `account.external_account.created`.
    fn on_event_account_external_account_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `account.external_account.deleted`.
    fn on_event_account_external_account_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `account.external_account.updated`.
    fn on_event_account_external_account_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `account.updated`.
    fn on_event_account_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `application_fee.created`.
    fn on_event_application_fee_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `application_fee.refund.updated`.
    fn on_event_application_fee_refund_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `application_fee.refunded`.
    fn on_event_application_fee_refunded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `balance.available`.
    fn on_event_balance_available(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `billing_portal.configuration.created`.
    fn on_event_billing_portal_configuration_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `billing_portal.configuration.updated`.
    fn on_event_billing_portal_configuration_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `capability.updated`.
    fn on_event_capability_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `cash_balance.funds_available`.
    fn on_event_cash_balance_funds_available(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.captured`.
    fn on_event_charge_captured(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.dispute.closed`.
    fn on_event_charge_dispute_closed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.dispute.created`.
    fn on_event_charge_dispute_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.dispute.funds_reinstated`.
    fn on_event_charge_dispute_funds_reinstated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.dispute.funds_withdrawn`.
    fn on_event_charge_dispute_funds_withdrawn(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.dispute.updated`.
    fn on_event_charge_dispute_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.expired`.
    fn on_event_charge_expired(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.failed`.
    fn on_event_charge_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.pending`.
    fn on_event_charge_pending(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.refund.updated`.
    fn on_event_charge_refund_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.refunded`.
    fn on_event_charge_refunded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.succeeded`.
    fn on_event_charge_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `charge.updated`.
    fn on_event_charge_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `checkout.session.async_payment_failed`.
    fn on_event_checkout_session_async_payment_failed(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `checkout.session.async_payment_succeeded`.
    fn on_event_checkout_session_async_payment_succeeded(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `checkout.session.completed`.
    fn on_event_checkout_session_completed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `checkout.session.expired`.
    fn on_event_checkout_session_expired(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `coupon.created`.
    fn on_event_coupon_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `coupon.deleted`.
    fn on_event_coupon_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `coupon.updated`.
    fn on_event_coupon_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `credit_note.created`.
    fn on_event_credit_note_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `credit_note.updated`.
    fn on_event_credit_note_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `credit_note.voided`.
    fn on_event_credit_note_voided(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.created`.
    fn on_event_customer_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.deleted`.
    fn on_event_customer_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.discount.created`.
    fn on_event_customer_discount_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.discount.deleted`.
    fn on_event_customer_discount_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.discount.updated`.
    fn on_event_customer_discount_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.source.created`.
    fn on_event_customer_source_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.source.deleted`.
    fn on_event_customer_source_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.source.expiring`.
    fn on_event_customer_source_expiring(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.source.updated`.
    fn on_event_customer_source_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.created`.
    fn on_event_customer_subscription_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.deleted`.
    fn on_event_customer_subscription_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.paused`.
    fn on_event_customer_subscription_paused(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.pending_update_applied`.
    fn on_event_customer_subscription_pending_update_applied(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.pending_update_expired`.
    fn on_event_customer_subscription_pending_update_expired(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.resumed`.
    fn on_event_customer_subscription_resumed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.trial_will_end`.
    fn on_event_customer_subscription_trial_will_end(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.subscription.updated`.
    fn on_event_customer_subscription_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.tax_id.created`.
    fn on_event_customer_tax_id_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.tax_id.deleted`.
    fn on_event_customer_tax_id_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.tax_id.updated`.
    fn on_event_customer_tax_id_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `customer.updated`.
    fn on_event_customer_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `file.created`.
    fn on_event_file_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `identity.verification_session.canceled`.
    fn on_event_identity_verification_session_canceled(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `identity.verification_session.created`.
    fn on_event_identity_verification_session_created(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `identity.verification_session.processing`.
    fn on_event_identity_verification_session_processing(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `identity.verification_session.redacted`.
    fn on_event_identity_verification_session_redacted(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `identity.verification_session.requires_input`.
    fn on_event_identity_verification_session_requires_input(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `identity.verification_session.verified`.
    fn on_event_identity_verification_session_verified(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.created`.
    fn on_event_invoice_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.deleted`.
    fn on_event_invoice_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.finalization_failed`.
    fn on_event_invoice_finalization_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.finalized`.
    fn on_event_invoice_finalized(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.marked_uncollectible`.
    fn on_event_invoice_marked_uncollectible(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.paid`.
    fn on_event_invoice_paid(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.payment_action_required`.
    fn on_event_invoice_payment_action_required(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.payment_failed`.
    fn on_event_invoice_payment_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.payment_succeeded`.
    fn on_event_invoice_payment_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.sent`.
    fn on_event_invoice_sent(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.upcoming`.
    fn on_event_invoice_upcoming(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.updated`.
    fn on_event_invoice_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoice.voided`.
    fn on_event_invoice_voided(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoiceitem.created`.
    fn on_event_invoice_item_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoiceitem.deleted`.
    fn on_event_invoice_item_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `invoiceitem.updated`.
    fn on_event_invoice_item_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_authorization.created`.
    fn on_event_issuing_authorization_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_authorization.request`.
    fn on_event_issuing_authorization_request(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_authorization.updated`.
    fn on_event_issuing_authorization_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_card.created`.
    fn on_event_issuing_card_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_card.updated`.
    fn on_event_issuing_card_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_cardholder.created`.
    fn on_event_issuing_cardholder_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_cardholder.updated`.
    fn on_event_issuing_cardholder_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_dispute.closed`.
    fn on_event_issuing_dispute_closed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_dispute.created`.
    fn on_event_issuing_dispute_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_dispute.funds_reinstated`.
    fn on_event_issuing_dispute_funds_reinstated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_dispute.submitted`.
    fn on_event_issuing_dispute_submitted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_dispute.updated`.
    fn on_event_issuing_dispute_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_transaction.created`.
    fn on_event_issuing_transaction_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `issuing_transaction.updated`.
    fn on_event_issuing_transaction_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `mandate.updated`.
    fn on_event_mandate_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `order.created`.
    fn on_event_order_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `order.payment_failed`.
    fn on_event_order_payment_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `order.payment_succeeded`.
    fn on_event_order_payment_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `order.updated`.
    fn on_event_order_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `order_return.created`.
    fn on_event_order_return_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `order_return.updated`.
    fn on_event_order_return_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.amount_capturable_updated`.
    fn on_event_payment_intent_amount_capturable_updated(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.canceled`.
    fn on_event_payment_intent_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.created`.
    fn on_event_payment_intent_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.partially_funded`.
    fn on_event_payment_intent_partially_funded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.payment_failed`.
    fn on_event_payment_intent_payment_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.processing`.
    fn on_event_payment_intent_processing(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.requires_action`.
    fn on_event_payment_intent_requires_action(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.requires_capture`.
    fn on_event_payment_intent_requires_capture(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_intent.succeeded`.
    fn on_event_payment_intent_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_link.created`.
    fn on_event_payment_link_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_link.updated`.
    fn on_event_payment_link_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_method.attached`.
    fn on_event_payment_method_attached(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_method.automatically_updated`.
    fn on_event_payment_method_automatically_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_method.detached`.
    fn on_event_payment_method_detached(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payment_method.updated`.
    fn on_event_payment_method_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payout.canceled`.
    fn on_event_payout_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payout.created`.
    fn on_event_payout_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payout.failed`.
    fn on_event_payout_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payout.paid`.
    fn on_event_payout_paid(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `payout.updated`.
    fn on_event_payout_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `person.created`.
    fn on_event_person_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `person.deleted`.
    fn on_event_person_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `person.updated`.
    fn on_event_person_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `plan.created`.
    fn on_event_plan_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `plan.deleted`.
    fn on_event_plan_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `plan.updated`.
    fn on_event_plan_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `price.created`.
    fn on_event_price_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `price.deleted`.
    fn on_event_price_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `price.updated`.
    fn on_event_price_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `product.created`.
    fn on_event_product_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `product.deleted`.
    fn on_event_product_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `product.updated`.
    fn on_event_product_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `promotion_code.created`.
    fn on_event_promotion_code_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `promotion_code.updated`.
    fn on_event_promotion_code_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `quote.accepted`.
    fn on_event_quote_accepted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `quote.canceled`.
    fn on_event_quote_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `quote.created`.
    fn on_event_quote_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `quote.finalized`.
    fn on_event_quote_finalized(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `radar.early_fraud_warning.created`.
    fn on_event_radar_early_fraud_warning_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `radar.early_fraud_warning.updated`.
    fn on_event_radar_early_fraud_warning_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `recipient.created`.
    fn on_event_recipient_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `recipient.deleted`.
    fn on_event_recipient_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `recipient.updated`.
    fn on_event_recipient_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `reporting.report_run.failed`.
    fn on_event_reporting_report_run_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `reporting.report_run.succeeded`.
    fn on_event_reporting_report_run_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `reporting.report_type.updated`.
    fn on_event_reporting_report_type_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `review.closed`.
    fn on_event_review_closed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `review.opened`.
    fn on_event_review_opened(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `setup_intent.canceled`.
    fn on_event_setup_intent_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `setup_intent.created`.
    fn on_event_setup_intent_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `setup_intent.requires_action`.
    fn on_event_setup_intent_requires_action(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `setup_intent.setup_failed`.
    fn on_event_setup_intent_setup_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `setup_intent.succeeded`.
    fn on_event_setup_intent_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `sigma.scheduled_query_run.created`.
    fn on_event_sigma_scheduled_query_run_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `sku.created`.
    fn on_event_sku_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `sku.deleted`.
    fn on_event_sku_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `sku.updated`.
    fn on_event_sku_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.canceled`.
    fn on_event_source_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.chargeable`.
    fn on_event_source_chargeable(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.failed`.
    fn on_event_source_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.mandate_notification`.
    fn on_event_source_mandate_notification(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.refund_attributes_required`.
    fn on_event_source_refund_attributes_required(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.transaction.created`.
    fn on_event_source_transaction_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `source.transaction.updated`.
    fn on_event_source_transaction_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `subscription_schedule.expiring`.
    fn on_event_subscription_schedule_expiring(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `subscription_schedule.released`.
    fn on_event_subscription_schedule_released(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `tax_rate.created`.
    fn on_event_tax_rate_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `tax_rate.updated`.
    fn on_event_tax_rate_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `terminal.reader.action_failed`.
    fn on_event_terminal_reader_action_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `terminal.reader.action_succeeded`.
    fn on_event_terminal_reader_action_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `test_helpers.test_clock.advancing`.
    fn on_event_test_helpers_test_clock_advancing(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `test_helpers.test_clock.created`.
    fn on_event_test_helpers_test_clock_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `test_helpers.test_clock.deleted`.
    fn on_event_test_helpers_test_clock_deleted(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `test_helpers.test_clock.internal_failure`.
    fn on_event_test_helpers_test_clock_internal_failure(
        &self,
        event: Event,
    ) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `test_helpers.test_clock.ready`.
    fn on_event_test_helpers_test_clock_ready(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `topup.canceled`.
    fn on_event_topup_canceled(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `topup.created`.
    fn on_event_topup_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `topup.failed`.
    fn on_event_topup_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `topup.reversed`.
    fn on_event_topup_reversed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `topup.succeeded`.
    fn on_event_topup_succeeded(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `transfer.created`.
    fn on_event_transfer_created(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `transfer.failed`.
    fn on_event_transfer_failed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `transfer.paid`.
    fn on_event_transfer_paid(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `transfer.reversed`.
    fn on_event_transfer_reversed(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `transfer.updated`.
    fn on_event_transfer_updated(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Stripe event `unknown`.
    fn on_event_unknown(&self, event: Event) -> BoxFut<LibResult<()>> {
        self.on_unhandled_event(event)
    }

    /// Default handler used by all unimplemented event hooks.
    ///
    /// Override any of the `on_event_*` methods to handle specific events.
    fn on_unhandled_event(&self, _event: Event) -> BoxFut<LibResult<()>> {
        // Not implemented: override in your app if you need to handle this event.
        Box::pin(async { Ok(()) })
    }
}
