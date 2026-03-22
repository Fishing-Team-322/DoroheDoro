package agentstatus

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
	"github.com/example/dorohedoro/internal/natsbridge/subjects"
)

type Bridge interface {
	Request(ctx context.Context, subject string, payload []byte) (*nats.Msg, error)
}

type Settings struct {
	CacheTTL              time.Duration
	RequestTimeout        time.Duration
	EventLookupTimeout    time.Duration
	HeartbeatStaleAfter   time.Duration
	DiagnosticsStaleAfter time.Duration
	NoLogsWindow          time.Duration
	DeploymentListLimit   uint32
}

type Dependencies struct {
	Bridge   Bridge
	Logger   *zap.Logger
	Subjects subjects.Registry
	Settings Settings
}

type Service struct {
	bridge   Bridge
	logger   *zap.Logger
	subjects subjects.Registry
	settings Settings
	now      func() time.Time

	cache memo

	eventStateMu sync.Mutex
	eventStates  map[string]eventState
}

type memo struct {
	mu    sync.Mutex
	items map[string]memoValue
}

type memoValue struct {
	expiresAt time.Time
	value     any
}

type eventState struct {
	problem bool
}

type requestError struct {
	StatusCode int
	Code       string
	Message    string
	Err        error
}

func (e *requestError) Error() string {
	if e == nil {
		return ""
	}
	if strings.TrimSpace(e.Message) != "" {
		return e.Message
	}
	if e.Err != nil {
		return e.Err.Error()
	}
	return e.Code
}

func (e *requestError) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.Err
}

func (e *requestError) HTTPStatus() int {
	if e == nil || e.StatusCode == 0 {
		return 500
	}
	return e.StatusCode
}

func (e *requestError) ErrorCode() string {
	if e == nil || strings.TrimSpace(e.Code) == "" {
		return "internal"
	}
	return e.Code
}

type hostReadModel struct {
	host         envelope.ControlHost
	cluster      *envelope.ControlClusterDetails
	agentSummary *envelope.AgentSummary
	agentDetail  *envelope.AgentDetail
	diagnostics  parsedDiagnostics
	deployment   hostDeploymentInfo
	traffic      trafficSummary

	missingSections []string
	freshness       map[string]model.AgentDataFreshnessSection
}

type parsedDiagnostics struct {
	CreatedAt string
	Raw       any
	Snapshot  *diagnosticsSnapshot
	ParseErr  error
}

type hostDeploymentInfo struct {
	CurrentJob             *envelope.DeploymentJobSummary
	CurrentTarget          *envelope.DeploymentTargetSummary
	CurrentSteps           []envelope.DeploymentStepSummary
	LastSuccessfulDeployAt string
	LastFailedDeployAt     string
	InstallMode            string
	ArtifactRef            string
}

type trafficSummary struct {
	LastLogSeenAt         string
	LastBatchSentAt       string
	RecentLogsPerMinute   uint64
	RecentErrorsPerMinute uint64
}

type diagnosticsPresentation struct {
	Summary  model.HostAgentDiagnosticsSummary
	Checks   []model.AgentDiagnosticsCheck
	Issues   []model.AgentIssue
	Errors   []model.AgentIssue
	Warnings []model.AgentIssue
	Degraded model.AgentDegradedMode
}

type diagnosticsSnapshot struct {
	AgentID                 string                       `json:"agent_id"`
	Hostname                string                       `json:"hostname"`
	Version                 string                       `json:"version"`
	CurrentPolicyRevision   *string                      `json:"current_policy_revision"`
	RuntimeStatus           string                       `json:"runtime_status"`
	RuntimeStatusReason     *string                      `json:"runtime_status_reason"`
	DegradedMode            bool                         `json:"degraded_mode"`
	DegradedReason          *string                      `json:"degraded_reason"`
	BlockedDelivery         bool                         `json:"blocked_delivery"`
	BlockedReason           *string                      `json:"blocked_reason"`
	RuntimeMode             string                       `json:"runtime_mode"`
	ActiveSources           int                          `json:"active_sources"`
	SpoolEnabled            bool                         `json:"spool_enabled"`
	SpooledBatches          int                          `json:"spooled_batches"`
	SpooledBytes            uint64                       `json:"spooled_bytes"`
	LastError               *string                      `json:"last_error"`
	LastErrorKind           *string                      `json:"last_error_kind"`
	LastSuccessfulSendAt    *int64                       `json:"last_successful_send_at"`
	ConsecutiveSendFailures uint32                       `json:"consecutive_send_failures"`
	TransportState          diagnosticsTransportState    `json:"transport_state"`
	PolicyState             diagnosticsPolicyState       `json:"policy_state"`
	ConnectivityState       diagnosticsConnectivityState `json:"connectivity_state"`
	SourceStatuses          []diagnosticsSourceStatus    `json:"source_statuses"`
	Install                 diagnosticsInstall           `json:"install"`
	Platform                diagnosticsPlatform          `json:"platform"`
	Cluster                 diagnosticsCluster           `json:"cluster"`
	Compatibility           diagnosticsCompatibility     `json:"compatibility"`
	IdentityStatus          diagnosticsIdentityStatus    `json:"identity_status"`
}

type diagnosticsTransportState struct {
	Mode                    string  `json:"mode"`
	ServerUnavailableForSec uint64  `json:"server_unavailable_for_sec"`
	LastErrorKind           *string `json:"last_error_kind"`
	BlockedDelivery         bool    `json:"blocked_delivery"`
	BlockedReason           *string `json:"blocked_reason"`
}

type diagnosticsPolicyState struct {
	CurrentPolicyRevision *string `json:"current_policy_revision"`
	LastPolicyFetchAt     *int64  `json:"last_policy_fetch_at"`
	LastPolicyApplyAt     *int64  `json:"last_policy_apply_at"`
	LastPolicyError       *string `json:"last_policy_error"`
	ActiveSourceCount     int     `json:"active_source_count"`
}

type diagnosticsConnectivityState struct {
	Endpoint               string  `json:"endpoint"`
	TLSEnabled             bool    `json:"tls_enabled"`
	MTLSEnabled            bool    `json:"mtls_enabled"`
	ServerName             *string `json:"server_name"`
	CaPathPresent          bool    `json:"ca_path_present"`
	CertPathPresent        bool    `json:"cert_path_present"`
	KeyPathPresent         bool    `json:"key_path_present"`
	LastConnectError       *string `json:"last_connect_error"`
	LastTLSError           *string `json:"last_tls_error"`
	LastHandshakeSuccessAt *int64  `json:"last_handshake_success_at"`
}

