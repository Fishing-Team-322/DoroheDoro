use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, ensure};
use async_nats::Client;
use axum::{body::Body, http::Request};
use common::{
    nats_subjects::{
        ANOMALIES_INSTANCES_LIST, ANOMALIES_RULES_CREATE, ANOMALIES_RULES_UPDATE,
        CONTROL_CLUSTERS_ADD_HOST, CONTROL_CLUSTERS_CREATE, CONTROL_CLUSTERS_GET,
        CONTROL_CLUSTERS_LIST, CONTROL_CREDENTIALS_CREATE, CONTROL_CREDENTIALS_GET,
        CONTROL_CREDENTIALS_LIST, CONTROL_HOSTS_CREATE, CONTROL_HOSTS_LIST, CONTROL_HOSTS_UPDATE,
        CONTROL_HOST_GROUPS_ADD_MEMBER, CONTROL_HOST_GROUPS_CREATE, CONTROL_HOST_GROUPS_LIST,
        CONTROL_HOST_GROUPS_REMOVE_MEMBER, CONTROL_HOST_GROUPS_UPDATE, CONTROL_INTEGRATIONS_BIND,
        CONTROL_INTEGRATIONS_CREATE, CONTROL_POLICIES_CREATE, CONTROL_POLICIES_LIST,
        CONTROL_POLICIES_REVISIONS, CONTROL_POLICIES_UPDATE, CONTROL_ROLE_BINDINGS_CREATE,
        CONTROL_ROLES_CREATE, CONTROL_ROLES_PERMISSIONS_SET, TICKETS_ASSIGN, TICKETS_CLOSE,
        TICKETS_COMMENT_ADD, TICKETS_CREATE, TICKETS_STATUS_CHANGE,
    },
    proto::{
        control::{
            AddHostGroupMemberRequest, AddTicketCommentRequest, AnomalyRule, AssignTicketRequest,
            BindIntegrationRequest, ChangeTicketStatusRequest, CloseTicketRequest,
            ClusterHostMutationRequest, CreateAnomalyRuleRequest, CreateClusterRequest,
            CreateCredentialsRequest, CreateHostGroupRequest, CreateHostRequest,
            CreateIntegrationRequest, CreatePolicyRequest, CreateRoleBindingRequest,
            CreateRoleRequest, CreateTicketRequest, CredentialProfileMetadata, GetClusterRequest,
            GetClusterResponse, GetCredentialsRequest, GetPolicyRevisionsRequest,
            GetPolicyRevisionsResponse, GetRolePermissionsResponse, GetTicketResponse, Host,
            HostGroup, HostGroupMember, HostInput, Integration, IntegrationBinding,
            ListAnomalyInstancesRequest, ListAnomalyInstancesResponse, ListClustersRequest,
            ListClustersResponse, ListCredentialsRequest, ListCredentialsResponse,
            ListHostGroupsRequest, ListHostGroupsResponse, ListHostsRequest, ListHostsResponse,
            ListPoliciesRequest, ListPoliciesResponse, Policy, RemoveHostGroupMemberRequest, Role,
            RoleBinding, SetRolePermissionsRequest, TicketDetails, UpdateAnomalyRuleRequest,
            UpdateHostGroupRequest, UpdateHostRequest, UpdatePolicyRequest,
        },
        decode_message, encode_message,
        runtime::{AuditContext, RuntimeReplyEnvelope},
    },
    ControlPlaneConfig,
};
use control_plane::{
    http::{self, HttpState},
    repository::ControlRepository,
    service::ControlService,
    transport,
};
use prost::Message;
use reqwest::StatusCode;
use serial_test::serial;
use sqlx::{postgres::PgPoolOptions, PgPool};
use serde_json::json;
use tokio::{net::TcpListener, task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

struct TestHarness {
    http_addr: String,
    http_client: reqwest::Client,
    nats: Client,
    #[allow(unused)]
    pool: PgPool,
    shutdown: CancellationToken,
    server_task: JoinHandle<()>,
    subscriber_tasks: Vec<JoinHandle<()>>,
}

impl TestHarness {
    async fn start() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let config = ControlPlaneConfig::from_env()?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.shared.postgres_dsn)
            .await?;
        run_migrations(&pool).await?;
        truncate_tables(&pool).await?;

        let repo = ControlRepository::new(pool.clone());
        let service_inner = ControlService::new(repo);
        service_inner
            .bootstrap()
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        let service = Arc::new(service_inner);

        let nats = async_nats::connect(&config.shared.nats_url).await?;
        let shutdown = CancellationToken::new();
        let subscriber_tasks =
            transport::spawn_handlers(nats.clone(), service, shutdown.clone()).await?;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let http_addr = format!("http://{}", addr);
        let http_shutdown = shutdown.clone();
        let pool_for_server = pool.clone();
        let nats_for_server = nats.clone();
        let server_task = tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                http::router(HttpState::new(pool_for_server, nats_for_server)),
            )
            .with_graceful_shutdown(async move {
                http_shutdown.cancelled().await;
            })
            .await;
        });

        let harness = Self {
            http_addr,
            http_client: reqwest::Client::new(),
            nats,
            pool,
            shutdown,
            server_task,
            subscriber_tasks,
        };

        harness.wait_until_ready().await?;
        Ok(harness)
    }

    async fn wait_until_ready(&self) -> anyhow::Result<()> {
        for _ in 0..40 {
            let response = self
                .http_client
                .get(format!("{}/readyz", self.http_addr))
                .send()
                .await;

            if let Ok(response) = response {
                if response.status() == StatusCode::OK {
                    return Ok(());
                }
            }

            sleep(Duration::from_millis(100)).await;
        }

        bail!("service did not become ready in time");
    }

    async fn shutdown(self) {
        self.shutdown.cancel();
        self.server_task.abort();
        let _ = self.server_task.await;
        for task in self.subscriber_tasks {
            task.abort();
            let _ = task.await;
        }
    }

    async fn request_payload<Req, Resp>(&self, subject: &str, request: Req) -> anyhow::Result<Resp>
    where
        Req: Message,
        Resp: Message + Default,
    {
        let envelope = self.request_envelope(subject, request).await?;
        let payload = decode_message(&envelope.payload)?;
        Ok(payload)
    }

    async fn request_ack<Req>(&self, subject: &str, request: Req) -> anyhow::Result<()>
    where
        Req: Message,
    {
        let _ = self.request_envelope(subject, request).await?;
        Ok(())
    }

    async fn request_envelope<Req>(
        &self,
        subject: &str,
        request: Req,
    ) -> anyhow::Result<RuntimeReplyEnvelope>
    where
        Req: Message,
    {
        let message = self
            .nats
            .request(subject.to_string(), encode_message(&request).into())
            .await?;
        let envelope: RuntimeReplyEnvelope = decode_message(message.payload.as_ref())?;
        ensure!(
            envelope.status == "ok",
            "subject {subject} failed: {} {}",
            envelope.code,
            envelope.message
        );
        Ok(envelope)
    }
}

