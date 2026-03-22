package model

type AgentIssue struct {
	Code     string `json:"code"`
	Severity string `json:"severity"`
	Domain   string `json:"domain"`
	Source   string `json:"source,omitempty"`
	Message  string `json:"message"`
}

type AgentDiagnosticsCheck struct {
	CheckID  string `json:"check_id"`
	Name     string `json:"name"`
	Status   string `json:"status"`
	Severity string `json:"severity"`
	Domain   string `json:"domain"`
	Message  string `json:"message"`
	Hint     string `json:"hint,omitempty"`
}

type AgentDataFreshnessSection struct {
	Status     string `json:"status"`
	ObservedAt string `json:"observed_at,omitempty"`
	AgeSec     int64  `json:"age_sec,omitempty"`
	Note       string `json:"note,omitempty"`
}

type AgentDataFreshness struct {
	GeneratedAt string                              `json:"generated_at"`
	Sections    map[string]AgentDataFreshnessSection `json:"sections,omitempty"`
}

type HostAgentStatusView struct {
	HostID      string `json:"host_id"`
	Hostname    string `json:"hostname"`
	ClusterID   string `json:"cluster_id,omitempty"`
	ClusterName string `json:"cluster_name,omitempty"`
	ServiceName string `json:"service_name,omitempty"`
	Environment string `json:"environment,omitempty"`

	DeploymentJobID         string `json:"deployment_job_id,omitempty"`
	DeploymentStatus        string `json:"deployment_status"`
	DeploymentCurrentPhase  string `json:"deployment_current_phase,omitempty"`
	LastSuccessfulDeployAt  string `json:"last_successful_deploy_at,omitempty"`
	LastFailedDeployAt      string `json:"last_failed_deploy_at,omitempty"`
	ExecutorKind            string `json:"executor_kind,omitempty"`
	InstallMode             string `json:"install_mode,omitempty"`
	ArtifactRef             string `json:"artifact_ref,omitempty"`

	AgentID          string `json:"agent_id,omitempty"`
	EnrollmentStatus string `json:"enrollment_status"`
	EnrolledAt       string `json:"enrolled_at,omitempty"`
	EdgeConnected    bool   `json:"edge_connected"`
	LastHeartbeatAt  string `json:"last_heartbeat_at,omitempty"`
	HeartbeatStatus  string `json:"heartbeat_status"`
	LastDiagnosticsAt string `json:"last_diagnostics_at,omitempty"`

	DoctorStatus  string       `json:"doctor_status"`
	WarningCount  int          `json:"warning_count"`
	FailureCount  int          `json:"failure_count"`
	TopIssues     []AgentIssue `json:"top_issues"`
	SpoolStatus   string       `json:"spool_status"`
	TransportStatus string     `json:"transport_status"`
	TLSStatus     string       `json:"tls_status"`
	SourceStatus  string       `json:"source_status"`

	LastLogSeenAt         string `json:"last_log_seen_at,omitempty"`
	LastBatchSentAt       string `json:"last_batch_sent_at,omitempty"`
	RecentLogsPerMinute   uint64 `json:"recent_logs_per_minute"`
	RecentErrorsPerMinute uint64 `json:"recent_errors_per_minute"`

	LastKnownErrors   []AgentIssue `json:"last_known_errors,omitempty"`
	LastKnownWarnings []AgentIssue `json:"last_known_warnings,omitempty"`

	PrimaryFailureDomain string `json:"primary_failure_domain"`
	HumanHint            string `json:"human_hint"`
	SuggestedNextStep    string `json:"suggested_next_step"`

	MissingSections []string           `json:"missing_sections,omitempty"`
	DataFreshness   AgentDataFreshness `json:"data_freshness"`
}

type HostAgentDiagnosticsSummary struct {
	DoctorStatus    string       `json:"doctor_status"`
	WarningCount    int          `json:"warning_count"`
	FailureCount    int          `json:"failure_count"`
	SpoolStatus     string       `json:"spool_status"`
	TransportStatus string       `json:"transport_status"`
	TLSStatus       string       `json:"tls_status"`
	SourceStatus    string       `json:"source_status"`
	TopIssues       []AgentIssue `json:"top_issues"`
}

type AgentDegradedMode struct {
	Active bool   `json:"active"`
	Reason string `json:"reason,omitempty"`
}

