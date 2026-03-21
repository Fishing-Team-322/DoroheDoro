use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{bail, ensure};
use async_nats::{Client, Subscriber};
use async_trait::async_trait;
use axum::{body::Body, http::Request};
use common::{
    nats_subjects::{
        CONTROL_CREDENTIALS_CREATE, CONTROL_HOSTS_CREATE, CONTROL_HOST_GROUPS_ADD_MEMBER,
        CONTROL_HOST_GROUPS_CREATE, CONTROL_POLICIES_CREATE, DEPLOYMENTS_JOBS_CANCEL,
        DEPLOYMENTS_JOBS_CREATE, DEPLOYMENTS_JOBS_GET, DEPLOYMENTS_JOBS_LIST,
        DEPLOYMENTS_JOBS_RETRY, DEPLOYMENTS_JOBS_STATUS, DEPLOYMENTS_JOBS_STEP,
        DEPLOYMENTS_PLAN_CREATE,
    },
    proto::{
        control::{
            AddHostGroupMemberRequest, ControlReplyEnvelope, CreateCredentialsRequest,
            CreateHostGroupRequest, CreateHostRequest, CreatePolicyRequest,
            CredentialProfileMetadata, Host, HostGroup, HostGroupMember, HostInput, Policy,
        },
        decode_message,
        deployment::{
            self, CancelDeploymentJobRequest, CreateDeploymentJobRequest,
            CreateDeploymentPlanRequest, CreateDeploymentPlanResponse, DeploymentReplyEnvelope,
            DeploymentStatusEvent, DeploymentStepEvent, GetDeploymentJobRequest,
            GetDeploymentJobResponse, ListDeploymentJobsRequest, ListDeploymentJobsResponse,
            RetryDeploymentJobRequest,
        },
        encode_message,
    },
    RuntimeConfig,
};
use control_plane::{
    http as control_http, repository::ControlRepository, service::ControlService,
    transport as control_transport,
};
use deployment_plane::{
    config::DeploymentConfig,
    executor::{
        traits::DeploymentExecutor, DynDeploymentExecutor, MockExecutor, MockExecutorOptions,
    },
    health::{self, HealthState},
    models::{
        DeploymentSnapshot, DeploymentStepStatus, DeploymentTargetSnapshot, DeploymentTargetStatus,
        ExecutorKind, StepExecutionResult, TargetExecutionResult,
    },
    repository::DeploymentRepository,
    service::DeploymentService,
    transport as deployment_transport,
};
use enrollment_plane::{
    http as enrollment_http, repository::EnrollmentRepository, service::EnrollmentService,
    transport as enrollment_transport,
};
use futures::StreamExt;
use reqwest::StatusCode;
use serde_json::json;
use serial_test::serial;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{net::TcpListener, task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ScenarioExecutor {
    step_delay: Duration,
    fail_first_for: BTreeSet<String>,
    attempts_by_host: Arc<Mutex<HashMap<String, usize>>>,
}

impl ScenarioExecutor {
    fn new(
        step_delay: Duration,
        fail_first_for: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            step_delay,
            fail_first_for: fail_first_for.into_iter().map(Into::into).collect(),
            attempts_by_host: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl DeploymentExecutor for ScenarioExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Mock
    }

    async fn readiness_check(&self) -> common::AppResult<()> {
        Ok(())
    }

    async fn execute_target(
        &self,
        snapshot: &DeploymentSnapshot,
        target: &DeploymentTargetSnapshot,
        cancellation: &CancellationToken,
    ) -> common::AppResult<TargetExecutionResult> {
        let execution_no = {
            let mut guard = self.attempts_by_host.lock().expect("executor state lock");
            let entry = guard.entry(target.host.hostname.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let mut steps = Vec::new();
        for (idx, step_name) in ["scenario.connect", "scenario.install", "scenario.verify"]
            .into_iter()
            .enumerate()
        {
            sleep(self.step_delay).await;
            if cancellation.is_cancelled() {
                steps.push(StepExecutionResult {
                    step_name: step_name.to_string(),
                    status: DeploymentStepStatus::Skipped,
                    message: "execution cancelled".to_string(),
                    payload_json: json!({
                        "hostname": target.host.hostname,
                        "job_type": snapshot.job_type.as_str(),
                        "step_index": idx,
                    }),
                });
                return Ok(TargetExecutionResult {
                    status: DeploymentTargetStatus::Cancelled,
                    error_message: Some("cancelled during execution".to_string()),
                    steps,
                });
            }

            let should_fail = execution_no == 1
                && step_name == "scenario.install"
                && self.fail_first_for.contains(&target.host.hostname);
            let status = if should_fail {
                DeploymentStepStatus::Failed
            } else {
                DeploymentStepStatus::Succeeded
            };
            let message = if should_fail {
                "scenario executor forced first-attempt failure"
            } else {
                "step completed"
            };

            steps.push(StepExecutionResult {
                step_name: step_name.to_string(),
                status,
                message: message.to_string(),
                payload_json: json!({
                    "hostname": target.host.hostname,
                    "job_type": snapshot.job_type.as_str(),
                    "step_index": idx,
                    "execution_no": execution_no,
                }),
            });

            if should_fail {
                return Ok(TargetExecutionResult {
                    status: DeploymentTargetStatus::Failed,
                    error_message: Some(format!(
                        "scenario executor failed host {} on first attempt",
                        target.host.hostname
                    )),
                    steps,
                });
            }
        }

        Ok(TargetExecutionResult {
            status: DeploymentTargetStatus::Succeeded,
            error_message: None,
            steps,
        })
    }
}

struct TestHarness {
    control_http_addr: String,
    enrollment_http_addr: String,
    deployment_http_addr: String,
    http_client: reqwest::Client,
    nats: Client,
    pool: PgPool,
    shutdown: CancellationToken,
    server_tasks: Vec<JoinHandle<()>>,
    subscriber_tasks: Vec<JoinHandle<()>>,
}

impl TestHarness {
    async fn start(executor: DynDeploymentExecutor) -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let runtime = RuntimeConfig::from_env()?;
        let deployment = DeploymentConfig::from_env()?;

        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(&runtime.postgres_dsn)
            .await?;
        run_migrations(&pool).await?;
        truncate_tables(&pool).await?;

        let nats = async_nats::connect(&runtime.nats_url).await?;
        let shutdown = CancellationToken::new();

        let mut server_tasks = Vec::new();
        let mut subscriber_tasks = Vec::new();

        let control_repo = ControlRepository::new(pool.clone());
        let control_service = Arc::new(ControlService::new(control_repo));
        subscriber_tasks.extend(
            control_transport::spawn_handlers(nats.clone(), control_service, shutdown.clone())
                .await?,
        );
        let control_listener = TcpListener::bind("127.0.0.1:0").await?;
        let control_addr = format!("http://{}", control_listener.local_addr()?);
        let control_shutdown = shutdown.clone();
        let control_pool = pool.clone();
        let control_nats = nats.clone();
        server_tasks.push(tokio::spawn(async move {
            let _ = axum::serve(
                control_listener,
                control_http::router(control_http::HttpState::new(control_pool, control_nats)),
            )
            .with_graceful_shutdown(async move {
                control_shutdown.cancelled().await;
            })
            .await;
        }));

        let enrollment_repo = EnrollmentRepository::new(pool.clone());
        let enrollment_service = Arc::new(EnrollmentService::new(enrollment_repo));
        subscriber_tasks.extend(
            enrollment_transport::spawn_handlers(
                nats.clone(),
                enrollment_service,
                shutdown.clone(),
            )
            .await?,
        );
        let enrollment_listener = TcpListener::bind("127.0.0.1:0").await?;
        let enrollment_addr = format!("http://{}", enrollment_listener.local_addr()?);
        let enrollment_shutdown = shutdown.clone();
        let enrollment_pool = pool.clone();
        let enrollment_nats = nats.clone();
        server_tasks.push(tokio::spawn(async move {
            let _ = axum::serve(
                enrollment_listener,
                enrollment_http::router(enrollment_http::HttpState::new(
                    enrollment_pool,
                    enrollment_nats,
                )),
            )
            .with_graceful_shutdown(async move {
                enrollment_shutdown.cancelled().await;
            })
            .await;
        }));

        let deployment_repo = DeploymentRepository::new(pool.clone());
        let deployment_service = Arc::new(DeploymentService::new(
            deployment_repo,
            nats.clone(),
            executor.clone(),
            deployment.edge_public_url,
            deployment.edge_grpc_addr,
            deployment.agent_state_dir_default,
        ));
        deployment_service.reconcile_stale_attempts().await?;
        subscriber_tasks.extend(
            deployment_transport::spawn_handlers(
                nats.clone(),
                deployment_service,
                shutdown.clone(),
            )
            .await?,
        );
        let deployment_listener = TcpListener::bind("127.0.0.1:0").await?;
        let deployment_addr = format!("http://{}", deployment_listener.local_addr()?);
        let deployment_shutdown = shutdown.clone();
        let deployment_pool = pool.clone();
        let deployment_nats = nats.clone();
        let deployment_executor = executor.clone();
        server_tasks.push(tokio::spawn(async move {
            let _ = axum::serve(
                deployment_listener,
                health::router(HealthState::new(
                    deployment_pool,
                    deployment_nats,
                    deployment_executor,
                )),
            )
            .with_graceful_shutdown(async move {
                deployment_shutdown.cancelled().await;
            })
            .await;
        }));

        let harness = Self {
            control_http_addr: control_addr,
            enrollment_http_addr: enrollment_addr,
            deployment_http_addr: deployment_addr,
            http_client: reqwest::Client::new(),
            nats,
            pool,
            shutdown,
            server_tasks,
            subscriber_tasks,
        };

        harness.wait_until_ready(&harness.control_http_addr).await?;
        harness
            .wait_until_ready(&harness.enrollment_http_addr)
            .await?;
        harness
            .wait_until_ready(&harness.deployment_http_addr)
            .await?;
        Ok(harness)
    }

    async fn wait_until_ready(&self, http_addr: &str) -> anyhow::Result<()> {
        for _ in 0..60 {
            if let Ok(response) = self
                .http_client
                .get(format!("{http_addr}/readyz"))
                .send()
                .await
            {
                if response.status() == StatusCode::OK {
                    return Ok(());
                }
            }

            sleep(Duration::from_millis(100)).await;
        }

        bail!("service at {http_addr} did not become ready in time");
    }

    async fn shutdown(self) {
        self.shutdown.cancel();
        for task in self.server_tasks {
            task.abort();
            let _ = task.await;
        }
        for task in self.subscriber_tasks {
            task.abort();
            let _ = task.await;
        }
    }

    async fn request_control_payload<Req, Resp>(
        &self,
        subject: &str,
        request: Req,
    ) -> anyhow::Result<Resp>
    where
        Req: prost::Message,
        Resp: prost::Message + Default,
    {
        let message = self
            .nats
            .request(subject.to_string(), encode_message(&request).into())
            .await?;
        let envelope: ControlReplyEnvelope = decode_message(message.payload.as_ref())?;
        ensure!(
            envelope.status == "ok",
            "control subject {subject} failed: {} {}",
            envelope.code,
            envelope.message
        );
        Ok(decode_message(&envelope.payload)?)
    }

    async fn request_deployment_payload<Req, Resp>(
        &self,
        subject: &str,
        request: Req,
    ) -> anyhow::Result<Resp>
    where
        Req: prost::Message,
        Resp: prost::Message + Default,
    {
        let message = self
            .nats
            .request(subject.to_string(), encode_message(&request).into())
            .await?;
        let envelope: DeploymentReplyEnvelope = decode_message(message.payload.as_ref())?;
        ensure!(
            envelope.status == "ok",
            "deployment subject {subject} failed: {} {}",
            envelope.code,
            envelope.message
        );
        Ok(decode_message(&envelope.payload)?)
    }
}

#[tokio::test]
#[ignore]
#[serial]
async fn health_and_readiness_work() -> anyhow::Result<()> {
    let harness =
        TestHarness::start(Arc::new(MockExecutor::new(MockExecutorOptions::default()))).await?;

    for http_addr in [
        &harness.control_http_addr,
        &harness.enrollment_http_addr,
        &harness.deployment_http_addr,
    ] {
        let health = harness
            .http_client
            .get(format!("{http_addr}/healthz"))
            .send()
            .await?;
        assert_eq!(health.status(), StatusCode::OK);

        let ready = harness
            .http_client
            .get(format!("{http_addr}/readyz"))
            .send()
            .await?;
        assert_eq!(ready.status(), StatusCode::OK);
    }

    let bad_pool =
        PgPoolOptions::new().connect_lazy("postgres://postgres:postgres@127.0.0.1:1/doro")?;
    let response = health::router(HealthState::new(
        bad_pool,
        harness.nats.clone(),
        Arc::new(MockExecutor::new(MockExecutorOptions::default())),
    ))
    .oneshot(Request::builder().uri("/readyz").body(Body::empty())?)
    .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
#[ignore]
#[serial]
async fn deployment_plan_job_retry_and_cancel_flow() -> anyhow::Result<()> {
    let executor: DynDeploymentExecutor = Arc::new(ScenarioExecutor::new(
        Duration::from_millis(150),
        ["retry-host"],
    ));
    let harness = TestHarness::start(executor).await?;

    let policy: Policy = harness
        .request_control_payload(
            CONTROL_POLICIES_CREATE,
            CreatePolicyRequest {
                correlation_id: new_corr_id(),
                name: "linux-files".to_string(),
                description: "File collection".to_string(),
                policy_body_json: r#"{"paths":["/var/log/syslog"]}"#.to_string(),
            },
        )
        .await?;

    let retry_host: Host = harness
        .request_control_payload(
            CONTROL_HOSTS_CREATE,
            CreateHostRequest {
                correlation_id: new_corr_id(),
                host: Some(HostInput {
                    hostname: "retry-host".to_string(),
                    ip: "10.0.0.11".to_string(),
                    ssh_port: 22,
                    remote_user: "root".to_string(),
                    labels: [("env".to_string(), "smoke".to_string())]
                        .into_iter()
                        .collect(),
                }),
            },
        )
        .await?;

    let cancel_host: Host = harness
        .request_control_payload(
            CONTROL_HOSTS_CREATE,
            CreateHostRequest {
                correlation_id: new_corr_id(),
                host: Some(HostInput {
                    hostname: "cancel-host".to_string(),
                    ip: "10.0.0.12".to_string(),
                    ssh_port: 22,
                    remote_user: "root".to_string(),
                    labels: [("env".to_string(), "smoke".to_string())]
                        .into_iter()
                        .collect(),
                }),
            },
        )
        .await?;

    let host_group: HostGroup = harness
        .request_control_payload(
            CONTROL_HOST_GROUPS_CREATE,
            CreateHostGroupRequest {
                correlation_id: new_corr_id(),
                name: "retry-group".to_string(),
                description: "retry hosts".to_string(),
            },
        )
        .await?;

    let _: HostGroupMember = harness
        .request_control_payload(
            CONTROL_HOST_GROUPS_ADD_MEMBER,
            AddHostGroupMemberRequest {
                correlation_id: new_corr_id(),
                host_group_id: host_group.host_group_id.clone(),
                host_id: retry_host.host_id.clone(),
            },
        )
        .await?;

    let credentials: CredentialProfileMetadata = harness
        .request_control_payload(
            CONTROL_CREDENTIALS_CREATE,
            CreateCredentialsRequest {
                correlation_id: new_corr_id(),
                name: "ssh-smoke".to_string(),
                kind: "ssh_key".to_string(),
                description: "Smoke SSH key".to_string(),
                vault_ref: "secret/data/ssh/smoke".to_string(),
            },
        )
        .await?;

    let plan: CreateDeploymentPlanResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_PLAN_CREATE,
            CreateDeploymentPlanRequest {
                correlation_id: new_corr_id(),
                job_type: deployment::DeploymentJobType::Install as i32,
                policy_id: policy.policy_id.clone(),
                target_host_ids: Vec::new(),
                target_host_group_ids: vec![host_group.host_group_id.clone()],
                credential_profile_id: credentials.credentials_profile_id.clone(),
                requested_by: "smoke-user".to_string(),
                preserve_state: false,
                force: false,
                dry_run: true,
            },
        )
        .await?;
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].hostname, "retry-host");
    assert_eq!(plan.executor_kind, deployment::ExecutorKind::Mock as i32);
    assert!(plan.warnings.is_empty());
    assert!(plan.bootstrap_previews[0]
        .bootstrap_yaml
        .contains("bootstrap_token: <preview-token>"));

    let mut status_sub = harness
        .nats
        .subscribe(DEPLOYMENTS_JOBS_STATUS.to_string())
        .await?;
    let mut step_sub = harness
        .nats
        .subscribe(DEPLOYMENTS_JOBS_STEP.to_string())
        .await?;
    harness.nats.flush().await?;

    let create_response: deployment::CreateDeploymentJobResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_JOBS_CREATE,
            CreateDeploymentJobRequest {
                correlation_id: new_corr_id(),
                job_type: deployment::DeploymentJobType::Install as i32,
                policy_id: policy.policy_id.clone(),
                target_host_ids: Vec::new(),
                target_host_group_ids: vec![host_group.host_group_id.clone()],
                credential_profile_id: credentials.credentials_profile_id.clone(),
                requested_by: "smoke-user".to_string(),
                preserve_state: false,
                force: false,
                dry_run: false,
            },
        )
        .await?;
    let job = create_response.job.expect("job summary");
    let job_id = job.job_id.clone();

    let failed_status = wait_for_status_event(
        &mut status_sub,
        &job_id,
        deployment::DeploymentJobStatus::Failed as i32,
    )
    .await?;
    assert_eq!(failed_status.failed_targets, 1);

    let finished_step = wait_for_step_event(&mut step_sub, &job_id, "attempt.finished").await?;
    assert_eq!(
        finished_step.status,
        deployment::DeploymentStepStatus::Failed as i32
    );

    let failed_job: GetDeploymentJobResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_JOBS_GET,
            GetDeploymentJobRequest {
                correlation_id: new_corr_id(),
                job_id: job_id.clone(),
            },
        )
        .await?;
    let failed_summary = failed_job.job.expect("failed job summary");
    assert_eq!(
        failed_summary.status,
        deployment::DeploymentJobStatus::Failed as i32
    );
    assert_eq!(failed_summary.failed_targets, 1);
    assert_eq!(failed_job.attempts.len(), 1);
    assert_eq!(failed_job.targets.len(), 1);
    assert!(failed_job.steps.len() >= 4);

    let retry_response: deployment::RetryDeploymentJobResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_JOBS_RETRY,
            RetryDeploymentJobRequest {
                correlation_id: new_corr_id(),
                job_id: job_id.clone(),
                strategy: deployment::RetryStrategy::FailedOnly as i32,
                triggered_by: "smoke-retry".to_string(),
                reason: "retry failed targets".to_string(),
            },
        )
        .await?;
    let retry_job = retry_response.job.expect("retry job summary");
    assert_eq!(retry_job.job_id, job_id);

    let succeeded_status = wait_for_status_event(
        &mut status_sub,
        &job_id,
        deployment::DeploymentJobStatus::Succeeded as i32,
    )
    .await?;
    assert_eq!(succeeded_status.succeeded_targets, 1);

    let succeeded_job: GetDeploymentJobResponse = eventually(Duration::from_secs(10), || {
        let harness = &harness;
        let job_id = job_id.clone();
        async move {
            let response: GetDeploymentJobResponse = harness
                .request_deployment_payload(
                    DEPLOYMENTS_JOBS_GET,
                    GetDeploymentJobRequest {
                        correlation_id: new_corr_id(),
                        job_id,
                    },
                )
                .await?;
            let summary = response.job.clone().expect("job summary");
            ensure!(
                summary.status == deployment::DeploymentJobStatus::Succeeded as i32,
                "job not succeeded yet"
            );
            Ok(response)
        }
    })
    .await?;
    let succeeded_summary = succeeded_job.job.expect("succeeded job summary");
    assert_eq!(succeeded_summary.attempt_count, 2);
    assert_eq!(succeeded_job.attempts.len(), 2);
    assert_eq!(succeeded_job.targets.len(), 1);
    assert!(succeeded_job.steps.len() >= 5);

    let attempts_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deployment_attempts WHERE deployment_job_id = $1",
    )
    .bind(Uuid::parse_str(&job_id)?)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(attempts_count, 2);

    let targets_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deployment_targets WHERE deployment_job_id = $1",
    )
    .bind(Uuid::parse_str(&job_id)?)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(targets_count, 2);

    let steps_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deployment_steps WHERE deployment_job_id = $1",
    )
    .bind(Uuid::parse_str(&job_id)?)
    .fetch_one(&harness.pool)
    .await?;
    assert!(steps_count >= 9);

    let pinned_policy_revision_ids = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT policy_revision_id::text FROM enrollment_tokens ORDER BY 1",
    )
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(
        pinned_policy_revision_ids,
        vec![succeeded_summary.policy_revision_id.clone()]
    );

    let list_response: ListDeploymentJobsResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_JOBS_LIST,
            ListDeploymentJobsRequest {
                correlation_id: new_corr_id(),
                status: deployment::DeploymentJobStatus::Succeeded as i32,
                job_type: deployment::DeploymentJobType::Install as i32,
                requested_by: "smoke-user".to_string(),
                created_after: String::new(),
                created_before: String::new(),
                limit: 10,
                offset: 0,
            },
        )
        .await?;
    assert_eq!(list_response.total, 1);
    assert_eq!(list_response.jobs.len(), 1);
    assert_eq!(list_response.jobs[0].job_id, job_id);

    let cancel_create: deployment::CreateDeploymentJobResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_JOBS_CREATE,
            CreateDeploymentJobRequest {
                correlation_id: new_corr_id(),
                job_type: deployment::DeploymentJobType::Install as i32,
                policy_id: policy.policy_id,
                target_host_ids: vec![cancel_host.host_id.clone()],
                target_host_group_ids: Vec::new(),
                credential_profile_id: credentials.credentials_profile_id,
                requested_by: "smoke-user".to_string(),
                preserve_state: false,
                force: false,
                dry_run: false,
            },
        )
        .await?;
    let cancel_job_id = cancel_create.job.expect("cancel job").job_id;

    sleep(Duration::from_millis(50)).await;

    let _: deployment::CancelDeploymentJobResponse = harness
        .request_deployment_payload(
            DEPLOYMENTS_JOBS_CANCEL,
            CancelDeploymentJobRequest {
                correlation_id: new_corr_id(),
                job_id: cancel_job_id.clone(),
                requested_by: "smoke-user".to_string(),
                reason: "cancel smoke job".to_string(),
            },
        )
        .await?;

    let cancelled_status = wait_for_status_event(
        &mut status_sub,
        &cancel_job_id,
        deployment::DeploymentJobStatus::Cancelled as i32,
    )
    .await?;
    assert_eq!(cancelled_status.cancelled_targets, 1);

    let cancelled_job: GetDeploymentJobResponse = eventually(Duration::from_secs(10), || {
        let harness = &harness;
        let cancel_job_id = cancel_job_id.clone();
        async move {
            let response: GetDeploymentJobResponse = harness
                .request_deployment_payload(
                    DEPLOYMENTS_JOBS_GET,
                    GetDeploymentJobRequest {
                        correlation_id: new_corr_id(),
                        job_id: cancel_job_id,
                    },
                )
                .await?;
            let summary = response.job.clone().expect("cancelled summary");
            ensure!(
                summary.status == deployment::DeploymentJobStatus::Cancelled as i32,
                "job not cancelled yet"
            );
            Ok(response)
        }
    })
    .await?;
    assert_eq!(
        cancelled_job.job.expect("cancelled job").status,
        deployment::DeploymentJobStatus::Cancelled as i32
    );

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
        "TRUNCATE TABLE deployment_steps, deployment_targets, deployment_attempts, deployment_jobs, agent_diagnostics, agent_policy_bindings, enrollment_tokens, credentials_profiles_metadata, host_group_members, host_groups, hosts, policy_revisions, policies, agents RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn wait_for_status_event(
    subscriber: &mut Subscriber,
    job_id: &str,
    expected_status: i32,
) -> anyhow::Result<DeploymentStatusEvent> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| Duration::from_millis(0));
        if remaining.is_zero() {
            bail!("timed out waiting for status event {expected_status} for job {job_id}");
        }

        let maybe_message = tokio::time::timeout(remaining, subscriber.next()).await?;
        let Some(message) = maybe_message else {
            bail!("status subscription closed while waiting for job {job_id}");
        };
        let event: DeploymentStatusEvent = decode_message(message.payload.as_ref())?;
        if event.job_id == job_id && event.status == expected_status {
            return Ok(event);
        }
    }
}

async fn wait_for_step_event(
    subscriber: &mut Subscriber,
    job_id: &str,
    step_name: &str,
) -> anyhow::Result<DeploymentStepEvent> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| Duration::from_millis(0));
        if remaining.is_zero() {
            bail!("timed out waiting for step event {step_name} for job {job_id}");
        }

        let maybe_message = tokio::time::timeout(remaining, subscriber.next()).await?;
        let Some(message) = maybe_message else {
            bail!("step subscription closed while waiting for job {job_id}");
        };
        let event: DeploymentStepEvent = decode_message(message.payload.as_ref())?;
        if event.job_id == job_id && event.step_name == step_name {
            return Ok(event);
        }
    }
}

async fn eventually<T, F, Fut>(timeout: Duration, mut check: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let started = Instant::now();
    loop {
        match check().await {
            Ok(value) => return Ok(value),
            Err(error) if started.elapsed() < timeout => {
                let _ = error;
                sleep(Duration::from_millis(100)).await;
            }
            Err(error) => return Err(error),
        }
    }
}
