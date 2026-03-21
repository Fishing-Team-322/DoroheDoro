package httpapi

import (
	"net/http"
	"strconv"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

type deploymentUpsertRequest struct {
	JobType             string   `json:"job_type"`
	PolicyID            string   `json:"policy_id"`
	TargetHostIDs       []string `json:"target_host_ids"`
	TargetHostGroupIDs  []string `json:"target_host_group_ids"`
	CredentialProfileID string   `json:"credential_profile_id"`
	RequestedBy         string   `json:"requested_by"`
	PreserveState       bool     `json:"preserve_state"`
	Force               bool     `json:"force"`
	DryRun              bool     `json:"dry_run"`
}

type deploymentRetryRequest struct {
	Strategy    string `json:"strategy"`
	TriggeredBy string `json:"triggered_by"`
	Reason      string `json:"reason"`
}

type deploymentCancelRequest struct {
	RequestedBy string `json:"requested_by"`
	Reason      string `json:"reason"`
}

type deploymentJobItem struct {
	JobID               string `json:"job_id"`
	JobType             string `json:"job_type"`
	Status              string `json:"status"`
	RequestedBy         string `json:"requested_by"`
	PolicyID            string `json:"policy_id"`
	PolicyRevisionID    string `json:"policy_revision_id"`
	CredentialProfileID string `json:"credential_profile_id"`
	ExecutorKind        string `json:"executor_kind"`
	CurrentPhase        string `json:"current_phase"`
	TotalTargets        uint32 `json:"total_targets"`
	PendingTargets      uint32 `json:"pending_targets"`
	RunningTargets      uint32 `json:"running_targets"`
	SucceededTargets    uint32 `json:"succeeded_targets"`
	FailedTargets       uint32 `json:"failed_targets"`
	CancelledTargets    uint32 `json:"cancelled_targets"`
	AttemptCount        uint32 `json:"attempt_count"`
	CreatedAt           string `json:"created_at"`
	StartedAt           string `json:"started_at,omitempty"`
	FinishedAt          string `json:"finished_at,omitempty"`
	UpdatedAt           string `json:"updated_at"`
}

type deploymentAttemptItem struct {
	DeploymentAttemptID string `json:"deployment_attempt_id"`
	AttemptNo           uint32 `json:"attempt_no"`
	Status              string `json:"status"`
	TriggeredBy         string `json:"triggered_by"`
	Reason              string `json:"reason,omitempty"`
	CreatedAt           string `json:"created_at"`
	StartedAt           string `json:"started_at,omitempty"`
	FinishedAt          string `json:"finished_at,omitempty"`
}

type deploymentTargetItem struct {
	DeploymentTargetID  string `json:"deployment_target_id"`
	DeploymentAttemptID string `json:"deployment_attempt_id"`
	HostID              string `json:"host_id"`
	HostnameSnapshot    string `json:"hostname_snapshot"`
	Status              string `json:"status"`
	ErrorMessage        string `json:"error_message,omitempty"`
	CreatedAt           string `json:"created_at"`
	StartedAt           string `json:"started_at,omitempty"`
	FinishedAt          string `json:"finished_at,omitempty"`
	UpdatedAt           string `json:"updated_at"`
}

type deploymentStepItem struct {
	DeploymentStepID    string `json:"deployment_step_id"`
	DeploymentAttemptID string `json:"deployment_attempt_id"`
	DeploymentTargetID  string `json:"deployment_target_id,omitempty"`
	StepName            string `json:"step_name"`
	Status              string `json:"status"`
	Message             string `json:"message,omitempty"`
	PayloadJSON         any    `json:"payload_json,omitempty"`
	CreatedAt           string `json:"created_at"`
	UpdatedAt           string `json:"updated_at"`
}

type deploymentPlanTargetItem struct {
	HostID     string `json:"host_id"`
	Hostname   string `json:"hostname"`
	IP         string `json:"ip"`
	SSHPort    uint32 `json:"ssh_port"`
	RemoteUser string `json:"remote_user"`
}

type bootstrapPreviewItem struct {
	HostID        string `json:"host_id"`
	Hostname      string `json:"hostname"`
	BootstrapYAML string `json:"bootstrap_yaml"`
}

type deploymentPlanItem struct {
	JobType             string                     `json:"job_type"`
	PolicyID            string                     `json:"policy_id"`
	PolicyRevisionID    string                     `json:"policy_revision_id"`
	PolicyRevision      string                     `json:"policy_revision"`
	CredentialProfileID string                     `json:"credential_profile_id"`
	CredentialSummary   string                     `json:"credential_summary"`
	ExecutorKind        string                     `json:"executor_kind"`
	ActionSummary       string                     `json:"action_summary"`
	Targets             []deploymentPlanTargetItem `json:"targets"`
	BootstrapPreviews   []bootstrapPreviewItem     `json:"bootstrap_previews"`
	Warnings            []string                   `json:"warnings,omitempty"`
}

func mapDeploymentJobItem(item envelope.DeploymentJobSummary) deploymentJobItem {
	return deploymentJobItem{
		JobID:               item.JobID,
		JobType:             deploymentJobTypeLabel(item.JobType),
		Status:              deploymentJobStatusLabel(item.Status),
		RequestedBy:         item.RequestedBy,
		PolicyID:            item.PolicyID,
		PolicyRevisionID:    item.PolicyRevisionID,
		CredentialProfileID: item.CredentialProfileID,
		ExecutorKind:        deploymentExecutorKindLabel(item.ExecutorKind),
		CurrentPhase:        item.CurrentPhase,
		TotalTargets:        item.TotalTargets,
		PendingTargets:      item.PendingTargets,
		RunningTargets:      item.RunningTargets,
		SucceededTargets:    item.SucceededTargets,
		FailedTargets:       item.FailedTargets,
		CancelledTargets:    item.CancelledTargets,
		AttemptCount:        item.AttemptCount,
		CreatedAt:           item.CreatedAt,
		StartedAt:           item.StartedAt,
		FinishedAt:          item.FinishedAt,
		UpdatedAt:           item.UpdatedAt,
	}
}

func mapDeploymentAttemptItem(item envelope.DeploymentAttemptSummary) deploymentAttemptItem {
	return deploymentAttemptItem{
		DeploymentAttemptID: item.DeploymentAttemptID,
		AttemptNo:           item.AttemptNo,
		Status:              deploymentJobStatusLabel(item.Status),
		TriggeredBy:         item.TriggeredBy,
		Reason:              item.Reason,
		CreatedAt:           item.CreatedAt,
		StartedAt:           item.StartedAt,
		FinishedAt:          item.FinishedAt,
	}
}

func mapDeploymentTargetItem(item envelope.DeploymentTargetSummary) deploymentTargetItem {
	return deploymentTargetItem{
		DeploymentTargetID:  item.DeploymentTargetID,
		DeploymentAttemptID: item.DeploymentAttemptID,
		HostID:              item.HostID,
		HostnameSnapshot:    item.HostnameSnapshot,
		Status:              deploymentTargetStatusLabel(item.Status),
		ErrorMessage:        item.ErrorMessage,
		CreatedAt:           item.CreatedAt,
		StartedAt:           item.StartedAt,
		FinishedAt:          item.FinishedAt,
		UpdatedAt:           item.UpdatedAt,
	}
}

func mapDeploymentStepItem(item envelope.DeploymentStepSummary) deploymentStepItem {
	return deploymentStepItem{
		DeploymentStepID:    item.DeploymentStepID,
		DeploymentAttemptID: item.DeploymentAttemptID,
		DeploymentTargetID:  item.DeploymentTargetID,
		StepName:            item.StepName,
		Status:              deploymentStepStatusLabel(item.Status),
		Message:             item.Message,
		PayloadJSON:         parseJSONString(item.PayloadJSON),
		CreatedAt:           item.CreatedAt,
		UpdatedAt:           item.UpdatedAt,
	}
}

func mapDeploymentPlanItem(item envelope.CreateDeploymentPlanResponse) deploymentPlanItem {
	targets := make([]deploymentPlanTargetItem, 0, len(item.Targets))
	for _, target := range item.Targets {
		targets = append(targets, deploymentPlanTargetItem{
			HostID:     target.HostID,
			Hostname:   target.Hostname,
			IP:         target.IP,
			SSHPort:    target.SSHPort,
			RemoteUser: target.RemoteUser,
		})
	}
	previews := make([]bootstrapPreviewItem, 0, len(item.BootstrapPreviews))
	for _, preview := range item.BootstrapPreviews {
		previews = append(previews, bootstrapPreviewItem{
			HostID:        preview.HostID,
			Hostname:      preview.Hostname,
			BootstrapYAML: preview.BootstrapYAML,
		})
	}
	return deploymentPlanItem{
		JobType:             deploymentJobTypeLabel(item.JobType),
		PolicyID:            item.PolicyID,
		PolicyRevisionID:    item.PolicyRevisionID,
		PolicyRevision:      item.PolicyRevision,
		CredentialProfileID: item.CredentialProfileID,
		CredentialSummary:   item.CredentialSummary,
		ExecutorKind:        deploymentExecutorKindLabel(item.ExecutorKind),
		ActionSummary:       item.ActionSummary,
		Targets:             targets,
		BootstrapPreviews:   previews,
		Warnings:            item.Warnings,
	}
}

func writeDeploymentReplyError(w http.ResponseWriter, r *http.Request, reply envelope.DeploymentReplyEnvelope) {
	writeReplyError(w, r, reply.Code, reply.Message)
}

func decodeDeploymentSpec(r *http.Request) (envelope.CreateDeploymentSpec, error) {
	var body deploymentUpsertRequest
	if err := decodeJSONBody(r, &body); err != nil {
		return envelope.CreateDeploymentSpec{}, err
	}
	jobType, ok := deploymentJobTypeFromString(body.JobType)
	if !ok {
		return envelope.CreateDeploymentSpec{}, httpError("job_type must be one of install, reinstall, upgrade, uninstall")
	}
	return envelope.CreateDeploymentSpec{
		JobType:             jobType,
		PolicyID:            strings.TrimSpace(body.PolicyID),
		TargetHostIDs:       trimStringSlice(body.TargetHostIDs),
		TargetHostGroupIDs:  trimStringSlice(body.TargetHostGroupIDs),
		CredentialProfileID: strings.TrimSpace(body.CredentialProfileID),
		RequestedBy:         strings.TrimSpace(body.RequestedBy),
		PreserveState:       body.PreserveState,
		Force:               body.Force,
		DryRun:              body.DryRun,
	}, nil
}

func trimStringSlice(values []string) []string {
	out := make([]string, 0, len(values))
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed != "" {
			out = append(out, trimmed)
		}
	}
	return out
}

