use std::{sync::Arc, time::Duration};

use anyhow::Context;
use async_nats::{Client, Subscriber};
use chrono::{DateTime, SecondsFormat, Utc};
use common::{
    json::{
        AuditAppendEvent, NotificationEnvelopeV1, OperatorStatus, TelegramDispatchRequestedEvent,
        TelegramDispatchResultEvent, TelegramHealthcheckRequestV1, TelegramHealthcheckResultV1,
    },
    nats_subjects::{
        AUDIT_EVENTS_APPEND, NOTIFICATIONS_DISPATCH_REQUESTED,
        NOTIFICATIONS_TELEGRAM_DISPATCH_FAILED, NOTIFICATIONS_TELEGRAM_DISPATCH_REQUESTED,
        NOTIFICATIONS_TELEGRAM_DISPATCH_SUCCEEDED, NOTIFICATIONS_TELEGRAM_HEALTHCHECK_REQUESTED,
        NOTIFICATIONS_TELEGRAM_HEALTHCHECK_RESULT,
    },
    AppError,
};
use futures::StreamExt;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::{
    select,
    task::JoinHandle,
    time::{interval, sleep, Instant, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    config::TelegramRuntimeConfig,
    telegram::{
        client::{TelegramBotClient, TelegramSendFailure, TelegramSendOutcome},
        normalize_telegram_config, sanitize_telegram_config_value, telegram_binding_matches,
        TelegramIntegrationConfig,
    },
};

use super::repository::{
    BatchCompletion, DeliveryStatusUpdate, HealthcheckCompletion, NewTelegramDelivery,
    NewTelegramDeliveryAttempt, NewTelegramHealthcheckRun, TelegramDeliveryRecord,
    TelegramHealthcheckRunRecord, TelegramRepository,
};
use super::vault::read_telegram_secret;

const SCHEMA_VERSION_V1: &str = "v1";
const SOURCE_COMPONENT: &str = "control-plane.telegram";
const DELIVERY_LEASE_FACTOR: u32 = 3;
const DEFAULT_DETAILS_PATH: &str = "/alerts/";
const MAX_MESSAGE_CHARS: usize = 3_200;

#[derive(Clone)]
struct TelegramRuntimeContext {
    repo: Arc<TelegramRepository>,
    nats: Client,
    config: TelegramRuntimeConfig,
    bot_client: TelegramBotClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeliveryProcessingResult {
    Success,
    RetryScheduled,
    DeadLetter,
}

pub async fn spawn_runtime(
    pool: PgPool,
    nats: Client,
    config: TelegramRuntimeConfig,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    if !config.worker_enabled {
        info!("telegram runtime worker is disabled");
        return Ok(Vec::new());
    }

    let dispatch_subscription = nats
        .subscribe(NOTIFICATIONS_DISPATCH_REQUESTED.to_string())
        .await
        .context("subscribe notifications.dispatch.requested.v1")?;
    let healthcheck_subscription = nats
        .subscribe(NOTIFICATIONS_TELEGRAM_HEALTHCHECK_REQUESTED.to_string())
        .await
        .context("subscribe notifications.telegram.healthcheck.requested.v1")?;
    let bot_client = TelegramBotClient::from_config(&config)
        .context("build telegram bot api client from runtime config")?;

    let ctx = TelegramRuntimeContext {
        repo: Arc::new(TelegramRepository::new(pool)),
        nats,
        config,
        bot_client,
    };
    let ctx = Arc::new(ctx);

    Ok(vec![
        tokio::spawn(run_dispatch_router(
            ctx.clone(),
            dispatch_subscription,
            shutdown.clone(),
        )),
        tokio::spawn(run_delivery_worker(ctx.clone(), shutdown.clone())),
        tokio::spawn(run_healthcheck_worker(
            ctx,
            healthcheck_subscription,
            shutdown,
        )),
    ])
}

async fn run_dispatch_router(
    ctx: Arc<TelegramRuntimeContext>,
    mut subscription: Subscriber,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            message = subscription.next() => {
                let Some(message) = message else { break; };
                message
            }
        };

        let envelope = match serde_json::from_slice::<NotificationEnvelopeV1>(
            message.payload.as_ref(),
        ) {
            Ok(envelope) => envelope,
            Err(error) => {
                warn!(error = %error, "failed to decode notification envelope for telegram routing");
                continue;
            }
        };

        if let Err(error) = route_notification(&ctx, envelope).await {
            error!(error = %error, "failed to route notification into telegram delivery queue");
        }
    }
}

