use std::sync::Arc;

use async_nats::{Client, Subject, Subscriber};
use common::{
    nats_subjects::*,
    proto::{control, control_ok_envelope, decode_message, encode_message},
    AppError, AppResult,
};
use futures::StreamExt;
use prost::Message;
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::service::{
    parse_policy_json, ControlService, CredentialProfileInput, HostGroupInput, HostUpsertInput,
    PolicyCreateInput, PolicyUpdateInput,
};

pub async fn spawn_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let mut tasks = Vec::new();
    tasks.extend(spawn_policy_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
    tasks.extend(spawn_host_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
    tasks.extend(
        spawn_host_group_handlers(client.clone(), service.clone(), shutdown.clone()).await?,
    );
    tasks.extend(spawn_credential_handlers(client, service, shutdown).await?);
    Ok(tasks)
}

async fn spawn_policy_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client.subscribe(CONTROL_POLICIES_LIST.to_string()).await?;
    let get_sub = client.subscribe(CONTROL_POLICIES_GET.to_string()).await?;
    let create_sub = client
        .subscribe(CONTROL_POLICIES_CREATE.to_string())
        .await?;
    let update_sub = client
        .subscribe(CONTROL_POLICIES_UPDATE.to_string())
        .await?;
    let revisions_sub = client
        .subscribe(CONTROL_POLICIES_REVISIONS.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_policies_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_policy_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_policy_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_policy_handler(
            update_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_policy_revisions_handler(
            revisions_sub,
            client,
            service,
            shutdown,
        )),
    ])
}

async fn spawn_host_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client.subscribe(CONTROL_HOSTS_LIST.to_string()).await?;
    let get_sub = client.subscribe(CONTROL_HOSTS_GET.to_string()).await?;
    let create_sub = client.subscribe(CONTROL_HOSTS_CREATE.to_string()).await?;
    let update_sub = client.subscribe(CONTROL_HOSTS_UPDATE.to_string()).await?;

    Ok(vec![
        tokio::spawn(run_list_hosts_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_host_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_host_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_host_handler(
            update_sub, client, service, shutdown,
        )),
    ])
}

async fn spawn_host_group_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client
        .subscribe(CONTROL_HOST_GROUPS_LIST.to_string())
        .await?;
    let get_sub = client
        .subscribe(CONTROL_HOST_GROUPS_GET.to_string())
        .await?;
    let create_sub = client
        .subscribe(CONTROL_HOST_GROUPS_CREATE.to_string())
        .await?;
    let update_sub = client
        .subscribe(CONTROL_HOST_GROUPS_UPDATE.to_string())
        .await?;
    let add_member_sub = client
        .subscribe(CONTROL_HOST_GROUPS_ADD_MEMBER.to_string())
        .await?;
    let remove_member_sub = client
        .subscribe(CONTROL_HOST_GROUPS_REMOVE_MEMBER.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_host_groups_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_host_group_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_host_group_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_host_group_handler(
            update_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_add_member_handler(
            add_member_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_remove_member_handler(
            remove_member_sub,
            client,
            service,
            shutdown,
        )),
    ])
}

async fn spawn_credential_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client
        .subscribe(CONTROL_CREDENTIALS_LIST.to_string())
        .await?;
    let get_sub = client
        .subscribe(CONTROL_CREDENTIALS_GET.to_string())
        .await?;
    let create_sub = client
        .subscribe(CONTROL_CREDENTIALS_CREATE.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_credentials_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_credentials_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_credentials_handler(
            create_sub, client, service, shutdown,
        )),
    ])
}

macro_rules! handle_request {
    ($subscription:ident, $shutdown:ident) => {
        select! {
            _ = $shutdown.cancelled() => break,
            msg = $subscription.next() => {
                let Some(message) = msg else { break; };
                message
            }
        }
    };
}

