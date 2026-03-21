package httpapi

import (
	"encoding/json"
	"errors"
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

type hostItem struct {
	HostID     string            `json:"host_id"`
	Hostname   string            `json:"hostname"`
	IP         string            `json:"ip"`
	SSHPort    uint32            `json:"ssh_port"`
	RemoteUser string            `json:"remote_user"`
	Labels     map[string]string `json:"labels,omitempty"`
	CreatedAt  string            `json:"created_at"`
	UpdatedAt  string            `json:"updated_at"`
}

type hostGroupMemberItem struct {
	HostGroupMemberID string `json:"host_group_member_id"`
	HostGroupID       string `json:"host_group_id"`
	HostID            string `json:"host_id"`
	Hostname          string `json:"hostname,omitempty"`
}

type hostGroupItem struct {
	HostGroupID string                `json:"host_group_id"`
	Name        string                `json:"name"`
	Description string                `json:"description,omitempty"`
	CreatedAt   string                `json:"created_at"`
	UpdatedAt   string                `json:"updated_at"`
	Members     []hostGroupMemberItem `json:"members,omitempty"`
}

type credentialItem struct {
	CredentialsProfileID string `json:"credentials_profile_id"`
	Name                 string `json:"name"`
	Kind                 string `json:"kind"`
	Description          string `json:"description,omitempty"`
	VaultRef             string `json:"vault_ref"`
	CreatedAt            string `json:"created_at"`
	UpdatedAt            string `json:"updated_at"`
}

type policyCreateRequest struct {
	Name           string          `json:"name"`
	Description    string          `json:"description"`
	PolicyBodyJSON json.RawMessage `json:"policy_body_json"`
}

type policyUpdateRequest struct {
	Description    string          `json:"description"`
	PolicyBodyJSON json.RawMessage `json:"policy_body_json"`
}

type hostUpsertRequest struct {
	Hostname   string            `json:"hostname"`
	IP         string            `json:"ip"`
	SSHPort    uint32            `json:"ssh_port"`
	RemoteUser string            `json:"remote_user"`
	Labels     map[string]string `json:"labels"`
}

type hostGroupUpsertRequest struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type hostGroupMemberMutationRequest struct {
	HostID string `json:"host_id"`
	Reason string `json:"reason"`
}

type credentialCreateRequest struct {
	Name        string `json:"name"`
	Kind        string `json:"kind"`
	Description string `json:"description"`
	VaultRef    string `json:"vault_ref"`
}

func controlPoliciesListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.ControlPoliciesList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlListPoliciesRequest(middleware.GetRequestID(r.Context())),
			envelope.DecodeControlListPoliciesResponse,
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
		items := make([]policyItem, 0, len(payload.Policies))
		for _, item := range payload.Policies {
			items = append(items, mapControlPolicyItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func controlPolicyDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		policyID := strings.TrimSpace(chi.URLParam(r, "id"))
		if policyID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlPoliciesGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlGetPolicyRequest(middleware.GetRequestID(r.Context()), policyID),
			envelope.DecodeControlPolicy,
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
			"item":       mapControlPolicyItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func policyCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body policyCreateRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		if strings.TrimSpace(body.Name) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "name is required")
			return
		}
		policyBodyJSON, err := marshalRawJSONObject(body.PolicyBodyJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlPoliciesCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlCreatePolicyRequest(middleware.GetRequestID(r.Context()), strings.TrimSpace(body.Name), strings.TrimSpace(body.Description), policyBodyJSON),
			envelope.DecodeControlPolicy,
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
			"item":       mapControlPolicyItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func policyUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		policyID := strings.TrimSpace(chi.URLParam(r, "id"))
		if policyID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy id is required")
			return
		}
		var body policyUpdateRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		policyBodyJSON, err := marshalRawJSONObject(body.PolicyBodyJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlPoliciesUpdate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlUpdatePolicyRequest(middleware.GetRequestID(r.Context()), policyID, strings.TrimSpace(body.Description), policyBodyJSON),
			envelope.DecodeControlPolicy,
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
			"item":       mapControlPolicyItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func controlPolicyRevisionsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		policyID := strings.TrimSpace(chi.URLParam(r, "id"))
		if policyID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlPoliciesRevisions
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlGetPolicyRevisionsRequest(middleware.GetRequestID(r.Context()), policyID),
			envelope.DecodeControlGetPolicyRevisionsResponse,
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
		items := make([]policyRevisionItem, 0, len(payload))
		for _, item := range payload {
			items = append(items, mapControlPolicyRevisionItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.ControlHostsList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlListHostsRequest(middleware.GetRequestID(r.Context())),
			envelope.DecodeControlListHostsResponse,
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
		items := make([]hostItem, 0, len(payload.Hosts))
		for _, item := range payload.Hosts {
			items = append(items, mapControlHostItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		hostID := strings.TrimSpace(chi.URLParam(r, "id"))
		if hostID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostsGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlGetHostRequest(middleware.GetRequestID(r.Context()), hostID),
			envelope.DecodeControlHost,
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
			"item":       mapControlHostItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body hostUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostsCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlCreateHostRequest(middleware.GetRequestID(r.Context()), envelope.HostInput{
				Hostname:   strings.TrimSpace(body.Hostname),
				IP:         strings.TrimSpace(body.IP),
				SSHPort:    body.SSHPort,
				RemoteUser: strings.TrimSpace(body.RemoteUser),
				Labels:     body.Labels,
			}),
			envelope.DecodeControlHost,
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
			"item":       mapControlHostItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		hostID := strings.TrimSpace(chi.URLParam(r, "id"))
		if hostID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host id is required")
			return
		}
		var body hostUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostsUpdate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlUpdateHostRequest(middleware.GetRequestID(r.Context()), hostID, envelope.HostInput{
				Hostname:   strings.TrimSpace(body.Hostname),
				IP:         strings.TrimSpace(body.IP),
				SSHPort:    body.SSHPort,
				RemoteUser: strings.TrimSpace(body.RemoteUser),
				Labels:     body.Labels,
			}),
			envelope.DecodeControlHost,
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
			"item":       mapControlHostItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostGroupsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.ControlHostGroupsList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlListHostGroupsRequest(middleware.GetRequestID(r.Context())),
			envelope.DecodeControlListHostGroupsResponse,
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
		items := make([]hostGroupItem, 0, len(payload.Groups))
		for _, item := range payload.Groups {
			items = append(items, mapControlHostGroupItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostGroupDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		groupID := strings.TrimSpace(chi.URLParam(r, "id"))
		if groupID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host group id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostGroupsGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlGetHostGroupRequest(middleware.GetRequestID(r.Context()), groupID),
			envelope.DecodeControlHostGroup,
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
			"item":       mapControlHostGroupItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostGroupCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body hostGroupUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostGroupsCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlCreateHostGroupRequest(middleware.GetRequestID(r.Context()), strings.TrimSpace(body.Name), strings.TrimSpace(body.Description)),
			envelope.DecodeControlHostGroup,
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
			"item":       mapControlHostGroupItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostGroupUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		groupID := strings.TrimSpace(chi.URLParam(r, "id"))
		if groupID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host group id is required")
			return
		}
		var body hostGroupUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostGroupsUpdate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlUpdateHostGroupRequest(middleware.GetRequestID(r.Context()), groupID, strings.TrimSpace(body.Name), strings.TrimSpace(body.Description)),
			envelope.DecodeControlHostGroup,
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
			"item":       mapControlHostGroupItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostGroupAddMemberHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		groupID := strings.TrimSpace(chi.URLParam(r, "id"))
		if groupID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host group id is required")
			return
		}
		var body hostGroupMemberMutationRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		if strings.TrimSpace(body.HostID) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host_id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostGroupsAddMember
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlAddHostGroupMemberRequest(
				middleware.GetRequestID(r.Context()),
				groupID,
				strings.TrimSpace(body.HostID),
				controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "host group member added")),
			),
			envelope.DecodeControlHostGroup,
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
			"item":       mapControlHostGroupItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func hostGroupRemoveMemberHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		groupID := strings.TrimSpace(chi.URLParam(r, "id"))
		hostID := strings.TrimSpace(chi.URLParam(r, "hostId"))
		if groupID == "" || hostID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host group id and host id are required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlHostGroupsRemoveMember
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlRemoveHostGroupMemberRequest(
				middleware.GetRequestID(r.Context()),
				groupID,
				hostID,
				controlAuditContextFromRequest(r, "host group member removed"),
			),
			envelope.DecodeControlHostGroup,
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
			"item":       mapControlHostGroupItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func credentialsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.ControlCredentialsList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlListCredentialsRequest(middleware.GetRequestID(r.Context())),
			envelope.DecodeControlListCredentialsResponse,
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
		items := make([]credentialItem, 0, len(payload.Profiles))
		for _, item := range payload.Profiles {
			items = append(items, mapControlCredentialItem(item))
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func credentialDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		credentialsProfileID := strings.TrimSpace(chi.URLParam(r, "id"))
		if credentialsProfileID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "credentials profile id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlCredentialsGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlGetCredentialsRequest(middleware.GetRequestID(r.Context()), credentialsProfileID),
			envelope.DecodeControlCredentialProfileMetadata,
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
			"item":       mapControlCredentialItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func credentialCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body credentialCreateRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlCredentialsCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeControlCreateCredentialsRequest(
				middleware.GetRequestID(r.Context()),
				strings.TrimSpace(body.Name),
				strings.TrimSpace(body.Kind),
				strings.TrimSpace(body.Description),
				strings.TrimSpace(body.VaultRef),
			),
			envelope.DecodeControlCredentialProfileMetadata,
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
			"item":       mapControlCredentialItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func mapControlPolicyItem(item envelope.ControlPolicy) policyItem {
	return policyItem{
		PolicyID:       item.PolicyID,
		Name:           item.Name,
		Description:    item.Description,
		IsActive:       item.IsActive,
		CreatedAt:      item.CreatedAt,
		UpdatedAt:      item.UpdatedAt,
		LatestRevision: item.LatestRevision,
		LatestBodyJSON: parseJSONString(item.PolicyBodyJSON),
	}
}

func mapControlPolicyRevisionItem(item envelope.ControlPolicyRevision) policyRevisionItem {
	return policyRevisionItem{
		PolicyRevisionID: item.PolicyRevisionID,
		Revision:         item.Revision,
		BodyJSON:         parseJSONString(item.PolicyBodyJSON),
		CreatedAt:        item.CreatedAt,
	}
}

func mapControlHostItem(item envelope.ControlHost) hostItem {
	return hostItem{
		HostID:     item.HostID,
		Hostname:   item.Hostname,
		IP:         item.IP,
		SSHPort:    item.SSHPort,
		RemoteUser: item.RemoteUser,
		Labels:     item.Labels,
		CreatedAt:  item.CreatedAt,
		UpdatedAt:  item.UpdatedAt,
	}
}

func mapControlHostGroupItem(item envelope.ControlHostGroup) hostGroupItem {
	members := make([]hostGroupMemberItem, 0, len(item.Members))
	for _, member := range item.Members {
		members = append(members, hostGroupMemberItem{
			HostGroupMemberID: member.HostGroupMemberID,
			HostGroupID:       member.HostGroupID,
			HostID:            member.HostID,
			Hostname:          member.Hostname,
		})
	}
	return hostGroupItem{
		HostGroupID: item.HostGroupID,
		Name:        item.Name,
		Description: item.Description,
		CreatedAt:   item.CreatedAt,
		UpdatedAt:   item.UpdatedAt,
		Members:     members,
	}
}

func mapControlCredentialItem(item envelope.ControlCredentialProfileMetadata) credentialItem {
	return credentialItem{
		CredentialsProfileID: item.CredentialsProfileID,
		Name:                 item.Name,
		Kind:                 item.Kind,
		Description:          item.Description,
		VaultRef:             item.VaultRef,
		CreatedAt:            item.CreatedAt,
		UpdatedAt:            item.UpdatedAt,
	}
}

func writeControlReplyError(w http.ResponseWriter, r *http.Request, reply envelope.ControlReplyEnvelope) {
	writeReplyError(w, r, reply.Code, reply.Message)
}

func writeReplyError(w http.ResponseWriter, r *http.Request, code, message string) {
	message = firstNonEmpty(message, "upstream request failed")
	switch strings.TrimSpace(code) {
	case "invalid_argument":
		middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", message)
	case "unauthenticated":
		middleware.WriteError(w, r, http.StatusUnauthorized, "unauthenticated", message)
	case "permission_denied":
		middleware.WriteError(w, r, http.StatusForbidden, "permission_denied", message)
	case "not_found":
		middleware.WriteError(w, r, http.StatusNotFound, "not_found", message)
	case "unavailable":
		middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", message)
	default:
		middleware.WriteError(w, r, http.StatusBadGateway, "internal", message)
	}
}

func marshalRawJSONObject(raw json.RawMessage) (string, error) {
	if len(raw) == 0 {
		return "", errors.New("policy_body_json is required")
	}
	var value any
	if err := json.Unmarshal(raw, &value); err != nil {
		return "", err
	}
	encoded, err := json.Marshal(value)
	if err != nil {
		return "", err
	}
	return string(encoded), nil
}

func parseJSONString(value string) any {
	if strings.TrimSpace(value) == "" {
		return nil
	}
	var decoded any
	if err := json.Unmarshal([]byte(value), &decoded); err != nil {
		return value
	}
	return decoded
}