async fn route_notification(
    ctx: &Arc<TelegramRuntimeContext>,
    envelope: NotificationEnvelopeV1,
) -> anyhow::Result<()> {
    if envelope.schema_version.trim() != SCHEMA_VERSION_V1 {
        warn!(
            schema_version = %envelope.schema_version,
            notification_id = %envelope.notification_id,
            "ignoring unsupported notification envelope schema version",
        );
        return Ok(());
    }

    if envelope.notification_id.trim().is_empty() {
        warn!("ignoring notification envelope without notification_id");
        return Ok(());
    }

    let cluster_id = parse_optional_uuid(&envelope.cluster_id);
    let routes = ctx
        .repo
        .list_matching_routes(cluster_id)
        .await
        .context("list telegram delivery routes")?;
    if routes.is_empty() {
        return Ok(());
    }

    let normalized_event_type = envelope.event_type.trim().to_ascii_lowercase();
    let normalized_severity = normalize_severity_fallback(&envelope.severity);
    let created_at = parse_timestamp(&envelope.created_at).unwrap_or_else(Utc::now);
    let details_url = derive_details_url(&ctx.config, &envelope);

    for route in routes {
        let config = match TelegramIntegrationConfig::from_value(
            &route.config_json,
            &route.integration_name,
        ) {
            Ok(config) => config,
            Err(error) => {
                warn!(
                    integration_id = %route.integration_id,
                    binding_id = %route.binding_id,
                    error = %error,
                    "telegram integration config is invalid; notification route rejected",
                );
                publish_audit_best_effort(
                    &ctx.nats,
                    "telegram.delivery.route_rejected",
                    "integration",
                    &route.integration_id.to_string(),
                    &envelope.correlation_id,
                    "telegram route rejected because integration config is invalid",
                    "system",
                    "system",
                    json!({
                        "integration_binding_id": route.binding_id,
                        "notification_id": &envelope.notification_id,
                        "error": error.to_string(),
                        "safe_config_preview": sanitize_telegram_config_value(&route.config_json, &route.integration_name),
                    }),
                    created_at,
                )
                .await;
                continue;
            }
        };

        if !config.delivery_enabled {
            continue;
        }
        if !telegram_binding_matches(
            &route.event_types_json,
            &route.severity_threshold,
            &normalized_event_type,
            &normalized_severity,
        ) {
            continue;
        }

        let delivery = NewTelegramDelivery {
            id: Uuid::new_v4(),
            integration_id: route.integration_id,
            integration_binding_id: route.binding_id,
            notification_id: envelope.notification_id.clone(),
            dedup_key: build_dedup_key(route.binding_id, &envelope.notification_id),
            event_type: normalized_event_type.clone(),
            cluster_id,
            cluster_name: non_empty_or(
                envelope.cluster_name.as_str(),
                fallback_cluster_name(cluster_id.as_ref()),
            ),
            severity: normalized_severity.clone(),
            title: non_empty_or(&envelope.title, &normalized_event_type),
            summary: non_empty_or(&envelope.summary, "no summary provided"),
            entity_kind: non_empty_or(&envelope.entity_kind, "notification"),
            entity_id: build_identifier(&envelope),
            details_url: details_url.clone(),
            telegram_chat_id: config.default_chat_id.clone().unwrap_or_default(),
            parse_mode: config.parse_mode.clone(),
            message_text: render_message(&envelope, &details_url, &config.parse_mode),
            notification_json: serde_json::to_value(&envelope)
                .context("serialize notification envelope to json")?,
            max_attempts: i32::try_from(ctx.config.max_attempts.max(1)).unwrap_or(i32::MAX),
            next_attempt_at: Utc::now(),
            correlation_id: envelope.correlation_id.clone(),
            created_at,
        };

        if let Some(inserted) = ctx
            .repo
            .insert_delivery_if_absent(delivery)
            .await
            .context("insert telegram delivery")?
        {
            publish_dispatch_requested_best_effort(&ctx.nats, &inserted).await;
            publish_audit_best_effort(
                &ctx.nats,
                "telegram.delivery.queued",
                "telegram_delivery",
                &inserted.id.to_string(),
                &inserted.correlation_id,
                "telegram delivery queued",
                "system",
                "system",
                json!({
                    "notification_id": inserted.notification_id,
                    "integration_id": inserted.integration_id,
                    "integration_binding_id": inserted.integration_binding_id,
                    "event_type": inserted.event_type,
                    "cluster_id": inserted.cluster_id,
                    "severity": inserted.severity,
                }),
                inserted.created_at,
            )
            .await;
        }
    }

    Ok(())
}

async fn run_delivery_worker(ctx: Arc<TelegramRuntimeContext>, shutdown: CancellationToken) {
    let mut ticker = interval(ctx.config.poll_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                if let Err(error) = process_due_deliveries(&ctx).await {
                    error!(error = %error, "telegram delivery worker cycle failed");
                }
            }
        }
    }
}

async fn process_due_deliveries(ctx: &Arc<TelegramRuntimeContext>) -> anyhow::Result<()> {
    let lease_token = Uuid::new_v4().to_string();
    let deliveries = ctx
        .repo
        .lease_due_deliveries(
            ctx.config.batch_size.max(1),
            &lease_token,
            delivery_lease_ms(&ctx.config),
        )
        .await
        .context("lease telegram deliveries")?;
    if deliveries.is_empty() {
        return Ok(());
    }

    let batch_id = Uuid::new_v4();
    let started_at = Utc::now();
    let started_instant = Instant::now();
    let correlation_id = deliveries
        .first()
        .map(|delivery| delivery.correlation_id.clone())
        .unwrap_or_else(|| batch_id.to_string());
    ctx.repo
        .start_batch(
            batch_id,
            &correlation_id,
            deliveries.len() as u32,
            started_at,
        )
        .await
        .context("start telegram delivery batch")?;

    let mut stats = BatchCompletion {
        success_count: 0,
        retryable_failure_count: 0,
        permanent_failure_count: 0,
        dead_letter_count: 0,
        completed_at: started_at,
        duration_ms: 0,
    };

    for (index, delivery) in deliveries.into_iter().enumerate() {
        if index > 0 && !ctx.config.min_send_interval.is_zero() {
            sleep(ctx.config.min_send_interval).await;
        }

        match execute_delivery(ctx, &delivery, Some(batch_id)).await {
            Ok(DeliveryProcessingResult::Success) => {
                stats.success_count += 1;
            }
            Ok(DeliveryProcessingResult::RetryScheduled) => {
                stats.retryable_failure_count += 1;
            }
            Ok(DeliveryProcessingResult::DeadLetter) => {
                stats.permanent_failure_count += 1;
                stats.dead_letter_count += 1;
            }
            Err(error) => {
                error!(
                    delivery_id = %delivery.id,
                    error = %error,
                    "telegram delivery execution failed before classification",
                );
            }
        }
    }

    stats.completed_at = Utc::now();
    stats.duration_ms = i64::try_from(started_instant.elapsed().as_millis()).unwrap_or(i64::MAX);
    ctx.repo
        .finish_batch(batch_id, stats)
        .await
        .context("finish telegram delivery batch")?;

    Ok(())
}