type diagnosticsSourceStatus struct {
	SourceID            string  `json:"source_id"`
	Path                string  `json:"path"`
	Source              string  `json:"source"`
	Service             string  `json:"service"`
	Status              string  `json:"status"`
	LastReadAt          *int64  `json:"last_read_at"`
	LastError           *string `json:"last_error"`
	LivePendingBytes    uint64  `json:"live_pending_bytes"`
	DurablePendingBytes uint64  `json:"durable_pending_bytes"`
}

type diagnosticsInstall struct {
	ResolvedMode string   `json:"resolved_mode"`
	Warnings     []string `json:"warnings"`
}

type diagnosticsPlatform struct {
	Hostname        string `json:"hostname"`
	ServiceManager  string `json:"service_manager"`
	SystemdDetected bool   `json:"systemd_detected"`
}

type diagnosticsCluster struct {
	ConfiguredClusterID *string           `json:"configured_cluster_id"`
	ClusterName         *string           `json:"cluster_name"`
	ServiceName         *string           `json:"service_name"`
	Environment         *string           `json:"environment"`
	HostLabels          map[string]string `json:"host_labels"`
}

type diagnosticsCompatibility struct {
	Notes             []string `json:"notes"`
	Warnings          []string `json:"warnings"`
	Errors            []string `json:"errors"`
	PermissionIssues  []string `json:"permission_issues"`
	SourcePathIssues  []string `json:"source_path_issues"`
	InsecureTransport bool     `json:"insecure_transport"`
}

type diagnosticsIdentityStatus struct {
	Status string  `json:"status"`
	Reason *string `json:"reason"`
}

type uiAgentStreamEvent struct {
	EventType  string `json:"event_type"`
	AgentID    string `json:"agent_id"`
	Hostname   string `json:"hostname"`
	Status     string `json:"status"`
	Version    string `json:"version"`
	LastSeenAt string `json:"last_seen_at"`
}

type logSearchPayload struct {
	Items []logEventItem `json:"items"`
	Total uint64         `json:"total"`
}

type logEventItem struct {
	Timestamp string `json:"timestamp"`
	Severity  string `json:"severity"`
}

type countBucketsPayload struct {
	Items []countBucket `json:"items"`
}

type countBucket struct {
	Key   string `json:"key"`
	Count uint64 `json:"count"`
}

type timelineEntry struct {
	OccurredAt time.Time
	Item       model.DeploymentTimelineItem
}

func New(deps Dependencies) *Service {
	settings := deps.Settings
	if settings.CacheTTL <= 0 {
		settings.CacheTTL = 10 * time.Second
	}
	if settings.RequestTimeout <= 0 {
		settings.RequestTimeout = 3 * time.Second
	}
	if settings.EventLookupTimeout <= 0 {
		settings.EventLookupTimeout = 2 * time.Second
	}
	if settings.HeartbeatStaleAfter <= 0 {
		settings.HeartbeatStaleAfter = 2 * time.Minute
	}
	if settings.DiagnosticsStaleAfter <= 0 {
		settings.DiagnosticsStaleAfter = 5 * time.Minute
	}
	if settings.NoLogsWindow <= 0 {
		settings.NoLogsWindow = 15 * time.Minute
	}
	if settings.DeploymentListLimit == 0 {
		settings.DeploymentListLimit = 50
	}
	return &Service{
		bridge:      deps.Bridge,
		logger:      deps.Logger,
		subjects:    deps.Subjects,
		settings:    settings,
		now:         func() time.Time { return time.Now().UTC() },
		cache:       memo{items: map[string]memoValue{}},
		eventStates: map[string]eventState{},
	}
}

func cachedValue[T any](s *Service, key string, loader func() (T, error)) (T, error) {
	var zero T
	now := s.now()
	if value, ok := s.cache.get(key, now); ok {
		cached, ok := value.(T)
		if ok {
			return cached, nil
		}
	}
	value, err := loader()
	if err != nil {
		return zero, err
	}
	s.cache.set(key, now.Add(s.settings.CacheTTL), value)
	return value, nil
}

func (m *memo) get(key string, now time.Time) (any, bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	item, ok := m.items[key]
	if !ok {
		return nil, false
	}
	if now.After(item.expiresAt) {
		delete(m.items, key)
		return nil, false
	}
	return item.value, true
}

func (m *memo) set(key string, expiresAt time.Time, value any) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.items[key] = memoValue{expiresAt: expiresAt, value: value}
}

func (m *memo) delete(key string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	delete(m.items, key)
}

func (s *Service) request(ctx context.Context, subject string, payload []byte) (*nats.Msg, error) {
	if s.bridge == nil {
		return nil, unavailableError("edge bridge is not ready")
	}
	ctx, cancel := s.withTimeout(ctx)
	defer cancel()
	return s.bridge.Request(ctx, subject, payload)
}

func (s *Service) withTimeout(ctx context.Context) (context.Context, context.CancelFunc) {
	if ctx == nil {
		return context.WithTimeout(context.Background(), s.settings.RequestTimeout)
	}
	if _, ok := ctx.Deadline(); ok {
		return ctx, func() {}
	}
	return context.WithTimeout(ctx, s.settings.RequestTimeout)
}

func requestJSONEnvelope[T any](s *Service, ctx context.Context, subject string, request any) (T, error) {
	var zero T
	data, err := json.Marshal(request)
	if err != nil {
		return zero, err
	}
	replyMsg, err := s.request(ctx, subject, data)
	if err != nil {
		return zero, err
	}
	reply, err := envelope.DecodeAgentReplyEnvelope(replyMsg.Data)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	if strings.EqualFold(reply.Status, "error") {
		return zero, mapReplyError(reply.Code, reply.Message)
	}
	if len(reply.Payload) == 0 {
		return zero, nil
	}
	if err := json.Unmarshal(reply.Payload, &zero); err != nil {
		return zero, badResponse(subject, err)
	}
	return zero, nil
}

func requestAgentEnvelope[T any](s *Service, ctx context.Context, subject string, request []byte, decode func([]byte) (T, error)) (T, error) {
	var zero T
	replyMsg, err := s.request(ctx, subject, request)
	if err != nil {
		return zero, err
	}
	reply, err := envelope.DecodeAgentReplyEnvelope(replyMsg.Data)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	if strings.EqualFold(reply.Status, "error") {
		return zero, mapReplyError(reply.Code, reply.Message)
	}
	if len(reply.Payload) == 0 {
		return zero, nil
	}
	value, err := decode(reply.Payload)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	return value, nil
}

