use std::{collections::BTreeMap, fmt, path::Path};

use chrono::{DateTime, SecondsFormat, Utc};
use common::{
    proto::deployment::{self, BootstrapPreview, DeploymentPlanTarget},
    AppError, AppResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub fn format_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn format_optional_ts(ts: Option<DateTime<Utc>>) -> String {
    ts.map(format_ts).unwrap_or_default()
}

pub fn parse_rfc3339_utc(label: &str, value: &str) -> AppResult<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|ts| ts.with_timezone(&Utc))
        .map_err(|error| AppError::invalid_argument(format!("invalid {label}: {error}")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentJobType {
    Install,
    Reinstall,
    Upgrade,
    Uninstall,
}

impl DeploymentJobType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Reinstall => "reinstall",
            Self::Upgrade => "upgrade",
            Self::Uninstall => "uninstall",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "install" => Some(Self::Install),
            "reinstall" => Some(Self::Reinstall),
            "upgrade" => Some(Self::Upgrade),
            "uninstall" => Some(Self::Uninstall),
            _ => None,
        }
    }

    pub fn from_proto(value: i32) -> AppResult<Self> {
        match deployment::DeploymentJobType::try_from(value).ok() {
            Some(deployment::DeploymentJobType::Install) => Ok(Self::Install),
            Some(deployment::DeploymentJobType::Reinstall) => Ok(Self::Reinstall),
            Some(deployment::DeploymentJobType::Upgrade) => Ok(Self::Upgrade),
            Some(deployment::DeploymentJobType::Uninstall) => Ok(Self::Uninstall),
            _ => Err(AppError::invalid_argument("invalid deployment job type")),
        }
    }

    pub fn to_proto(self) -> i32 {
        match self {
            Self::Install => deployment::DeploymentJobType::Install as i32,
            Self::Reinstall => deployment::DeploymentJobType::Reinstall as i32,
            Self::Upgrade => deployment::DeploymentJobType::Upgrade as i32,
            Self::Uninstall => deployment::DeploymentJobType::Uninstall as i32,
        }
    }
}

impl fmt::Display for DeploymentJobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentJobStatus {
    Queued,
    Running,
    PartialSuccess,
    Succeeded,
    Failed,
    Cancelled,
}

impl DeploymentJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::PartialSuccess => "partial_success",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "partial_success" => Some(Self::PartialSuccess),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn from_proto(value: i32) -> Option<Self> {
        match deployment::DeploymentJobStatus::try_from(value).ok() {
            Some(deployment::DeploymentJobStatus::Queued) => Some(Self::Queued),
            Some(deployment::DeploymentJobStatus::Running) => Some(Self::Running),
            Some(deployment::DeploymentJobStatus::PartialSuccess) => Some(Self::PartialSuccess),
            Some(deployment::DeploymentJobStatus::Succeeded) => Some(Self::Succeeded),
            Some(deployment::DeploymentJobStatus::Failed) => Some(Self::Failed),
            Some(deployment::DeploymentJobStatus::Cancelled) => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn to_proto(self) -> i32 {
        match self {
            Self::Queued => deployment::DeploymentJobStatus::Queued as i32,
            Self::Running => deployment::DeploymentJobStatus::Running as i32,
            Self::PartialSuccess => deployment::DeploymentJobStatus::PartialSuccess as i32,
            Self::Succeeded => deployment::DeploymentJobStatus::Succeeded as i32,
            Self::Failed => deployment::DeploymentJobStatus::Failed as i32,
            Self::Cancelled => deployment::DeploymentJobStatus::Cancelled as i32,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::PartialSuccess | Self::Succeeded | Self::Failed | Self::Cancelled
        )
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Queued => matches!(
                next,
                Self::Queued | Self::Running | Self::Failed | Self::Cancelled
            ),
            Self::Running => matches!(
                next,
                Self::Running
                    | Self::PartialSuccess
                    | Self::Succeeded
                    | Self::Failed
                    | Self::Cancelled
            ),
            Self::PartialSuccess | Self::Succeeded | Self::Failed | Self::Cancelled => self == next,
        }
    }
}

impl fmt::Display for DeploymentJobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentTargetStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl DeploymentTargetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn to_proto(self) -> i32 {
        match self {
            Self::Pending => deployment::DeploymentTargetStatus::Pending as i32,
            Self::Running => deployment::DeploymentTargetStatus::Running as i32,
            Self::Succeeded => deployment::DeploymentTargetStatus::Succeeded as i32,
            Self::Failed => deployment::DeploymentTargetStatus::Failed as i32,
            Self::Cancelled => deployment::DeploymentTargetStatus::Cancelled as i32,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Pending => matches!(
                next,
                Self::Pending | Self::Running | Self::Failed | Self::Cancelled
            ),
            Self::Running => matches!(
                next,
                Self::Running | Self::Succeeded | Self::Failed | Self::Cancelled
            ),
            Self::Succeeded | Self::Failed | Self::Cancelled => self == next,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

impl DeploymentStepStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }

