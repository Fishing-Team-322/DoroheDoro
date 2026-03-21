use std::{collections::HashMap, sync::Arc};

use async_nats::Client;
use common::{
    nats_subjects::{AGENTS_BOOTSTRAP_TOKEN_ISSUE, DEPLOYMENTS_JOBS_STATUS, DEPLOYMENTS_JOBS_STEP},
    proto::{
        agent::{IssueBootstrapTokenRequest, IssueBootstrapTokenResponse},
        decode_message, deployment, encode_message, runtime,
    },
    AppError, AppResult,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    credentials::resolver::CredentialsResolver,
    executor::DynDeploymentExecutor,
    inventory::resolver::InventoryResolver,
    models::{
        parse_rfc3339_utc, DeploymentCreateSpec, DeploymentFlags, DeploymentJobPayload,
        DeploymentJobStatus, DeploymentPlan, DeploymentSnapshot, DeploymentStepStatus,
        DeploymentTargetRecord, DeploymentTargetStatus, ExecutionAuditContext, ExecutionResult,
        ExecutionSnapshot, ExecutionTarget, ListJobsFilter, RetryStrategy, RunningAttempt,
    },
    policy::resolver::PolicyResolver,
    render::{
        bootstrap::{build_bootstrap_config, render_bootstrap_yaml},
        inventory::render_target_vars,
        snapshot::{policy_to_source_paths, policy_to_source_paths_preview},
    },
    repository::DeploymentRepository,
};

#[derive(Clone)]
pub struct DeploymentService {
    repo: DeploymentRepository,
    nats: Client,
    inventory_resolver: InventoryResolver,
    policy_resolver: PolicyResolver,
    credentials_resolver: CredentialsResolver,
    executor: DynDeploymentExecutor,
    edge_public_url: String,
    edge_grpc_addr: String,
    agent_state_dir_default: String,
    execution_handles: Arc<Mutex<HashMap<Uuid, CancellationToken>>>,
}