async fn execute_delivery(
    ctx: &Arc<TelegramRuntimeContext>,
    delivery: &TelegramDeliveryRecord,
    batch_id: Option<Uuid>,
) -> anyhow::Result<DeliveryProcessingResult> {
    let attempt_number = delivery.attempt_count.saturating_add(1);
    let started = Instant::now();
    let completed_at = Utc::now();

    let target_state = ctx
        .repo
        .get_delivery_target_state(delivery.integration_id, delivery.integration_binding_id)
        .await
        .context("load telegram delivery target state")?;
    let Some(target_state) = target_state else {
        return finalize_delivery_without_send(
            ctx,
            delivery,
            batch_id,
            attempt_number,
            completed_at,
            &TelegramSendFailure {
                classification: "permanent".to_string(),
                status_code: "integration_not_found".to_string(),
                status_message: "telegram integration no longer exists".to_string(),
                status_severity: "error".to_string(),
                suggested_action:
                    "Delete the orphaned binding or recreate the telegram integration instance."
                        .to_string(),
                retry_after_seconds: None,
                http_status: None,
            },
            started.elapsed(),
        )
        .await;
    };

    if !target_state.integration_is_active || !target_state.binding_is_active {
        return finalize_delivery_without_send(
            ctx,
            delivery,
            batch_id,
            attempt_number,
            completed_at,
            &TelegramSendFailure {
                classification: "permanent".to_string(),
                status_code: "delivery_disabled".to_string(),
                status_message:
                    "telegram delivery skipped because the integration or binding is inactive"
                        .to_string(),
                status_severity: "warning".to_string(),
                suggested_action:
                    "Re-enable the telegram integration and its binding before retrying."
                        .to_string(),
                retry_after_seconds: None,
                http_status: None,
            },
            started.elapsed(),
        )
        .await;
    }

    let normalized_config = match normalize_telegram_config(
        "telegram_bot",
        &target_state.integration_name,
        &target_state.config_json,
    ) {
        Ok(config) => config,
        Err(error) => {
            return finalize_delivery_without_send(
                ctx,
                delivery,
                batch_id,
                attempt_number,
                completed_at,
                &TelegramSendFailure {
                    classification: "invalid_configuration".to_string(),
                    status_code: "integration_config_invalid".to_string(),
                    status_message: error.to_string(),
                    status_severity: "error".to_string(),
                    suggested_action:
                        "Fix the telegram integration config_json and retry the delivery."
                            .to_string(),
                    retry_after_seconds: None,
                    http_status: None,
                },
                started.elapsed(),
            )
            .await;
        }
    };
    let telegram_config = match TelegramIntegrationConfig::from_value(
        &normalized_config,
        &target_state.integration_name,
    ) {
        Ok(config) => config,
        Err(error) => {
            return finalize_delivery_without_send(
                ctx,
                delivery,
                batch_id,
                attempt_number,
                completed_at,
                &TelegramSendFailure {
                    classification: "invalid_configuration".to_string(),
                    status_code: "integration_config_invalid".to_string(),
                    status_message: error.to_string(),
                    status_severity: "error".to_string(),
                    suggested_action:
                        "Fix the telegram integration config_json and retry the delivery."
                            .to_string(),
                    retry_after_seconds: None,
                    http_status: None,
                },
                started.elapsed(),
            )
            .await;
        }
    };

    if !telegram_config.delivery_enabled {
        return finalize_delivery_without_send(
            ctx,
            delivery,
            batch_id,
            attempt_number,
            completed_at,
            &TelegramSendFailure {
                classification: "permanent".to_string(),
                status_code: "delivery_disabled".to_string(),
                status_message: "telegram delivery is disabled for this integration".to_string(),
                status_severity: "warning".to_string(),
                suggested_action:
                    "Set config_json.delivery_enabled=true before retrying telegram delivery."
                        .to_string(),
                retry_after_seconds: None,
                http_status: None,
            },
            started.elapsed(),
        )
        .await;
    }

    let chat_id = resolve_chat_id(delivery.telegram_chat_id.as_str(), &telegram_config);
    let bot_token = match load_bot_token(ctx, &telegram_config).await {
        Ok(token) => token,
        Err(failure) => {
            return finalize_delivery_without_send(
                ctx,
                delivery,
                batch_id,
                attempt_number,
                completed_at,
                &failure,
                started.elapsed(),
            )
            .await;
        }
    };

    let send_outcome = ctx
        .bot_client
        .send_message(
            &bot_token,
            &chat_id,
            &delivery.message_text,
            &delivery.parse_mode,
        )
        .await;
    let duration = started.elapsed();

    match send_outcome {
        TelegramSendOutcome::Success(success) => {
            let attempt_id = record_attempt(
                ctx,
                delivery,
                batch_id,
                attempt_number,
                "success",
                None,
                "",
                None,
                duration,
                &success.status_code,
                &success.status_message,
                "info",
                "",
            )
            .await?;
            let status_update = DeliveryStatusUpdate {
                status_code: &success.status_code,
                status_message: &success.status_message,
                status_severity: "info",
                source_component: SOURCE_COMPONENT,
                suggested_action: "",
            };
            ctx.repo
                .mark_delivery_succeeded(
                    delivery.id,
                    attempt_number as u32,
                    completed_at,
                    &success.telegram_message_id,
                    status_update,
                )
                .await
                .context("mark telegram delivery as succeeded")?;

            publish_dispatch_result_best_effort(
                &ctx.nats,
                delivery,
                attempt_id,
                attempt_number as u32,
                "success",
                "delivered",
                &success.telegram_message_id,
                "",
                build_operator_status(
                    &success.status_code,
                    &success.status_message,
                    "info",
                    &delivery.correlation_id,
                    "",
                    completed_at,
                ),
                true,
            )
            .await;
            publish_audit_best_effort(
                &ctx.nats,
                "telegram.delivery.succeeded",
                "telegram_delivery",
                &delivery.id.to_string(),
                &delivery.correlation_id,
                "telegram delivery succeeded",
                "system",
                "system",
                json!({
                    "notification_id": &delivery.notification_id,
                    "attempt_id": attempt_id,
                    "attempt_number": attempt_number,
                    "telegram_message_id": &success.telegram_message_id,
                }),
                completed_at,
            )
            .await;

            Ok(DeliveryProcessingResult::Success)
        }
        TelegramSendOutcome::Failure(failure) => {
            finalize_delivery_without_send(
                ctx,
                delivery,
                batch_id,
                attempt_number,
                completed_at,
                &failure,
                duration,
            )
            .await
        }
    }
}

#[derive(Debug, Clone)]
struct HealthcheckCompletionOwned {
    resolved_chat_id: String,
    status: String,
    classification: String,
    telegram_message_id: String,
    status_code: String,
    status_message: String,
    status_severity: String,
    suggested_action: String,
    delivery_status: String,
    completed_at: DateTime<Utc>,
}