func deploymentJobTypeFromString(value string) (int32, bool) {
	switch strings.TrimSpace(strings.ToLower(value)) {
	case "install":
		return 1, true
	case "reinstall":
		return 2, true
	case "upgrade":
		return 3, true
	case "uninstall":
		return 4, true
	default:
		return 0, false
	}
}

func deploymentStatusFromString(value string) (int32, bool) {
	switch strings.TrimSpace(strings.ToLower(value)) {
	case "":
		return 0, true
	case "queued":
		return 1, true
	case "running":
		return 2, true
	case "partial_success":
		return 3, true
	case "succeeded":
		return 4, true
	case "failed":
		return 5, true
	case "cancelled":
		return 6, true
	default:
		return 0, false
	}
}

func retryStrategyFromString(value string) (int32, bool) {
	switch strings.TrimSpace(strings.ToLower(value)) {
	case "", "failed_only":
		return 1, true
	case "all":
		return 2, true
	default:
		return 0, false
	}
}

func deploymentJobTypeLabel(value int32) string {
	switch value {
	case 1:
		return "install"
	case 2:
		return "reinstall"
	case 3:
		return "upgrade"
	case 4:
		return "uninstall"
	default:
		return "unspecified"
	}
}

func deploymentJobStatusLabel(value int32) string {
	switch value {
	case 1:
		return "queued"
	case 2:
		return "running"
	case 3:
		return "partial_success"
	case 4:
		return "succeeded"
	case 5:
		return "failed"
	case 6:
		return "cancelled"
	default:
		return "unspecified"
	}
}