async fn run_list_policies_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::ListPoliciesRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        match service.list_policies().await {
            Ok(policies) => {
                let response = control::ListPoliciesResponse {
                    policies: policies
                        .into_iter()
                        .map(|policy| policy.into_proto())
                        .collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_policy_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::GetPolicyRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let correlation_id = request.correlation_id.clone();
        let policy_id = match parse_uuid(&request.policy_id, "policy_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, correlation_id).await;
                continue;
            }
        };

        match service.get_policy(policy_id).await {
            Ok(policy) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &policy.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_policy_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::CreatePolicyRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let body = match parse_policy_json("policy_body_json", &request.policy_body_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        let input = PolicyCreateInput {
            name: request.name.clone(),
            description: request.description.clone(),
            body_json: body,
        };

        match service.create_policy(input).await {
            Ok(policy) => {
                info!(policy_id = %policy.id, "created policy");
                send_control_ok(
                    &client,
                    &message.reply,
                    &policy.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_update_policy_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::UpdatePolicyRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let policy_id = match parse_uuid(&request.policy_id, "policy_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        let body = match parse_policy_json("policy_body_json", &request.policy_body_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        let input = PolicyUpdateInput {
            policy_id,
            description: request.description.clone(),
            body_json: body,
        };

        match service.update_policy(input).await {
            Ok(policy) => {
                info!(policy_id = %policy.id, "updated policy");
                send_control_ok(
                    &client,
                    &message.reply,
                    &policy.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_policy_revisions_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::GetPolicyRevisionsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let policy_id = match parse_uuid(&request.policy_id, "policy_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.list_policy_revisions(policy_id).await {
            Ok(revisions) => {
                let response = control::GetPolicyRevisionsResponse {
                    revisions: revisions
                        .into_iter()
                        .map(|revision| revision.into_proto())
                        .collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_hosts_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::ListHostsRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        match service.list_hosts().await {
            Ok(hosts) => {
                let response = control::ListHostsResponse {
                    hosts: hosts.into_iter().map(|host| host.into_proto()).collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_host_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::GetHostRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let host_id = match parse_uuid(&request.host_id, "host_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.get_host(host_id).await {
            Ok(host) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &host.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_host_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::CreateHostRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let input = match host_input_from_proto(request.host.clone()) {
            Ok(input) => input,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.create_host(input).await {
            Ok(host) => {
                info!(host_id = %host.id, "created host");
                send_control_ok(
                    &client,
                    &message.reply,
                    &host.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_update_host_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::UpdateHostRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let host_id = match parse_uuid(&request.host_id, "host_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        let input = match host_input_from_proto(request.host.clone()) {
            Ok(input) => input,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.update_host(host_id, input).await {
            Ok(host) => {
                info!(host_id = %host.id, "updated host");
                send_control_ok(
                    &client,
                    &message.reply,
                    &host.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_host_groups_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ListHostGroupsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        match service.list_host_groups().await {
            Ok(groups) => {
                let response = control::ListHostGroupsResponse {
                    groups: groups.into_iter().map(|group| group.into_proto()).collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_host_group_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::GetHostGroupRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let group_id = match parse_uuid(&request.host_group_id, "host_group_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.get_host_group(group_id).await {
            Ok(group) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &group.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_host_group_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::CreateHostGroupRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let input = HostGroupInput {
            name: request.name.clone(),
            description: request.description.clone(),
        };

        match service.create_host_group(input).await {
            Ok(group) => {
                info!(host_group_id = %group.id, "created host group");
                send_control_ok(
                    &client,
                    &message.reply,
                    &group.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_update_host_group_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::UpdateHostGroupRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let group_id = match parse_uuid(&request.host_group_id, "host_group_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        let input = HostGroupInput {
            name: request.name.clone(),
            description: request.description.clone(),
        };

        match service.update_host_group(group_id, input).await {
            Ok(group) => {
                info!(host_group_id = %group.id, "updated host group");
                send_control_ok(
                    &client,
                    &message.reply,
                    &group.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_add_member_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::AddHostGroupMemberRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let group_id = match parse_uuid(&request.host_group_id, "host_group_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };
        let host_id = match parse_uuid(&request.host_id, "host_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.add_host_to_group(group_id, host_id).await {
            Ok(member) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &member.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_remove_member_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::RemoveHostGroupMemberRequest>(message.payload.as_ref())
            {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let group_id = match parse_uuid(&request.host_group_id, "host_group_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };
        let host_id = match parse_uuid(&request.host_id, "host_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.remove_host_from_group(group_id, host_id).await {
            Ok(()) => {
                send_control_ack(&client, &message.reply, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_credentials_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ListCredentialsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        match service.list_credentials().await {
            Ok(profiles) => {
                let response = control::ListCredentialsResponse {
                    profiles: profiles
                        .into_iter()
                        .map(|profile| profile.into_proto())
                        .collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_credentials_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::GetCredentialsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let profile_id = match parse_uuid(&request.credentials_profile_id, "credentials_profile_id")
        {
            Ok(id) => id,
            Err(error) => {
                send_control_error(
                    &client,
                    &message.reply,
                    error,
                    request.correlation_id.clone(),
                )
                .await;
                continue;
            }
        };

        match service.get_credentials(profile_id).await {
            Ok(profile) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &profile.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_credentials_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::CreateCredentialsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let input = CredentialProfileInput {
            name: request.name.clone(),
            kind: request.kind.clone(),
            description: request.description.clone(),
            vault_ref: request.vault_ref.clone(),
        };

        match service.create_credentials(input).await {
            Ok(profile) => {
                info!(credentials_profile_id = %profile.id, "created credentials profile");
                send_control_ok(
                    &client,
                    &message.reply,
                    &profile.into_proto(),
                    &request.correlation_id,
                )
                .await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

fn host_input_from_proto(host: Option<control::HostInput>) -> AppResult<HostUpsertInput> {
    let host = host.ok_or_else(|| AppError::invalid_argument("host payload is required"))?;
    let ssh_port = u16::try_from(host.ssh_port)
        .map_err(|_| AppError::invalid_argument("ssh_port must fit into u16"))?;

    Ok(HostUpsertInput {
        hostname: host.hostname,
        ip: host.ip,
        ssh_port,
        remote_user: host.remote_user,
        labels: host.labels.into_iter().collect(),
    })
}

fn parse_uuid(value: &str, field: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value)
        .map_err(|_| AppError::invalid_argument(format!("{field} must be a valid UUID")))
}

async fn send_control_ok<T: Message>(
    client: &Client,
    reply_subject: &Option<Subject>,
    payload: &T,
    correlation_id: &str,
) {
    let envelope = control_ok_envelope(payload, correlation_id.to_string());
    send_control_envelope(client, reply_subject, envelope).await;
}

async fn send_control_ack(client: &Client, reply_subject: &Option<Subject>, correlation_id: &str) {
    let envelope = control::ControlReplyEnvelope {
        status: "ok".to_string(),
        code: "ok".to_string(),
        message: String::new(),
        payload: Vec::new(),
        correlation_id: correlation_id.to_string(),
    };
    send_control_envelope(client, reply_subject, envelope).await;
}

async fn send_control_error(
    client: &Client,
    reply_subject: &Option<Subject>,
    error: AppError,
    correlation_id: impl Into<String>,
) {
    let envelope = error.to_control_envelope(correlation_id);
    send_control_envelope(client, reply_subject, envelope).await;
}

async fn send_control_envelope(
    client: &Client,
    reply_subject: &Option<Subject>,
    envelope: control::ControlReplyEnvelope,
) {
    let Some(reply_subject) = reply_subject.clone() else {
        warn!("received request without reply subject");
        return;
    };

    if let Err(error) = client
        .publish(reply_subject, encode_message(&envelope).into())
        .await
    {
        error!(error = %error, "failed to publish control reply");
    }
}
