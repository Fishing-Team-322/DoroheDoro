package httpapi

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

type ticketCreateRequest struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	ClusterID   string `json:"cluster_id"`
	SourceType  string `json:"source_type"`
	SourceID    string `json:"source_id"`
	Severity    string `json:"severity"`
	Reason      string `json:"reason"`
}

type ticketAssignRequest struct {
	AssigneeUserID string `json:"assignee_user_id"`
	Reason         string `json:"reason"`
}

type ticketCommentRequest struct {
	Body   string `json:"body"`
	Reason string `json:"reason"`
}

type ticketStatusRequest struct {
	Status     string `json:"status"`
	Resolution string `json:"resolution"`
	Reason     string `json:"reason"`
}

type ticketCloseRequest struct {
	Resolution string `json:"resolution"`
	Reason     string `json:"reason"`
}

type anomalyRuleUpsertRequest struct {
	Name       string          `json:"name"`
	Kind       string          `json:"kind"`
	ScopeType  string          `json:"scope_type"`
	ScopeID    string          `json:"scope_id"`
	ConfigJSON json.RawMessage `json:"config_json"`
	IsActive   bool            `json:"is_active"`
	Reason     string          `json:"reason"`
}

type ticketItem struct {
	TicketID       string              `json:"ticket_id"`
	TicketKey      string              `json:"ticket_key"`
	Title          string              `json:"title"`
	Description    string              `json:"description,omitempty"`
	ClusterID      string              `json:"cluster_id,omitempty"`
	ClusterName    string              `json:"cluster_name,omitempty"`
	SourceType     string              `json:"source_type,omitempty"`
	SourceID       string              `json:"source_id,omitempty"`
	Severity       string              `json:"severity,omitempty"`
	Status         string              `json:"status,omitempty"`
	AssigneeUserID string              `json:"assignee_user_id,omitempty"`
	CreatedBy      string              `json:"created_by,omitempty"`
	Resolution     string              `json:"resolution,omitempty"`
	CreatedAt      string              `json:"created_at"`
	UpdatedAt      string              `json:"updated_at"`
	ResolvedAt     string              `json:"resolved_at,omitempty"`
	ClosedAt       string              `json:"closed_at,omitempty"`
	Comments       []ticketCommentItem `json:"comments,omitempty"`
	Events         []ticketEventItem   `json:"events,omitempty"`
}

type ticketCommentItem struct {
	TicketCommentID string `json:"ticket_comment_id"`
	TicketID        string `json:"ticket_id"`
	AuthorUserID    string `json:"author_user_id,omitempty"`
	Body            string `json:"body"`
	CreatedAt       string `json:"created_at"`
}

type ticketEventItem struct {
	TicketEventID string `json:"ticket_event_id"`
	TicketID      string `json:"ticket_id"`
	EventType     string `json:"event_type"`
	PayloadJSON   any    `json:"payload_json,omitempty"`
	CreatedAt     string `json:"created_at"`
}

type anomalyRuleItem struct {
	AnomalyRuleID string `json:"anomaly_rule_id"`
	Name          string `json:"name"`
	Kind          string `json:"kind"`
	ScopeType     string `json:"scope_type,omitempty"`
	ScopeID       string `json:"scope_id,omitempty"`
	ConfigJSON    any    `json:"config_json,omitempty"`
	IsActive      bool   `json:"is_active"`
	CreatedAt     string `json:"created_at"`
	UpdatedAt     string `json:"updated_at"`
	CreatedBy     string `json:"created_by,omitempty"`
	UpdatedBy     string `json:"updated_by,omitempty"`
}

type anomalyInstanceItem struct {
	AnomalyInstanceID string `json:"anomaly_instance_id"`
	AnomalyRuleID     string `json:"anomaly_rule_id"`
	ClusterID         string `json:"cluster_id,omitempty"`
	Severity          string `json:"severity,omitempty"`
	Status            string `json:"status,omitempty"`
	StartedAt         string `json:"started_at,omitempty"`
	ResolvedAt        string `json:"resolved_at,omitempty"`
	PayloadJSON       any    `json:"payload_json,omitempty"`
}

func ticketsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.TicketsList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListTicketsRequest(envelope.ListTicketsRequest{
				CorrelationID:  middleware.GetRequestID(r.Context()),
				Paging:         paging,
				ClusterID:      strings.TrimSpace(r.URL.Query().Get("cluster_id")),
				Status:         strings.TrimSpace(r.URL.Query().Get("status")),
				Severity:       strings.TrimSpace(r.URL.Query().Get("severity")),
				AssigneeUserID: strings.TrimSpace(r.URL.Query().Get("assignee_user_id")),
			}),
			envelope.DecodeControlListTicketsResponse,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		items := make([]ticketItem, 0, len(payload.Tickets))
		for _, item := range payload.Tickets {
			items = append(items, mapControlTicketItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"limit":      payload.Paging.Limit,
			"offset":     payload.Paging.Offset,
			"total":      payload.Paging.Total,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func ticketDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ticketID := strings.TrimSpace(chi.URLParam(r, "id"))
		if ticketID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "ticket id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.TicketsGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetTicketRequest(middleware.GetRequestID(r.Context()), ticketID),
			envelope.DecodeControlGetTicketResponse,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapControlTicketDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func ticketCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body ticketCreateRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.TicketsCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateTicketRequest(envelope.CreateTicketRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Title:         strings.TrimSpace(body.Title),
				Description:   strings.TrimSpace(body.Description),
				ClusterID:     strings.TrimSpace(body.ClusterID),
				SourceType:    strings.TrimSpace(body.SourceType),
				SourceID:      strings.TrimSpace(body.SourceID),
				Severity:      strings.TrimSpace(body.Severity),
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "ticket created")),
			}),
			envelope.DecodeControlGetTicketResponse,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusCreated, map[string]any{
			"item":       mapControlTicketDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func ticketAssignHandler(deps RouterDeps) http.HandlerFunc {
	return ticketMutationTicketHandler(deps, func(r *http.Request, ticketID string) ([]byte, string, error) {
		var body ticketAssignRequest
		if err := decodeJSONBody(r, &body); err != nil {
			return nil, "", err
		}
		return envelope.EncodeAssignTicketRequest(envelope.AssignTicketRequest{
			CorrelationID:  middleware.GetRequestID(r.Context()),
			TicketID:       ticketID,
			AssigneeUserID: strings.TrimSpace(body.AssigneeUserID),
			Audit:          controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "ticket assigned")),
		}), deps.Config.NATS.Subjects.TicketsAssign, nil
	})
}

func ticketUnassignHandler(deps RouterDeps) http.HandlerFunc {
	return ticketMutationTicketHandler(deps, func(r *http.Request, ticketID string) ([]byte, string, error) {
		var body ticketAssignRequest
		if err := decodeJSONBody(r, &body); err != nil {
			return nil, "", err
		}
		return envelope.EncodeUnassignTicketRequest(envelope.UnassignTicketRequest{
			CorrelationID: middleware.GetRequestID(r.Context()),
			TicketID:      ticketID,
			Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "ticket unassigned")),
		}), deps.Config.NATS.Subjects.TicketsUnassign, nil
	})
}

func ticketStatusHandler(deps RouterDeps) http.HandlerFunc {
	return ticketMutationTicketHandler(deps, func(r *http.Request, ticketID string) ([]byte, string, error) {
		var body ticketStatusRequest
		if err := decodeJSONBody(r, &body); err != nil {
			return nil, "", err
		}
		return envelope.EncodeChangeTicketStatusRequest(envelope.ChangeTicketStatusRequest{
			CorrelationID: middleware.GetRequestID(r.Context()),
			TicketID:      ticketID,
			Status:        strings.TrimSpace(body.Status),
			Resolution:    strings.TrimSpace(body.Resolution),
			Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "ticket status changed")),
		}), deps.Config.NATS.Subjects.TicketsStatusChange, nil
	})
}

func ticketCloseHandler(deps RouterDeps) http.HandlerFunc {
	return ticketMutationTicketHandler(deps, func(r *http.Request, ticketID string) ([]byte, string, error) {
		var body ticketCloseRequest
		if err := decodeJSONBody(r, &body); err != nil {
			return nil, "", err
		}
		return envelope.EncodeCloseTicketRequest(envelope.CloseTicketRequest{
			CorrelationID: middleware.GetRequestID(r.Context()),
			TicketID:      ticketID,
			Resolution:    strings.TrimSpace(body.Resolution),
			Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "ticket closed")),
		}), deps.Config.NATS.Subjects.TicketsClose, nil
	})
}

func ticketCommentAddHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ticketID := strings.TrimSpace(chi.URLParam(r, "id"))
		if ticketID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "ticket id is required")
			return
		}
		var body ticketCommentRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.TicketsCommentAdd
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeAddTicketCommentRequest(envelope.AddTicketCommentRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				TicketID:      ticketID,
				Body:          strings.TrimSpace(body.Body),
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "ticket comment added")),
			}),
			envelope.DecodeControlTicketComment,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusCreated, map[string]any{
			"item":       mapControlTicketCommentItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func anomalyRulesListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.AnomalyRulesList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListAnomalyRulesRequest(envelope.ListAnomalyRulesRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Paging:        paging,
				ScopeType:     strings.TrimSpace(r.URL.Query().Get("scope_type")),
				ScopeID:       strings.TrimSpace(r.URL.Query().Get("scope_id")),
			}),
			envelope.DecodeControlListAnomalyRulesResponse,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		items := make([]anomalyRuleItem, 0, len(payload.Rules))
		for _, item := range payload.Rules {
			items = append(items, mapControlAnomalyRuleItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"limit":      payload.Paging.Limit,
			"offset":     payload.Paging.Offset,
			"total":      payload.Paging.Total,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func anomalyRuleDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ruleID := strings.TrimSpace(chi.URLParam(r, "id"))
		if ruleID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "anomaly rule id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AnomalyRulesGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetAnomalyRuleRequest(middleware.GetRequestID(r.Context()), ruleID),
			envelope.DecodeControlAnomalyRule,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapControlAnomalyRuleItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func anomalyRuleCreateHandler(deps RouterDeps) http.HandlerFunc {
	return anomalyRuleMutationHandler(deps, "", deps.Config.NATS.Subjects.AnomalyRulesCreate, http.StatusCreated)
}

func anomalyRuleUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return anomalyRuleMutationHandler(deps, "id", deps.Config.NATS.Subjects.AnomalyRulesUpdate, http.StatusOK)
}

func anomalyInstancesListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.AnomalyInstancesList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListAnomalyInstancesRequest(envelope.ListAnomalyInstancesRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Paging:        paging,
				AnomalyRuleID: strings.TrimSpace(r.URL.Query().Get("anomaly_rule_id")),
				ClusterID:     strings.TrimSpace(r.URL.Query().Get("cluster_id")),
				Status:        strings.TrimSpace(r.URL.Query().Get("status")),
			}),
			envelope.DecodeControlListAnomalyInstancesResponse,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		items := make([]anomalyInstanceItem, 0, len(payload.Instances))
		for _, item := range payload.Instances {
			items = append(items, mapControlAnomalyInstanceItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"limit":      payload.Paging.Limit,
			"offset":     payload.Paging.Offset,
			"total":      payload.Paging.Total,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func anomalyInstanceDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		instanceID := strings.TrimSpace(chi.URLParam(r, "id"))
		if instanceID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "anomaly instance id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AnomalyInstancesGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetAnomalyInstanceRequest(middleware.GetRequestID(r.Context()), instanceID),
			envelope.DecodeControlAnomalyInstance,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapControlAnomalyInstanceItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func ticketMutationTicketHandler(
	deps RouterDeps,
	build func(r *http.Request, ticketID string) ([]byte, string, error),
) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ticketID := strings.TrimSpace(chi.URLParam(r, "id"))
		if ticketID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "ticket id is required")
			return
		}
		requestBytes, subject, err := build(r, ticketID)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			requestBytes,
			envelope.DecodeControlTicket,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       mapControlTicketItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func anomalyRuleMutationHandler(deps RouterDeps, pathParam, subject string, statusCode int) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body anomalyRuleUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		configJSON, err := marshalOptionalRawJSON(body.ConfigJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		requestID := middleware.GetRequestID(r.Context())
		var requestBytes []byte
		if pathParam == "" {
			requestBytes = envelope.EncodeCreateAnomalyRuleRequest(envelope.CreateAnomalyRuleRequest{
				CorrelationID: requestID,
				Name:          strings.TrimSpace(body.Name),
				Kind:          strings.TrimSpace(body.Kind),
				ScopeType:     strings.TrimSpace(body.ScopeType),
				ScopeID:       strings.TrimSpace(body.ScopeID),
				ConfigJSON:    configJSON,
				IsActive:      body.IsActive,
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "anomaly rule created")),
			})
		} else {
			ruleID := strings.TrimSpace(chi.URLParam(r, pathParam))
			if ruleID == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "anomaly rule id is required")
				return
			}
			requestBytes = envelope.EncodeUpdateAnomalyRuleRequest(envelope.UpdateAnomalyRuleRequest{
				CorrelationID: requestID,
				AnomalyRuleID: ruleID,
				Name:          strings.TrimSpace(body.Name),
				ConfigJSON:    configJSON,
				IsActive:      body.IsActive,
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "anomaly rule updated")),
			})
		}
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			requestBytes,
			envelope.DecodeControlAnomalyRule,
		)
		if err != nil {
			deps.Logger.Error("control request failed", zap.String("subject", subject), zap.String("request_id", requestID), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeControlReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, statusCode, map[string]any{
			"item":       mapControlAnomalyRuleItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, requestID),
		})
	}
}