func deploymentTargetStatusLabel(value int32) string {
	switch value {
	case 1:
		return "pending"
	case 2:
		return "running"
	case 3:
		return "succeeded"
	case 4:
		return "failed"
	case 5:
		return "cancelled"
	default:
		return "unspecified"
	}
}

func deploymentStepStatusLabel(value int32) string {
	switch value {
	case 1:
		return "pending"
	case 2:
		return "running"
	case 3:
		return "succeeded"
	case 4:
		return "failed"
	case 5:
		return "skipped"
	default:
		return "unspecified"
	}
}

func deploymentExecutorKindLabel(value int32) string {
	switch value {
	case 1:
		return "mock"
	case 2:
		return "ansible"
	default:
		return "unspecified"
	}
}

type transportParseError string

func (e transportParseError) Error() string { return string(e) }

func httpError(message string) error { return transportParseError(message) }

func deploymentsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		status, ok := deploymentStatusFromString(r.URL.Query().Get("status"))
		if !ok {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "invalid deployment status filter")
			return
		}
		jobTypeRaw := strings.TrimSpace(r.URL.Query().Get("job_type"))
		jobType := int32(0)
		if jobTypeRaw != "" {
			parsed, ok := deploymentJobTypeFromString(jobTypeRaw)
			if !ok {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "invalid deployment job_type filter")
				return
			}
			jobType = parsed
		}
		limit := uint32(0)
		if raw := strings.TrimSpace(r.URL.Query().Get("limit")); raw != "" {
			parsed, err := strconv.ParseUint(raw, 10, 32)
			if err != nil {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "limit must be an unsigned integer")
				return
			}
			limit = uint32(parsed)
		}
		offset := uint64(0)
		if raw := strings.TrimSpace(r.URL.Query().Get("offset")); raw != "" {
			parsed, err := strconv.ParseUint(raw, 10, 64)
			if err != nil {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "offset must be an unsigned integer")
				return
			}
			offset = parsed
		}

		subject := deps.Config.NATS.Subjects.DeploymentsJobsList
		payload, reply, err := requestDeploymentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListDeploymentJobsRequest(envelope.ListDeploymentJobsRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Status:        status,
				JobType:       jobType,
				RequestedBy:   strings.TrimSpace(r.URL.Query().Get("requested_by")),
				CreatedAfter:  strings.TrimSpace(r.URL.Query().Get("created_after")),
				CreatedBefore: strings.TrimSpace(r.URL.Query().Get("created_before")),
				Limit:         limit,
				Offset:        offset,
			}),
			envelope.DecodeListDeploymentJobsResponse,
		)
		if err != nil {
			deps.Logger.Error("deployment request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeDeploymentReplyError(w, r, reply)
			return
		}
		items := make([]deploymentJobItem, 0, len(payload.Jobs))
		for _, item := range payload.Jobs {
			items = append(items, mapDeploymentJobItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"limit":      payload.Limit,
			"offset":     payload.Offset,
			"total":      payload.Total,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		spec, err := decodeDeploymentSpec(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.DeploymentsJobsCreate
		payload, reply, err := requestDeploymentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateDeploymentJobRequest(middleware.GetRequestID(r.Context()), spec),
			envelope.DecodeCreateDeploymentJobResponse,
		)
		if err != nil {
			deps.Logger.Error("deployment request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeDeploymentReplyError(w, r, reply)
			return
		}
		if payload.Job == nil {
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "deployment runtime returned empty job payload")
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusCreated, map[string]any{
			"item":       mapDeploymentJobItem(*payload.Job),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jobID := strings.TrimSpace(chi.URLParam(r, "id"))
		if jobID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment job id is required")
			return
		}
		payload, reply, err := getDeploymentJobView(w, r, deps, jobID)
		if err != nil {
			return
		}
		attempts := make([]deploymentAttemptItem, 0, len(payload.Attempts))
		for _, attempt := range payload.Attempts {
			attempts = append(attempts, mapDeploymentAttemptItem(attempt))
		}
		targets := make([]deploymentTargetItem, 0, len(payload.Targets))
		for _, target := range payload.Targets {
			targets = append(targets, mapDeploymentTargetItem(target))
		}
		steps := make([]deploymentStepItem, 0, len(payload.Steps))
		for _, step := range payload.Steps {
			steps = append(steps, mapDeploymentStepItem(step))
		}
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapDeploymentJobItem(*payload.Job),
			"attempts":   attempts,
			"targets":    targets,
			"steps":      steps,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentStepsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jobID := strings.TrimSpace(chi.URLParam(r, "id"))
		if jobID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment job id is required")
			return
		}
		payload, reply, err := getDeploymentJobView(w, r, deps, jobID)
		if err != nil {
			return
		}
		items := make([]deploymentStepItem, 0, len(payload.Steps))
		for _, step := range payload.Steps {
			items = append(items, mapDeploymentStepItem(step))
		}
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentTargetsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jobID := strings.TrimSpace(chi.URLParam(r, "id"))
		if jobID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment job id is required")
			return
		}
		payload, reply, err := getDeploymentJobView(w, r, deps, jobID)
		if err != nil {
			return
		}
		items := make([]deploymentTargetItem, 0, len(payload.Targets))
		for _, target := range payload.Targets {
			items = append(items, mapDeploymentTargetItem(target))
		}
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentRetryHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jobID := strings.TrimSpace(chi.URLParam(r, "id"))
		if jobID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment job id is required")
			return
		}
		var body deploymentRetryRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		strategy, ok := retryStrategyFromString(body.Strategy)
		if !ok {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "strategy must be failed_only or all")
			return
		}
		subject := deps.Config.NATS.Subjects.DeploymentsJobsRetry
		payload, reply, err := requestDeploymentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeRetryDeploymentJobRequest(envelope.RetryDeploymentJobRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				JobID:         jobID,
				Strategy:      strategy,
				TriggeredBy:   strings.TrimSpace(body.TriggeredBy),
				Reason:        strings.TrimSpace(body.Reason),
			}),
			envelope.DecodeRetryDeploymentJobResponse,
		)
		if err != nil {
			deps.Logger.Error("deployment request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeDeploymentReplyError(w, r, reply)
			return
		}
		if payload.Job == nil {
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "deployment runtime returned empty retry payload")
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapDeploymentJobItem(*payload.Job),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentCancelHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		jobID := strings.TrimSpace(chi.URLParam(r, "id"))
		if jobID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment job id is required")
			return
		}
		var body deploymentCancelRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.DeploymentsJobsCancel
		payload, reply, err := requestDeploymentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCancelDeploymentJobRequest(envelope.CancelDeploymentJobRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				JobID:         jobID,
				RequestedBy:   strings.TrimSpace(body.RequestedBy),
				Reason:        strings.TrimSpace(body.Reason),
			}),
			envelope.DecodeCancelDeploymentJobResponse,
		)
		if err != nil {
			deps.Logger.Error("deployment request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeDeploymentReplyError(w, r, reply)
			return
		}
		if payload.Job == nil {
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "deployment runtime returned empty cancel payload")
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapDeploymentJobItem(*payload.Job),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func deploymentPlanHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		spec, err := decodeDeploymentSpec(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.DeploymentsPlanCreate
		payload, reply, err := requestDeploymentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateDeploymentPlanRequest(middleware.GetRequestID(r.Context()), spec),
			envelope.DecodeCreateDeploymentPlanResponse,
		)
		if err != nil {
			deps.Logger.Error("deployment request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeDeploymentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapDeploymentPlanItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func getDeploymentJobView(w http.ResponseWriter, r *http.Request, deps RouterDeps, jobID string) (envelope.GetDeploymentJobResponse, envelope.DeploymentReplyEnvelope, error) {
	subject := deps.Config.NATS.Subjects.DeploymentsJobsGet
	payload, reply, err := requestDeploymentEnvelope(
		r.Context(),
		deps.Bridge,
		deps.Logger,
		subject,
		envelope.EncodeGetDeploymentJobRequest(middleware.GetRequestID(r.Context()), jobID),
		envelope.DecodeGetDeploymentJobResponse,
	)
	if err != nil {
		deps.Logger.Error("deployment request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
		middleware.WriteTransportError(w, r, err)
		return envelope.GetDeploymentJobResponse{}, envelope.DeploymentReplyEnvelope{}, err
	}
	if strings.EqualFold(reply.Status, "error") {
		writeDeploymentReplyError(w, r, reply)
		return envelope.GetDeploymentJobResponse{}, reply, httpError(firstNonEmpty(reply.Message, "upstream request failed"))
	}
	if payload.Job == nil {
		middleware.WriteError(w, r, http.StatusBadGateway, "internal", "deployment runtime returned empty job payload")
		return envelope.GetDeploymentJobResponse{}, reply, httpError("deployment runtime returned empty job payload")
	}
	w.Header().Set("X-NATS-Subject", subject)
	return payload, reply, nil
}
