package httpapi

import (
	"encoding/json"
	"net/http"
	"strconv"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/middleware"
)

type logSearchRequestBody struct {
	Query    string `json:"query"`
	From     string `json:"from"`
	To       string `json:"to"`
	AgentID  string `json:"agent_id"`
	Host     string `json:"host"`
	Service  string `json:"service"`
	Severity string `json:"severity"`
	Limit    uint32 `json:"limit"`
	Offset   uint64 `json:"offset"`
}

type logContextRequestBody struct {
	EventID string `json:"event_id"`
	Before  uint32 `json:"before"`
	After   uint32 `json:"after"`
}

type logEventItem struct {
	ID          string            `json:"id"`
	Timestamp   string            `json:"timestamp"`
	Host        string            `json:"host"`
	AgentID     string            `json:"agent_id"`
	SourceType  string            `json:"source_type"`
	Source      string            `json:"source"`
	Service     string            `json:"service"`
	Severity    string            `json:"severity"`
	Message     string            `json:"message"`
	Fingerprint string            `json:"fingerprint"`
	Labels      map[string]string `json:"labels"`
	FieldsJSON  string            `json:"fields_json"`
	Raw         string            `json:"raw"`
}

type logSearchPayload struct {
	Items  []logEventItem `json:"items"`
	Total  uint64         `json:"total"`
	Limit  uint32         `json:"limit"`
	Offset uint64         `json:"offset"`
	TookMS uint32         `json:"took_ms"`
}

type logEventPayload struct {
	Item   *logEventItem `json:"item"`
	TookMS uint32        `json:"took_ms"`
}

type logContextPayload struct {
	Anchor *logEventItem  `json:"anchor"`
	Before []logEventItem `json:"before"`
	After  []logEventItem `json:"after"`
	TookMS uint32         `json:"took_ms"`
}

type countBucketItem struct {
	Key   string `json:"key"`
	Count uint64 `json:"count"`
}

type histogramBucketItem struct {
	Bucket string `json:"bucket"`
	Count  uint64 `json:"count"`
}

type heatmapBucketItem struct {
	Bucket   string `json:"bucket"`
	Severity string `json:"severity"`
	Count    uint64 `json:"count"`
}

type patternBucketItem struct {
	Fingerprint   string `json:"fingerprint"`
	SampleMessage string `json:"sample_message"`
	Count         uint64 `json:"count"`
}

type countBucketsPayload struct {
	Items []countBucketItem `json:"items"`
}

type histogramPayload struct {
	Items []histogramBucketItem `json:"items"`
}

type heatmapPayload struct {
	Items []heatmapBucketItem `json:"items"`
}

type topPatternsPayload struct {
	Items []patternBucketItem `json:"items"`
}

type logAnomalyItem struct {
	AlertInstanceID string `json:"alert_instance_id"`
	AlertRuleID     string `json:"alert_rule_id"`
	Status          string `json:"status"`
	Severity        string `json:"severity"`
	Title           string `json:"title"`
	Fingerprint     string `json:"fingerprint"`
	Host            string `json:"host"`
	Service         string `json:"service"`
	TriggeredAt     string `json:"triggered_at"`
	PayloadJSON     string `json:"payload_json"`
}

type logAnomaliesPayload struct {
	Items  []logAnomalyItem `json:"items"`
	Total  uint64           `json:"total"`
	Limit  uint32           `json:"limit"`
	Offset uint64           `json:"offset"`
}

type dashboardMetricItem struct {
	Key         string  `json:"key"`
	Label       string  `json:"label"`
	Value       string  `json:"value"`
	Change      float64 `json:"change"`
	Trend       string  `json:"trend"`
	Description string  `json:"description"`
}

type dashboardActivityItem struct {
	Kind        string `json:"kind"`
	Title       string `json:"title"`
	Description string `json:"description"`
	Timestamp   string `json:"timestamp"`
}

type dashboardOverviewPayload struct {
	Metrics        []dashboardMetricItem   `json:"metrics"`
	ActiveHosts    uint64                  `json:"active_hosts"`
	OpenAlerts     uint64                  `json:"open_alerts"`
	DeploymentJobs uint64                  `json:"deployment_jobs"`
	IngestedEvents uint64                  `json:"ingested_events"`
	RecentActivity []dashboardActivityItem `json:"recent_activity"`
	LogHistogram   []histogramBucketItem   `json:"log_histogram"`
	TopServices    []countBucketItem       `json:"top_services"`
	TopHosts       []countBucketItem       `json:"top_hosts"`
}

type pagingMeta struct {
	Limit  uint32 `json:"limit"`
	Offset uint64 `json:"offset"`
	Total  uint64 `json:"total"`
}