impl DeploymentService {
    pub fn new(
        repo: DeploymentRepository,
        nats: Client,
        executor: DynDeploymentExecutor,
        edge_public_url: String,
        edge_grpc_addr: String,
        agent_state_dir_default: String,
    ) -> Self {
        Self {
            repo,
            inventory_resolver: InventoryResolver::new(nats.clone()),
            policy_resolver: PolicyResolver::new(nats.clone()),
            credentials_resolver: CredentialsResolver::new(nats.clone()),
            nats,
            executor,
            edge_public_url,
            edge_grpc_addr,
            agent_state_dir_default,
            execution_handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn reconcile_stale_attempts(&self) -> AppResult<()> {
        let reconciled = self
            .repo
            .reconcile_stale_attempts()
            .await
            .map_err(map_db_error)?;
        for job_id in reconciled {
            if let Some(job) = self.repo.get_job(job_id).await.map_err(map_db_error)? {
                self.publish_status(&job).await;
            }
        }
        Ok(())
    }

    pub async fn create_plan(
        &self,
        request: deployment::CreateDeploymentPlanRequest,
    ) -> AppResult<deployment::CreateDeploymentPlanResponse> {
        let spec = parse_create_spec(
            request.job_type,
            &request.policy_id,
            &request.target_host_ids,
            &request.target_host_group_ids,
            &request.credential_profile_id,
            &request.requested_by,
            request.preserve_state,
            request.force,
            request.dry_run,
        )?;
        let plan = self.build_plan_from_spec(&spec, false, false).await?;
        Ok(plan.into_proto())
    }

    pub async fn create_job(
        self: &Arc<Self>,
        request: deployment::CreateDeploymentJobRequest,
    ) -> AppResult<deployment::CreateDeploymentJobResponse> {
        let spec = parse_create_spec(
            request.job_type,
            &request.policy_id,
            &request.target_host_ids,
            &request.target_host_group_ids,
            &request.credential_profile_id,
            &request.requested_by,
            request.preserve_state,
            request.force,
            request.dry_run,
        )?;
        let plan = self.build_plan_from_spec(&spec, true, true).await?;
        let payload = DeploymentJobPayload {
            request: spec,
            snapshot: plan.snapshot.clone(),
        };
        let running = self
            .repo
            .create_job(&payload, self.executor.kind())
            .await
            .map_err(map_db_error)?;
        let job_id = running.job.id;
        self.publish_status(&running.job).await;
        self.spawn_execution(running).await;

        Ok(deployment::CreateDeploymentJobResponse {
            job: Some(
                self.repo
                    .get_job(job_id)
                    .await
                    .map_err(map_db_error)?
                    .ok_or_else(|| AppError::internal("deployment job disappeared after creation"))?
                    .into_proto(),
            ),
        })
    }

    pub async fn get_job(
        &self,
        request: deployment::GetDeploymentJobRequest,
    ) -> AppResult<deployment::GetDeploymentJobResponse> {
        let job_id = parse_uuid("job_id", &request.job_id)?;
        let view = self
            .repo
            .get_job_view(job_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("deployment job {job_id} not found")))?;
        Ok(view.into_proto())
    }

    pub async fn list_jobs(
        &self,
        request: deployment::ListDeploymentJobsRequest,
    ) -> AppResult<deployment::ListDeploymentJobsResponse> {
        let filter = ListJobsFilter {
            status: if request.status == deployment::DeploymentJobStatus::Unspecified as i32 {
                None
            } else {
                crate::models::DeploymentJobStatus::from_proto(request.status)
            },
            job_type: if request.job_type == deployment::DeploymentJobType::Unspecified as i32 {
                None
            } else {
                Some(crate::models::DeploymentJobType::from_proto(
                    request.job_type,
                )?)
            },
            requested_by: if request.requested_by.trim().is_empty() {
                None
            } else {
                Some(request.requested_by)
            },
            created_after: if request.created_after.trim().is_empty() {
                None
            } else {
                Some(parse_rfc3339_utc("created_after", &request.created_after)?)
            },
            created_before: if request.created_before.trim().is_empty() {
                None
            } else {
                Some(parse_rfc3339_utc(
                    "created_before",
                    &request.created_before,
                )?)
            },
            limit: if request.limit == 0 {
                50
            } else {
                request.limit.min(200)
            },
            offset: request.offset,
        };

        let (jobs, total) = self.repo.list_jobs(&filter).await.map_err(map_db_error)?;
        Ok(deployment::ListDeploymentJobsResponse {
            jobs: jobs.into_iter().map(|job| job.into_proto()).collect(),
            limit: filter.limit,
            offset: filter.offset,
            total,
        })
    }

    pub async fn retry_job(
        self: &Arc<Self>,
        request: deployment::RetryDeploymentJobRequest,
    ) -> AppResult<deployment::RetryDeploymentJobResponse> {
        let job_id = parse_uuid("job_id", &request.job_id)?;
        let strategy = RetryStrategy::from_proto(request.strategy)?;
        let view = self
            .repo
            .get_job_view(job_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("deployment job {job_id} not found")))?;
        if !view.job.status.is_terminal() {
            return Err(AppError::invalid_argument(
                "retry is allowed only for terminal deployment jobs",
            ));
        }

        let payload = self
            .repo
            .load_job_payload(job_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::internal("deployment payload not found"))?;
        let mut retry_spec = payload.request;
        retry_spec.requested_by = non_empty_or(&request.triggered_by, &view.job.requested_by);
        if strategy == RetryStrategy::FailedOnly {
            let failed_hosts = view
                .targets
                .iter()
                .filter(|target| target.status == DeploymentTargetStatus::Failed)
                .map(|target| target.host_id)
                .collect::<Vec<_>>();
            if failed_hosts.is_empty() {
                return Err(AppError::invalid_argument(
                    "retry failed_only requested but there are no failed targets",
                ));
            }
            retry_spec.target_host_ids = failed_hosts;
            retry_spec.target_host_group_ids.clear();
        }

        let plan = self.build_plan_from_spec(&retry_spec, true, true).await?;
        let running = self
            .repo
            .create_retry_attempt(
                job_id,
                &plan.snapshot,
                &non_empty_or(&request.triggered_by, &view.job.requested_by),
                &non_empty_or(&request.reason, "retry requested"),
                strategy,
                &view.targets,
            )
            .await
            .map_err(map_db_error)?;
        self.publish_status(&running.job).await;
        self.spawn_execution(running).await;

        Ok(deployment::RetryDeploymentJobResponse {
            job: Some(
                self.repo
                    .get_job(job_id)
                    .await
                    .map_err(map_db_error)?
                    .ok_or_else(|| AppError::internal("deployment job disappeared after retry"))?
                    .into_proto(),
            ),
        })
    }