    pub fn to_proto(self) -> i32 {
        match self {
            Self::Pending => deployment::DeploymentStepStatus::Pending as i32,
            Self::Running => deployment::DeploymentStepStatus::Running as i32,
            Self::Succeeded => deployment::DeploymentStepStatus::Succeeded as i32,
            Self::Failed => deployment::DeploymentStepStatus::Failed as i32,
            Self::Skipped => deployment::DeploymentStepStatus::Skipped as i32,
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Pending => matches!(
                next,
                Self::Pending | Self::Running | Self::Succeeded | Self::Failed | Self::Skipped
            ),
            Self::Running => matches!(
                next,
                Self::Running | Self::Succeeded | Self::Failed | Self::Skipped
            ),
            Self::Succeeded | Self::Failed | Self::Skipped => self == next,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    Mock,
    Ansible,
}

impl ExecutorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Ansible => "ansible",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "mock" => Some(Self::Mock),
            "ansible" => Some(Self::Ansible),
            _ => None,
        }
    }

    pub fn to_proto(self) -> i32 {
        match self {
            Self::Mock => deployment::ExecutorKind::Mock as i32,
            Self::Ansible => deployment::ExecutorKind::Ansible as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryStrategy {
    FailedOnly,
    All,
}

impl RetryStrategy {
    pub fn from_proto(value: i32) -> AppResult<Self> {
        match deployment::RetryStrategy::try_from(value).ok() {
            Some(deployment::RetryStrategy::FailedOnly) => Ok(Self::FailedOnly),
            Some(deployment::RetryStrategy::All) => Ok(Self::All),
            _ => Err(AppError::invalid_argument("invalid retry strategy")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentFlags {
    pub preserve_state: bool,
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentCreateSpec {
    pub job_type: DeploymentJobType,
    pub policy_id: Uuid,
    pub target_host_ids: Vec<Uuid>,
    pub target_host_group_ids: Vec<Uuid>,
    pub credential_profile_id: Uuid,
    pub requested_by: String,
    pub flags: DeploymentFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedHost {
    pub host_id: Uuid,
    pub hostname: String,
    pub ip: String,
    pub ssh_port: u16,
    pub remote_user: String,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPolicy {
    pub policy_id: Uuid,
    pub policy_revision_id: Uuid,
    pub policy_revision: String,
    pub policy_body_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedCredentialProfile {
    pub credential_profile_id: Uuid,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub vault_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapArtifact {
    pub token_id: String,
    pub bootstrap_token: String,
    pub expires_at: DateTime<Utc>,
    pub bootstrap_yaml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentTargetSnapshot {
    pub host: ResolvedHost,
    pub bootstrap: BootstrapArtifact,
    pub rendered_vars: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSnapshot {
    pub job_type: DeploymentJobType,
    pub requested_by: String,
    pub policy: ResolvedPolicy,
    pub credentials: ResolvedCredentialProfile,
    pub flags: DeploymentFlags,
    pub targets: Vec<DeploymentTargetSnapshot>,
    pub executor_kind: ExecutorKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeploymentPlan {
    pub snapshot: DeploymentSnapshot,
    pub action_summary: String,
    pub credential_summary: String,
    pub warnings: Vec<String>,
}

impl DeploymentPlan {
    pub fn into_proto(self) -> deployment::CreateDeploymentPlanResponse {
        let warnings = self.warnings;
        let targets = self
            .snapshot
            .targets
            .iter()
            .map(|target| DeploymentPlanTarget {
                host_id: target.host.host_id.to_string(),
                hostname: target.host.hostname.clone(),
                ip: target.host.ip.clone(),
                ssh_port: target.host.ssh_port as u32,
                remote_user: target.host.remote_user.clone(),
            })
            .collect();
        let bootstrap_previews = self
            .snapshot
            .targets
            .iter()
            .map(|target| BootstrapPreview {
                host_id: target.host.host_id.to_string(),
                hostname: target.host.hostname.clone(),
                bootstrap_yaml: target.bootstrap.bootstrap_yaml.clone(),
            })
            .collect();

        deployment::CreateDeploymentPlanResponse {
            job_type: self.snapshot.job_type.to_proto(),
            policy_id: self.snapshot.policy.policy_id.to_string(),
            policy_revision_id: self.snapshot.policy.policy_revision_id.to_string(),
            policy_revision: self.snapshot.policy.policy_revision.clone(),
            credential_profile_id: self.snapshot.credentials.credential_profile_id.to_string(),
            credential_summary: self.credential_summary,
            executor_kind: self.snapshot.executor_kind.to_proto(),
            action_summary: self.action_summary,
            targets,
            bootstrap_previews,
            warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummaryData {
    pub current_phase: String,
    pub total_targets: u32,
    pub pending_targets: u32,
    pub running_targets: u32,
    pub succeeded_targets: u32,
    pub failed_targets: u32,
    pub cancelled_targets: u32,
    pub attempt_count: u32,
    pub current_attempt_id: Option<Uuid>,
}

impl Default for JobSummaryData {
    fn default() -> Self {
        Self {
            current_phase: "queued".to_string(),
            total_targets: 0,
            pending_targets: 0,
            running_targets: 0,
            succeeded_targets: 0,
            failed_targets: 0,
            cancelled_targets: 0,
            attempt_count: 0,
            current_attempt_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentJobPayload {
    pub request: DeploymentCreateSpec,
    pub snapshot: DeploymentSnapshot,
}

#[derive(Debug, Clone)]
pub struct DeploymentJobRecord {
    pub id: Uuid,
    pub job_type: DeploymentJobType,
    pub status: DeploymentJobStatus,
    pub requested_by: String,
    pub policy_id: Uuid,
    pub policy_revision_id: Uuid,
    pub credential_profile_id: Uuid,
    pub executor_kind: ExecutorKind,
    pub payload_json: Value,
    pub summary_json: Value,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl DeploymentJobRecord {
    pub fn summary_data(&self) -> JobSummaryData {
        serde_json::from_value(self.summary_json.clone()).unwrap_or_default()
    }

    pub fn into_proto(self) -> deployment::DeploymentJobSummary {
        let summary = self.summary_data();
        deployment::DeploymentJobSummary {
            job_id: self.id.to_string(),
            job_type: self.job_type.to_proto(),
            status: self.status.to_proto(),
            requested_by: self.requested_by,
            policy_id: self.policy_id.to_string(),
            policy_revision_id: self.policy_revision_id.to_string(),
            credential_profile_id: self.credential_profile_id.to_string(),
            executor_kind: self.executor_kind.to_proto(),
            current_phase: summary.current_phase,
            total_targets: summary.total_targets,
            pending_targets: summary.pending_targets,
            running_targets: summary.running_targets,
            succeeded_targets: summary.succeeded_targets,
            failed_targets: summary.failed_targets,
            cancelled_targets: summary.cancelled_targets,
            attempt_count: summary.attempt_count,
            created_at: format_ts(self.created_at),
            started_at: format_optional_ts(self.started_at),
            finished_at: format_optional_ts(self.finished_at),
            updated_at: format_ts(self.updated_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentAttemptRecord {
    pub id: Uuid,
    pub deployment_job_id: Uuid,
    pub attempt_no: i32,
    pub status: DeploymentJobStatus,
    pub triggered_by: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl DeploymentAttemptRecord {
    pub fn into_proto(self) -> deployment::DeploymentAttemptSummary {
        deployment::DeploymentAttemptSummary {
            deployment_attempt_id: self.id.to_string(),
            attempt_no: self.attempt_no.max(0) as u32,
            status: self.status.to_proto(),
            triggered_by: self.triggered_by,
            reason: self.reason,
            created_at: format_ts(self.created_at),
            started_at: format_optional_ts(self.started_at),
            finished_at: format_optional_ts(self.finished_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentTargetRecord {
    pub id: Uuid,
    pub deployment_job_id: Uuid,
    pub deployment_attempt_id: Uuid,
    pub host_id: Uuid,
    pub hostname_snapshot: String,
    pub status: DeploymentTargetStatus,
    pub bootstrap_payload_json: Value,
    pub rendered_vars_json: Value,
    pub error_message: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl DeploymentTargetRecord {
    pub fn into_proto(self) -> deployment::DeploymentTargetSummary {
        deployment::DeploymentTargetSummary {
            deployment_target_id: self.id.to_string(),
            deployment_attempt_id: self.deployment_attempt_id.to_string(),
            host_id: self.host_id.to_string(),
            hostname_snapshot: self.hostname_snapshot,
            status: self.status.to_proto(),
            error_message: self.error_message,
            created_at: format_ts(self.created_at),
            started_at: format_optional_ts(self.started_at),
            finished_at: format_optional_ts(self.finished_at),
            updated_at: format_ts(self.updated_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentStepRecord {
    pub id: Uuid,
    pub deployment_job_id: Uuid,
    pub deployment_attempt_id: Uuid,
    pub deployment_target_id: Option<Uuid>,
    pub step_name: String,
    pub status: DeploymentStepStatus,
    pub message: String,
    pub payload_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DeploymentStepRecord {
    pub fn into_proto(self) -> deployment::DeploymentStepSummary {
        deployment::DeploymentStepSummary {
            deployment_step_id: self.id.to_string(),
            deployment_attempt_id: self.deployment_attempt_id.to_string(),
            deployment_target_id: self
                .deployment_target_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            step_name: self.step_name,
            status: self.status.to_proto(),
            message: self.message,
            payload_json: self.payload_json.to_string(),
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentJobView {
    pub job: DeploymentJobRecord,
    pub attempts: Vec<DeploymentAttemptRecord>,
    pub targets: Vec<DeploymentTargetRecord>,
    pub steps: Vec<DeploymentStepRecord>,
}

impl DeploymentJobView {
    pub fn into_proto(self) -> deployment::GetDeploymentJobResponse {
        deployment::GetDeploymentJobResponse {
            job: Some(self.job.into_proto()),
            attempts: self
                .attempts
                .into_iter()
                .map(DeploymentAttemptRecord::into_proto)
                .collect(),
            targets: self
                .targets
                .into_iter()
                .map(DeploymentTargetRecord::into_proto)
                .collect(),
            steps: self
                .steps
                .into_iter()
                .map(DeploymentStepRecord::into_proto)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListJobsFilter {
    pub status: Option<DeploymentJobStatus>,
    pub job_type: Option<DeploymentJobType>,
    pub requested_by: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: u32,
    pub offset: u64,
}

impl Default for ListJobsFilter {
    fn default() -> Self {
        Self {
            status: None,
            job_type: None,
            requested_by: None,
            created_after: None,
            created_before: None,
            limit: 50,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepExecutionResult {
    pub step_name: String,
    pub status: DeploymentStepStatus,
    pub message: String,
    pub payload_json: Value,
}

#[derive(Debug, Clone)]
pub struct TargetExecutionResult {
    pub status: DeploymentTargetStatus,
    pub error_message: Option<String>,
    pub steps: Vec<StepExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct ExecutorResult {
    pub job_status: DeploymentJobStatus,
    pub current_phase: String,
}

#[derive(Debug, Clone)]
pub struct RunningAttempt {
    pub job: DeploymentJobRecord,
    pub attempt: DeploymentAttemptRecord,
    pub snapshot: DeploymentSnapshot,
    pub targets: Vec<DeploymentTargetRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileSourceConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub source_id: String,
    pub path: String,
    pub start_at: String,
    pub source: String,
    pub service: String,
    pub severity_hint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransportConfigYaml {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntervalConfigYaml {
    pub interval_sec: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchConfigYaml {
    pub max_events: usize,
    pub max_bytes: usize,
    pub flush_interval_ms: u64,
    pub compress_threshold_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpoolConfigYaml {
    pub enabled: bool,
    pub dir: String,
    pub max_disk_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeploymentBootstrapConfig {
    pub edge_url: String,
    pub edge_grpc_addr: String,
    pub bootstrap_token: String,
    pub state_dir: String,
    pub log_level: String,
    pub transport: TransportConfigYaml,
    pub heartbeat: IntervalConfigYaml,
    pub diagnostics: IntervalConfigYaml,
    pub batch: BatchConfigYaml,
    pub spool: SpoolConfigYaml,
    pub sources: Vec<FileSourceConfig>,
}

pub fn default_source_id(path: &str) -> String {
    format!("file:{path}")
}

pub fn default_source_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("host-log")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{DeploymentJobStatus, DeploymentStepStatus, DeploymentTargetStatus};

    #[test]
    fn validates_job_transitions() {
        assert!(DeploymentJobStatus::Queued.can_transition_to(DeploymentJobStatus::Running));
        assert!(!DeploymentJobStatus::Succeeded.can_transition_to(DeploymentJobStatus::Failed));
    }

    #[test]
    fn validates_target_transitions() {
        assert!(DeploymentTargetStatus::Pending.can_transition_to(DeploymentTargetStatus::Running));
        assert!(
            !DeploymentTargetStatus::Cancelled.can_transition_to(DeploymentTargetStatus::Succeeded)
        );
    }

    #[test]
    fn validates_step_transitions() {
        assert!(DeploymentStepStatus::Pending.can_transition_to(DeploymentStepStatus::Skipped));
        assert!(!DeploymentStepStatus::Succeeded.can_transition_to(DeploymentStepStatus::Failed));
    }
}
