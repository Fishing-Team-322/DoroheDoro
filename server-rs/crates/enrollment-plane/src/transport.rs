use std::sync::Arc;

use async_nats::{Client, Subject, Subscriber};
use common::{
    nats_subjects::{
        AGENTS_DIAGNOSTICS, AGENTS_ENROLL_REQUEST, AGENTS_HEARTBEAT, AGENTS_POLICY_FETCH,
    },
    proto::{
        agent::{DiagnosticsPayload, EnrollRequest, FetchPolicyRequest, HeartbeatPayload},
        decode_message, encode_message, ok_envelope,
    },
    AppError,
};
use futures::StreamExt;
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::service::EnrollmentService;

pub async fn spawn_handlers(
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let enroll_sub = client.subscribe(AGENTS_ENROLL_REQUEST.to_string()).await?;
    let fetch_sub = client.subscribe(AGENTS_POLICY_FETCH.to_string()).await?;
    let heartbeat_sub = client.subscribe(AGENTS_HEARTBEAT.to_string()).await?;
    let diagnostics_sub = client.subscribe(AGENTS_DIAGNOSTICS.to_string()).await?;

    Ok(vec![
        tokio::spawn(run_enroll_handler(
            enroll_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_policy_handler(
            fetch_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_heartbeat_handler(
            heartbeat_sub,
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_diagnostics_handler(diagnostics_sub, service, shutdown)),
    ])
}

async fn run_enroll_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next_message = subscription.next() => next_message,
        };

        let Some(message) = message else {
            break;
        };

        let request = match decode_message::<EnrollRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        match service.enroll(request).await {
            Ok(response) => {
                let envelope = ok_envelope(&response, correlation_id.clone());
                send_reply(&client, &message.reply, envelope).await;
                info!(
                    correlation_id,
                    agent_id = response.agent_id,
                    "handled enrollment request"
                );
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn run_policy_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next_message = subscription.next() => next_message,
        };

        let Some(message) = message else {
            break;
        };

        let request = match decode_message::<FetchPolicyRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        match service.fetch_policy(request).await {
            Ok(response) => {
                let envelope = ok_envelope(&response, correlation_id.clone());
                send_reply(&client, &message.reply, envelope).await;
                info!(
                    correlation_id,
                    agent_id = response.agent_id,
                    "handled policy fetch request"
                );
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn run_heartbeat_handler(
    mut subscription: Subscriber,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next_message = subscription.next() => next_message,
        };

        let Some(message) = message else {
            break;
        };

        let payload = match decode_message::<HeartbeatPayload>(message.payload.as_ref()) {
            Ok(payload) => payload,
            Err(error) => {
                warn!(error = %error, "failed to decode heartbeat payload");
                continue;
            }
        };

        let agent_id = payload.agent_id.clone();
        if let Err(error) = service.record_heartbeat(payload).await {
            error!(agent_id, error_code = error.code().as_str(), error = %error, "failed to persist heartbeat");
            continue;
        }

        info!(agent_id, "persisted heartbeat");
    }
}

async fn run_diagnostics_handler(
    mut subscription: Subscriber,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next_message = subscription.next() => next_message,
        };

        let Some(message) = message else {
            break;
        };

        let payload = match decode_message::<DiagnosticsPayload>(message.payload.as_ref()) {
            Ok(payload) => payload,
            Err(error) => {
                warn!(error = %error, "failed to decode diagnostics payload");
                continue;
            }
        };

        let agent_id = payload.agent_id.clone();
        if let Err(error) = service.record_diagnostics(payload).await {
            error!(agent_id, error_code = error.code().as_str(), error = %error, "failed to persist diagnostics");
            continue;
        }

        info!(agent_id, "persisted diagnostics snapshot");
    }
}

async fn send_error_reply(
    client: &Client,
    reply_subject: &Option<Subject>,
    error: AppError,
    correlation_id: impl Into<String>,
) {
    let envelope = error.to_envelope(correlation_id.into());
    send_reply(client, reply_subject, envelope).await;
}

async fn send_reply(
    client: &Client,
    reply_subject: &Option<Subject>,
    envelope: common::proto::agent::AgentReplyEnvelope,
) {
    let Some(reply_subject) = reply_subject.clone() else {
        warn!("received request without reply subject");
        return;
    };

    if let Err(error) = client
        .publish(reply_subject, encode_message(&envelope).into())
        .await
    {
        error!(error = %error, "failed to publish NATS reply");
    }
}