    pub async fn cancel_job(
        self: &Arc<Self>,
        request: deployment::CancelDeploymentJobRequest,
    ) -> AppResult<deployment::CancelDeploymentJobResponse> {
        let job_id = parse_uuid("job_id", &request.job_id)?;
        let reason = non_empty_or(&request.reason, "cancel requested");
        let job = self
            .repo
            .get_job(job_id)
            .await
            .map_err(map_db_error)?
            .ok_or_else(|| AppError::not_found(format!("deployment job {job_id} not found")))?;

        if job.status.is_terminal() {
            return Ok(deployment::CancelDeploymentJobResponse {
                job: Some(job.into_proto()),
            });
        }

        let maybe_handle = self.execution_handles.lock().await.get(&job_id).cloned();

        let response_job = match job.status {
            DeploymentJobStatus::Queued => {
                if let Some(handle) = maybe_handle {
                    handle.cancel();
                }
                let cancelled = self
                    .repo
                    .force_cancel_job(job_id, &reason)
                    .await
                    .map_err(map_db_error)?
                    .ok_or_else(|| AppError::internal("job disappeared during cancellation"))?;
                self.publish_status(&cancelled).await;
                cancelled
            }
            DeploymentJobStatus::Running => {
                if let Some(handle) = maybe_handle {
                    handle.cancel();
                    let _ = self.executor.cancel(job_id).await;
                    self.repo
                        .get_job(job_id)
                        .await
                        .map_err(map_db_error)?
                        .ok_or_else(|| AppError::internal("job disappeared during cancellation"))?
                } else {
                    let cancelled = self
                        .repo
                        .force_cancel_job(job_id, &reason)
                        .await
                        .map_err(map_db_error)?
                        .ok_or_else(|| AppError::internal("job disappeared during cancellation"))?;
                    self.publish_status(&cancelled).await;
                    cancelled
                }
            }
            _ => job,
        };

        Ok(deployment::CancelDeploymentJobResponse {
            job: Some(response_job.into_proto()),
        })
    }

    async fn build_plan_from_spec(
        &self,
        spec: &DeploymentCreateSpec,
        issue_real_tokens: bool,
        strict_validation: bool,
    ) -> AppResult<DeploymentPlan> {
        let hosts = self
            .inventory_resolver
            .resolve(&spec.target_host_ids, &spec.target_host_group_ids)
            .await?;
        let policy = self.policy_resolver.resolve(spec.policy_id).await?;
        let credentials = self
            .credentials_resolver
            .resolve(spec.credential_profile_id)
            .await?;

        let (source_paths, warnings) = if strict_validation {
            (
                policy_to_source_paths(&policy.policy_body_json)?,
                Vec::new(),
            )
        } else {
            policy_to_source_paths_preview(&policy.policy_body_json)?
        };

        let mut targets = Vec::new();
        for host in hosts {
            let (token_id, bootstrap_token, expires_at) = if issue_real_tokens {
                let issued = self
                    .issue_bootstrap_token(
                        policy.policy_id,
                        policy.policy_revision_id,
                        &spec.requested_by,
                    )
                    .await?;
                (
                    issued.token_id,
                    issued.bootstrap_token,
                    chrono::DateTime::from_timestamp_millis(issued.expires_at_unix_ms)
                        .unwrap_or_else(chrono::Utc::now),
                )
            } else {
                (
                    "preview-token".to_string(),
                    "<preview-token>".to_string(),
                    chrono::Utc::now() + chrono::Duration::hours(1),
                )
            };

            let bootstrap_config = build_bootstrap_config(
                &self.edge_public_url,
                &self.edge_grpc_addr,
                &bootstrap_token,
                &self.agent_state_dir_default,
                "info",
                &source_paths,
            );
            let bootstrap_yaml = render_bootstrap_yaml(&bootstrap_config)?;
            let rendered_vars =
                render_target_vars(&host, spec.job_type.as_str(), "/etc/doro-agent/config.yaml");

            targets.push(crate::models::DeploymentTargetSnapshot {
                host,
                bootstrap: crate::models::BootstrapArtifact {
                    token_id,
                    bootstrap_token,
                    expires_at,
                    bootstrap_yaml,
                },
                rendered_vars,
            });
        }

        let snapshot = DeploymentSnapshot {
            job_type: spec.job_type,
            requested_by: spec.requested_by.clone(),
            policy,
            credentials: credentials.clone(),
            flags: spec.flags.clone(),
            targets,
            executor_kind: self.executor.kind(),
            created_at: chrono::Utc::now(),
        };

        Ok(DeploymentPlan {
            action_summary: format!(
                "{} {} targets via {} executor",
                snapshot.job_type.as_str(),
                snapshot.targets.len(),
                self.executor.kind().as_str()
            ),
            credential_summary: format!("{} ({})", credentials.name, credentials.kind),
            warnings,
            snapshot,
        })
    }

