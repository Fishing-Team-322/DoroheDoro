package httpapi

import (
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

type roleUpsertRequest struct {
	Name        string `json:"name"`
	Slug        string `json:"slug"`
	Description string `json:"description"`
	Reason      string `json:"reason"`
}

type rolePermissionsRequest struct {
	PermissionCodes []string `json:"permission_codes"`
	Reason          string   `json:"reason"`
}

type roleBindingCreateRequest struct {
	UserID    string `json:"user_id"`
	RoleID    string `json:"role_id"`
	ScopeType string `json:"scope_type"`
	ScopeID   string `json:"scope_id"`
	Reason    string `json:"reason"`
}

type permissionItem struct {
	PermissionID string `json:"permission_id"`
	Code         string `json:"code"`
	Description  string `json:"description,omitempty"`
}

type roleItem struct {
	RoleID      string           `json:"role_id"`
	Name        string           `json:"name"`
	Slug        string           `json:"slug"`
	Description string           `json:"description,omitempty"`
	IsSystem    bool             `json:"is_system"`
	CreatedAt   string           `json:"created_at"`
	UpdatedAt   string           `json:"updated_at"`
	CreatedBy   string           `json:"created_by,omitempty"`
	UpdatedBy   string           `json:"updated_by,omitempty"`
	Permissions []permissionItem `json:"permissions,omitempty"`
}

type roleBindingItem struct {
	RoleBindingID string `json:"role_binding_id"`
	UserID        string `json:"user_id"`
	RoleID        string `json:"role_id"`
	ScopeType     string `json:"scope_type,omitempty"`
	ScopeID       string `json:"scope_id,omitempty"`
	CreatedAt     string `json:"created_at"`
}

func rolesListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRolesList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListRolesRequest(envelope.ListRolesRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Paging:        paging,
			}),
			envelope.DecodeControlListRolesResponse,
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
		items := make([]roleItem, 0, len(payload.Roles))
		for _, item := range payload.Roles {
			items = append(items, mapControlRoleItem(item))
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

func roleDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleID := strings.TrimSpace(chi.URLParam(r, "id"))
		if roleID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "role id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRolesGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetRoleRequest(middleware.GetRequestID(r.Context()), roleID),
			envelope.DecodeControlRole,
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
			"item":       mapControlRoleItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func roleCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body roleUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRolesCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateRoleRequest(envelope.CreateRoleRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Name:          strings.TrimSpace(body.Name),
				Slug:          strings.TrimSpace(body.Slug),
				Description:   strings.TrimSpace(body.Description),
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "role created")),
			}),
			envelope.DecodeControlRole,
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
			"item":       mapControlRoleItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func roleUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleID := strings.TrimSpace(chi.URLParam(r, "id"))
		if roleID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "role id is required")
			return
		}
		var body roleUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRolesUpdate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeUpdateRoleRequest(envelope.UpdateRoleRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				RoleID:        roleID,
				Name:          strings.TrimSpace(body.Name),
				Description:   strings.TrimSpace(body.Description),
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "role updated")),
			}),
			envelope.DecodeControlRole,
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
			"item":       mapControlRoleItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func rolePermissionsGetHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleID := strings.TrimSpace(chi.URLParam(r, "id"))
		if roleID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "role id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRolesPermissionsGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetRolePermissionsRequest(middleware.GetRequestID(r.Context()), roleID),
			envelope.DecodeControlGetRolePermissionsResponse,
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
			"item":       mapControlRolePermissionsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func rolePermissionsSetHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		roleID := strings.TrimSpace(chi.URLParam(r, "id"))
		if roleID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "role id is required")
			return
		}
		var body rolePermissionsRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRolesPermissionsSet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeSetRolePermissionsRequest(envelope.SetRolePermissionsRequest{
				CorrelationID:   middleware.GetRequestID(r.Context()),
				RoleID:          roleID,
				PermissionCodes: trimStringSlice(body.PermissionCodes),
				Audit:           controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "role permissions updated")),
			}),
			envelope.DecodeControlGetRolePermissionsResponse,
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
			"item":       mapControlRolePermissionsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func roleBindingsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRoleBindingsList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListRoleBindingsRequest(envelope.ListRoleBindingsRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				UserID:        strings.TrimSpace(r.URL.Query().Get("user_id")),
				RoleID:        strings.TrimSpace(r.URL.Query().Get("role_id")),
				ScopeType:     strings.TrimSpace(r.URL.Query().Get("scope_type")),
				ScopeID:       strings.TrimSpace(r.URL.Query().Get("scope_id")),
				Paging:        paging,
			}),
			envelope.DecodeControlListRoleBindingsResponse,
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
		items := make([]roleBindingItem, 0, len(payload.Bindings))
		for _, item := range payload.Bindings {
			items = append(items, mapControlRoleBindingItem(item))
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

func roleBindingCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body roleBindingCreateRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRoleBindingsCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateRoleBindingRequest(envelope.CreateRoleBindingRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				UserID:        strings.TrimSpace(body.UserID),
				RoleID:        strings.TrimSpace(body.RoleID),
				ScopeType:     strings.TrimSpace(body.ScopeType),
				ScopeID:       strings.TrimSpace(body.ScopeID),
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "role binding created")),
			}),
			envelope.DecodeControlRoleBinding,
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
			"item":       mapControlRoleBindingItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func roleBindingDeleteHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		bindingID := strings.TrimSpace(chi.URLParam(r, "id"))
		if bindingID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "role binding id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlRoleBindingsDelete
		_, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeDeleteRoleBindingRequest(envelope.DeleteRoleBindingRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				RoleBindingID: bindingID,
				Audit:         controlAuditContextFromRequest(r, "role binding deleted"),
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

func mapControlRoleItem(item envelope.ControlRole) roleItem {
	return roleItem{
		RoleID:      item.RoleID,
		Name:        item.Name,
		Slug:        item.Slug,
		Description: item.Description,
		IsSystem:    item.IsSystem,
		CreatedAt:   item.CreatedAt,
		UpdatedAt:   item.UpdatedAt,
		CreatedBy:   item.CreatedBy,
		UpdatedBy:   item.UpdatedBy,
	}
}

func mapControlRolePermissionsItem(item envelope.ControlRolePermissionsResponse) roleItem {
	out := roleItem{}
	if item.Role != nil {
		out = mapControlRoleItem(*item.Role)
	}
	out.Permissions = make([]permissionItem, 0, len(item.Permissions))
	for _, permission := range item.Permissions {
		out.Permissions = append(out.Permissions, permissionItem{
			PermissionID: permission.PermissionID,
			Code:         permission.Code,
			Description:  permission.Description,
		})
	}
	return out
}

func mapControlRoleBindingItem(item envelope.ControlRoleBinding) roleBindingItem {
	return roleBindingItem{
		RoleBindingID: item.RoleBindingID,
		UserID:        item.UserID,
		RoleID:        item.RoleID,
		ScopeType:     item.ScopeType,
		ScopeID:       item.ScopeID,
		CreatedAt:     item.CreatedAt,
	}
}