type alertRuleItem struct {
	AlertRuleID   string `json:"alert_rule_id"`
	Name          string `json:"name"`
	Description   string `json:"description"`
	Status        string `json:"status"`
	Severity      string `json:"severity"`
	ScopeType     string `json:"scope_type"`
	ScopeID       string `json:"scope_id"`
	ConditionJSON string `json:"condition_json"`
	CreatedAt     string `json:"created_at"`
	UpdatedAt     string `json:"updated_at"`
	CreatedBy     string `json:"created_by"`
	UpdatedBy     string `json:"updated_by"`
}

type alertInstanceItem struct {
	AlertInstanceID string `json:"alert_instance_id"`
	AlertRuleID     string `json:"alert_rule_id"`
	Title           string `json:"title"`
	Status          string `json:"status"`
	Severity        string `json:"severity"`
	TriggeredAt     string `json:"triggered_at"`
	AcknowledgedAt  string `json:"acknowledged_at"`
	ResolvedAt      string `json:"resolved_at"`
	Host            string `json:"host"`
	Service         string `json:"service"`
	Fingerprint     string `json:"fingerprint"`
	PayloadJSON     string `json:"payload_json"`
}

type alertInstancesPayload struct {
	Items  []alertInstanceItem `json:"items"`
	Paging pagingMeta          `json:"paging"`
}

type alertInstancePayload struct {
	Item *alertInstanceItem `json:"item"`
}

type alertRulesPayload struct {
	Items  []alertRuleItem `json:"items"`
	Paging pagingMeta      `json:"paging"`
}

type alertRulePayload struct {
	Item *alertRuleItem `json:"item"`
}

type alertRuleMutationRequest struct {
	Name          string          `json:"name"`
	Description   string          `json:"description"`
	Status        string          `json:"status"`
	Severity      string          `json:"severity"`
	ScopeType     string          `json:"scope_type"`
	ScopeID       string          `json:"scope_id"`
	ConditionJSON json.RawMessage `json:"condition_json"`
	Reason        string          `json:"reason"`
}

type auditEventItem struct {
	AuditEventID string `json:"audit_event_id"`
	EventType    string `json:"event_type"`
	EntityType   string `json:"entity_type"`
	EntityID     string `json:"entity_id"`
	ActorID      string `json:"actor_id"`
	ActorType    string `json:"actor_type"`
	RequestID    string `json:"request_id"`
	Reason       string `json:"reason"`
	PayloadJSON  string `json:"payload_json"`
	CreatedAt    string `json:"created_at"`
}

type auditEventsPayload struct {
	Items  []auditEventItem `json:"items"`
	Paging pagingMeta       `json:"paging"`
}

func logsSearchHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.QueryLogsSearch
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		var body logSearchRequestBody
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}

		payload, reply, err := requestJSONEnvelope[logSearchPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"filter":         buildLogQueryFilter(body.Query, body.From, body.To, body.AgentID, body.Host, body.Service, body.Severity),
				"limit":          body.Limit,
				"offset":         body.Offset,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      mapLogItems(payload.Items),
			"total":      payload.Total,
			"limit":      payload.Limit,
			"offset":     payload.Offset,
			"took_ms":    payload.TookMS,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func logDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		eventID := strings.TrimSpace(chi.URLParam(r, "eventId"))
		if eventID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "eventId is required")
			return
		}
		subject := deps.Config.NATS.Subjects.QueryLogsGet
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[logEventPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"event_id":       eventID,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapLogItem(payload.Item),
			"took_ms":    payload.TookMS,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func logsContextHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.QueryLogsContext
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		var body logContextRequestBody
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		if strings.TrimSpace(body.EventID) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "event_id is required")
			return
		}

		payload, reply, err := requestJSONEnvelope[logContextPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"event_id":       strings.TrimSpace(body.EventID),
				"before":         body.Before,
				"after":          body.After,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"anchor":     mapLogItem(payload.Anchor),
			"before":     mapLogItems(payload.Before),
			"after":      mapLogItems(payload.After),
			"took_ms":    payload.TookMS,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func logsHistogramHandler(deps RouterDeps) http.HandlerFunc {
	return logsAnalyticsHandler(deps, deps.Config.NATS.Subjects.QueryLogsHistogram, func(payload histogramPayload) map[string]any {
		return map[string]any{"items": payload.Items}
	})
}

func logsSeverityHandler(deps RouterDeps) http.HandlerFunc {
	return logsAnalyticsHandler(deps, deps.Config.NATS.Subjects.QueryLogsSeverity, func(payload countBucketsPayload) map[string]any {
		return map[string]any{"items": payload.Items}
	})
}

func logsTopHostsHandler(deps RouterDeps) http.HandlerFunc {
	return logsAnalyticsHandler(deps, deps.Config.NATS.Subjects.QueryLogsTopHosts, func(payload countBucketsPayload) map[string]any {
		return map[string]any{"items": payload.Items}
	})
}

func logsTopServicesHandler(deps RouterDeps) http.HandlerFunc {
	return logsAnalyticsHandler(deps, deps.Config.NATS.Subjects.QueryLogsTopServices, func(payload countBucketsPayload) map[string]any {
		return map[string]any{"items": payload.Items}
	})
}

func logsHeatmapHandler(deps RouterDeps) http.HandlerFunc {
	return logsAnalyticsHandler(deps, deps.Config.NATS.Subjects.QueryLogsHeatmap, func(payload heatmapPayload) map[string]any {
		return map[string]any{"items": payload.Items}
	})
}

func logsTopPatternsHandler(deps RouterDeps) http.HandlerFunc {
	return logsAnalyticsHandler(deps, deps.Config.NATS.Subjects.QueryLogsTopPatterns, func(payload topPatternsPayload) map[string]any {
		return map[string]any{"items": payload.Items}
	})
}

func logsAnalyticsHandler[T any](deps RouterDeps, subject string, mapper func(T) map[string]any) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}
		filter := queryFilterFromRequest(r)
		limit := uint32(parseUint64Query(r, "limit", 0, 200))

		payload, reply, err := requestJSONEnvelope[T](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"filter":         filter,
				"limit":          limit,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		response := mapper(payload)
		response["request_id"] = firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context()))
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, response)
	}
}

func logsAnomaliesHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.QueryLogsAnomalies
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		filter := queryFilterFromRequest(r)
		limit := uint32(parseUint64Query(r, "limit", 50, 200))
		offset := parseUint64Query(r, "offset", 0, 100000)
		payload, reply, err := requestJSONEnvelope[logAnomaliesPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"filter":         filter,
				"limit":          limit,
				"offset":         offset,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      mapLogAnomalies(payload.Items),
			"total":      payload.Total,
			"limit":      payload.Limit,
			"offset":     payload.Offset,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func dashboardOverviewHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.QueryDashboardsOverview
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[dashboardOverviewPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"from":           strings.TrimSpace(r.URL.Query().Get("from")),
				"to":             strings.TrimSpace(r.URL.Query().Get("to")),
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"metrics":         payload.Metrics,
			"active_hosts":    payload.ActiveHosts,
			"open_alerts":     payload.OpenAlerts,
			"deployment_jobs": payload.DeploymentJobs,
			"ingested_events": payload.IngestedEvents,
			"recent_activity": payload.RecentActivity,
			"log_histogram":   payload.LogHistogram,
			"top_services":    payload.TopServices,
			"top_hosts":       payload.TopHosts,
			"request_id":      firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func alertsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.AlertsList
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[alertInstancesPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"paging":         pagingFromRequest(r),
				"status":         strings.TrimSpace(r.URL.Query().Get("status")),
				"severity":       strings.TrimSpace(r.URL.Query().Get("severity")),
				"host":           strings.TrimSpace(r.URL.Query().Get("host")),
				"service":        strings.TrimSpace(r.URL.Query().Get("service")),
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      mapAlertInstances(payload.Items),
			"paging":     payload.Paging,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func alertDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		alertID := strings.TrimSpace(chi.URLParam(r, "id"))
		if alertID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "alert id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AlertsGet
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[alertInstancePayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id":   middleware.GetRequestID(r.Context()),
				"alert_instance_id": alertID,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapAlertInstance(payload.Item),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func alertRulesListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.AlertsRulesList
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[alertRulesPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"paging":         pagingFromRequest(r),
				"query":          strings.TrimSpace(r.URL.Query().Get("query")),
				"status":         strings.TrimSpace(r.URL.Query().Get("status")),
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      mapAlertRules(payload.Items),
			"paging":     payload.Paging,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func alertRuleDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		alertRuleID := strings.TrimSpace(chi.URLParam(r, "id"))
		if alertRuleID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "alert rule id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AlertsRulesGet
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[alertRulePayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"alert_rule_id":  alertRuleID,
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapAlertRule(payload.Item),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func alertRuleCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		handleAlertRuleMutation(w, r, deps, deps.Config.NATS.Subjects.AlertsRulesCreate, "")
	}
}

func alertRuleUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		handleAlertRuleMutation(w, r, deps, deps.Config.NATS.Subjects.AlertsRulesUpdate, strings.TrimSpace(chi.URLParam(r, "id")))
	}
}

func handleAlertRuleMutation(w http.ResponseWriter, r *http.Request, deps RouterDeps, subject, alertRuleID string) {
	if !ensureRuntimeBridge(w, r, deps, subject) {
		return
	}

	var body alertRuleMutationRequest
	if err := decodeJSONBody(r, &body); err != nil {
		middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
		return
	}

	conditionJSON, err := marshalOptionalRawJSON(body.ConditionJSON)
	if err != nil {
		middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "condition_json must be valid JSON")
		return
	}

	request := map[string]any{
		"correlation_id": middleware.GetRequestID(r.Context()),
		"name":           strings.TrimSpace(body.Name),
		"description":    strings.TrimSpace(body.Description),
		"severity":       strings.TrimSpace(body.Severity),
		"scope_type":     strings.TrimSpace(body.ScopeType),
		"scope_id":       strings.TrimSpace(body.ScopeID),
		"condition_json": conditionJSON,
		"audit": map[string]any{
			"actor_id":   controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "alert rule updated")).ActorID,
			"actor_type": controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "alert rule updated")).ActorType,
			"request_id": middleware.GetRequestID(r.Context()),
			"reason":     firstNonEmpty(strings.TrimSpace(body.Reason), "alert rule updated"),
		},
	}
	if subject == deps.Config.NATS.Subjects.AlertsRulesUpdate {
		request["alert_rule_id"] = alertRuleID
		request["status"] = strings.TrimSpace(body.Status)
	}

	payload, reply, err := requestJSONEnvelope[alertRulePayload](
		r.Context(),
		deps.Bridge,
		deps.Logger,
		subject,
		request,
	)
	if err != nil {
		logRuntimeTransportError(deps.Logger, subject, r, err)
		middleware.WriteTransportError(w, r, err)
		return
	}
	if strings.EqualFold(reply.Status, "error") {
		writeAgentReplyError(w, r, reply)
		return
	}

	w.Header().Set("X-NATS-Subject", subject)
	middleware.WriteJSON(w, http.StatusOK, map[string]any{
		"item":       mapAlertRule(payload.Item),
		"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
	})
}

func auditListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.AuditList
		if !ensureRuntimeBridge(w, r, deps, subject) {
			return
		}

		payload, reply, err := requestJSONEnvelope[auditEventsPayload](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]any{
				"correlation_id": middleware.GetRequestID(r.Context()),
				"paging":         pagingFromRequest(r),
				"event_type":     strings.TrimSpace(r.URL.Query().Get("event_type")),
				"entity_type":    strings.TrimSpace(r.URL.Query().Get("entity_type")),
				"entity_id":      strings.TrimSpace(r.URL.Query().Get("entity_id")),
				"actor_id":       strings.TrimSpace(r.URL.Query().Get("actor_id")),
			},
		)
		if err != nil {
			logRuntimeTransportError(deps.Logger, subject, r, err)
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      mapAuditEvents(payload.Items),
			"paging":     payload.Paging,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func ensureRuntimeBridge(w http.ResponseWriter, r *http.Request, deps RouterDeps, subject string) bool {
	if strings.TrimSpace(subject) != "" {
		w.Header().Set("X-NATS-Subject", subject)
	}
	if deps.Bridge != nil {
		return true
	}
	middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "edge bridge is not ready")
	return false
}

func buildLogQueryFilter(query, from, to, agentID, host, service, severity string) map[string]any {
	return map[string]any{
		"query":    strings.TrimSpace(query),
		"from":     strings.TrimSpace(from),
		"to":       strings.TrimSpace(to),
		"agent_id": strings.TrimSpace(agentID),
		"host":     strings.TrimSpace(host),
		"service":  strings.TrimSpace(service),
		"severity": strings.TrimSpace(severity),
	}
}

func queryFilterFromRequest(r *http.Request) map[string]any {
	return buildLogQueryFilter(
		r.URL.Query().Get("query"),
		r.URL.Query().Get("from"),
		r.URL.Query().Get("to"),
		r.URL.Query().Get("agent_id"),
		r.URL.Query().Get("host"),
		r.URL.Query().Get("service"),
		r.URL.Query().Get("severity"),
	)
}

func pagingFromRequest(r *http.Request) map[string]any {
	return map[string]any{
		"limit":  uint32(parseUint64Query(r, "limit", 50, 200)),
		"offset": parseUint64Query(r, "offset", 0, 100000),
		"query":  strings.TrimSpace(r.URL.Query().Get("query")),
	}
}