type HostAgentDiagnosticsView struct {
	HostID         string `json:"host_id"`
	Hostname       string `json:"hostname"`
	AgentID        string `json:"agent_id,omitempty"`
	CollectedAt    string `json:"collected_at,omitempty"`
	SnapshotJSON   any    `json:"snapshot_json,omitempty"`

	Summary      HostAgentDiagnosticsSummary `json:"summary"`
	Checks       []AgentDiagnosticsCheck     `json:"checks"`
	DegradedMode AgentDegradedMode           `json:"degraded_mode"`

	RuntimeErrors   []AgentIssue `json:"runtime_errors"`
	TransportErrors []AgentIssue `json:"transport_errors"`
	SpoolWarnings   []AgentIssue `json:"spool_warnings"`

	MissingSections []string           `json:"missing_sections,omitempty"`
	DataFreshness   AgentDataFreshness `json:"data_freshness"`
}

type ClusterAgentHostSummary struct {
	HostID               string `json:"host_id"`
	Hostname             string `json:"hostname"`
	ClusterID            string `json:"cluster_id,omitempty"`
	AgentID              string `json:"agent_id,omitempty"`
	DeploymentStatus     string `json:"deployment_status"`
	EnrollmentStatus     string `json:"enrollment_status"`
	HeartbeatStatus      string `json:"heartbeat_status"`
	DoctorStatus         string `json:"doctor_status"`
	LastHeartbeatAt      string `json:"last_heartbeat_at,omitempty"`
	LastDiagnosticsAt    string `json:"last_diagnostics_at,omitempty"`
	LastLogSeenAt        string `json:"last_log_seen_at,omitempty"`
	PrimaryFailureDomain string `json:"primary_failure_domain"`
	HumanHint            string `json:"human_hint"`
}

type ClusterAgentsOverviewView struct {
	ClusterID        string `json:"cluster_id"`
	ClusterName      string `json:"cluster_name,omitempty"`
	NoLogsWindowMin  int64  `json:"no_logs_window_min"`
	TotalHosts       int    `json:"total_hosts"`
	DeployedAgents   int    `json:"deployed_agents"`
	HealthyAgents    int    `json:"healthy_agents"`
	StaleHeartbeat   int    `json:"stale_heartbeat"`
	NeverEnrolled    int    `json:"never_enrolled"`
	DeploymentFailed int    `json:"deployment_failed"`
	DiagnosticsWarn  int    `json:"diagnostics_warn"`
	DiagnosticsFail  int    `json:"diagnostics_fail"`
	NoLogsInLastNMin int    `json:"no_logs_in_last_n_min"`
	Hosts            []ClusterAgentHostSummary `json:"hosts"`

	MissingSections []string           `json:"missing_sections,omitempty"`
	DataFreshness   AgentDataFreshness `json:"data_freshness"`
}

type DeploymentTimelineItem struct {
	PhaseType            string `json:"phase_type"`
	Status               string `json:"status"`
	OccurredAt           string `json:"occurred_at"`
	DeploymentAttemptID  string `json:"deployment_attempt_id,omitempty"`
	DeploymentTargetID   string `json:"deployment_target_id,omitempty"`
	AttemptNo            uint32 `json:"attempt_no,omitempty"`
	HostID               string `json:"host_id,omitempty"`
	Hostname             string `json:"hostname,omitempty"`
	RawStepName          string `json:"raw_step_name,omitempty"`
	Message              string `json:"message,omitempty"`
	PayloadJSON          any    `json:"payload_json,omitempty"`
}

type DeploymentTimelineView struct {
	JobID         string                   `json:"job_id"`
	Status        string                   `json:"status"`
	CurrentPhase  string                   `json:"current_phase,omitempty"`
	Items         []DeploymentTimelineItem `json:"items"`
	MissingSections []string               `json:"missing_sections,omitempty"`
	DataFreshness AgentDataFreshness       `json:"data_freshness"`
}

type AgentRuntimeEvent struct {
	EventType  string `json:"event_type"`
	HostID     string `json:"host_id,omitempty"`
	ClusterID  string `json:"cluster_id,omitempty"`
	Severity   string `json:"severity"`
	OccurredAt string `json:"occurred_at"`
	Payload    any    `json:"payload"`
}