impl HealthcheckCompletionOwned {
    fn as_repo_completion(&self) -> HealthcheckCompletion<'_> {
        HealthcheckCompletion {
            resolved_chat_id: &self.resolved_chat_id,
            status: &self.status,
            classification: &self.classification,
            telegram_message_id: &self.telegram_message_id,
            status_code: &self.status_code,
            status_message: &self.status_message,
            status_severity: &self.status_severity,
            source_component: SOURCE_COMPONENT,
            suggested_action: &self.suggested_action,
            completed_at: self.completed_at,
        }
    }
}

async fn finalize_delivery_without_send(
    ctx: &Arc<TelegramRuntimeContext>,
    delivery: &TelegramDeliveryRecord,
    batch_id: Option<Uuid>,
    attempt_number: i32,
    completed_at: DateTime<Utc>,
    failure: &TelegramSendFailure,
    duration: Duration,
) -> anyhow::Result<DeliveryProcessingResult> {
    let retry_at = if failure.is_retryable()
        && attempt_number < delivery.max_attempts
        && failure.classification != "invalid_configuration"
    {
        Some(
            completed_at
                + chrono::Duration::from_std(retry_delay(
                    attempt_number as u32,
                    failure.retry_after_seconds,
                    ctx.config.poll_interval,
                ))?,
        )
    } else {
        None
    };
    let attempt_id = record_attempt(
        ctx,
        delivery,
        batch_id,
        attempt_number,
        &failure.classification,
        failure.http_status.map(i32::from),
        &failure.status_code,
        failure
            .retry_after_seconds
            .map(|value| i32::try_from(value).unwrap_or(i32::MAX)),
        duration,
        &failure.status_code,
        &failure.status_message,
        &failure.status_severity,
        &failure.suggested_action,
    )
    .await?;

    if let Some(retry_at) = retry_at {
        let status = DeliveryStatusUpdate {
            status_code: &failure.status_code,
            status_message: &failure.status_message,
            status_severity: &failure.status_severity,
            source_component: SOURCE_COMPONENT,
            suggested_action: &failure.suggested_action,
        };
        ctx.repo
            .mark_delivery_retryable_failure(
                delivery.id,
                attempt_number as u32,
                retry_at,
                completed_at,
                status,
            )
            .await
            .context("mark telegram delivery as retry_pending")?;

        publish_dispatch_result_best_effort(
            &ctx.nats,
            delivery,
            attempt_id,
            attempt_number as u32,
            &failure.classification,
            "retry_pending",
            "",
            &format_timestamp(retry_at),
            build_operator_status(
                &failure.status_code,
                &failure.status_message,
                &failure.status_severity,
                &delivery.correlation_id,
                &failure.suggested_action,
                completed_at,
            ),
            false,
        )
        .await;
        publish_audit_best_effort(
            &ctx.nats,
            "telegram.delivery.retry_scheduled",
            "telegram_delivery",
            &delivery.id.to_string(),
            &delivery.correlation_id,
            "telegram delivery scheduled for retry",
            "system",
            "system",
            json!({
                "attempt_id": attempt_id,
                "attempt_number": attempt_number,
                "classification": failure.classification,
                "retry_at": format_timestamp(retry_at),
                "status_code": failure.status_code,
            }),
            completed_at,
        )
        .await;
        return Ok(DeliveryProcessingResult::RetryScheduled);
    }

    let status = DeliveryStatusUpdate {
        status_code: &failure.status_code,
        status_message: &failure.status_message,
        status_severity: &failure.status_severity,
        source_component: SOURCE_COMPONENT,
        suggested_action: &failure.suggested_action,
    };
    ctx.repo
        .mark_delivery_dead_letter(delivery.id, attempt_number as u32, completed_at, status)
        .await
        .context("mark telegram delivery as dead_letter")?;

    publish_dispatch_result_best_effort(
        &ctx.nats,
        delivery,
        attempt_id,
        attempt_number as u32,
        &failure.classification,
        "dead_letter",
        "",
        "",
        build_operator_status(
            &failure.status_code,
            &failure.status_message,
            &failure.status_severity,
            &delivery.correlation_id,
            &failure.suggested_action,
            completed_at,
        ),
        false,
    )
    .await;
    publish_audit_best_effort(
        &ctx.nats,
        "telegram.delivery.dead_lettered",
        "telegram_delivery",
        &delivery.id.to_string(),
        &delivery.correlation_id,
        "telegram delivery exhausted retries or hit a permanent failure",
        "system",
        "system",
        json!({
            "attempt_id": attempt_id,
            "attempt_number": attempt_number,
            "classification": failure.classification,
            "status_code": failure.status_code,
        }),
        completed_at,
    )
    .await;

    Ok(DeliveryProcessingResult::DeadLetter)
}

async fn run_healthcheck_worker(
    ctx: Arc<TelegramRuntimeContext>,
    mut subscription: Subscriber,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            message = subscription.next() => {
                let Some(message) = message else { break; };
                message
            }
        };

        let request = match serde_json::from_slice::<TelegramHealthcheckRequestV1>(
            message.payload.as_ref(),
        ) {
            Ok(request) => request,
            Err(error) => {
                warn!(error = %error, "failed to decode telegram healthcheck request");
                continue;
            }
        };

        if let Err(error) = handle_healthcheck(&ctx, request).await {
            error!(error = %error, "failed to handle telegram healthcheck request");
        }
    }
}