func requestControlEnvelope[T any](s *Service, ctx context.Context, subject string, request []byte, decode func([]byte) (T, error)) (T, error) {
	var zero T
	replyMsg, err := s.request(ctx, subject, request)
	if err != nil {
		return zero, err
	}
	reply, err := envelope.DecodeControlReplyEnvelope(replyMsg.Data)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	if strings.EqualFold(reply.Status, "error") {
		return zero, mapReplyError(reply.Code, reply.Message)
	}
	if len(reply.Payload) == 0 {
		return zero, nil
	}
	value, err := decode(reply.Payload)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	return value, nil
}

func requestDeploymentEnvelope[T any](s *Service, ctx context.Context, subject string, request []byte, decode func([]byte) (T, error)) (T, error) {
	var zero T
	replyMsg, err := s.request(ctx, subject, request)
	if err != nil {
		return zero, err
	}
	reply, err := envelope.DecodeDeploymentReplyEnvelope(replyMsg.Data)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	if strings.EqualFold(reply.Status, "error") {
		return zero, mapReplyError(reply.Code, reply.Message)
	}
	if len(reply.Payload) == 0 {
		return zero, nil
	}
	value, err := decode(reply.Payload)
	if err != nil {
		return zero, badResponse(subject, err)
	}
	return value, nil
}

func (s *Service) GetHostAgentStatus(ctx context.Context, hostID string) (model.HostAgentStatusView, error) {
	hostID = strings.TrimSpace(hostID)
	if hostID == "" {
		return model.HostAgentStatusView{}, invalidArgument("host id is required")
	}
	return cachedValue(s, "host-status:"+hostID, func() (model.HostAgentStatusView, error) {
		rm, err := s.collectHostReadModel(ctx, hostID, true)
		if err != nil {
			return model.HostAgentStatusView{}, err
		}
		return s.buildHostStatusView(rm), nil
	})
}

func (s *Service) GetHostDiagnostics(ctx context.Context, hostID string) (model.HostAgentDiagnosticsView, error) {
	hostID = strings.TrimSpace(hostID)
	if hostID == "" {
		return model.HostAgentDiagnosticsView{}, invalidArgument("host id is required")
	}
	return cachedValue(s, "host-diagnostics:"+hostID, func() (model.HostAgentDiagnosticsView, error) {
		rm, err := s.collectHostReadModel(ctx, hostID, false)
		if err != nil {
			return model.HostAgentDiagnosticsView{}, err
		}
		return s.buildHostDiagnosticsView(rm), nil
	})
}

func (s *Service) GetClusterAgentsOverview(ctx context.Context, clusterID string) (model.ClusterAgentsOverviewView, error) {
	clusterID = strings.TrimSpace(clusterID)
	if clusterID == "" {
		return model.ClusterAgentsOverviewView{}, invalidArgument("cluster id is required")
	}
	return cachedValue(s, "cluster-overview:"+clusterID, func() (model.ClusterAgentsOverviewView, error) {
		cluster, err := s.getCluster(ctx, clusterID)
		if err != nil {
			return model.ClusterAgentsOverviewView{}, err
		}
		out := model.ClusterAgentsOverviewView{
			ClusterID:       cluster.Cluster.ClusterID,
			ClusterName:     cluster.Cluster.Name,
			NoLogsWindowMin: int64(s.settings.NoLogsWindow / time.Minute),
			Hosts:           make([]model.ClusterAgentHostSummary, 0, len(cluster.Hosts)),
			DataFreshness:   s.newFreshness(),
		}
		s.observeSection(out.DataFreshness.Sections, "cluster", cluster.Cluster.UpdatedAt, s.settings.CacheTTL, "")

		for _, hostBinding := range cluster.Hosts {
			rm, err := s.collectHostReadModel(ctx, hostBinding.HostID, false)
			if err != nil {
				var reqErr *requestError
				if errors.As(err, &reqErr) && reqErr.Code == "not_found" {
					continue
				}
				out.MissingSections = appendMissing(out.MissingSections, "hosts:"+hostBinding.HostID)
				continue
			}
			status := s.buildHostStatusView(rm)
			out.TotalHosts++
			if status.AgentID != "" || status.LastSuccessfulDeployAt != "" {
				out.DeployedAgents++
			}
			if status.EnrollmentStatus == "not_enrolled" {
				out.NeverEnrolled++
			}
			if status.HeartbeatStatus == "stale" {
				out.StaleHeartbeat++
			}
			if status.DeploymentStatus == "failed" {
				out.DeploymentFailed++
			}
			if status.DoctorStatus == "warn" {
				out.DiagnosticsWarn++
			}
			if status.DoctorStatus == "fail" {
				out.DiagnosticsFail++
			}
			if status.EnrollmentStatus == "enrolled" && status.HeartbeatStatus == "healthy" && status.DoctorStatus == "pass" {
				out.HealthyAgents++
			}
			if s.isNoLogs(status.LastLogSeenAt) {
				out.NoLogsInLastNMin++
			}
			out.Hosts = append(out.Hosts, model.ClusterAgentHostSummary{
				HostID:               status.HostID,
				Hostname:             status.Hostname,
				ClusterID:            firstNonEmpty(status.ClusterID, clusterID),
				AgentID:              status.AgentID,
				DeploymentStatus:     status.DeploymentStatus,
				EnrollmentStatus:     status.EnrollmentStatus,
				HeartbeatStatus:      status.HeartbeatStatus,
				DoctorStatus:         status.DoctorStatus,
				LastHeartbeatAt:      status.LastHeartbeatAt,
				LastDiagnosticsAt:    status.LastDiagnosticsAt,
				LastLogSeenAt:        status.LastLogSeenAt,
				PrimaryFailureDomain: status.PrimaryFailureDomain,
				HumanHint:            status.HumanHint,
			})
			out.MissingSections = appendMissing(out.MissingSections, status.MissingSections...)
		}

		sort.Slice(out.Hosts, func(i, j int) bool {
			return strings.ToLower(out.Hosts[i].Hostname) < strings.ToLower(out.Hosts[j].Hostname)
		})
		return out, nil
	})
}