fn test_audit() -> Option<AuditContext> {
    Some(AuditContext {
        actor_id: "smoke-user".to_string(),
        actor_type: "test".to_string(),
        request_id: new_corr_id(),
        reason: "smoke test".to_string(),
    })
}

#[tokio::test]
#[ignore]
#[serial]
async fn health_and_readiness_work() -> anyhow::Result<()> {
    let harness = TestHarness::start().await?;

    let health = harness
        .http_client
        .get(format!("{}/healthz", harness.http_addr))
        .send()
        .await?;
    assert_eq!(health.status(), StatusCode::OK);

    let ready = harness
        .http_client
        .get(format!("{}/readyz", harness.http_addr))
        .send()
        .await?;
    assert_eq!(ready.status(), StatusCode::OK);

    let bad_pool =
        PgPoolOptions::new().connect_lazy("postgres://postgres:postgres@127.0.0.1:1/doro")?;
    let response = http::router(HttpState::new(bad_pool, harness.nats.clone()))
        .oneshot(Request::builder().uri("/readyz").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
#[ignore]
#[serial]
async fn policies_inventory_credentials_flow() -> anyhow::Result<()> {
    let harness = TestHarness::start().await?;

    let list: ListPoliciesResponse = harness
        .request_payload(
            CONTROL_POLICIES_LIST,
            ListPoliciesRequest {
                correlation_id: new_corr_id(),
                paging: None,
            },
        )
        .await?;
    assert!(list.policies.is_empty());

    let policy: Policy = harness
        .request_payload(
            CONTROL_POLICIES_CREATE,
            CreatePolicyRequest {
                correlation_id: new_corr_id(),
                name: "baseline".to_string(),
                description: "baseline policy".to_string(),
                policy_body_json: r#"{"paths":["/var/log/*.log"]}"#.to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(policy.name, "baseline");
    assert_eq!(policy.latest_revision, "rev-1");

    let updated_policy: Policy = harness
        .request_payload(
            CONTROL_POLICIES_UPDATE,
            UpdatePolicyRequest {
                correlation_id: new_corr_id(),
                policy_id: policy.policy_id.clone(),
                description: "baseline updated".to_string(),
                policy_body_json: r#"{"paths":["/var/log/syslog"]}"#.to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(updated_policy.latest_revision, "rev-2");

    let revisions: GetPolicyRevisionsResponse = harness
        .request_payload(
            CONTROL_POLICIES_REVISIONS,
            GetPolicyRevisionsRequest {
                correlation_id: new_corr_id(),
                policy_id: policy.policy_id.clone(),
                paging: None,
            },
        )
        .await?;
    assert_eq!(revisions.revisions.len(), 2);

    let host: Host = harness
        .request_payload(
            CONTROL_HOSTS_CREATE,
            CreateHostRequest {
                correlation_id: new_corr_id(),
                host: Some(HostInput {
                    hostname: "host-1".to_string(),
                    ip: "10.0.0.5".to_string(),
                    ssh_port: 22,
                    remote_user: "root".to_string(),
                    labels: [
                        ("env".to_string(), "dev".to_string()),
                        ("role".to_string(), "web".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                }),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(host.hostname, "host-1");
    assert_eq!(host.labels.get("env"), Some(&"dev".to_string()));

    let host: Host = harness
        .request_payload(
            CONTROL_HOSTS_UPDATE,
            UpdateHostRequest {
                correlation_id: new_corr_id(),
                host_id: host.host_id.clone(),
                host: Some(HostInput {
                    hostname: "host-1".to_string(),
                    ip: "10.0.0.8".to_string(),
                    ssh_port: 2222,
                    remote_user: "admin".to_string(),
                    labels: [("env".to_string(), "prod".to_string())]
                        .into_iter()
                        .collect(),
                }),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(host.ip, "10.0.0.8");
    assert_eq!(host.ssh_port, 2222);

    let host_list: ListHostsResponse = harness
        .request_payload(
            CONTROL_HOSTS_LIST,
            ListHostsRequest {
                correlation_id: new_corr_id(),
                paging: None,
            },
        )
        .await?;
    assert_eq!(host_list.hosts.len(), 1);

    let group: HostGroup = harness
        .request_payload(
            CONTROL_HOST_GROUPS_CREATE,
            CreateHostGroupRequest {
                correlation_id: new_corr_id(),
                name: "linux-nodes".to_string(),
                description: "all linux".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(group.name, "linux-nodes");

    let member: HostGroupMember = harness
        .request_payload(
            CONTROL_HOST_GROUPS_ADD_MEMBER,
            AddHostGroupMemberRequest {
                correlation_id: new_corr_id(),
                host_group_id: group.host_group_id.clone(),
                host_id: host.host_id.clone(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(member.host_group_id, group.host_group_id);

    let groups: ListHostGroupsResponse = harness
        .request_payload(
            CONTROL_HOST_GROUPS_LIST,
            ListHostGroupsRequest {
                correlation_id: new_corr_id(),
                paging: None,
            },
        )
        .await?;
    assert_eq!(groups.groups.len(), 1);
    assert_eq!(groups.groups[0].members.len(), 1);

    harness
        .request_ack(
            CONTROL_HOST_GROUPS_REMOVE_MEMBER,
            RemoveHostGroupMemberRequest {
                correlation_id: new_corr_id(),
                host_group_id: group.host_group_id.clone(),
                host_id: host.host_id.clone(),
                audit: test_audit(),
            },
        )
        .await?;

    let updated_group: HostGroup = harness
        .request_payload(
            CONTROL_HOST_GROUPS_UPDATE,
            UpdateHostGroupRequest {
                correlation_id: new_corr_id(),
                host_group_id: group.host_group_id.clone(),
                name: "linux-critical".to_string(),
                description: "critical linux".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(updated_group.name, "linux-critical");
    assert!(updated_group.members.is_empty());

    let credential: CredentialProfileMetadata = harness
        .request_payload(
            CONTROL_CREDENTIALS_CREATE,
            CreateCredentialsRequest {
                correlation_id: new_corr_id(),
                name: "ssh-default".to_string(),
                kind: "ssh_key".to_string(),
                description: "Default SSH key".to_string(),
                vault_ref: "secret/data/ssh/default".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(credential.kind, "ssh_key");

    let listed: ListCredentialsResponse = harness
        .request_payload(
            CONTROL_CREDENTIALS_LIST,
            ListCredentialsRequest {
                correlation_id: new_corr_id(),
                paging: None,
            },
        )
        .await?;
    assert_eq!(listed.profiles.len(), 1);

    let fetched: CredentialProfileMetadata = harness
        .request_payload(
            CONTROL_CREDENTIALS_GET,
            GetCredentialsRequest {
                correlation_id: new_corr_id(),
                credentials_profile_id: credential.credentials_profile_id.clone(),
            },
        )
        .await?;
    assert_eq!(fetched.vault_ref, "secret/data/ssh/default");

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
#[ignore]
#[serial]
async fn clusters_rbac_integrations_tickets_anomalies_flow() -> anyhow::Result<()> {
    let harness = TestHarness::start().await?;

    let host: Host = harness
        .request_payload(
            CONTROL_HOSTS_CREATE,
            CreateHostRequest {
                correlation_id: new_corr_id(),
                host: Some(HostInput {
                    hostname: "cluster-host-1".to_string(),
                    ip: "10.20.0.5".to_string(),
                    ssh_port: 22,
                    remote_user: "root".to_string(),
                    labels: [("role".to_string(), "ops".to_string())]
                        .into_iter()
                        .collect(),
                }),
                audit: test_audit(),
            },
        )
        .await?;

    let cluster_response: GetClusterResponse = harness
        .request_payload(
            CONTROL_CLUSTERS_CREATE,
            CreateClusterRequest {
                correlation_id: new_corr_id(),
                name: "prod-cluster".to_string(),
                slug: "prod-cluster".to_string(),
                description: "Production scope".to_string(),
                is_active: true,
                metadata_json: json!({ "tier": "prod" }).to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    let cluster_details = cluster_response
        .cluster
        .and_then(|details| details.cluster)
        .ok_or_else(|| anyhow!("cluster not returned"))?;
    let cluster_id = cluster_details.cluster_id.clone();

    harness
        .request_ack(
            CONTROL_CLUSTERS_ADD_HOST,
            ClusterHostMutationRequest {
                correlation_id: new_corr_id(),
                cluster_id: cluster_id.clone(),
                host_id: host.host_id.clone(),
                audit: test_audit(),
            },
        )
        .await?;

    let fetched: GetClusterResponse = harness
        .request_payload(
            CONTROL_CLUSTERS_GET,
            GetClusterRequest {
                correlation_id: new_corr_id(),
                cluster_id: cluster_id.clone(),
                include_members: true,
            },
        )
        .await?;
    let host_bindings = fetched
        .cluster
        .map(|details| details.hosts)
        .unwrap_or_default();
    assert_eq!(host_bindings.len(), 1);

    let listed: ListClustersResponse = harness
        .request_payload(
            CONTROL_CLUSTERS_LIST,
            ListClustersRequest {
                correlation_id: new_corr_id(),
                paging: None,
                query: String::new(),
                host_id: host.host_id.clone(),
                include_members: false,
            },
        )
        .await?;
    assert_eq!(listed.clusters.len(), 1);
    assert_eq!(listed.clusters[0].host_count, 1);

    let role: Role = harness
        .request_payload(
            CONTROL_ROLES_CREATE,
            CreateRoleRequest {
                correlation_id: new_corr_id(),
                name: "cluster-admin".to_string(),
                slug: "cluster-admin".to_string(),
                description: "Manages prod cluster".to_string(),
                audit: test_audit(),
            },
        )
        .await?;

    let perms: GetRolePermissionsResponse = harness
        .request_payload(
            CONTROL_ROLES_PERMISSIONS_SET,
            SetRolePermissionsRequest {
                correlation_id: new_corr_id(),
                role_id: role.role_id.clone(),
                permission_codes: vec![
                    "clusters.view".to_string(),
                    "clusters.manage".to_string(),
                    "tickets.manage".to_string(),
                ],
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(perms.permissions.len(), 3);

    let binding: RoleBinding = harness
        .request_payload(
            CONTROL_ROLE_BINDINGS_CREATE,
            CreateRoleBindingRequest {
                correlation_id: new_corr_id(),
                user_id: "ops@example.com".to_string(),
                role_id: role.role_id.clone(),
                scope_type: "cluster".to_string(),
                scope_id: cluster_id.clone(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(binding.scope_id, cluster_id);

    let integration: Integration = harness
        .request_payload(
            CONTROL_INTEGRATIONS_CREATE,
            CreateIntegrationRequest {
                correlation_id: new_corr_id(),
                name: "ops-telegram".to_string(),
                kind: "telegram_bot".to_string(),
                description: "Telegram on-call".to_string(),
                config_json: json!({ "token": "fake" }).to_string(),
                is_active: true,
                audit: test_audit(),
            },
        )
        .await?;

    let integration_binding: IntegrationBinding = harness
        .request_payload(
            CONTROL_INTEGRATIONS_BIND,
            BindIntegrationRequest {
                correlation_id: new_corr_id(),
                integration_id: integration.integration_id.clone(),
                scope_type: "cluster".to_string(),
                scope_id: cluster_id.clone(),
                event_types_json: json!(["ticket.created"]).to_string(),
                severity_threshold: "high".to_string(),
                is_active: true,
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(integration_binding.scope_id, cluster_id);

    let ticket_response: GetTicketResponse = harness
        .request_payload(
            TICKETS_CREATE,
            CreateTicketRequest {
                correlation_id: new_corr_id(),
                title: "Disk pressure".to_string(),
                description: "rootfs at 95%".to_string(),
                cluster_id: cluster_id.clone(),
                source_type: "manual".to_string(),
                source_id: "ops-test".to_string(),
                severity: "high".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    let ticket = ticket_response
        .ticket
        .and_then(|details| details.ticket)
        .ok_or_else(|| anyhow!("ticket payload missing"))?;
    let ticket_id = ticket.ticket_id.clone();

    let assigned: TicketDetails = harness
        .request_payload(
            TICKETS_ASSIGN,
            AssignTicketRequest {
                correlation_id: new_corr_id(),
                ticket_id: ticket_id.clone(),
                assignee_user_id: "ops@example.com".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(
        assigned
            .ticket
            .as_ref()
            .map(|t| t.assignee_user_id.clone()),
        Some("ops@example.com".to_string())
    );

    let commented: TicketDetails = harness
        .request_payload(
            TICKETS_COMMENT_ADD,
            AddTicketCommentRequest {
                correlation_id: new_corr_id(),
                ticket_id: ticket_id.clone(),
                body: "Investigating disk usage".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(commented.comments.len(), 1);

    let in_progress: TicketDetails = harness
        .request_payload(
            TICKETS_STATUS_CHANGE,
            ChangeTicketStatusRequest {
                correlation_id: new_corr_id(),
                ticket_id: ticket_id.clone(),
                status: "in_progress".to_string(),
                resolution: String::new(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(
        in_progress
            .ticket
            .as_ref()
            .map(|t| t.status.clone()),
        Some("in_progress".to_string())
    );

    let closed: TicketDetails = harness
        .request_payload(
            TICKETS_CLOSE,
            CloseTicketRequest {
                correlation_id: new_corr_id(),
                ticket_id: ticket_id.clone(),
                resolution: "Cleaned tmp".to_string(),
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(
        closed
            .ticket
            .as_ref()
            .map(|t| t.status.clone()),
        Some("closed".to_string())
    );

    let rule: AnomalyRule = harness
        .request_payload(
            ANOMALIES_RULES_CREATE,
            CreateAnomalyRuleRequest {
                correlation_id: new_corr_id(),
                name: "CPU spike".to_string(),
                kind: "threshold".to_string(),
                scope_type: "cluster".to_string(),
                scope_id: cluster_id.clone(),
                config_json: json!({ "metric": "cpu", "threshold": 90 }).to_string(),
                is_active: true,
                audit: test_audit(),
            },
        )
        .await?;

    let updated_rule: AnomalyRule = harness
        .request_payload(
            ANOMALIES_RULES_UPDATE,
            UpdateAnomalyRuleRequest {
                correlation_id: new_corr_id(),
                anomaly_rule_id: rule.anomaly_rule_id.clone(),
                name: "CPU spike severe".to_string(),
                config_json: json!({ "metric": "cpu", "threshold": 95 }).to_string(),
                is_active: true,
                audit: test_audit(),
            },
        )
        .await?;
    assert_eq!(updated_rule.name, "CPU spike severe");

    let rule_uuid = Uuid::parse_str(&rule.anomaly_rule_id)?;
    let cluster_uuid = Uuid::parse_str(&cluster_id)?;
    sqlx::query(
        "INSERT INTO anomaly_instances (id, rule_id, cluster_id, severity, status, started_at, payload_json)
         VALUES ($1, $2, $3, $4, $5, NOW(), $6)",
    )
    .bind(Uuid::new_v4())
    .bind(rule_uuid)
    .bind(cluster_uuid)
    .bind("critical")
    .bind("open")
    .bind(json!({ "source": "test" }))
    .execute(&harness.pool)
    .await?;

    let instances: ListAnomalyInstancesResponse = harness
        .request_payload(
            ANOMALIES_INSTANCES_LIST,
            ListAnomalyInstancesRequest {
                correlation_id: new_corr_id(),
                paging: None,
                anomaly_rule_id: rule.anomaly_rule_id.clone(),
                cluster_id: cluster_id.clone(),
                status: "open".to_string(),
            },
        )
        .await?;
    assert_eq!(instances.instances.len(), 1);
    assert_eq!(instances.instances[0].severity, "critical");

    harness.shutdown().await;
    Ok(())
}

fn new_corr_id() -> String {
    Uuid::new_v4().to_string()
}

async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let migrations_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("migrations");
    let migrator = sqlx::migrate::Migrator::new(migrations_path.as_path()).await?;
    migrator.run(pool).await?;
    Ok(())
}

async fn truncate_tables(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "TRUNCATE TABLE
            control_audit_events,
            anomaly_instances,
            anomaly_rules,
            ticket_events,
            ticket_comments,
            tickets,
            integration_bindings,
            integrations,
            role_permissions,
            user_role_bindings,
            roles,
            permissions,
            cluster_hosts,
            cluster_agents,
            cluster_metadata,
            clusters,
            host_group_members,
            host_groups,
            hosts,
            credentials_profiles_metadata,
            policy_revisions,
            policies
        RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await?;
    Ok(())
}