async fn handle_healthcheck(
    ctx: &Arc<TelegramRuntimeContext>,
    request: TelegramHealthcheckRequestV1,
) -> anyhow::Result<()> {
    if request.schema_version.trim() != SCHEMA_VERSION_V1 {
        warn!(
            schema_version = %request.schema_version,
            request_id = %request.request_id,
            "ignoring unsupported telegram healthcheck schema version",
        );
        return Ok(());
    }

    let integration_id = Uuid::parse_str(&request.integration_id)
        .map_err(|_| AppError::invalid_argument("integration_id must be a valid UUID"))?;
    let created_at = parse_timestamp(&request.created_at).unwrap_or_else(Utc::now);
    let requested_run_id = Uuid::new_v4();
    let run = ctx
        .repo
        .create_healthcheck_run(NewTelegramHealthcheckRun {
            id: requested_run_id,
            request_id: request.request_id.clone(),
            integration_id,
            chat_id_override: request.chat_id_override.trim().to_string(),
            correlation_id: request.correlation_id.clone(),
            created_at,
        })
        .await
        .context("create telegram healthcheck run")?;

    if run.id != requested_run_id {
        if run.completed_at.is_some() {
            publish_healthcheck_result_best_effort(
                &ctx.nats,
                &run,
                effective_healthcheck_delivery_status(&run),
                build_operator_status(
                    &run.status_code,
                    &run.status_message,
                    &run.status_severity,
                    &run.correlation_id,
                    &run.suggested_action,
                    run.completed_at.unwrap_or(run.updated_at),
                ),
            )
            .await;
        } else {
            info!(
                request_id = %request.request_id,
                healthcheck_run_id = %run.id,
                "telegram healthcheck request already exists and is still running",
            );
        }
        return Ok(());
    }

    let actor_id = non_empty_or(&request.actor_id, "system");
    let actor_type = non_empty_or(&request.actor_type, "system");
    publish_audit_best_effort(
        &ctx.nats,
        "telegram.healthcheck.requested",
        "telegram_healthcheck",
        &run.id.to_string(),
        &request.correlation_id,
        "telegram healthcheck requested",
        &actor_id,
        &actor_type,
        json!({
            "integration_id": run.integration_id,
            "request_id": &run.request_id,
            "chat_id_override": &request.chat_id_override,
        }),
        created_at,
    )
    .await;

    let completion = perform_healthcheck(ctx, &request, &run).await;
    ctx.repo
        .complete_healthcheck_run(run.id, completion.as_repo_completion())
        .await
        .context("complete telegram healthcheck run")?;

    let completed_run = TelegramHealthcheckRunRecord {
        completed_at: Some(completion.completed_at),
        resolved_chat_id: completion.resolved_chat_id.clone(),
        status: completion.status.clone(),
        classification: completion.classification.clone(),
        telegram_message_id: completion.telegram_message_id.clone(),
        status_code: completion.status_code.clone(),
        status_message: completion.status_message.clone(),
        status_severity: completion.status_severity.clone(),
        source_component: SOURCE_COMPONENT.to_string(),
        suggested_action: completion.suggested_action.clone(),
        updated_at: completion.completed_at,
        ..run.clone()
    };

    publish_healthcheck_result_best_effort(
        &ctx.nats,
        &completed_run,
        &completion.delivery_status,
        build_operator_status(
            &completion.status_code,
            &completion.status_message,
            &completion.status_severity,
            &request.correlation_id,
            &completion.suggested_action,
            completion.completed_at,
        ),
    )
    .await;
    publish_audit_best_effort(
        &ctx.nats,
        if completion.delivery_status == "delivered" {
            "telegram.healthcheck.succeeded"
        } else {
            "telegram.healthcheck.failed"
        },
        "telegram_healthcheck",
        &run.id.to_string(),
        &request.correlation_id,
        "telegram healthcheck completed",
        &actor_id,
        &actor_type,
        json!({
            "integration_id": run.integration_id,
            "request_id": &run.request_id,
            "classification": &completion.classification,
            "delivery_status": &completion.delivery_status,
            "resolved_chat_id": &completion.resolved_chat_id,
            "telegram_message_id": &completion.telegram_message_id,
        }),
        completion.completed_at,
    )
    .await;

    Ok(())
}

async fn perform_healthcheck(
    ctx: &Arc<TelegramRuntimeContext>,
    request: &TelegramHealthcheckRequestV1,
    run: &TelegramHealthcheckRunRecord,
) -> HealthcheckCompletionOwned {
    let completed_at = Utc::now();
    let integration = match ctx.repo.get_telegram_integration(run.integration_id).await {
        Ok(Some(integration)) => integration,
        Ok(None) => {
            return HealthcheckCompletionOwned {
                resolved_chat_id: String::new(),
                status: "failed".to_string(),
                classification: "permanent".to_string(),
                telegram_message_id: String::new(),
                status_code: "integration_not_found".to_string(),
                status_message: "telegram integration not found".to_string(),
                status_severity: "error".to_string(),
                suggested_action:
                    "Recreate the telegram integration instance and retry the healthcheck."
                        .to_string(),
                delivery_status: "failed".to_string(),
                completed_at,
            };
        }
        Err(error) => {
            warn!(error = %error, integration_id = %run.integration_id, "failed to load telegram integration for healthcheck");
            return HealthcheckCompletionOwned {
                resolved_chat_id: String::new(),
                status: "failed".to_string(),
                classification: "retryable".to_string(),
                telegram_message_id: String::new(),
                status_code: "integration_lookup_failed".to_string(),
                status_message: "failed to load telegram integration state".to_string(),
                status_severity: "warning".to_string(),
                suggested_action:
                    "Retry the healthcheck; if repeated, inspect control-plane database connectivity."
                        .to_string(),
                delivery_status: "failed".to_string(),
                completed_at,
            };
        }
    };

    if !integration.integration_is_active {
        return HealthcheckCompletionOwned {
            resolved_chat_id: String::new(),
            status: "failed".to_string(),
            classification: "invalid_configuration".to_string(),
            telegram_message_id: String::new(),
            status_code: "delivery_disabled".to_string(),
            status_message: "telegram integration is inactive".to_string(),
            status_severity: "warning".to_string(),
            suggested_action: "Enable the telegram integration before retrying the healthcheck."
                .to_string(),
            delivery_status: "failed".to_string(),
            completed_at,
        };
    }

    let normalized_config = match normalize_telegram_config(
        "telegram_bot",
        &integration.integration_name,
        &integration.config_json,
    ) {
        Ok(value) => value,
        Err(error) => {
            return HealthcheckCompletionOwned {
                resolved_chat_id: String::new(),
                status: "failed".to_string(),
                classification: "invalid_configuration".to_string(),
                telegram_message_id: String::new(),
                status_code: "integration_config_invalid".to_string(),
                status_message: error.to_string(),
                status_severity: "error".to_string(),
                suggested_action:
                    "Fix the telegram integration config_json and retry the healthcheck."
                        .to_string(),
                delivery_status: "failed".to_string(),
                completed_at,
            };
        }
    };
    let telegram_config = match TelegramIntegrationConfig::from_value(
        &normalized_config,
        &integration.integration_name,
    ) {
        Ok(config) => config,
        Err(error) => {
            return HealthcheckCompletionOwned {
                resolved_chat_id: String::new(),
                status: "failed".to_string(),
                classification: "invalid_configuration".to_string(),
                telegram_message_id: String::new(),
                status_code: "integration_config_invalid".to_string(),
                status_message: error.to_string(),
                status_severity: "error".to_string(),
                suggested_action:
                    "Fix the telegram integration config_json and retry the healthcheck."
                        .to_string(),
                delivery_status: "failed".to_string(),
                completed_at,
            };
        }
    };

    let resolved_chat_id = if !request.chat_id_override.trim().is_empty() {
        request.chat_id_override.trim().to_string()
    } else {
        telegram_config.default_chat_id.clone().unwrap_or_default()
    };
    if resolved_chat_id.is_empty() {
        return HealthcheckCompletionOwned {
            resolved_chat_id,
            status: "failed".to_string(),
            classification: "invalid_configuration".to_string(),
            telegram_message_id: String::new(),
            status_code: "telegram_bot.default_chat_id".to_string(),
            status_message:
                "telegram healthcheck requires a chat_id override or default_chat_id".to_string(),
            status_severity: "error".to_string(),
            suggested_action:
                "Set config_json.default_chat_id or provide chat_id_override in the healthcheck request."
                    .to_string(),
            delivery_status: "failed".to_string(),
            completed_at,
        };
    }

    let bot_token = match load_bot_token(ctx, &telegram_config).await {
        Ok(token) => token,
        Err(failure) => {
            return HealthcheckCompletionOwned {
                resolved_chat_id,
                status: "failed".to_string(),
                classification: failure.classification,
                telegram_message_id: String::new(),
                status_code: failure.status_code,
                status_message: failure.status_message,
                status_severity: failure.status_severity,
                suggested_action: failure.suggested_action,
                delivery_status: "failed".to_string(),
                completed_at,
            };
        }
    };

    let test_message = render_healthcheck_message(&integration.integration_name, &resolved_chat_id);
    match ctx
        .bot_client
        .send_message(
            &bot_token,
            &resolved_chat_id,
            &test_message,
            &telegram_config.parse_mode,
        )
        .await
    {
        TelegramSendOutcome::Success(success) => HealthcheckCompletionOwned {
            resolved_chat_id,
            status: "succeeded".to_string(),
            classification: "success".to_string(),
            telegram_message_id: success.telegram_message_id,
            status_code: success.status_code,
            status_message: success.status_message,
            status_severity: "info".to_string(),
            suggested_action: String::new(),
            delivery_status: "delivered".to_string(),
            completed_at,
        },
        TelegramSendOutcome::Failure(failure) => HealthcheckCompletionOwned {
            resolved_chat_id,
            status: "failed".to_string(),
            classification: failure.classification,
            telegram_message_id: String::new(),
            status_code: failure.status_code,
            status_message: failure.status_message,
            status_severity: failure.status_severity,
            suggested_action: failure.suggested_action,
            delivery_status: "failed".to_string(),
            completed_at,
        },
    }
}