func (s *Service) GetDeploymentTimeline(ctx context.Context, jobID string) (model.DeploymentTimelineView, error) {
	jobID = strings.TrimSpace(jobID)
	if jobID == "" {
		return model.DeploymentTimelineView{}, invalidArgument("deployment job id is required")
	}
	return cachedValue(s, "deployment-timeline:"+jobID, func() (model.DeploymentTimelineView, error) {
		job, err := s.getDeploymentJob(ctx, jobID)
		if err != nil {
			return model.DeploymentTimelineView{}, err
		}
		items := make([]timelineEntry, 0, len(job.Steps)+len(job.Targets)+len(job.Attempts)+1)
		if job.Job != nil && strings.TrimSpace(job.Job.CreatedAt) != "" {
			items = append(items, timelineEntry{
				OccurredAt: parseTimestamp(job.Job.CreatedAt),
				Item: model.DeploymentTimelineItem{
					PhaseType:  "plan_created",
					Status:     "succeeded",
					OccurredAt: job.Job.CreatedAt,
				},
			})
		}

		attemptNumbers := make(map[string]uint32, len(job.Attempts))
		targets := make(map[string]envelope.DeploymentTargetSummary, len(job.Targets))
		for _, attempt := range job.Attempts {
			attemptNumbers[attempt.DeploymentAttemptID] = attempt.AttemptNo
			occurredAt := firstNonEmpty(attempt.StartedAt, attempt.CreatedAt)
			if strings.TrimSpace(occurredAt) != "" {
				items = append(items, timelineEntry{
					OccurredAt: parseTimestamp(occurredAt),
					Item: model.DeploymentTimelineItem{
						PhaseType:           "ansible_started",
						Status:              deploymentJobStatusLabel(attempt.Status),
						OccurredAt:          occurredAt,
						DeploymentAttemptID: attempt.DeploymentAttemptID,
						AttemptNo:           attempt.AttemptNo,
						Message:             firstNonEmpty(attempt.Reason, "deployment attempt started"),
					},
				})
			}
		}
		for _, target := range job.Targets {
			targets[target.DeploymentTargetID] = target
		}
		for _, step := range job.Steps {
			target := targets[step.DeploymentTargetID]
			phase := normalizeTimelinePhase(step.StepName, step.Status)
			if phase == "" {
				continue
			}
			occurredAt := firstNonEmpty(step.UpdatedAt, step.CreatedAt)
			items = append(items, timelineEntry{
				OccurredAt: parseTimestamp(occurredAt),
				Item: model.DeploymentTimelineItem{
					PhaseType:           phase,
					Status:              deploymentStepStatusLabel(step.Status),
					OccurredAt:          occurredAt,
					DeploymentAttemptID: step.DeploymentAttemptID,
					DeploymentTargetID:  step.DeploymentTargetID,
					AttemptNo:           attemptNumbers[step.DeploymentAttemptID],
					HostID:              target.HostID,
					Hostname:            target.HostnameSnapshot,
					RawStepName:         step.StepName,
					Message:             step.Message,
					PayloadJSON:         parseJSONString(step.PayloadJSON),
				},
			})
		}
		for _, target := range job.Targets {
			occurredAt := firstNonEmpty(target.FinishedAt, target.UpdatedAt)
			if strings.TrimSpace(occurredAt) == "" {
				continue
			}
			phase := ""
			switch target.Status {
			case 3:
				phase = "health_check_passed"
			case 4:
				phase = "health_check_failed"
			}
			if phase == "" {
				continue
			}
			items = append(items, timelineEntry{
				OccurredAt: parseTimestamp(occurredAt),
				Item: model.DeploymentTimelineItem{
					PhaseType:           phase,
					Status:              deploymentTargetStatusLabel(target.Status),
					OccurredAt:          occurredAt,
					DeploymentAttemptID: target.DeploymentAttemptID,
					DeploymentTargetID:  target.DeploymentTargetID,
					AttemptNo:           attemptNumbers[target.DeploymentAttemptID],
					HostID:              target.HostID,
					Hostname:            target.HostnameSnapshot,
					Message:             target.ErrorMessage,
				},
			})
		}
		sort.SliceStable(items, func(i, j int) bool {
			if items[i].OccurredAt.Equal(items[j].OccurredAt) {
				return items[i].Item.PhaseType < items[j].Item.PhaseType
			}
			return items[i].OccurredAt.Before(items[j].OccurredAt)
		})
		responseItems := make([]model.DeploymentTimelineItem, 0, len(items))
		for _, item := range items {
			responseItems = append(responseItems, item.Item)
		}

		out := model.DeploymentTimelineView{
			JobID:         jobID,
			Status:        "unknown",
			Items:         responseItems,
			DataFreshness: s.newFreshness(),
		}
		if job.Job != nil {
			out.Status = deploymentJobStatusLabel(job.Job.Status)
			out.CurrentPhase = job.Job.CurrentPhase
			s.observeSection(out.DataFreshness.Sections, "deployment", firstNonEmpty(job.Job.UpdatedAt, job.Job.CreatedAt), s.settings.CacheTTL, "")
		}
		if len(out.Items) == 0 {
			out.MissingSections = appendMissing(out.MissingSections, "timeline.items")
		}
		return out, nil
	})
}

func (s *Service) MapAgentStreamEventJSON(data []byte) ([]byte, error) {
	var event uiAgentStreamEvent
	if err := json.Unmarshal(data, &event); err != nil {
		return nil, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), s.settings.EventLookupTimeout)
	defer cancel()

	host, _ := s.findHostByHostname(ctx, event.Hostname)
	hostID := ""
	if host != nil {
		hostID = host.HostID
		s.invalidateHost(hostID)
	}

	clusterID := ""
	var status model.HostAgentStatusView
	var err error
	if hostID != "" {
		status, err = s.GetHostAgentStatus(ctx, hostID)
	} else {
		err = notFoundError("host is not known yet")
	}
	if err == nil && status.HostID != "" {
		hostID = status.HostID
		clusterID = status.ClusterID
	}

	eventType := "agent.heartbeat.updated"
	switch strings.TrimSpace(event.EventType) {
	case "enrolled":
		eventType = "agent.enrollment.updated"
	case "diagnostics":
		eventType = "agent.diagnostics.updated"
	case "heartbeat":
		eventType = "agent.heartbeat.updated"
	}

	severity := "info"
	problem := false
	if err == nil && status.HostID != "" {
		severity = severityFromStatus(status)
		problem = severity == "warn" || severity == "error"
		if eventType == "agent.heartbeat.updated" && (status.PrimaryFailureDomain == "network" || status.PrimaryFailureDomain == "tls") {
			eventType = "agent.connectivity.problem"
		}
		eventType = s.adjustRecoveredEvent(hostID, eventType, problem)
	}

	return json.Marshal(model.AgentRuntimeEvent{
		EventType:  eventType,
		HostID:     hostID,
		ClusterID:  clusterID,
		Severity:   severity,
		OccurredAt: firstNonEmpty(event.LastSeenAt, s.now().Format(time.RFC3339)),
		Payload: map[string]any{
			"agent_id":     event.AgentID,
			"hostname":     event.Hostname,
			"status":       event.Status,
			"version":      event.Version,
			"last_seen_at": event.LastSeenAt,
		},
	})
}