func parseUint64Query(r *http.Request, name string, fallback uint64, max uint64) uint64 {
	raw := strings.TrimSpace(r.URL.Query().Get(name))
	if raw == "" {
		return fallback
	}
	value, err := strconv.ParseUint(raw, 10, 64)
	if err != nil {
		return fallback
	}
	if max > 0 && value > max {
		return max
	}
	return value
}

func mapLogItems(items []logEventItem) []map[string]any {
	out := make([]map[string]any, 0, len(items))
	for _, item := range items {
		out = append(out, mapLogItem(&item))
	}
	return out
}

func mapLogItem(item *logEventItem) map[string]any {
	if item == nil {
		return nil
	}
	return map[string]any{
		"id":          item.ID,
		"timestamp":   item.Timestamp,
		"host":        item.Host,
		"agent_id":    item.AgentID,
		"source_type": item.SourceType,
		"source":      item.Source,
		"service":     item.Service,
		"severity":    item.Severity,
		"message":     item.Message,
		"fingerprint": item.Fingerprint,
		"labels":      item.Labels,
		"fields_json": parseStructuredJSON(item.FieldsJSON),
		"raw":         item.Raw,
	}
}

func mapLogAnomalies(items []logAnomalyItem) []map[string]any {
	out := make([]map[string]any, 0, len(items))
	for _, item := range items {
		out = append(out, map[string]any{
			"alert_instance_id": item.AlertInstanceID,
			"alert_rule_id":     item.AlertRuleID,
			"status":            item.Status,
			"severity":          item.Severity,
			"title":             item.Title,
			"fingerprint":       item.Fingerprint,
			"host":              item.Host,
			"service":           item.Service,
			"triggered_at":      item.TriggeredAt,
			"payload_json":      parseStructuredJSON(item.PayloadJSON),
		})
	}
	return out
}

func mapAlertRules(items []alertRuleItem) []map[string]any {
	out := make([]map[string]any, 0, len(items))
	for _, item := range items {
		out = append(out, mapAlertRule(&item))
	}
	return out
}

func mapAlertRule(item *alertRuleItem) map[string]any {
	if item == nil {
		return nil
	}
	return map[string]any{
		"alert_rule_id": item.AlertRuleID,
		"name":          item.Name,
		"description":   item.Description,
		"status":        item.Status,
		"severity":      item.Severity,
		"scope_type":    item.ScopeType,
		"scope_id":      item.ScopeID,
		"condition_json": parseStructuredJSON(item.ConditionJSON),
		"created_at":    item.CreatedAt,
		"updated_at":    item.UpdatedAt,
		"created_by":    item.CreatedBy,
		"updated_by":    item.UpdatedBy,
	}
}

func mapAlertInstances(items []alertInstanceItem) []map[string]any {
	out := make([]map[string]any, 0, len(items))
	for _, item := range items {
		out = append(out, mapAlertInstance(&item))
	}
	return out
}

func mapAlertInstance(item *alertInstanceItem) map[string]any {
	if item == nil {
		return nil
	}
	return map[string]any{
		"alert_instance_id": item.AlertInstanceID,
		"alert_rule_id":     item.AlertRuleID,
		"title":             item.Title,
		"status":            item.Status,
		"severity":          item.Severity,
		"triggered_at":      item.TriggeredAt,
		"acknowledged_at":   item.AcknowledgedAt,
		"resolved_at":       item.ResolvedAt,
		"host":              item.Host,
		"service":           item.Service,
		"fingerprint":       item.Fingerprint,
		"payload_json":      parseStructuredJSON(item.PayloadJSON),
	}
}

func mapAuditEvents(items []auditEventItem) []map[string]any {
	out := make([]map[string]any, 0, len(items))
	for _, item := range items {
		out = append(out, map[string]any{
			"audit_event_id": item.AuditEventID,
			"event_type":     item.EventType,
			"entity_type":    item.EntityType,
			"entity_id":      item.EntityID,
			"actor_id":       item.ActorID,
			"actor_type":     item.ActorType,
			"request_id":     item.RequestID,
			"reason":         item.Reason,
			"payload_json":   parseStructuredJSON(item.PayloadJSON),
			"created_at":     item.CreatedAt,
		})
	}
	return out
}

func parseStructuredJSON(raw string) any {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return map[string]any{}
	}
	var value any
	if err := json.Unmarshal([]byte(raw), &value); err != nil {
		return raw
	}
	return value
}

func logRuntimeTransportError(logger *zap.Logger, subject string, r *http.Request, err error) {
	if logger == nil {
		return
	}
	logger.Error("runtime request failed",
		zap.String("subject", subject),
		zap.String("request_id", middleware.GetRequestID(r.Context())),
		zap.Error(err),
	)
}