    async fn issue_bootstrap_token(
        &self,
        policy_id: Uuid,
        policy_revision_id: Uuid,
        requested_by: &str,
    ) -> AppResult<IssueBootstrapTokenResponse> {
        let request = IssueBootstrapTokenRequest {
            correlation_id: format!("bootstrap-token-{}", Uuid::new_v4()),
            policy_id: policy_id.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            requested_by: requested_by.to_string(),
            expires_at_unix_ms: (chrono::Utc::now() + chrono::Duration::hours(1))
                .timestamp_millis(),
        };

        let message = self
            .nats
            .request(
                AGENTS_BOOTSTRAP_TOKEN_ISSUE.to_string(),
                encode_message(&request).into(),
            )
            .await
            .map_err(|error| {
                AppError::internal(format!("issue bootstrap token request: {error}"))
            })?;
        let envelope: runtime::RuntimeReplyEnvelope = decode_message(message.payload.as_ref())?;
        if envelope.status != "ok" {
            return Err(match envelope.code.as_str() {
                "invalid_argument" => AppError::invalid_argument(envelope.message),
                "not_found" => AppError::not_found(envelope.message),
                _ => AppError::internal(format!(
                    "enrollment-plane token issue failed: {} {}",
                    envelope.code, envelope.message
                )),
            });
        }
        decode_message(&envelope.payload)
    }