func (s *Service) MapDeploymentStepEventJSON(data []byte) ([]byte, error) {
	event, err := envelope.DecodeDeploymentStepEvent(data)
	if err != nil {
		return nil, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), s.settings.EventLookupTimeout)
	defer cancel()

	job, err := s.getDeploymentJob(ctx, event.JobID)
	if err != nil {
		return nil, err
	}
	var target *envelope.DeploymentTargetSummary
	for _, item := range job.Targets {
		if item.DeploymentTargetID == event.DeploymentTargetID {
			targetCopy := item
			target = &targetCopy
			break
		}
	}
	if target == nil {
		return nil, fmt.Errorf("deployment target %s is not present in job %s", event.DeploymentTargetID, event.JobID)
	}

	clusterID := ""
	if target.HostID != "" {
		rm, err := s.collectHostReadModel(ctx, target.HostID, false)
		if err == nil && rm.cluster != nil {
			clusterID = rm.cluster.Cluster.ClusterID
		}
		s.invalidateHost(target.HostID)
	}
	s.invalidateJob(event.JobID)

	phase := normalizeTimelinePhase(event.StepName, event.Status)
	if phase == "" {
		phase = "service_restarted"
	}
	severity := "info"
	if event.Status == 4 {
		severity = "error"
	}
	return json.Marshal(model.AgentRuntimeEvent{
		EventType:  "agent.deployment.updated",
		HostID:     target.HostID,
		ClusterID:  clusterID,
		Severity:   severity,
		OccurredAt: firstNonEmpty(event.UpdatedAt, s.now().Format(time.RFC3339)),
		Payload: map[string]any{
			"job_id":                event.JobID,
			"deployment_attempt_id": event.DeploymentAttemptID,
			"deployment_step_id":    event.DeploymentStepID,
			"deployment_target_id":  event.DeploymentTargetID,
			"phase_type":            phase,
			"raw_step_name":         event.StepName,
			"status":                deploymentStepStatusLabel(event.Status),
			"message":               event.Message,
			"hostname":              target.HostnameSnapshot,
		},
	})
}

func (s *Service) collectHostReadModel(ctx context.Context, hostID string, includeTraffic bool) (hostReadModel, error) {
	host, err := s.getHost(ctx, hostID)
	if err != nil {
		return hostReadModel{}, err
	}
	out := hostReadModel{
		host:      host,
		freshness: s.newFreshness().Sections,
	}
	s.observeSection(out.freshness, "host", firstNonEmpty(host.UpdatedAt, host.CreatedAt), s.settings.CacheTTL, "")

	cluster, err := s.getClusterForHost(ctx, hostID)
	if err != nil {
		out.missingSections = appendMissing(out.missingSections, "identity.cluster")
		s.observeMissing(out.freshness, "cluster", "cluster membership is unavailable")
	} else if cluster != nil {
		out.cluster = cluster
		s.observeSection(out.freshness, "cluster", cluster.Cluster.UpdatedAt, s.settings.CacheTTL, "")
	}

	agentSummary, agentErr := s.findAgentForHost(ctx, host)
	if agentErr != nil {
		out.missingSections = appendMissing(out.missingSections, "enrollment.agent_lookup")
		s.observeMissing(out.freshness, "enrollment", "agent lookup is unavailable")
	} else if agentSummary != nil {
		out.agentSummary = agentSummary
		agentDetail, err := s.getAgent(ctx, agentSummary.AgentID)
		if err != nil {
			out.missingSections = appendMissing(out.missingSections, "enrollment.agent_detail")
			s.observeMissing(out.freshness, "enrollment", "agent detail is unavailable")
		} else {
			out.agentDetail = &agentDetail
			s.observeSection(out.freshness, "enrollment", firstNonEmpty(agentDetail.LastSeenAt, agentDetail.FirstSeenAt), s.settings.HeartbeatStaleAfter, "")
		}

		diagnostics, err := s.getAgentDiagnostics(ctx, agentSummary.AgentID)
		if err != nil {
			var reqErr *requestError
			if !errors.As(err, &reqErr) || reqErr.Code != "not_found" {
				out.missingSections = appendMissing(out.missingSections, "diagnostics.snapshot")
			}
			s.observeMissing(out.freshness, "diagnostics", "diagnostics snapshot is unavailable")
		} else {
			out.diagnostics = diagnostics
			s.observeSection(out.freshness, "diagnostics", diagnostics.CreatedAt, s.settings.DiagnosticsStaleAfter, "")
			if diagnostics.ParseErr != nil {
				out.missingSections = appendMissing(out.missingSections, "diagnostics.parsed")
				s.observeMissing(out.freshness, "diagnostics.parsed", "diagnostics payload partially available, normalized checks degraded")
			}
		}
	} else {
		s.observeMissing(out.freshness, "enrollment", "agent has not enrolled yet")
	}

	deployment, err := s.findDeploymentForHost(ctx, hostID)
	if err != nil {
		out.missingSections = appendMissing(out.missingSections, "deployment.summary")
		s.observeMissing(out.freshness, "deployment", "deployment history is unavailable")
	} else {
		out.deployment = deployment
		if deployment.CurrentJob != nil {
			s.observeSection(out.freshness, "deployment", firstNonEmpty(deployment.CurrentJob.UpdatedAt, deployment.CurrentJob.CreatedAt), s.settings.CacheTTL, "")
		} else {
			s.observeMissing(out.freshness, "deployment", "deployment history is empty")
		}
	}

	if out.diagnostics.Snapshot != nil {
		out.traffic.LastBatchSentAt = unixMillisToRFC3339(out.diagnostics.Snapshot.LastSuccessfulSendAt)
	}
	if includeTraffic {
		traffic, err := s.lookupTrafficSummary(ctx, host)
		if err != nil {
			out.missingSections = appendMissing(out.missingSections, "traffic.logs")
			s.observeMissing(out.freshness, "traffic", "query-plane traffic summary is unavailable")
		} else {
			out.traffic = traffic
			s.observeSection(out.freshness, "traffic", firstNonEmpty(traffic.LastLogSeenAt, traffic.LastBatchSentAt), s.settings.NoLogsWindow, "")
		}
	}
	if strings.TrimSpace(out.traffic.LastLogSeenAt) == "" {
		out.traffic.LastLogSeenAt = s.fallbackLastLogSeenAt(out.diagnostics.Snapshot)
	}
	return out, nil
}

