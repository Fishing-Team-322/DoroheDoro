use std::sync::Arc;

use async_nats::{Client, Subject, Subscriber};
use common::{
    nats_subjects::{
        AGENTS_BOOTSTRAP_TOKEN_ISSUE, AGENTS_DIAGNOSTICS, AGENTS_DIAGNOSTICS_GET,
        AGENTS_ENROLL_REQUEST, AGENTS_GET, AGENTS_HEARTBEAT, AGENTS_LIST, AGENTS_POLICY_FETCH,
        AGENTS_POLICY_GET,
    },
    proto::{
        agent::{
            DiagnosticsPayload, EnrollRequest, FetchPolicyRequest, GetAgentDiagnosticsRequest,
            GetAgentPolicyRequest, GetAgentRequest, HeartbeatPayload, IssueBootstrapTokenRequest,
            ListAgentsRequest,
        },
        decode_message, encode_message, ok_envelope, runtime,
    },
    AppError,
};
use futures::StreamExt;
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::service::{EnrollmentService, ListInput};

pub async fn spawn_handlers(
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let enroll_sub = client.subscribe(AGENTS_ENROLL_REQUEST.to_string()).await?;
    let fetch_sub = client.subscribe(AGENTS_POLICY_FETCH.to_string()).await?;
    let issue_bootstrap_token_sub = client
        .subscribe(AGENTS_BOOTSTRAP_TOKEN_ISSUE.to_string())
        .await?;
    let heartbeat_sub = client.subscribe(AGENTS_HEARTBEAT.to_string()).await?;
    let diagnostics_sub = client.subscribe(AGENTS_DIAGNOSTICS.to_string()).await?;
    let agents_list_sub = client.subscribe(AGENTS_LIST.to_string()).await?;
    let agents_get_sub = client.subscribe(AGENTS_GET.to_string()).await?;
    let diagnostics_get_sub = client.subscribe(AGENTS_DIAGNOSTICS_GET.to_string()).await?;
    let policy_get_sub = client.subscribe(AGENTS_POLICY_GET.to_string()).await?;

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
        tokio::spawn(run_issue_bootstrap_token_handler(
            issue_bootstrap_token_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_heartbeat_handler(
            heartbeat_sub,
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_diagnostics_handler(
            diagnostics_sub,
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_agents_list_handler(
            agents_list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_agents_get_handler(
            agents_get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_diagnostics_get_handler(
            diagnostics_get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_policy_get_handler(
            policy_get_sub,
            client,
            service,
            shutdown,
        )),
    ])
}

macro_rules! handle_request {
    ($subscription:ident, $shutdown:ident) => {
        select! {
            _ = $shutdown.cancelled() => break,
            next_message = $subscription.next() => {
                let Some(message) = next_message else { break; };
                message
            }
        }
    };
}

async fn run_issue_bootstrap_token_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<IssueBootstrapTokenRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        match service.issue_bootstrap_token(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await;
                info!(
                    correlation_id,
                    token_id = response.token_id,
                    "handled bootstrap token issue request"
                );
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn run_enroll_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

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
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await;
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
        let message = handle_request!(subscription, shutdown);

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
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await;
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
        let message = handle_request!(subscription, shutdown);

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
        let message = handle_request!(subscription, shutdown);

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

async fn run_agents_list_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<ListAgentsRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let list = ListInput::from_proto(request.paging);
        match service.list_agents(&list).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_agents_get_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<GetAgentRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        match service.get_agent(&request.agent_id).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await;
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn run_diagnostics_get_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<GetAgentDiagnosticsRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        match service.latest_diagnostics(&request.agent_id).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await;
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn run_policy_get_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<EnrollmentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<GetAgentPolicyRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        match service.get_agent_policy(&request.agent_id).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await;
            }
            Err(error) => {
                send_error_reply(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn send_ok_reply<T>(
    client: &Client,
    reply_subject: &Option<Subject>,
    payload: &T,
    correlation_id: &str,
) where
    T: prost::Message,
{
    send_reply(client, reply_subject, ok_envelope(payload, correlation_id)).await;
}

async fn send_error_reply(
    client: &Client,
    reply_subject: &Option<Subject>,
    error: AppError,
    correlation_id: impl Into<String>,
) {
    send_reply(client, reply_subject, error.to_envelope(correlation_id)).await;
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
        error!(error = %error, "failed to publish NATS reply");
    }
}
