use std::sync::Arc;

use async_nats::{Client, Subject, Subscriber};
use common::{
    json::AuditAppendEvent,
    nats_subjects::*,
    proto::{
        audit, control, decode_message, empty_ok_envelope, encode_message, ok_envelope,
        ok_json_envelope, runtime,
    },
    AppError, AppResult,
};
use futures::StreamExt;
use prost::Message;
use serde_json::Value;
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::service::{
    parse_policy_json, AnomalyInstanceFilter, AnomalyRuleInput, AnomalyRuleListFilter,
    AnomalyRuleUpdateInput, AuditEventListFilter, AuditInfo, ClusterCreateInput,
    ClusterHostMutationInput, ClusterListFilter, ClusterUpdateInput, ControlService,
    CredentialProfileInput, HostGroupInput, HostUpsertInput, IntegrationBindingInput,
    IntegrationInput, IntegrationUpdateInput, ListInput, PolicyCreateInput, PolicyUpdateInput,
    RoleBindingInput, RoleBindingListFilter, RoleCreateInput, RoleUpdateInput, TicketAssignInput,
    TicketCommentInput, TicketCreateInput, TicketListFilter, TicketStatusChangeInput,
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
    tasks.extend(spawn_cluster_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
    tasks.extend(spawn_role_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
    tasks.extend(
        spawn_integration_handlers(client.clone(), service.clone(), shutdown.clone()).await?,
    );
    tasks.extend(spawn_ticket_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
    tasks.extend(spawn_anomaly_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
    tasks.extend(spawn_audit_handlers(client.clone(), service.clone(), shutdown.clone()).await?);
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

async fn spawn_cluster_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client.subscribe(CONTROL_CLUSTERS_LIST.to_string()).await?;
    let get_sub = client.subscribe(CONTROL_CLUSTERS_GET.to_string()).await?;
    let create_sub = client
        .subscribe(CONTROL_CLUSTERS_CREATE.to_string())
        .await?;
    let update_sub = client
        .subscribe(CONTROL_CLUSTERS_UPDATE.to_string())
        .await?;
    let add_host_sub = client
        .subscribe(CONTROL_CLUSTERS_ADD_HOST.to_string())
        .await?;
    let remove_host_sub = client
        .subscribe(CONTROL_CLUSTERS_REMOVE_HOST.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_clusters_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_cluster_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_cluster_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_cluster_handler(
            update_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_cluster_add_host_handler(
            add_host_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_cluster_remove_host_handler(
            remove_host_sub,
            client,
            service,
            shutdown,
        )),
    ])
}

async fn spawn_role_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client.subscribe(CONTROL_ROLES_LIST.to_string()).await?;
    let get_sub = client.subscribe(CONTROL_ROLES_GET.to_string()).await?;
    let create_sub = client.subscribe(CONTROL_ROLES_CREATE.to_string()).await?;
    let update_sub = client.subscribe(CONTROL_ROLES_UPDATE.to_string()).await?;
    let perms_get_sub = client
        .subscribe(CONTROL_ROLES_PERMISSIONS_GET.to_string())
        .await?;
    let perms_set_sub = client
        .subscribe(CONTROL_ROLES_PERMISSIONS_SET.to_string())
        .await?;
    let bindings_list_sub = client
        .subscribe(CONTROL_ROLE_BINDINGS_LIST.to_string())
        .await?;
    let bindings_create_sub = client
        .subscribe(CONTROL_ROLE_BINDINGS_CREATE.to_string())
        .await?;
    let bindings_delete_sub = client
        .subscribe(CONTROL_ROLE_BINDINGS_DELETE.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_roles_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_role_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_role_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_role_handler(
            update_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_role_permissions_handler(
            perms_get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_set_role_permissions_handler(
            perms_set_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_list_role_bindings_handler(
            bindings_list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_role_binding_handler(
            bindings_create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_delete_role_binding_handler(
            bindings_delete_sub,
            client,
            service,
            shutdown,
        )),
    ])
}

async fn spawn_integration_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client
        .subscribe(CONTROL_INTEGRATIONS_LIST.to_string())
        .await?;
    let get_sub = client
        .subscribe(CONTROL_INTEGRATIONS_GET.to_string())
        .await?;
    let create_sub = client
        .subscribe(CONTROL_INTEGRATIONS_CREATE.to_string())
        .await?;
    let update_sub = client
        .subscribe(CONTROL_INTEGRATIONS_UPDATE.to_string())
        .await?;
    let bind_sub = client
        .subscribe(CONTROL_INTEGRATIONS_BIND.to_string())
        .await?;
    let unbind_sub = client
        .subscribe(CONTROL_INTEGRATIONS_UNBIND.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_integrations_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_integration_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_integration_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_integration_handler(
            update_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_bind_integration_handler(
            bind_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_unbind_integration_handler(
            unbind_sub, client, service, shutdown,
        )),
    ])
}

async fn spawn_ticket_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client.subscribe(TICKETS_LIST.to_string()).await?;
    let get_sub = client.subscribe(TICKETS_GET.to_string()).await?;
    let create_sub = client.subscribe(TICKETS_CREATE.to_string()).await?;
    let assign_sub = client.subscribe(TICKETS_ASSIGN.to_string()).await?;
    let unassign_sub = client.subscribe(TICKETS_UNASSIGN.to_string()).await?;
    let comment_sub = client.subscribe(TICKETS_COMMENT_ADD.to_string()).await?;
    let status_sub = client.subscribe(TICKETS_STATUS_CHANGE.to_string()).await?;
    let close_sub = client.subscribe(TICKETS_CLOSE.to_string()).await?;

    Ok(vec![
        tokio::spawn(run_list_tickets_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_ticket_handler(
            get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_ticket_handler(
            create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_assign_ticket_handler(
            assign_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_unassign_ticket_handler(
            unassign_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_comment_ticket_handler(
            comment_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_change_ticket_status_handler(
            status_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_close_ticket_handler(
            close_sub, client, service, shutdown,
        )),
    ])
}

async fn spawn_anomaly_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let rules_list_sub = client.subscribe(ANOMALIES_RULES_LIST.to_string()).await?;
    let rules_get_sub = client.subscribe(ANOMALIES_RULES_GET.to_string()).await?;
    let rules_create_sub = client.subscribe(ANOMALIES_RULES_CREATE.to_string()).await?;
    let rules_update_sub = client.subscribe(ANOMALIES_RULES_UPDATE.to_string()).await?;
    let instances_list_sub = client
        .subscribe(ANOMALIES_INSTANCES_LIST.to_string())
        .await?;
    let instances_get_sub = client
        .subscribe(ANOMALIES_INSTANCES_GET.to_string())
        .await?;

    Ok(vec![
        tokio::spawn(run_list_anomaly_rules_handler(
            rules_list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_anomaly_rule_handler(
            rules_get_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_create_anomaly_rule_handler(
            rules_create_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_update_anomaly_rule_handler(
            rules_update_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_list_anomaly_instances_handler(
            instances_list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_get_anomaly_instance_handler(
            instances_get_sub,
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

async fn spawn_audit_handlers(
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let list_sub = client.subscribe(AUDIT_LIST.to_string()).await?;
    let append_sub = client.subscribe(AUDIT_EVENTS_APPEND.to_string()).await?;

    Ok(vec![
        tokio::spawn(run_list_audit_events_handler(
            list_sub,
            client.clone(),
            service.clone(),
            shutdown.clone(),
        )),
        tokio::spawn(run_append_audit_event_handler(
            append_sub, service, shutdown,
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

        let list = ListInput::from_proto(request.paging);
        match service.list_policies(&list).await {
            Ok((policies, paging)) => {
                let response = control::ListPoliciesResponse {
                    policies: policies
                        .into_iter()
                        .map(|policy| policy.into_proto())
                        .collect(),
                    paging: Some(paging),
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
        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "policy created");

        match service.create_policy(input, &audit).await {
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
        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "policy updated");

        match service.update_policy(input, &audit).await {
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

        let list = ListInput::from_proto(request.paging);
        match service.list_policy_revisions(policy_id, &list).await {
            Ok((revisions, paging)) => {
                let response = control::GetPolicyRevisionsResponse {
                    revisions: revisions
                        .into_iter()
                        .map(|revision| revision.into_proto())
                        .collect(),
                    paging: Some(paging),
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

        let list = ListInput::from_proto(request.paging);
        match service.list_hosts(&list).await {
            Ok((hosts, paging)) => {
                let response = control::ListHostsResponse {
                    hosts: hosts.into_iter().map(|host| host.into_proto()).collect(),
                    paging: Some(paging),
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
        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "host created");

        match service.create_host(input, &audit).await {
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
        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "host updated");

        match service.update_host(host_id, input, &audit).await {
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

        let list = ListInput::from_proto(request.paging);
        match service.list_host_groups(&list).await {
            Ok((groups, paging)) => {
                let response = control::ListHostGroupsResponse {
                    groups: groups.into_iter().map(|group| group.into_proto()).collect(),
                    paging: Some(paging),
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
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "host group created");

        match service.create_host_group(input, &audit).await {
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
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "host group updated");

        match service.update_host_group(group_id, input, &audit).await {
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
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "host group member added",
        );

        match service.add_host_to_group(group_id, host_id, &audit).await {
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
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "host group member removed",
        );

        match service
            .remove_host_from_group(group_id, host_id, &audit)
            .await
        {
            Ok(()) => {
                send_control_ack(&client, &message.reply, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_clusters_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::ListClustersRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let list = ListInput::from_proto(request.paging);
        let host_filter = if request.host_id.trim().is_empty() {
            None
        } else {
            match parse_uuid(&request.host_id, "host_id") {
                Ok(id) => Some(id),
                Err(error) => {
                    send_control_error(&client, &message.reply, error, request.correlation_id)
                        .await;
                    continue;
                }
            }
        };
        let filter = ClusterListFilter {
            host_id: host_filter,
        };

        match service.list_clusters(&list, &filter).await {
            Ok((clusters, paging)) => {
                let response = control::ListClustersResponse {
                    clusters: clusters.into_iter().map(|c| c.into_proto()).collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_cluster_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::GetClusterRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let cluster_id = match parse_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_cluster(cluster_id).await {
            Ok(cluster) => {
                let response = control::GetClusterResponse {
                    cluster: Some(cluster.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_cluster_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::CreateClusterRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let metadata = match parse_json_value("metadata_json", &request.metadata_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = ClusterCreateInput {
            name: request.name.clone(),
            slug: request.slug.clone(),
            description: request.description.clone(),
            is_active: request.is_active,
            metadata,
        };
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "cluster created");

        match service.create_cluster(input, &audit).await {
            Ok(cluster) => {
                let response = control::GetClusterResponse {
                    cluster: Some(cluster.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_update_cluster_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::UpdateClusterRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let cluster_id = match parse_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let metadata = match parse_json_value("metadata_json", &request.metadata_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = ClusterUpdateInput {
            cluster_id,
            name: request.name.clone(),
            slug: request.slug.clone(),
            description: request.description.clone(),
            is_active: request.is_active,
            metadata,
        };
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "cluster updated");

        match service.update_cluster(input, &audit).await {
            Ok(cluster) => {
                let response = control::GetClusterResponse {
                    cluster: Some(cluster.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_cluster_add_host_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ClusterHostMutationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let cluster_id = match parse_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let host_id = match parse_uuid(&request.host_id, "host_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = ClusterHostMutationInput {
            cluster_id,
            host_id,
        };
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "cluster host added");

        match service.add_host_to_cluster(input, &audit).await {
            Ok(cluster) => {
                let response = control::GetClusterResponse {
                    cluster: Some(cluster.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_cluster_remove_host_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ClusterHostMutationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let cluster_id = match parse_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let host_id = match parse_uuid(&request.host_id, "host_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = ClusterHostMutationInput {
            cluster_id,
            host_id,
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "cluster host removed",
        );

        match service.remove_host_from_cluster(input, &audit).await {
            Ok(cluster) => {
                let response = control::GetClusterResponse {
                    cluster: Some(cluster.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_roles_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::ListRolesRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let list = ListInput::from_proto(request.paging);
        match service.list_roles(&list).await {
            Ok((roles, paging)) => {
                let response = control::ListRolesResponse {
                    roles: roles.into_iter().map(|role| role.into_proto()).collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_role_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::GetRoleRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let role_id = match parse_uuid(&request.role_id, "role_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_role(role_id).await {
            Ok(role) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &role.into_proto(),
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

async fn run_create_role_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::CreateRoleRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let input = RoleCreateInput {
            name: request.name.clone(),
            slug: request.slug.clone(),
            description: request.description.clone(),
        };
        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "role created");

        match service.create_role(input, &audit).await {
            Ok(role) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &role.into_proto(),
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

async fn run_update_role_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::UpdateRoleRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let role_id = match parse_uuid(&request.role_id, "role_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = RoleUpdateInput {
            role_id,
            name: request.name.clone(),
            description: request.description.clone(),
        };
        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "role updated");

        match service.update_role(input, &audit).await {
            Ok(role) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &role.into_proto(),
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

async fn run_get_role_permissions_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::GetRolePermissionsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let role_id = match parse_uuid(&request.role_id, "role_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_role_permissions(role_id).await {
            Ok((role, perms)) => {
                let response = control::GetRolePermissionsResponse {
                    role: Some(role.into_proto()),
                    permissions: perms.into_iter().map(|p| p.into_proto()).collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_set_role_permissions_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::SetRolePermissionsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let role_id = match parse_uuid(&request.role_id, "role_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "role permissions updated",
        );

        match service
            .set_role_permissions(role_id, request.permission_codes, &audit)
            .await
        {
            Ok((role, perms)) => {
                let response = control::GetRolePermissionsResponse {
                    role: Some(role.into_proto()),
                    permissions: perms.into_iter().map(|p| p.into_proto()).collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_role_bindings_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ListRoleBindingsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let list = ListInput::from_proto(request.paging);
        let role_id = if request.role_id.trim().is_empty() {
            None
        } else {
            match parse_uuid(&request.role_id, "role_id") {
                Ok(id) => Some(id),
                Err(error) => {
                    send_control_error(&client, &message.reply, error, request.correlation_id)
                        .await;
                    continue;
                }
            }
        };
        let scope_id = match parse_optional_uuid(&request.scope_id, "scope_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let filter = RoleBindingListFilter {
            user_id: normalize_optional(request.user_id),
            role_id,
            scope_type: normalize_optional(request.scope_type),
            scope_id,
        };

        match service.list_role_bindings(&list, &filter).await {
            Ok((bindings, paging)) => {
                let response = control::ListRoleBindingsResponse {
                    bindings: bindings.into_iter().map(|b| b.into_proto()).collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_role_binding_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::CreateRoleBindingRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let role_id = match parse_uuid(&request.role_id, "role_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let scope_id = match parse_optional_uuid(&request.scope_id, "scope_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = RoleBindingInput {
            user_id: request.user_id.clone(),
            role_id,
            scope_type: request.scope_type.clone(),
            scope_id,
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "role binding created",
        );

        match service.create_role_binding(input, &audit).await {
            Ok(binding) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &binding.into_proto(),
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

async fn run_delete_role_binding_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::DeleteRoleBindingRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let binding_id = match parse_uuid(&request.role_binding_id, "role_binding_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "role binding deleted",
        );

        match service.delete_role_binding(binding_id, &audit).await {
            Ok(()) => {
                send_control_ack(&client, &message.reply, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_integrations_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ListIntegrationsRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let list = ListInput::from_proto(request.paging);

        match service.list_integrations(&list).await {
            Ok((integrations, paging)) => {
                let response = control::ListIntegrationsResponse {
                    integrations: integrations
                        .into_iter()
                        .map(|integration| integration.into_proto())
                        .collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_integration_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::GetIntegrationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let integration_id = match parse_uuid(&request.integration_id, "integration_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_integration(integration_id).await {
            Ok((integration, bindings)) => {
                let response = control::GetIntegrationResponse {
                    integration: Some(integration.into_proto()),
                    bindings: bindings.into_iter().map(|b| b.into_proto()).collect(),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_integration_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::CreateIntegrationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let config = match parse_json_value("config_json", &request.config_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = IntegrationInput {
            name: request.name.clone(),
            kind: request.kind.clone(),
            description: request.description.clone(),
            config,
            is_active: request.is_active,
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "integration created",
        );

        match service.create_integration(input, &audit).await {
            Ok(integration) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &integration.into_proto(),
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

async fn run_update_integration_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::UpdateIntegrationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let integration_id = match parse_uuid(&request.integration_id, "integration_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let config = match parse_json_value("config_json", &request.config_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = IntegrationUpdateInput {
            integration_id,
            name: request.name.clone(),
            description: request.description.clone(),
            config,
            is_active: request.is_active,
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "integration updated",
        );

        match service.update_integration(input, &audit).await {
            Ok(integration) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &integration.into_proto(),
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

async fn run_bind_integration_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::BindIntegrationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let integration_id = match parse_uuid(&request.integration_id, "integration_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let scope_id = match parse_optional_uuid(&request.scope_id, "scope_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let event_types = match parse_json_value("event_types_json", &request.event_types_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = IntegrationBindingInput {
            integration_id,
            scope_type: request.scope_type.clone(),
            scope_id,
            event_types,
            severity_threshold: request.severity_threshold.clone(),
            is_active: request.is_active,
        };
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "integration bound");

        match service.bind_integration(input, &audit).await {
            Ok(binding) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &binding.into_proto(),
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

async fn run_unbind_integration_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::UnbindIntegrationRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let binding_id = match parse_uuid(&request.integration_binding_id, "integration_binding_id")
        {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "integration unbound",
        );

        match service.unbind_integration(binding_id, &audit).await {
            Ok(()) => {
                send_control_ack(&client, &message.reply, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_list_tickets_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::ListTicketsRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let list = ListInput::from_proto(request.paging);
        let cluster_id = match parse_optional_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let filter = TicketListFilter {
            cluster_id,
            status: normalize_optional(request.status),
            severity: normalize_optional(request.severity),
            assignee_user_id: normalize_optional(request.assignee_user_id),
        };

        match service.list_tickets(&list, &filter).await {
            Ok((tickets, paging)) => {
                let response = control::ListTicketsResponse {
                    tickets: tickets.into_iter().map(|t| t.into_proto()).collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_ticket_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::GetTicketRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let ticket_id = match parse_uuid(&request.ticket_id, "ticket_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_ticket(ticket_id).await {
            Ok(ticket) => {
                let response = control::GetTicketResponse {
                    ticket: Some(ticket.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_create_ticket_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::CreateTicketRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let cluster_id = match parse_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "ticket created");
        let input = TicketCreateInput {
            title: request.title.clone(),
            description: request.description.clone(),
            cluster_id,
            source_type: request.source_type.clone(),
            source_id: normalize_optional(request.source_id),
            severity: request.severity.clone(),
            created_by: audit.actor_id.clone(),
        };

        match service.create_ticket(input, &audit).await {
            Ok(ticket) => {
                let response = control::GetTicketResponse {
                    ticket: Some(ticket.into_proto()),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_assign_ticket_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::AssignTicketRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let ticket_id = match parse_uuid(&request.ticket_id, "ticket_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = TicketAssignInput {
            ticket_id,
            assignee_user_id: request.assignee_user_id.clone(),
        };
        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "ticket assigned");

        match service.assign_ticket(input, &audit).await {
            Ok(ticket) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &ticket.into_proto(),
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

async fn run_unassign_ticket_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::UnassignTicketRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let ticket_id = match parse_uuid(&request.ticket_id, "ticket_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit =
            AuditInfo::from_proto(&request.correlation_id, request.audit, "ticket unassigned");

        match service.unassign_ticket(ticket_id, &audit).await {
            Ok(ticket) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &ticket.into_proto(),
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

async fn run_comment_ticket_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::AddTicketCommentRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let ticket_id = match parse_uuid(&request.ticket_id, "ticket_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "ticket comment added",
        );
        let input = TicketCommentInput {
            ticket_id,
            body: request.body.clone(),
            author_user_id: audit.actor_id.clone(),
        };

        match service.comment_ticket(input, &audit).await {
            Ok(comment) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &comment.into_proto(),
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

async fn run_change_ticket_status_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ChangeTicketStatusRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let ticket_id = match parse_uuid(&request.ticket_id, "ticket_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = TicketStatusChangeInput {
            ticket_id,
            status: request.status.clone(),
            resolution: request.resolution.clone(),
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "ticket status changed",
        );

        match service.change_ticket_status(input, &audit).await {
            Ok(ticket) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &ticket.into_proto(),
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

async fn run_close_ticket_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::CloseTicketRequest>(message.payload.as_ref())
        {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let ticket_id = match parse_uuid(&request.ticket_id, "ticket_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let audit = AuditInfo::from_proto(&request.correlation_id, request.audit, "ticket closed");

        match service
            .close_ticket(ticket_id, &request.resolution, &audit)
            .await
        {
            Ok(ticket) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &ticket.into_proto(),
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

async fn run_list_anomaly_rules_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::ListAnomalyRulesRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let list = ListInput::from_proto(request.paging);
        let scope_id = match parse_optional_uuid(&request.scope_id, "scope_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let filter = AnomalyRuleListFilter {
            scope_type: normalize_optional(request.scope_type),
            scope_id,
        };

        match service.list_anomaly_rules(&list, &filter).await {
            Ok((rules, paging)) => {
                let response = control::ListAnomalyRulesResponse {
                    rules: rules.into_iter().map(|r| r.into_proto()).collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_anomaly_rule_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::GetAnomalyRuleRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let rule_id = match parse_uuid(&request.anomaly_rule_id, "anomaly_rule_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_anomaly_rule(rule_id).await {
            Ok(rule) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &rule.into_proto(),
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

async fn run_create_anomaly_rule_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::CreateAnomalyRuleRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let scope_id = match parse_optional_uuid(&request.scope_id, "scope_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let config = match parse_json_value("config_json", &request.config_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = AnomalyRuleInput {
            name: request.name.clone(),
            kind: request.kind.clone(),
            scope_type: request.scope_type.clone(),
            scope_id,
            config,
            is_active: request.is_active,
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "anomaly rule created",
        );

        match service.create_anomaly_rule(input, &audit).await {
            Ok(rule) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &rule.into_proto(),
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

async fn run_update_anomaly_rule_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::UpdateAnomalyRuleRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let rule_id = match parse_uuid(&request.anomaly_rule_id, "anomaly_rule_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let config = match parse_json_value("config_json", &request.config_json) {
            Ok(value) => value,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        let input = AnomalyRuleUpdateInput {
            anomaly_rule_id: rule_id,
            name: request.name.clone(),
            config,
            is_active: request.is_active,
        };
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "anomaly rule updated",
        );

        match service.update_anomaly_rule(input, &audit).await {
            Ok(rule) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &rule.into_proto(),
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

async fn run_list_anomaly_instances_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request = match decode_message::<control::ListAnomalyInstancesRequest>(
            message.payload.as_ref(),
        ) {
            Ok(request) => request,
            Err(error) => {
                send_control_error(&client, &message.reply, error, "").await;
                continue;
            }
        };

        let list = ListInput::from_proto(request.paging);
        let rule_id = match parse_optional_uuid(&request.anomaly_rule_id, "anomaly_rule_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let cluster_id = match parse_optional_uuid(&request.cluster_id, "cluster_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };
        let filter = AnomalyInstanceFilter {
            anomaly_rule_id: rule_id,
            cluster_id,
            status: normalize_optional(request.status),
        };

        match service.list_anomaly_instances(&list, &filter).await {
            Ok((instances, paging)) => {
                let response = control::ListAnomalyInstancesResponse {
                    instances: instances
                        .into_iter()
                        .map(|instance| instance.into_proto())
                        .collect(),
                    paging: Some(paging),
                };
                send_control_ok(&client, &message.reply, &response, &request.correlation_id).await;
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
            }
        }
    }
}

async fn run_get_anomaly_instance_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let request =
            match decode_message::<control::GetAnomalyInstanceRequest>(message.payload.as_ref()) {
                Ok(request) => request,
                Err(error) => {
                    send_control_error(&client, &message.reply, error, "").await;
                    continue;
                }
            };

        let instance_id = match parse_uuid(&request.anomaly_instance_id, "anomaly_instance_id") {
            Ok(id) => id,
            Err(error) => {
                send_control_error(&client, &message.reply, error, request.correlation_id).await;
                continue;
            }
        };

        match service.get_anomaly_instance(instance_id).await {
            Ok(instance) => {
                send_control_ok(
                    &client,
                    &message.reply,
                    &instance.into_proto(),
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

        let list = ListInput::from_proto(request.paging);
        match service.list_credentials(&list).await {
            Ok((profiles, paging)) => {
                let response = control::ListCredentialsResponse {
                    profiles: profiles
                        .into_iter()
                        .map(|profile| profile.into_proto())
                        .collect(),
                    paging: Some(paging),
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
        let audit = AuditInfo::from_proto(
            &request.correlation_id,
            request.audit,
            "credentials metadata created",
        );

        match service.create_credentials(input, &audit).await {
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

async fn run_list_audit_events_handler(
    mut subscription: Subscriber,
    client: Client,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let (request, wants_json) =
            match decode_message::<audit::ListAuditEventsRequest>(message.payload.as_ref()) {
                Ok(request) => (request, false),
                Err(proto_error) => match serde_json::from_slice::<audit::ListAuditEventsRequest>(
                    message.payload.as_ref(),
                ) {
                    Ok(request) => (request, true),
                    Err(_) => {
                        send_control_error(&client, &message.reply, proto_error, "").await;
                        continue;
                    }
                },
            };

        let correlation_id = request.correlation_id.clone();
        let list = ListInput::from_proto(request.paging.clone());
        let filter = AuditEventListFilter {
            event_type: normalize_optional(request.event_type),
            entity_type: normalize_optional(request.entity_type),
            entity_id: normalize_optional(request.entity_id),
            actor_id: normalize_optional(request.actor_id),
        };

        match service.list_runtime_audit_events(&list, &filter).await {
            Ok((items, paging)) => {
                let response = audit::ListAuditEventsResponse {
                    items: items.into_iter().map(|item| item.into_proto()).collect(),
                    paging: Some(paging),
                };
                if wants_json {
                    match ok_json_envelope(&response, correlation_id.clone()) {
                        Ok(envelope) => {
                            send_control_envelope(&client, &message.reply, envelope).await;
                        }
                        Err(error) => {
                            send_control_error(&client, &message.reply, error, correlation_id)
                                .await;
                        }
                    }
                } else {
                    send_control_ok(&client, &message.reply, &response, &correlation_id).await;
                }
            }
            Err(error) => {
                send_control_error(&client, &message.reply, error, correlation_id).await;
            }
        }
    }
}

async fn run_append_audit_event_handler(
    mut subscription: Subscriber,
    service: Arc<ControlService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = handle_request!(subscription, shutdown);

        let event = match serde_json::from_slice::<AuditAppendEvent>(message.payload.as_ref()) {
            Ok(event) => event,
            Err(error) => {
                warn!(error = %error, "failed to decode audit append event");
                continue;
            }
        };

        if let Err(error) = service.append_runtime_audit_event(event).await {
            error!(error_code = error.code().as_str(), error = %error, "failed to append runtime audit event");
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

fn parse_json_value(field: &str, payload: &str) -> AppResult<Value> {
    if payload.trim().is_empty() {
        return Ok(Value::Object(Default::default()));
    }
    serde_json::from_str(payload).map_err(|error| {
        AppError::invalid_argument(format!("invalid {field} json payload: {error}"))
    })
}

fn parse_uuid(value: &str, field: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value)
        .map_err(|_| AppError::invalid_argument(format!("{field} must be a valid UUID")))
}

fn parse_optional_uuid(value: &str, field: &str) -> AppResult<Option<Uuid>> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn normalize_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn send_control_ok<T: Message>(
    client: &Client,
    reply_subject: &Option<Subject>,
    payload: &T,
    correlation_id: &str,
) {
    let envelope = ok_envelope(payload, correlation_id.to_string());
    send_control_envelope(client, reply_subject, envelope).await;
}

async fn send_control_ack(client: &Client, reply_subject: &Option<Subject>, correlation_id: &str) {
    send_control_envelope(client, reply_subject, empty_ok_envelope(correlation_id)).await;
}

async fn send_control_error(
    client: &Client,
    reply_subject: &Option<Subject>,
    error: AppError,
    correlation_id: impl Into<String>,
) {
    let envelope = error.to_envelope(correlation_id);
    send_control_envelope(client, reply_subject, envelope).await;
}

async fn send_control_envelope(
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
        error!(error = %error, "failed to publish control reply");
    }
}