func (s *Service) buildHostStatusView(rm hostReadModel) model.HostAgentStatusView {
	enrollmentStatus := "not_enrolled"
	enrolledAt := ""
	lastHeartbeatAt := ""
	heartbeatStatus := "missing"
	edgeConnected := false
	agentID := ""
	if rm.agentDetail != nil {
		agentID = rm.agentDetail.AgentID
		enrollmentStatus = enrollmentStatusFromAgent(*rm.agentDetail)
		enrolledAt = rm.agentDetail.FirstSeenAt
		lastHeartbeatAt = rm.agentDetail.LastSeenAt
		heartbeatStatus = s.heartbeatStatus(rm.agentDetail.LastSeenAt)
		edgeConnected = heartbeatStatus == "healthy"
	}
	presentation := s.presentDiagnostics(rm.diagnostics.Snapshot)
	clusterID, clusterName := s.identityCluster(rm)
	serviceName, environment := s.identityScope(rm)

	out := model.HostAgentStatusView{
		HostID:                rm.host.HostID,
		Hostname:              rm.host.Hostname,
		ClusterID:             clusterID,
		ClusterName:           clusterName,
		ServiceName:           serviceName,
		Environment:           environment,
		AgentID:               agentID,
		DeploymentStatus:      "unknown",
		EnrollmentStatus:      enrollmentStatus,
		EnrolledAt:            enrolledAt,
		EdgeConnected:         edgeConnected,
		LastHeartbeatAt:       lastHeartbeatAt,
		HeartbeatStatus:       heartbeatStatus,
		LastDiagnosticsAt:     rm.diagnostics.CreatedAt,
		DoctorStatus:          presentation.Summary.DoctorStatus,
		WarningCount:          presentation.Summary.WarningCount,
		FailureCount:          presentation.Summary.FailureCount,
		TopIssues:             presentation.Summary.TopIssues,
		SpoolStatus:           presentation.Summary.SpoolStatus,
		TransportStatus:       presentation.Summary.TransportStatus,
		TLSStatus:             presentation.Summary.TLSStatus,
		SourceStatus:          presentation.Summary.SourceStatus,
		LastLogSeenAt:         rm.traffic.LastLogSeenAt,
		LastBatchSentAt:       rm.traffic.LastBatchSentAt,
		RecentLogsPerMinute:   rm.traffic.RecentLogsPerMinute,
		RecentErrorsPerMinute: rm.traffic.RecentErrorsPerMinute,
		LastKnownErrors:       presentation.Errors,
		LastKnownWarnings:     presentation.Warnings,
		PrimaryFailureDomain:  "unknown",
		HumanHint:             "Снимок состояния собран, но явная доменная причина пока не выделена.",
		SuggestedNextStep:     "Открыть diagnostics snapshot и последние deployment steps.",
		MissingSections:       rm.missingSections,
		DataFreshness: model.AgentDataFreshness{
			GeneratedAt: s.now().Format(time.RFC3339),
			Sections:    rm.freshness,
		},
	}
	if rm.deployment.CurrentJob != nil {
		out.DeploymentJobID = rm.deployment.CurrentJob.JobID
		out.DeploymentStatus = deploymentJobStatusLabel(rm.deployment.CurrentJob.Status)
		out.DeploymentCurrentPhase = rm.deployment.CurrentJob.CurrentPhase
		out.ExecutorKind = deploymentExecutorKindLabel(rm.deployment.CurrentJob.ExecutorKind)
	}
	out.LastSuccessfulDeployAt = rm.deployment.LastSuccessfulDeployAt
	out.LastFailedDeployAt = rm.deployment.LastFailedDeployAt
	out.InstallMode = firstNonEmpty(rm.deployment.InstallMode, diagnosticsInstallMode(rm.diagnostics.Snapshot))
	out.ArtifactRef = rm.deployment.ArtifactRef
	out.PrimaryFailureDomain = determinePrimaryFailureDomain(out, presentation)
	out.HumanHint, out.SuggestedNextStep = buildHint(out, presentation)
	return out
}

func (s *Service) buildHostDiagnosticsView(rm hostReadModel) model.HostAgentDiagnosticsView {
	presentation := s.presentDiagnostics(rm.diagnostics.Snapshot)
	agentID := ""
	if rm.agentDetail != nil {
		agentID = rm.agentDetail.AgentID
	}
	return model.HostAgentDiagnosticsView{
		HostID:          rm.host.HostID,
		Hostname:        rm.host.Hostname,
		AgentID:         agentID,
		CollectedAt:     rm.diagnostics.CreatedAt,
		SnapshotJSON:    rm.diagnostics.Raw,
		Summary:         presentation.Summary,
		Checks:          presentation.Checks,
		DegradedMode:    presentation.Degraded,
		RuntimeErrors:   presentation.Errors,
		TransportErrors: filterIssuesByDomains(presentation.Errors, "transport", "tls"),
		SpoolWarnings:   filterIssuesByDomains(presentation.Warnings, "spool", "ingestion"),
		MissingSections: rm.missingSections,
		DataFreshness: model.AgentDataFreshness{
			GeneratedAt: s.now().Format(time.RFC3339),
			Sections:    rm.freshness,
		},
	}
}

func (s *Service) getHost(ctx context.Context, hostID string) (envelope.ControlHost, error) {
	return cachedValue(s, "control-host:"+hostID, func() (envelope.ControlHost, error) {
		host, err := requestControlEnvelope(
			s,
			ctx,
			s.subjects.ControlHostsGet,
			envelope.EncodeControlGetHostRequest("", hostID),
			envelope.DecodeControlHost,
		)
		if err != nil {
			return envelope.ControlHost{}, err
		}
		if strings.TrimSpace(host.HostID) == "" {
			return envelope.ControlHost{}, notFoundError("host not found")
		}
		return host, nil
	})
}

func (s *Service) listHosts(ctx context.Context) ([]envelope.ControlHost, error) {
	return cachedValue(s, "control-hosts", func() ([]envelope.ControlHost, error) {
		payload, err := requestControlEnvelope(
			s,
			ctx,
			s.subjects.ControlHostsList,
			envelope.EncodeControlListHostsRequest(""),
			envelope.DecodeControlListHostsResponse,
		)
		if err != nil {
			return nil, err
		}
		return payload.Hosts, nil
	})
}

func (s *Service) findHostByHostname(ctx context.Context, hostname string) (*envelope.ControlHost, error) {
	hosts, err := s.listHosts(ctx)
	if err != nil {
		return nil, err
	}
	for _, item := range hosts {
		if strings.EqualFold(item.Hostname, strings.TrimSpace(hostname)) {
			host := item
			return &host, nil
		}
	}
	return nil, nil
}