    async fn spawn_execution(self: &Arc<Self>, running: RunningAttempt) {
        let cancellation = CancellationToken::new();
        self.execution_handles
            .lock()
            .await
            .insert(running.job.id, cancellation.clone());
        let service = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(error) = service.execute_attempt(running, cancellation).await {
                error!(error = %error, "deployment attempt execution failed");
            }
        });
    }

    async fn execute_attempt(
        self: Arc<Self>,
        running: RunningAttempt,
        cancellation: CancellationToken,
    ) -> AppResult<()> {
        let job_id = running.job.id;
        let result = self.run_attempt_inner(running, cancellation.clone()).await;
        self.execution_handles.lock().await.remove(&job_id);
        result
    }

    async fn run_attempt_inner(
        &self,
        running: RunningAttempt,
        cancellation: CancellationToken,
    ) -> AppResult<()> {
        let job_id = running.job.id;
        let attempt_id = running.attempt.id;

        if cancellation.is_cancelled() {
            if let Some(cancelled) = self
                .repo
                .force_cancel_job(job_id, "cancelled before execution started")
                .await
                .map_err(map_db_error)?
            {
                self.publish_status(&cancelled).await;
            }
            return Ok(());
        }

        let job = self
            .repo
            .mark_attempt_running(job_id, attempt_id, "running")
            .await
            .map_err(map_db_error)?;
        self.publish_status(&job).await;

        let start_step = self
            .repo
            .insert_step(
                job_id,
                attempt_id,
                None,
                "attempt.started",
                DeploymentStepStatus::Succeeded,
                "deployment attempt started",
                &serde_json::json!({ "executor_kind": self.executor.kind().as_str() }),
            )
            .await
            .map_err(map_db_error)?;
        self.publish_step(job_id, attempt_id, &start_step).await;
        let execution_snapshot = build_execution_snapshot(&running)?;
        for target in &running.targets {
            self.repo
                .mark_target_running(target.id)
                .await
                .map_err(map_db_error)?;
        }
        let job = self
            .repo
            .refresh_job_summary(
                job_id,
                Some(attempt_id),
                "executor.running",
                DeploymentJobStatus::Running,
            )
            .await
            .map_err(map_db_error)?;
        self.publish_status(&job).await;

        let execution_result = match self.executor.execute(&execution_snapshot, &cancellation).await
        {
            Ok(result) => result,
            Err(error) => executor_failure_result(&running, error),
        };

        for step in &execution_result.steps {
            let stored_step = self
                .repo
                .insert_step(
                    job_id,
                    attempt_id,
                    None,
                    &step.step_name,
                    step.status,
                    &step.message,
                    &step.payload_json,
                )
                .await
                .map_err(map_db_error)?;
            self.publish_step(job_id, attempt_id, &stored_step).await;
        }

        for target in execution_result.targets {
            for step in target.steps {
                let stored_step = self
                    .repo
                    .insert_step(
                        job_id,
                        attempt_id,
                        Some(target.deployment_target_id),
                        &step.step_name,
                        step.status,
                        &step.message,
                        &step.payload_json,
                    )
                    .await
                    .map_err(map_db_error)?;
                self.publish_step(job_id, attempt_id, &stored_step).await;
            }

            self.repo
                .complete_target(
                    target.deployment_target_id,
                    target.status,
                    target.error_message.as_deref(),
                )
                .await
                .map_err(map_db_error)?;

            let job = self
                .repo
                .refresh_job_summary(
                    job_id,
                    Some(attempt_id),
                    &execution_result.current_phase,
                    DeploymentJobStatus::Running,
                )
                .await
                .map_err(map_db_error)?;
            self.publish_status(&job).await;
        }

        let final_targets = self
            .repo
            .list_targets_for_attempt(attempt_id)
            .await
            .map_err(map_db_error)?;
        let final_status = derive_final_job_status(&final_targets);
        let finish_step = self
            .repo
            .insert_step(
                job_id,
                attempt_id,
                None,
                "attempt.finished",
                final_status_to_step_status(final_status),
                &execution_result.current_phase,
                &serde_json::json!({
                    "final_status": final_status.as_str(),
                    "current_phase": execution_result.current_phase,
                    "exit_code": execution_result.output.exit_code,
                    "stdout_ref": execution_result.output.stdout_ref,
                    "stdout_excerpt": execution_result.output.stdout_excerpt,
                    "stderr_ref": execution_result.output.stderr_ref,
                    "stderr_excerpt": execution_result.output.stderr_excerpt,
                }),
            )
            .await
            .map_err(map_db_error)?;
        self.publish_step(job_id, attempt_id, &finish_step).await;

        let job = self
            .repo
            .finalize_attempt(job_id, attempt_id, final_status, &execution_result.current_phase)
            .await
            .map_err(map_db_error)?;
        self.publish_status(&job).await;

        Ok(())
    }

    async fn publish_status(&self, job: &crate::models::DeploymentJobRecord) {
        let summary = job.summary_data();
        let event = deployment::DeploymentStatusEvent {
            job_id: job.id.to_string(),
            deployment_attempt_id: summary
                .current_attempt_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            status: job.status.to_proto(),
            current_phase: summary.current_phase,
            pending_targets: summary.pending_targets,
            running_targets: summary.running_targets,
            succeeded_targets: summary.succeeded_targets,
            failed_targets: summary.failed_targets,
            cancelled_targets: summary.cancelled_targets,
            updated_at: crate::models::format_ts(job.updated_at),
        };
        if let Err(error) = self
            .nats
            .publish(
                DEPLOYMENTS_JOBS_STATUS.to_string(),
                encode_message(&event).into(),
            )
            .await
        {
            warn!(error = %error, job_id = %job.id, "failed to publish deployment status event");
        }
    }

    async fn publish_step(
        &self,
        job_id: Uuid,
        attempt_id: Uuid,
        step: &crate::models::DeploymentStepRecord,
    ) {
        let event = deployment::DeploymentStepEvent {
            job_id: job_id.to_string(),
            deployment_attempt_id: attempt_id.to_string(),
            deployment_step_id: step.id.to_string(),
            deployment_target_id: step
                .deployment_target_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            step_name: step.step_name.clone(),
            status: step.status.to_proto(),
            message: step.message.clone(),
            updated_at: crate::models::format_ts(step.updated_at),
        };
        if let Err(error) = self
            .nats
            .publish(
                DEPLOYMENTS_JOBS_STEP.to_string(),
                encode_message(&event).into(),
            )
            .await
        {
            warn!(error = %error, job_id = %job_id, step_id = %step.id, "failed to publish deployment step event");
        }
    }
}

