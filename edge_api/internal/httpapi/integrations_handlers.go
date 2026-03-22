package httpapi

import (
	"encoding/json"
	"net/http"
	"strings"
	"time"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

type integrationUpsertRequest struct {
	Name        string          `json:"name"`
	Kind        string          `json:"kind"`
	Description string          `json:"description"`
	ConfigJSON  json.RawMessage `json:"config_json"`
	IsActive    bool            `json:"is_active"`
	Reason      string          `json:"reason"`
}

type integrationBindingRequest struct {
	ScopeType         string          `json:"scope_type"`
	ScopeID           string          `json:"scope_id"`
	EventTypesJSON    json.RawMessage `json:"event_types_json"`
	SeverityThreshold string          `json:"severity_threshold"`
	IsActive          bool            `json:"is_active"`
	Reason            string          `json:"reason"`
}

type integrationTelegramHealthcheckRequest struct {
	ChatIDOverride string `json:"chat_id_override"`
	Reason         string `json:"reason"`
}

type telegramHealthcheckEvent struct {
	SchemaVersion  string `json:"schema_version"`
	RequestID      string `json:"request_id"`
	IntegrationID  string `json:"integration_id"`
	CorrelationID  string `json:"correlation_id"`
	CreatedAt      string `json:"created_at"`
	ChatIDOverride string `json:"chat_id_override,omitempty"`
	ActorID        string `json:"actor_id,omitempty"`
	ActorType      string `json:"actor_type,omitempty"`
	Reason         string `json:"reason,omitempty"`
}

type integrationItem struct {
	IntegrationID string                   `json:"integration_id"`
	Name          string                   `json:"name"`
	Kind          string                   `json:"kind"`
	Description   string                   `json:"description,omitempty"`
	ConfigJSON    any                      `json:"config_json,omitempty"`
	IsActive      bool                     `json:"is_active"`
	CreatedAt     string                   `json:"created_at"`
	UpdatedAt     string                   `json:"updated_at"`
	CreatedBy     string                   `json:"created_by,omitempty"`
	UpdatedBy     string                   `json:"updated_by,omitempty"`
	Bindings      []integrationBindingItem `json:"bindings,omitempty"`
}

type integrationBindingItem struct {
	IntegrationBindingID string `json:"integration_binding_id"`
	IntegrationID        string `json:"integration_id"`
	ScopeType            string `json:"scope_type,omitempty"`
	ScopeID              string `json:"scope_id,omitempty"`
	EventTypesJSON       any    `json:"event_types_json,omitempty"`
	SeverityThreshold    string `json:"severity_threshold,omitempty"`
	IsActive             bool   `json:"is_active"`
	CreatedAt            string `json:"created_at"`
	UpdatedAt            string `json:"updated_at"`
}

func integrationsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlIntegrationsList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListIntegrationsRequest(envelope.ListIntegrationsRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Paging:        paging,
			}),
			envelope.DecodeControlListIntegrationsResponse,
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
		items := make([]integrationItem, 0, len(payload.Integrations))
		for _, item := range payload.Integrations {
			items = append(items, mapControlIntegrationItem(item))
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

func integrationDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		integrationID := strings.TrimSpace(chi.URLParam(r, "id"))
		if integrationID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "integration id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlIntegrationsGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetIntegrationRequest(middleware.GetRequestID(r.Context()), integrationID),
			envelope.DecodeControlGetIntegrationResponse,
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
		item := integrationItem{}
		if payload.Integration != nil {
			item = mapControlIntegrationItem(*payload.Integration)
		}
		item.Bindings = make([]integrationBindingItem, 0, len(payload.Bindings))
		for _, binding := range payload.Bindings {
			item.Bindings = append(item.Bindings, mapControlIntegrationBindingItem(binding))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       item,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func integrationCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body integrationUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		configJSON, err := marshalOptionalRawJSON(body.ConfigJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlIntegrationsCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateIntegrationRequest(envelope.CreateIntegrationRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Name:          strings.TrimSpace(body.Name),
				Kind:          strings.TrimSpace(body.Kind),
				Description:   strings.TrimSpace(body.Description),
				ConfigJSON:    configJSON,
				IsActive:      body.IsActive,
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "integration created")),
			}),
			envelope.DecodeControlIntegration,
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
			"item":       mapControlIntegrationItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func integrationUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		integrationID := strings.TrimSpace(chi.URLParam(r, "id"))
		if integrationID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "integration id is required")
			return
		}
		var body integrationUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		configJSON, err := marshalOptionalRawJSON(body.ConfigJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlIntegrationsUpdate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeUpdateIntegrationRequest(envelope.UpdateIntegrationRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				IntegrationID: integrationID,
				Name:          strings.TrimSpace(body.Name),
				Description:   strings.TrimSpace(body.Description),
				ConfigJSON:    configJSON,
				IsActive:      body.IsActive,
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "integration updated")),
			}),
			envelope.DecodeControlIntegration,
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
			"item":       mapControlIntegrationItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func integrationBindHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		integrationID := strings.TrimSpace(chi.URLParam(r, "id"))
		if integrationID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "integration id is required")
			return
		}
		var body integrationBindingRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		eventTypesJSON, err := marshalOptionalRawJSON(body.EventTypesJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlIntegrationsBind
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeBindIntegrationRequest(envelope.BindIntegrationRequest{
				CorrelationID:     middleware.GetRequestID(r.Context()),
				IntegrationID:     integrationID,
				ScopeType:         strings.TrimSpace(body.ScopeType),
				ScopeID:           strings.TrimSpace(body.ScopeID),
				EventTypesJSON:    eventTypesJSON,
				SeverityThreshold: strings.TrimSpace(body.SeverityThreshold),
				IsActive:          body.IsActive,
				Audit:             controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "integration bound")),
			}),
			envelope.DecodeControlIntegrationBinding,
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
			"item":       mapControlIntegrationBindingItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func integrationUnbindHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bindingID := strings.TrimSpace(chi.URLParam(r, "bindingId"))
		if bindingID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "integration binding id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlIntegrationsUnbind
		_, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeUnbindIntegrationRequest(envelope.UnbindIntegrationRequest{
				CorrelationID:        middleware.GetRequestID(r.Context()),
				IntegrationBindingID: bindingID,
				Audit:                controlAuditContextFromRequest(r, "integration unbound"),
			}),
			func(data []byte) (struct{}, error) { return struct{}{}, nil },
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
			"status":     "ok",
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func integrationTelegramHealthcheckHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		integrationID := strings.TrimSpace(chi.URLParam(r, "id"))
		if integrationID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "integration id is required")
			return
		}

		var body integrationTelegramHealthcheckRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}

		ac := auth.Context(r.Context())
		requestID := middleware.GetRequestID(r.Context())
		subject := deps.Config.NATS.Subjects.NotificationsTelegramHealthcheckRequest
		event := telegramHealthcheckEvent{
			SchemaVersion:  "v1",
			RequestID:      requestID,
			IntegrationID:  integrationID,
			CorrelationID:  requestID,
			CreatedAt:      time.Now().UTC().Format(time.RFC3339Nano),
			ChatIDOverride: strings.TrimSpace(body.ChatIDOverride),
			ActorID:        strings.TrimSpace(ac.Subject),
			ActorType:      "user",
			Reason:         firstNonEmpty(strings.TrimSpace(body.Reason), "website telegram healthcheck requested"),
		}

		if err := deps.Bridge.PublishJSON(r.Context(), subject, event); err != nil {
			deps.Logger.Error("telegram healthcheck publish failed",
				zap.String("subject", subject),
				zap.String("request_id", requestID),
				zap.Error(err),
			)
			middleware.WriteTransportError(w, r, err)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusAccepted, map[string]any{
			"status":         "queued",
			"request_id":     requestID,
			"correlation_id": requestID,
			"integration_id": integrationID,
			"subject":        subject,
		})
	}
}

func mapControlIntegrationItem(item envelope.ControlIntegration) integrationItem {
	return integrationItem{
		IntegrationID: item.IntegrationID,
		Name:          item.Name,
		Kind:          item.Kind,
		Description:   item.Description,
		ConfigJSON:    parseJSONString(item.ConfigJSON),
		IsActive:      item.IsActive,
		CreatedAt:     item.CreatedAt,
		UpdatedAt:     item.UpdatedAt,
		CreatedBy:     item.CreatedBy,
		UpdatedBy:     item.UpdatedBy,
	}
}

func mapControlIntegrationBindingItem(item envelope.ControlIntegrationBinding) integrationBindingItem {
	return integrationBindingItem{
		IntegrationBindingID: item.IntegrationBindingID,
		IntegrationID:        item.IntegrationID,
		ScopeType:            item.ScopeType,
		ScopeID:              item.ScopeID,
		EventTypesJSON:       parseJSONString(item.EventTypesJSON),
		SeverityThreshold:    item.SeverityThreshold,
		IsActive:             item.IsActive,
		CreatedAt:            item.CreatedAt,
		UpdatedAt:            item.UpdatedAt,
	}
}
