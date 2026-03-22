use std::{future::Future, sync::Arc, time::Duration};

use async_nats::{Client, Subject, Subscriber};
use common::{
    json::NormalizedLogEvent,
    nats_subjects::*,
    proto::{
        agent, alerts, decode_message, encode_message, ok_envelope, ok_json_envelope, query,
        runtime,
    },
    AppError, AppResult,
};
use futures::StreamExt;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tokio::{select, task::JoinHandle, time::interval};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::service::QueryAlertService;

pub async fn spawn_handlers(
    client: Client,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let mut tasks = Vec::new();

    tasks.push(tokio::spawn(run_proto_handler::<
        query::SearchLogsRequest,
        query::SearchLogsResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_SEARCH.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.search_logs(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::GetLogEventRequest,
        query::GetLogEventResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_GET.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.get_log_event(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::GetLogContextRequest,
        query::GetLogContextResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_CONTEXT.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.get_log_context(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::LogAnalyticsRequest,
        query::HistogramResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_HISTOGRAM.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.histogram(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::LogAnalyticsRequest,
        query::CountBucketsResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_SEVERITY.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.severity(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::LogAnalyticsRequest,
        query::CountBucketsResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_TOP_HOSTS.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.top_hosts(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::LogAnalyticsRequest,
        query::CountBucketsResponse,
        _,
        _,
    >(
        client
            .subscribe(QUERY_LOGS_TOP_SERVICES.to_string())
            .await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.top_services(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::LogAnalyticsRequest,
        query::HeatmapResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_HEATMAP.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.heatmap(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::LogAnalyticsRequest,
        query::TopPatternsResponse,
        _,
        _,
    >(
        client
            .subscribe(QUERY_LOGS_TOP_PATTERNS.to_string())
            .await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.top_patterns(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::ListLogAnomaliesRequest,
        query::ListLogAnomaliesResponse,
        _,
        _,
    >(
        client.subscribe(QUERY_LOGS_ANOMALIES.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.list_log_anomalies(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        query::DashboardOverviewRequest,
        query::DashboardOverviewResponse,
        _,
        _,
    >(
        client
            .subscribe(QUERY_DASHBOARDS_OVERVIEW.to_string())
            .await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.dashboard_overview(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        alerts::ListAlertInstancesRequest,
        alerts::ListAlertInstancesResponse,
        _,
        _,
    >(
        client.subscribe(ALERTS_LIST.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.list_alert_instances(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        alerts::GetAlertInstanceRequest,
        alerts::GetAlertInstanceResponse,
        _,
        _,
    >(
        client.subscribe(ALERTS_GET.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.get_alert_instance(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        alerts::ListAlertRulesRequest,
        alerts::ListAlertRulesResponse,
        _,
        _,
    >(
        client.subscribe(ALERTS_RULES_LIST.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.list_alert_rules(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        alerts::GetAlertRuleRequest,
        alerts::GetAlertRuleResponse,
        _,
        _,
    >(
        client.subscribe(ALERTS_RULES_GET.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.get_alert_rule(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        alerts::CreateAlertRuleRequest,
        alerts::AlertRuleMutationResponse,
        _,
        _,
    >(
        client.subscribe(ALERTS_RULES_CREATE.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.create_alert_rule(request).await },
    )));
    tasks.push(tokio::spawn(run_proto_handler::<
        alerts::UpdateAlertRuleRequest,
        alerts::AlertRuleMutationResponse,
        _,
        _,
    >(
        client.subscribe(ALERTS_RULES_UPDATE.to_string()).await?,
        client.clone(),
        service.clone(),
        shutdown.clone(),
        |service, request| async move { service.update_alert_rule(request).await },
    )));

    tasks.push(tokio::spawn(run_normalized_log_consumer(
        client.subscribe(LOGS_INGEST_NORMALIZED.to_string()).await?,
        service.clone(),
        shutdown.clone(),
    )));
    tasks.push(tokio::spawn(run_agent_heartbeat_consumer(
        client.subscribe(AGENTS_HEARTBEAT.to_string()).await?,
        service.clone(),
        shutdown.clone(),
    )));
    tasks.push(tokio::spawn(run_agent_diagnostics_consumer(
        client.subscribe(AGENTS_DIAGNOSTICS.to_string()).await?,
        service.clone(),
        shutdown.clone(),
    )));
    tasks.push(tokio::spawn(run_security_posture_consumer(
        client
            .subscribe(SECURITY_POSTURE_REPORTS.to_string())
            .await?,
        service.clone(),
        shutdown.clone(),
    )));
    tasks.push(tokio::spawn(run_alert_resolution_loop(
        service.clone(),
        shutdown.clone(),
    )));
    tasks.push(tokio::spawn(run_anomaly_evaluator_loop(
        service.clone(),
        shutdown.clone(),
    )));

    Ok(tasks)
}

async fn run_proto_handler<Req, Res, F, Fut>(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
    handler: F,
) where
    Req: prost::Message + Default + DeserializeOwned + Send + 'static,
    Res: prost::Message + Serialize + Send + 'static,
    F: Fn(Arc<QueryAlertService>, Req) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Res, AppError>> + Send,
{
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next = subscription.next() => {
                let Some(message) = next else { break; };
                message
            }
        };

        let (request, wants_json) = match decode_message::<Req>(message.payload.as_ref()) {
            Ok(request) => (request, false),
            Err(proto_error) => match serde_json::from_slice::<Req>(message.payload.as_ref()) {
                Ok(request) => (request, true),
                Err(_) => {
                    send_reply(&client, &message.reply, proto_error.to_envelope("")).await;
                    continue;
                }
            },
        };

        match handler(service.clone(), request).await {
            Ok(response) => {
                let envelope = if wants_json {
                    match ok_json_envelope(&response, "") {
                        Ok(envelope) => envelope,
                        Err(error) => {
                            send_reply(&client, &message.reply, error.to_envelope("")).await;
                            continue;
                        }
                    }
                } else {
                    ok_envelope(&response, "")
                };
                send_reply(&client, &message.reply, envelope).await;
            }
            Err(error) => {
                send_reply(&client, &message.reply, error.to_envelope("")).await;
            }
        }
    }
}

async fn run_normalized_log_consumer(
    mut subscription: Subscriber,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next = subscription.next() => {
                let Some(message) = next else { break; };
                message
            }
        };

        let event = match serde_json::from_slice::<NormalizedLogEvent>(message.payload.as_ref()) {
            Ok(event) => event,
            Err(error) => {
                warn!(error = %error, "failed to decode normalized log event");
                continue;
            }
        };

        if let Err(error) = service.handle_normalized_event(event).await {
            error!(error_code = error.code().as_str(), error = %error, "failed to evaluate alert rules");
        }
    }
}

async fn run_agent_heartbeat_consumer(
    mut subscription: Subscriber,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next = subscription.next() => {
                let Some(message) = next else { break; };
                message
            }
        };

        match decode_message::<agent::HeartbeatPayload>(message.payload.as_ref()) {
            Ok(payload) => {
                if let Err(error) = service.handle_heartbeat_payload(payload).await {
                    error!(error_code = error.code().as_str(), error = %error, "failed to process heartbeat signal");
                }
            }
            Err(proto_error) => {
                let payload = match serde_json::from_slice::<Value>(message.payload.as_ref()) {
                    Ok(payload) => payload,
                    Err(_) => {
                        warn!(error = %proto_error, "failed to decode heartbeat payload");
                        continue;
                    }
                };

                if let Err(error) = service.handle_heartbeat_signal(payload).await {
                    error!(error_code = error.code().as_str(), error = %error, "failed to process heartbeat signal");
                }
            }
        }
    }
}

async fn run_agent_diagnostics_consumer(
    mut subscription: Subscriber,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next = subscription.next() => {
                let Some(message) = next else { break; };
                message
            }
        };

        match decode_message::<agent::DiagnosticsPayload>(message.payload.as_ref()) {
            Ok(payload) => {
                if let Err(error) = service.handle_diagnostics_payload(payload).await {
                    error!(error_code = error.code().as_str(), error = %error, "failed to process diagnostics signal");
                }
            }
            Err(proto_error) => {
                let payload = match serde_json::from_slice::<Value>(message.payload.as_ref()) {
                    Ok(payload) => payload,
                    Err(_) => {
                        warn!(error = %proto_error, "failed to decode diagnostics payload");
                        continue;
                    }
                };

                if let Err(error) = service.handle_diagnostics_signal(payload).await {
                    error!(error_code = error.code().as_str(), error = %error, "failed to process diagnostics signal");
                }
            }
        }
    }
}

async fn run_security_posture_consumer(
    subscription: Subscriber,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
) {
    consume_json_stream(
        subscription,
        service,
        shutdown,
        |service, payload| async move { service.handle_security_signal(payload).await },
    )
    .await;
}

async fn consume_json_stream<F, Fut>(
    mut subscription: Subscriber,
    service: Arc<QueryAlertService>,
    shutdown: CancellationToken,
    handler: F,
) where
    F: Fn(Arc<QueryAlertService>, Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AppResult<()>> + Send,
{
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next = subscription.next() => {
                let Some(message) = next else { break; };
                message
            }
        };

        let payload = match serde_json::from_slice::<Value>(message.payload.as_ref()) {
            Ok(payload) => payload,
            Err(error) => {
                warn!(error = %error, "failed to decode signal payload");
                continue;
            }
        };

        if let Err(error) = handler(service.clone(), payload).await {
            error!(error_code = error.code().as_str(), error = %error, "failed to process signal");
        }
    }
}

async fn run_alert_resolution_loop(service: Arc<QueryAlertService>, shutdown: CancellationToken) {
    let mut ticker = interval(Duration::from_secs(60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                if let Err(error) = service.resolve_stale_alerts().await {
                    error!(error_code = error.code().as_str(), error = %error, "failed to resolve stale alerts");
                } else {
                    info!("query-alert-plane stale alert reconciliation tick completed");
                }
            }
        }
    }
}

async fn run_anomaly_evaluator_loop(service: Arc<QueryAlertService>, shutdown: CancellationToken) {
    let mut ticker = interval(service.anomaly_interval());
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        select! {
            _ = shutdown.cancelled() => break,
            _ = ticker.tick() => {
                if let Err(error) = service.evaluate_anomaly_rules().await {
                    error!(error_code = error.code().as_str(), error = %error, "failed to evaluate anomaly rules");
                } else {
                    info!("query-alert-plane anomaly evaluation tick completed");
                }
            }
        }
    }
}

async fn send_reply(
    client: &Client,
    reply_subject: &Option<Subject>,
    envelope: runtime::RuntimeReplyEnvelope,
) {
    let Some(reply_subject) = reply_subject.clone() else {
        warn!("received request without reply subject");
        return;
    };

    if let Err(error) = client
        .publish(reply_subject, encode_message(&envelope).into())
        .await
    {
        error!(error = %error, "failed to publish query-alert reply");
    }
}