async fn load_bot_token(
    ctx: &Arc<TelegramRuntimeContext>,
    config: &TelegramIntegrationConfig,
) -> Result<String, TelegramSendFailure> {
    let Some(vault) = ctx.config.vault.as_ref() else {
        return Err(TelegramSendFailure {
            classification: "invalid_configuration".to_string(),
            status_code: "vault_unconfigured".to_string(),
            status_message: "Vault runtime configuration is missing".to_string(),
            status_severity: "error".to_string(),
            suggested_action:
                "Configure VAULT_ADDR, VAULT_ROLE_ID and VAULT_SECRET_ID for control-plane."
                    .to_string(),
            retry_after_seconds: None,
            http_status: None,
        });
    };

    read_telegram_secret(vault, &config.secret_ref)
        .await
        .map(|secret| secret.bot_token)
        .map_err(classify_vault_error)
}

async fn record_attempt(
    ctx: &Arc<TelegramRuntimeContext>,
    delivery: &TelegramDeliveryRecord,
    batch_id: Option<Uuid>,
    attempt_number: i32,
    classification: &str,
    http_status: Option<i32>,
    telegram_error_code: &str,
    retry_after_seconds: Option<i32>,
    duration: Duration,
    status_code: &str,
    status_message: &str,
    status_severity: &str,
    suggested_action: &str,
) -> anyhow::Result<Uuid> {
    let attempt_id = Uuid::new_v4();
    ctx.repo
        .record_attempt(NewTelegramDeliveryAttempt {
            id: attempt_id,
            delivery_id: delivery.id,
            batch_id,
            attempt_number,
            classification: classification.to_string(),
            http_status,
            telegram_error_code: telegram_error_code.to_string(),
            retry_after_seconds,
            duration_ms: i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
            status_code: status_code.to_string(),
            status_message: status_message.to_string(),
            status_severity: status_severity.to_string(),
            source_component: SOURCE_COMPONENT.to_string(),
            suggested_action: suggested_action.to_string(),
            correlation_id: delivery.correlation_id.clone(),
            created_at: Utc::now(),
        })
        .await
        .context("record telegram delivery attempt")
}

fn classify_vault_error(error: AppError) -> TelegramSendFailure {
    let message = error.to_string();
    if message.contains("missing telegram bot token material")
        || message.contains("did not return a kv-like data object")
    {
        return TelegramSendFailure {
            classification: "invalid_configuration".to_string(),
            status_code: "vault_secret_invalid".to_string(),
            status_message: message,
            status_severity: "error".to_string(),
            suggested_action:
                "Fix the Vault secret content so it exposes bot_token/token/telegram_token."
                    .to_string(),
            retry_after_seconds: None,
            http_status: None,
        };
    }

    TelegramSendFailure {
        classification: "retryable".to_string(),
        status_code: "vault_unavailable".to_string(),
        status_message: message,
        status_severity: "warning".to_string(),
        suggested_action:
            "Retry the operation; if repeated, inspect Vault connectivity and AppRole credentials."
                .to_string(),
        retry_after_seconds: None,
        http_status: None,
    }
}