func (s *Service) getClusterForHost(ctx context.Context, hostID string) (*envelope.ControlClusterDetails, error) {
	return cachedValue(s, "control-cluster-for-host:"+hostID, func() (*envelope.ControlClusterDetails, error) {
		payload, err := requestControlEnvelope(
			s,
			ctx,
			s.subjects.ControlClustersList,
			envelope.EncodeListClustersRequest(envelope.ListClustersRequest{
				HostID:         hostID,
				IncludeMembers: false,
			}),
			envelope.DecodeControlListClustersResponse,
		)
		if err != nil {
			return nil, err
		}
		if len(payload.Clusters) == 0 {
			return nil, nil
		}
		details, err := s.getCluster(ctx, payload.Clusters[0].ClusterID)
		if err != nil {
			return nil, err
		}
		return &details, nil
	})
}

func (s *Service) getCluster(ctx context.Context, clusterID string) (envelope.ControlClusterDetails, error) {
	return cachedValue(s, "control-cluster:"+clusterID, func() (envelope.ControlClusterDetails, error) {
		cluster, err := requestControlEnvelope(
			s,
			ctx,
			s.subjects.ControlClustersGet,
			envelope.EncodeGetClusterRequest("", clusterID, true),
			envelope.DecodeControlGetClusterResponse,
		)
		if err != nil {
			return envelope.ControlClusterDetails{}, err
		}
		if strings.TrimSpace(cluster.Cluster.ClusterID) == "" {
			return envelope.ControlClusterDetails{}, notFoundError("cluster not found")
		}
		return cluster, nil
	})
}

func (s *Service) listAgents(ctx context.Context) ([]envelope.AgentSummary, error) {
	return cachedValue(s, "agents-list", func() ([]envelope.AgentSummary, error) {
		payload, err := requestAgentEnvelope(
			s,
			ctx,
			s.subjects.AgentsList,
			envelope.EncodeListAgentsRequest(envelope.ListAgentsRequest{}),
			envelope.DecodeListAgentsResponse,
		)
		if err != nil {
			return nil, err
		}
		return payload.Agents, nil
	})
}

func (s *Service) findAgentForHost(ctx context.Context, host envelope.ControlHost) (*envelope.AgentSummary, error) {
	agents, err := s.listAgents(ctx)
	if err != nil {
		return nil, err
	}
	if agentID := strings.TrimSpace(host.Labels["agent_id"]); agentID != "" {
		for _, agent := range agents {
			if agent.AgentID == agentID {
				out := agent
				return &out, nil
			}
		}
	}
	candidates := make([]envelope.AgentSummary, 0, 2)
	for _, agent := range agents {
		if strings.EqualFold(agent.Hostname, host.Hostname) {
			candidates = append(candidates, agent)
		}
	}
	if len(candidates) == 0 {
		return nil, nil
	}
	sort.SliceStable(candidates, func(i, j int) bool {
		return parseTimestamp(candidates[i].LastSeenAt).After(parseTimestamp(candidates[j].LastSeenAt))
	})
	out := candidates[0]
	return &out, nil
}

func (s *Service) getAgent(ctx context.Context, agentID string) (envelope.AgentDetail, error) {
	return cachedValue(s, "agent-detail:"+agentID, func() (envelope.AgentDetail, error) {
		agent, err := requestAgentEnvelope(
			s,
			ctx,
			s.subjects.AgentsGet,
			envelope.EncodeGetAgentRequest(envelope.GetAgentRequest{AgentID: agentID}),
			envelope.DecodeAgentDetail,
		)
		if err != nil {
			return envelope.AgentDetail{}, err
		}
		if strings.TrimSpace(agent.AgentID) == "" {
			return envelope.AgentDetail{}, notFoundError("agent not found")
		}
		return agent, nil
	})
}

func (s *Service) getAgentDiagnostics(ctx context.Context, agentID string) (parsedDiagnostics, error) {
	return cachedValue(s, "agent-diagnostics:"+agentID, func() (parsedDiagnostics, error) {
		payload, err := requestAgentEnvelope(
			s,
			ctx,
			s.subjects.AgentsDiagnosticsGet,
			envelope.EncodeGetAgentDiagnosticsRequest(envelope.GetAgentDiagnosticsRequest{AgentID: agentID}),
			envelope.DecodeDiagnosticsSnapshot,
		)
		if err != nil {
			return parsedDiagnostics{}, err
		}
		out := parsedDiagnostics{CreatedAt: payload.CreatedAt}
		if strings.TrimSpace(payload.PayloadJSON) == "" {
			return out, nil
		}
		var raw any
		if err := json.Unmarshal([]byte(payload.PayloadJSON), &raw); err != nil {
			out.Raw = payload.PayloadJSON
			out.ParseErr = err
			return out, nil
		}
		out.Raw = raw
		var snapshot diagnosticsSnapshot
		if err := json.Unmarshal([]byte(payload.PayloadJSON), &snapshot); err != nil {
			out.ParseErr = err
			return out, nil
		}
		out.Snapshot = &snapshot
		return out, nil
	})
}

func (s *Service) listDeploymentJobs(ctx context.Context) ([]envelope.DeploymentJobSummary, error) {
	return cachedValue(s, "deployment-jobs", func() ([]envelope.DeploymentJobSummary, error) {
		payload, err := requestDeploymentEnvelope(
			s,
			ctx,
			s.subjects.DeploymentsJobsList,
			envelope.EncodeListDeploymentJobsRequest(envelope.ListDeploymentJobsRequest{Limit: s.settings.DeploymentListLimit}),
			envelope.DecodeListDeploymentJobsResponse,
		)
		if err != nil {
			return nil, err
		}
		return payload.Jobs, nil
	})
}

func (s *Service) getDeploymentJob(ctx context.Context, jobID string) (envelope.GetDeploymentJobResponse, error) {
	return cachedValue(s, "deployment-job:"+jobID, func() (envelope.GetDeploymentJobResponse, error) {
		job, err := requestDeploymentEnvelope(
			s,
			ctx,
			s.subjects.DeploymentsJobsGet,
			envelope.EncodeGetDeploymentJobRequest("", jobID),
			envelope.DecodeGetDeploymentJobResponse,
		)
		if err != nil {
			return envelope.GetDeploymentJobResponse{}, err
		}
		if job.Job == nil || strings.TrimSpace(job.Job.JobID) == "" {
			return envelope.GetDeploymentJobResponse{}, notFoundError("deployment job not found")
		}
		return job, nil
	})
}

