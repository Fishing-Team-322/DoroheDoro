use std::sync::Arc;

use async_nats::{Client, Subject, Subscriber};
use common::{
    nats_subjects::*,
    proto::{decode_message, deployment, encode_message, ok_envelope, runtime},
    AppError,
};
use futures::StreamExt;
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::service::DeploymentService;

pub async fn spawn_handlers(
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let create_sub = client
        .subscribe(DEPLOYMENTS_JOBS_CREATE.to_string())
        .await?;
    let get_sub = client.subscribe(DEPLOYMENTS_JOBS_GET.to_string()).await?;
    let list_sub = client.subscribe(DEPLOYMENTS_JOBS_LIST.to_string()).await?;
    let retry_sub = client.subscribe(DEPLOYMENTS_JOBS_RETRY.to_string()).await?;
    let cancel_sub = client
        .subscribe(DEPLOYMENTS_JOBS_CANCEL.to_string())
        .await?;
    let plan_sub = client
        .subscribe(DEPLOYMENTS_PLAN_CREATE.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_create_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_list_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_retry_handler(
            retry_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_cancel_handler(
            cancel_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_plan_handler(plan_sub, client, service, shutdown)),
    ])
}

macro_rules! next_message {
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

async fn run_create_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = next_message!(subscription, shutdown);
        let request = match decode_message::<deployment::CreateDeploymentJobRequest>(
            message.payload.as_ref(),
        ) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };
        let correlation_id = request.correlation_id.clone();
        match service.create_job(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await
            }
            Err(error) => send_error_reply(&client, &message.reply, error, correlation_id).await,
        }
    }
}

async fn run_get_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = next_message!(subscription, shutdown);
        let request =
            match decode_message::<deployment::GetDeploymentJobRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_error_reply(&client, &message.reply, error, "").await;
                    continue;
                }
            };
        let correlation_id = request.correlation_id.clone();
        match service.get_job(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await
            }
            Err(error) => send_error_reply(&client, &message.reply, error, correlation_id).await,
        }
    }
}

async fn run_list_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = next_message!(subscription, shutdown);
        let request =
            match decode_message::<deployment::ListDeploymentJobsRequest>(message.payload.as_ref())
            {
                Ok(request) => request,
                Err(error) => {
                    send_error_reply(&client, &message.reply, error, "").await;
                    continue;
                }
            };
        let correlation_id = request.correlation_id.clone();
        match service.list_jobs(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await
            }
            Err(error) => send_error_reply(&client, &message.reply, error, correlation_id).await,
        }
    }
}

async fn run_retry_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = next_message!(subscription, shutdown);
        let request =
            match decode_message::<deployment::RetryDeploymentJobRequest>(message.payload.as_ref())
            {
                Ok(request) => request,
                Err(error) => {
                    send_error_reply(&client, &message.reply, error, "").await;
                    continue;
                }
            };
        let correlation_id = request.correlation_id.clone();
        match service.retry_job(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await
            }
            Err(error) => send_error_reply(&client, &message.reply, error, correlation_id).await,
        }
    }
}

async fn run_cancel_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = next_message!(subscription, shutdown);
        let request = match decode_message::<deployment::CancelDeploymentJobRequest>(
            message.payload.as_ref(),
        ) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };
        let correlation_id = request.correlation_id.clone();
        match service.cancel_job(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await
            }
            Err(error) => send_error_reply(&client, &message.reply, error, correlation_id).await,
        }
    }
}

async fn run_plan_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<DeploymentService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = next_message!(subscription, shutdown);
        let request = match decode_message::<deployment::CreateDeploymentPlanRequest>(
            message.payload.as_ref(),
        ) {
            Ok(request) => request,
            Err(error) => {
                send_error_reply(&client, &message.reply, error, "").await;
                continue;
            }
        };
        let correlation_id = request.correlation_id.clone();
        match service.create_plan(request).await {
            Ok(response) => {
                send_ok_reply(&client, &message.reply, &response, &correlation_id).await
            }
            Err(error) => send_error_reply(&client, &message.reply, error, correlation_id).await,
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
    let envelope = ok_envelope(payload, correlation_id.to_string());
    send_reply(client, reply_subject, envelope).await;
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
    envelope: runtime::RuntimeReplyEnvelope,
) {
    let Some(reply_subject) = reply_subject.clone() else {
        warn!("received deployment request without reply subject");
        return;
    };

    if let Err(error) = client
        .publish(reply_subject, encode_message(&envelope).into())
        .await
    {
        error!(error = %error, "failed to publish deployment NATS reply");
    }
}