async fn publish_dispatch_requested_best_effort(
    client: &Client,
    delivery: &TelegramDeliveryRecord,
) {
    let payload = TelegramDispatchRequestedEvent {
        schema_version: SCHEMA_VERSION_V1.to_string(),
        delivery_id: delivery.id.to_string(),
        notification_id: delivery.notification_id.clone(),
        integration_id: delivery.integration_id.to_string(),
        integration_binding_id: delivery.integration_binding_id.to_string(),
        event_type: delivery.event_type.clone(),
        cluster_id: delivery
            .cluster_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        created_at: format_timestamp(delivery.created_at),
        correlation_id: delivery.correlation_id.clone(),
        status: build_operator_status(
            "queued",
            "telegram delivery queued",
            "info",
            &delivery.correlation_id,
            "",
            delivery.created_at,
        ),
    };
    publish_json_best_effort(client, NOTIFICATIONS_TELEGRAM_DISPATCH_REQUESTED, &payload).await;
}

async fn publish_dispatch_result_best_effort(
    client: &Client,
    delivery: &TelegramDeliveryRecord,
    attempt_id: Uuid,
    attempt_number: u32,
    classification: &str,
    delivery_status: &str,
    telegram_message_id: &str,
    retry_at: &str,
    status: OperatorStatus,
    success: bool,
) {
    let payload = TelegramDispatchResultEvent {
        schema_version: SCHEMA_VERSION_V1.to_string(),
        delivery_id: delivery.id.to_string(),
        notification_id: delivery.notification_id.clone(),
        integration_id: delivery.integration_id.to_string(),
        integration_binding_id: delivery.integration_binding_id.to_string(),
        attempt_id: attempt_id.to_string(),
        attempt_number,
        classification: classification.to_string(),
        delivery_status: delivery_status.to_string(),
        telegram_message_id: telegram_message_id.to_string(),
        retry_at: retry_at.to_string(),
        created_at: status.created_at.clone(),
        correlation_id: delivery.correlation_id.clone(),
        status,
    };
    let subject = if success {
        NOTIFICATIONS_TELEGRAM_DISPATCH_SUCCEEDED
    } else {
        NOTIFICATIONS_TELEGRAM_DISPATCH_FAILED
    };
    publish_json_best_effort(client, subject, &payload).await;
}

async fn publish_healthcheck_result_best_effort(
    client: &Client,
    run: &TelegramHealthcheckRunRecord,
    delivery_status: &str,
    status: OperatorStatus,
) {
    let payload = TelegramHealthcheckResultV1 {
        schema_version: SCHEMA_VERSION_V1.to_string(),
        request_id: run.request_id.clone(),
        healthcheck_run_id: run.id.to_string(),
        integration_id: run.integration_id.to_string(),
        resolved_chat_id: run.resolved_chat_id.clone(),
        classification: run.classification.clone(),
        delivery_status: delivery_status.to_string(),
        telegram_message_id: run.telegram_message_id.clone(),
        created_at: status.created_at.clone(),
        correlation_id: run.correlation_id.clone(),
        status,
    };
    publish_json_best_effort(client, NOTIFICATIONS_TELEGRAM_HEALTHCHECK_RESULT, &payload).await;
}

async fn publish_json_best_effort<T: Serialize>(client: &Client, subject: &str, payload: &T) {
    let body = match serde_json::to_vec(payload) {
        Ok(body) => body,
        Err(error) => {
            error!(subject, error = %error, "failed to serialize telegram runtime event");
            return;
        }
    };

    if let Err(error) = client.publish(subject.to_string(), body.into()).await {
        error!(subject, error = %error, "failed to publish telegram runtime event");
    }
}

async fn publish_audit_best_effort(
    client: &Client,
    event_type: &str,
    entity_type: &str,
    entity_id: &str,
    request_id: &str,
    reason: &str,
    actor_id: &str,
    actor_type: &str,
    payload_json: serde_json::Value,
    created_at: DateTime<Utc>,
) {
    let event = AuditAppendEvent {
        event_type: event_type.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        actor_id: actor_id.to_string(),
        actor_type: actor_type.to_string(),
        request_id: request_id.to_string(),
        reason: reason.to_string(),
        payload_json,
        created_at: Some(format_timestamp(created_at)),
    };
    publish_json_best_effort(client, AUDIT_EVENTS_APPEND, &event).await;
}

fn build_operator_status(
    code: &str,
    message: &str,
    severity: &str,
    correlation_id: &str,
    suggested_action: &str,
    created_at: DateTime<Utc>,
) -> OperatorStatus {
    OperatorStatus {
        code: code.to_string(),
        message: message.to_string(),
        severity: severity.to_string(),
        source_component: SOURCE_COMPONENT.to_string(),
        created_at: format_timestamp(created_at),
        correlation_id: correlation_id.to_string(),
        suggested_action: suggested_action.to_string(),
    }
}

fn render_message(
    envelope: &NotificationEnvelopeV1,
    details_url: &str,
    parse_mode: &str,
) -> String {
    let cluster_name = non_empty_or(
        &envelope.cluster_name,
        fallback_cluster_name(parse_optional_uuid(&envelope.cluster_id).as_ref()),
    );
    let identifier = build_identifier(envelope);
    let mut lines = vec![
        format!("Cluster: {cluster_name}"),
        format!(
            "Severity: {}",
            normalize_severity_fallback(&envelope.severity)
        ),
        format!("Event: {}", envelope.event_type.trim().to_ascii_lowercase()),
        format!("Title: {}", non_empty_or(&envelope.title, "notification")),
        format!(
            "Summary: {}",
            non_empty_or(&envelope.summary, "no summary provided")
        ),
        format!("Identifier: {identifier}"),
    ];
    if !envelope.host.trim().is_empty() {
        lines.push(format!("Host: {}", envelope.host.trim()));
    }
    if !envelope.service.trim().is_empty() {
        lines.push(format!("Service: {}", envelope.service.trim()));
    }
    if !details_url.trim().is_empty() {
        lines.push(format!("Link: {}", details_url.trim()));
    }

    let raw = truncate_text(&lines.join("\n"), MAX_MESSAGE_CHARS);
    if parse_mode.eq_ignore_ascii_case("HTML") {
        escape_html(&raw)
    } else {
        raw
    }
}

fn render_healthcheck_message(integration_name: &str, chat_id: &str) -> String {
    truncate_text(
        &format!(
            "Telegram integration healthcheck\nIntegration: {}\nChat: {}\nGenerated at: {}",
            integration_name.trim(),
            chat_id.trim(),
            format_timestamp(Utc::now()),
        ),
        MAX_MESSAGE_CHARS,
    )
}