fn build_execution_snapshot(running: &RunningAttempt) -> AppResult<ExecutionSnapshot> {
    let target_map = running
        .snapshot
        .targets
        .iter()
        .map(|target| (target.host.host_id, target))
        .collect::<HashMap<_, _>>();

    let targets = running
        .targets
        .iter()
        .map(|target| {
            let snapshot_target = target_map
                .get(&target.host_id)
                .ok_or_else(|| AppError::internal("deployment snapshot target missing"))?;
            Ok(ExecutionTarget {
                deployment_target_id: target.id,
                host: snapshot_target.host.clone(),
                bootstrap: snapshot_target.bootstrap.clone(),
                rendered_vars: snapshot_target.rendered_vars.clone(),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(ExecutionSnapshot {
        deployment_job_id: running.job.id,
        deployment_attempt_id: running.attempt.id,
        audit: ExecutionAuditContext {
            actor_id: running.attempt.triggered_by.clone(),
            actor_type: "user".to_string(),
            request_id: running.attempt.id.to_string(),
            reason: running.attempt.reason.clone(),
        },
        job_type: running.snapshot.job_type,
        requested_by: running.snapshot.requested_by.clone(),
        policy: running.snapshot.policy.clone(),
        credentials: running.snapshot.credentials.clone(),
        flags: running.snapshot.flags.clone(),
        targets,
        executor_kind: running.snapshot.executor_kind,
        created_at: running.snapshot.created_at,
    })
}

fn executor_failure_result(running: &RunningAttempt, error: AppError) -> ExecutionResult {
    let message = error.to_string();
    ExecutionResult {
        current_phase: "executor.error".to_string(),
        targets: running
            .targets
            .iter()
            .map(|target| crate::models::ExecutionTargetResult {
                deployment_target_id: target.id,
                status: DeploymentTargetStatus::Failed,
                error_message: Some(message.clone()),
                steps: vec![crate::models::StepExecutionResult {
                    step_name: "executor.error".to_string(),
                    status: DeploymentStepStatus::Failed,
                    message: message.clone(),
                    payload_json: serde_json::json!({
                        "hostname": target.hostname_snapshot,
                        "host_id": target.host_id,
                    }),
                }],
            })
            .collect(),
        steps: vec![],
        output: crate::models::ExecutionOutput {
            exit_code: None,
            stdout_ref: None,
            stdout_excerpt: None,
            stderr_ref: None,
            stderr_excerpt: Some(message),
        },
    }
}

fn parse_create_spec(
    job_type_raw: i32,
    policy_id: &str,
    target_host_ids: &[String],
    target_host_group_ids: &[String],
    credential_profile_id: &str,
    requested_by: &str,
    preserve_state: bool,
    force: bool,
    dry_run: bool,
) -> AppResult<DeploymentCreateSpec> {
    if requested_by.trim().is_empty() {
        return Err(AppError::invalid_argument("requested_by is required"));
    }

    Ok(DeploymentCreateSpec {
        job_type: crate::models::DeploymentJobType::from_proto(job_type_raw)?,
        policy_id: parse_uuid("policy_id", policy_id)?,
        target_host_ids: parse_uuid_vec("target_host_ids", target_host_ids)?,
        target_host_group_ids: parse_uuid_vec("target_host_group_ids", target_host_group_ids)?,
        credential_profile_id: parse_uuid("credential_profile_id", credential_profile_id)?,
        requested_by: requested_by.trim().to_string(),
        flags: DeploymentFlags {
            preserve_state,
            force,
            dry_run,
        },
    })
}

fn parse_uuid(label: &str, value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value.trim())
        .map_err(|error| AppError::invalid_argument(format!("invalid {label}: {error}")))
}

fn parse_uuid_vec(label: &str, values: &[String]) -> AppResult<Vec<Uuid>> {
    values
        .iter()
        .map(|value| parse_uuid(label, value))
        .collect()
}

fn non_empty_or(value: &str, default: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn derive_final_job_status(targets: &[DeploymentTargetRecord]) -> DeploymentJobStatus {
    let total = targets.len();
    let succeeded = targets
        .iter()
        .filter(|target| target.status == DeploymentTargetStatus::Succeeded)
        .count();
    let failed = targets
        .iter()
        .filter(|target| target.status == DeploymentTargetStatus::Failed)
        .count();
    let cancelled = targets
        .iter()
        .filter(|target| target.status == DeploymentTargetStatus::Cancelled)
        .count();

    if total == 0 {
        return DeploymentJobStatus::Failed;
    }
    if cancelled == total {
        return DeploymentJobStatus::Cancelled;
    }
    if succeeded == total {
        return DeploymentJobStatus::Succeeded;
    }
    if failed == total {
        return DeploymentJobStatus::Failed;
    }
    if failed > 0 || cancelled > 0 {
        return DeploymentJobStatus::PartialSuccess;
    }
    DeploymentJobStatus::Running
}

fn final_status_to_step_status(status: DeploymentJobStatus) -> DeploymentStepStatus {
    match status {
        DeploymentJobStatus::Succeeded | DeploymentJobStatus::PartialSuccess => {
            DeploymentStepStatus::Succeeded
        }
        DeploymentJobStatus::Failed => DeploymentStepStatus::Failed,
        DeploymentJobStatus::Cancelled => DeploymentStepStatus::Skipped,
        DeploymentJobStatus::Queued | DeploymentJobStatus::Running => DeploymentStepStatus::Running,
    }
}

fn map_db_error(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(db_error) => {
            if let Some(code) = db_error.code() {
                if code.as_ref() == "23505" {
                    return AppError::invalid_argument(format!(
                        "constraint violation: {}",
                        db_error.message()
                    ));
                }
            }
            AppError::internal(format!("database error: {db_error}"))
        }
        _ => AppError::internal(format!("database error: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{derive_final_job_status, final_status_to_step_status};
    use crate::models::{DeploymentJobStatus, DeploymentTargetRecord, DeploymentTargetStatus};

    fn target(status: DeploymentTargetStatus) -> DeploymentTargetRecord {
        let now = chrono::Utc::now();
        DeploymentTargetRecord {
            id: Uuid::new_v4(),
            deployment_job_id: Uuid::new_v4(),
            deployment_attempt_id: Uuid::new_v4(),
            host_id: Uuid::new_v4(),
            hostname_snapshot: "demo".to_string(),
            status,
            bootstrap_payload_json: serde_json::json!({}),
            rendered_vars_json: serde_json::json!({}),
            error_message: String::new(),
            created_at: now,
            started_at: None,
            finished_at: None,
            updated_at: now,
        }
    }

    #[test]
    fn derives_partial_success() {
        let status = derive_final_job_status(&[
            target(DeploymentTargetStatus::Succeeded),
            target(DeploymentTargetStatus::Failed),
        ]);
        assert_eq!(status, DeploymentJobStatus::PartialSuccess);
        assert_eq!(
            final_status_to_step_status(DeploymentJobStatus::Cancelled),
            crate::models::DeploymentStepStatus::Skipped
        );
    }
}