func (s *Service) findDeploymentForHost(ctx context.Context, hostID string) (hostDeploymentInfo, error) {
	return cachedValue(s, "host-deployment:"+hostID, func() (hostDeploymentInfo, error) {
		jobs, err := s.listDeploymentJobs(ctx)
		if err != nil {
			return hostDeploymentInfo{}, err
		}
		var out hostDeploymentInfo
		for _, summary := range jobs {
			detail, err := s.getDeploymentJob(ctx, summary.JobID)
			if err != nil {
				continue
			}
			var matched *envelope.DeploymentTargetSummary
			for _, target := range detail.Targets {
				if target.HostID == hostID {
					targetCopy := target
					matched = &targetCopy
					break
				}
			}
			if matched == nil {
				continue
			}
			if out.CurrentJob == nil && detail.Job != nil {
				out.CurrentJob = detail.Job
				out.CurrentTarget = matched
				out.CurrentSteps = append([]envelope.DeploymentStepSummary(nil), detail.Steps...)
				out.ArtifactRef = extractArtifactRef(detail.Steps)
			}
			if out.LastSuccessfulDeployAt == "" && matched.Status == 3 {
				out.LastSuccessfulDeployAt = firstNonEmpty(matched.FinishedAt, summary.FinishedAt, summary.UpdatedAt)
			}
			if out.LastFailedDeployAt == "" && matched.Status == 4 {
				out.LastFailedDeployAt = firstNonEmpty(matched.FinishedAt, summary.FinishedAt, summary.UpdatedAt)
			}
			if out.CurrentJob != nil && out.LastSuccessfulDeployAt != "" && out.LastFailedDeployAt != "" {
				break
			}
		}
		return out, nil
	})
}

func (s *Service) lookupTrafficSummary(ctx context.Context, host envelope.ControlHost) (trafficSummary, error) {
	return cachedValue(s, "traffic:"+host.Hostname, func() (trafficSummary, error) {
		now := s.now()
		windowStart := now.Add(-time.Minute).Format(time.RFC3339)
		latest, err := requestJSONEnvelope[logSearchPayload](
			s,
			ctx,
			s.subjects.QueryLogsSearch,
			map[string]any{
				"filter": map[string]any{"host": host.Hostname},
				"limit":  1,
			},
		)
		if err != nil {
			return trafficSummary{}, err
		}
		recent, err := requestJSONEnvelope[logSearchPayload](
			s,
			ctx,
			s.subjects.QueryLogsSearch,
			map[string]any{
				"filter": map[string]any{
					"host": host.Hostname,
					"from": windowStart,
					"to":   now.Format(time.RFC3339),
				},
				"limit": 1,
			},
		)
		if err != nil {
			return trafficSummary{}, err
		}
		severity, err := requestJSONEnvelope[countBucketsPayload](
			s,
			ctx,
			s.subjects.QueryLogsSeverity,
			map[string]any{
				"filter": map[string]any{
					"host": host.Hostname,
					"from": windowStart,
					"to":   now.Format(time.RFC3339),
				},
				"limit": 16,
			},
		)
		if err != nil {
			return trafficSummary{}, err
		}
		out := trafficSummary{RecentLogsPerMinute: recent.Total}
		if len(latest.Items) > 0 {
			out.LastLogSeenAt = latest.Items[0].Timestamp
		}
		for _, item := range severity.Items {
			switch strings.ToLower(strings.TrimSpace(item.Key)) {
			case "error", "fatal", "critical", "panic":
				out.RecentErrorsPerMinute += item.Count
			}
		}
		return out, nil
	})
}

func (s *Service) invalidateHost(hostID string) {
	if strings.TrimSpace(hostID) == "" {
		return
	}
	s.cache.delete("host-status:" + hostID)
	s.cache.delete("host-diagnostics:" + hostID)
	s.cache.delete("host-deployment:" + hostID)
}

func (s *Service) invalidateJob(jobID string) {
	if strings.TrimSpace(jobID) == "" {
		return
	}
	s.cache.delete("deployment-job:" + jobID)
	s.cache.delete("deployment-jobs")
	s.cache.delete("deployment-timeline:" + jobID)
}

func (s *Service) adjustRecoveredEvent(hostID, fallback string, problem bool) string {
	if strings.TrimSpace(hostID) == "" {
		return fallback
	}
	s.eventStateMu.Lock()
	defer s.eventStateMu.Unlock()
	state := s.eventStates[hostID]
	next := fallback
	if state.problem && !problem {
		next = "agent.recovered"
	}
	state.problem = problem
	s.eventStates[hostID] = state
	return next
}

func (s *Service) heartbeatStatus(lastHeartbeatAt string) string {
	lastHeartbeatAt = strings.TrimSpace(lastHeartbeatAt)
	if lastHeartbeatAt == "" {
		return "missing"
	}
	if s.now().Sub(parseTimestamp(lastHeartbeatAt)) > s.settings.HeartbeatStaleAfter {
		return "stale"
	}
	return "healthy"
}

func (s *Service) fallbackLastLogSeenAt(snapshot *diagnosticsSnapshot) string {
	if snapshot == nil {
		return ""
	}
	var latest *int64
	for _, source := range snapshot.SourceStatuses {
		if source.LastReadAt == nil {
			continue
		}
		if latest == nil || *source.LastReadAt > *latest {
			value := *source.LastReadAt
			latest = &value
		}
	}
	return unixMillisToRFC3339(latest)
}

func (s *Service) identityCluster(rm hostReadModel) (string, string) {
	clusterID := ""
	clusterName := ""
	if rm.cluster != nil {
		clusterID = rm.cluster.Cluster.ClusterID
		clusterName = rm.cluster.Cluster.Name
	}
	if rm.diagnostics.Snapshot != nil {
		clusterID = firstNonEmpty(clusterID, ptrValue(rm.diagnostics.Snapshot.Cluster.ConfiguredClusterID))
		clusterName = firstNonEmpty(clusterName, ptrValue(rm.diagnostics.Snapshot.Cluster.ClusterName))
	}
	clusterID = firstNonEmpty(clusterID, rm.host.Labels["cluster_id"])
	clusterName = firstNonEmpty(clusterName, rm.host.Labels["cluster_name"])
	return clusterID, clusterName
}

func (s *Service) identityScope(rm hostReadModel) (string, string) {
	serviceName := rm.host.Labels["service_name"]
	environment := rm.host.Labels["environment"]
	if rm.diagnostics.Snapshot != nil {
		serviceName = firstNonEmpty(serviceName, ptrValue(rm.diagnostics.Snapshot.Cluster.ServiceName))
		environment = firstNonEmpty(environment, ptrValue(rm.diagnostics.Snapshot.Cluster.Environment))
	}
	return serviceName, environment
}