fn resolve_chat_id(existing_delivery_chat_id: &str, config: &TelegramIntegrationConfig) -> String {
    let chat_id = existing_delivery_chat_id.trim();
    if !chat_id.is_empty() {
        return chat_id.to_string();
    }
    config.default_chat_id.clone().unwrap_or_default()
}

fn build_dedup_key(binding_id: Uuid, notification_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(binding_id.as_bytes());
    hasher.update(b":");
    hasher.update(notification_id.trim().as_bytes());
    hex::encode(hasher.finalize())
}

fn retry_delay(
    attempt_number: u32,
    retry_after_seconds: Option<u32>,
    poll_interval: Duration,
) -> Duration {
    if let Some(retry_after_seconds) = retry_after_seconds {
        return Duration::from_secs(u64::from(retry_after_seconds.max(1)));
    }

    let exponent = attempt_number.saturating_sub(1).min(5);
    let seconds = 2_u64.pow(exponent).max(1);
    Duration::from_secs(seconds.max(poll_interval.as_secs().max(1)))
}

fn delivery_lease_ms(config: &TelegramRuntimeConfig) -> i64 {
    let poll = i64::try_from(config.poll_interval.as_millis()).unwrap_or(i64::MAX / 2);
    let timeout = i64::try_from(config.request_timeout.as_millis()).unwrap_or(i64::MAX / 2);
    poll.max(timeout)
        .saturating_mul(i64::from(DELIVERY_LEASE_FACTOR))
}

fn parse_optional_uuid(raw: &str) -> Option<Uuid> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Uuid::parse_str(trimmed).ok()
    }
}

fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn format_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn build_identifier(envelope: &NotificationEnvelopeV1) -> String {
    if !envelope.entity_kind.trim().is_empty() && !envelope.entity_id.trim().is_empty() {
        format!(
            "{}/{}",
            envelope.entity_kind.trim(),
            envelope.entity_id.trim()
        )
    } else if !envelope.entity_id.trim().is_empty() {
        envelope.entity_id.trim().to_string()
    } else {
        envelope.notification_id.trim().to_string()
    }
}

fn derive_details_url(config: &TelegramRuntimeConfig, envelope: &NotificationEnvelopeV1) -> String {
    if !envelope.details_url.trim().is_empty() {
        return envelope.details_url.trim().to_string();
    }
    let Some(edge_public_url) = config.edge_public_url.as_deref() else {
        return String::new();
    };
    if envelope.notification_id.trim().is_empty() {
        return String::new();
    }
    format!(
        "{}{}{}",
        edge_public_url.trim_end_matches('/'),
        DEFAULT_DETAILS_PATH,
        envelope.notification_id.trim(),
    )
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn normalize_severity_fallback(severity: &str) -> String {
    super::integration::normalize_delivery_severity(severity)
        .unwrap_or_else(|_| severity.trim().to_ascii_lowercase())
}

fn non_empty_or(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn fallback_cluster_name(cluster_id: Option<&Uuid>) -> &'static str {
    if cluster_id.is_some() {
        "cluster"
    } else {
        "global"
    }
}

fn effective_healthcheck_delivery_status(run: &TelegramHealthcheckRunRecord) -> &'static str {
    if run.status == "succeeded" {
        "delivered"
    } else {
        "failed"
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use serde_json::json;

    use super::{build_dedup_key, derive_details_url, render_message, retry_delay, truncate_text};
    use crate::config::TelegramRuntimeConfig;
    use common::json::NotificationEnvelopeV1;

    fn sample_envelope() -> NotificationEnvelopeV1 {
        serde_json::from_value(json!({
            "schema_version": "v1",
            "notification_id": "notif-1",
            "correlation_id": "corr-1",
            "created_at": Utc::now().to_rfc3339(),
            "event_type": "alerts.firing",
            "severity": "warning",
            "source_component": "query-alert-plane",
            "cluster_id": "",
            "cluster_name": "prod-eu",
            "title": "CPU high",
            "summary": "load crossed 95%",
            "entity_kind": "alert",
            "entity_id": "alert-42",
            "host": "node-1",
            "service": "kernel",
            "fingerprint": "fp",
            "details_url": "",
            "labels": {}
        }))
        .unwrap()
    }

    #[test]
    fn retry_delay_prefers_retry_after() {
        assert_eq!(
            retry_delay(3, Some(37), Duration::from_secs(2)),
            Duration::from_secs(37)
        );
    }

    #[test]
    fn dedup_key_is_stable() {
        let binding = uuid::Uuid::nil();
        assert_eq!(
            build_dedup_key(binding, "notif-1"),
            build_dedup_key(binding, "notif-1")
        );
        assert_ne!(
            build_dedup_key(binding, "notif-1"),
            build_dedup_key(binding, "notif-2")
        );
    }

    #[test]
    fn rendered_message_contains_required_fields() {
        let message = render_message(
            &sample_envelope(),
            "https://edge.example.local/alerts/notif-1",
            "plain",
        );

        assert!(message.contains("Cluster: prod-eu"));
        assert!(message.contains("Severity: medium"));
        assert!(message.contains("Title: CPU high"));
        assert!(message.contains("Summary: load crossed 95%"));
        assert!(message.contains("Identifier: alert/alert-42"));
        assert!(message.contains("Link: https://edge.example.local/alerts/notif-1"));
    }

    #[test]
    fn derives_default_details_url() {
        let config = TelegramRuntimeConfig {
            worker_enabled: true,
            api_base_url: "https://api.telegram.org".to_string(),
            request_timeout: Duration::from_secs(5),
            min_send_interval: Duration::from_millis(250),
            poll_interval: Duration::from_secs(2),
            batch_size: 10,
            max_attempts: 4,
            edge_public_url: Some("https://edge.example.local".to_string()),
            vault: None,
        };

        assert_eq!(
            derive_details_url(&config, &sample_envelope()),
            "https://edge.example.local/alerts/notif-1"
        );
    }

    #[test]
    fn truncate_text_appends_ellipsis() {
        let long = "x".repeat(20);
        assert_eq!(truncate_text(&long, 8), "xxxxx...");
    }
}