func mapControlTicketItem(item envelope.ControlTicket) ticketItem {
	return ticketItem{
		TicketID:       item.TicketID,
		TicketKey:      item.TicketKey,
		Title:          item.Title,
		Description:    item.Description,
		ClusterID:      item.ClusterID,
		ClusterName:    item.ClusterName,
		SourceType:     item.SourceType,
		SourceID:       item.SourceID,
		Severity:       item.Severity,
		Status:         item.Status,
		AssigneeUserID: item.AssigneeUserID,
		CreatedBy:      item.CreatedBy,
		Resolution:     item.Resolution,
		CreatedAt:      item.CreatedAt,
		UpdatedAt:      item.UpdatedAt,
		ResolvedAt:     item.ResolvedAt,
		ClosedAt:       item.ClosedAt,
	}
}

func mapControlTicketCommentItem(item envelope.ControlTicketComment) ticketCommentItem {
	return ticketCommentItem{
		TicketCommentID: item.TicketCommentID,
		TicketID:        item.TicketID,
		AuthorUserID:    item.AuthorUserID,
		Body:            item.Body,
		CreatedAt:       item.CreatedAt,
	}
}

func mapControlTicketEventItem(item envelope.ControlTicketEvent) ticketEventItem {
	return ticketEventItem{
		TicketEventID: item.TicketEventID,
		TicketID:      item.TicketID,
		EventType:     item.EventType,
		PayloadJSON:   parseJSONString(item.PayloadJSON),
		CreatedAt:     item.CreatedAt,
	}
}

func mapControlTicketDetailsItem(item envelope.ControlTicketDetails) ticketItem {
	out := ticketItem{}
	if item.Ticket != nil {
		out = mapControlTicketItem(*item.Ticket)
	}
	out.Comments = make([]ticketCommentItem, 0, len(item.Comments))
	for _, comment := range item.Comments {
		out.Comments = append(out.Comments, mapControlTicketCommentItem(comment))
	}
	out.Events = make([]ticketEventItem, 0, len(item.Events))
	for _, event := range item.Events {
		out.Events = append(out.Events, mapControlTicketEventItem(event))
	}
	return out
}

func mapControlAnomalyRuleItem(item envelope.ControlAnomalyRule) anomalyRuleItem {
	return anomalyRuleItem{
		AnomalyRuleID: item.AnomalyRuleID,
		Name:          item.Name,
		Kind:          item.Kind,
		ScopeType:     item.ScopeType,
		ScopeID:       item.ScopeID,
		ConfigJSON:    parseJSONString(item.ConfigJSON),
		IsActive:      item.IsActive,
		CreatedAt:     item.CreatedAt,
		UpdatedAt:     item.UpdatedAt,
		CreatedBy:     item.CreatedBy,
		UpdatedBy:     item.UpdatedBy,
	}
}

func mapControlAnomalyInstanceItem(item envelope.ControlAnomalyInstance) anomalyInstanceItem {
	return anomalyInstanceItem{
		AnomalyInstanceID: item.AnomalyInstanceID,
		AnomalyRuleID:     item.AnomalyRuleID,
		ClusterID:         item.ClusterID,
		Severity:          item.Severity,
		Status:            item.Status,
		StartedAt:         item.StartedAt,
		ResolvedAt:        item.ResolvedAt,
		PayloadJSON:       parseJSONString(item.PayloadJSON),
	}
}
