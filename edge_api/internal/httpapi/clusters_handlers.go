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

type clusterUpsertRequest struct {
	Name         string          `json:"name"`
	Slug         string          `json:"slug"`
	Description  string          `json:"description"`
	IsActive     bool            `json:"is_active"`
	MetadataJSON json.RawMessage `json:"metadata_json"`
	Reason       string          `json:"reason"`
}

type clusterHostMutationBody struct {
	HostID string `json:"host_id"`
	Reason string `json:"reason"`
}

type clusterHostBindingItem struct {
	ClusterHostID string `json:"cluster_host_id"`
	HostID        string `json:"host_id"`
	Hostname      string `json:"hostname"`
	CreatedAt     string `json:"created_at"`
}

type clusterAgentBindingItem struct {
	ClusterAgentID string `json:"cluster_agent_id"`
	AgentID        string `json:"agent_id"`
	CreatedAt      string `json:"created_at"`
}

type clusterItem struct {
	ClusterID    string                    `json:"cluster_id"`
	Name         string                    `json:"name"`
	Slug         string                    `json:"slug"`
	Description  string                    `json:"description,omitempty"`
	IsActive     bool                      `json:"is_active"`
	CreatedAt    string                    `json:"created_at"`
	UpdatedAt    string                    `json:"updated_at"`
	CreatedBy    string                    `json:"created_by,omitempty"`
	UpdatedBy    string                    `json:"updated_by,omitempty"`
	MetadataJSON any                       `json:"metadata_json,omitempty"`
	HostCount    uint32                    `json:"host_count"`
	AgentCount   uint32                    `json:"agent_count"`
	Hosts        []clusterHostBindingItem  `json:"hosts,omitempty"`
	Agents       []clusterAgentBindingItem `json:"agents,omitempty"`
}

func clustersListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		paging, err := parsePagingRequest(r)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		includeMembers, err := parseOptionalBoolQuery(r, "include_members")
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		subject := deps.Config.NATS.Subjects.ControlClustersList
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListClustersRequest(envelope.ListClustersRequest{
				CorrelationID:  middleware.GetRequestID(r.Context()),
				Paging:         paging,
				Query:          paging.Query,
				HostID:         strings.TrimSpace(r.URL.Query().Get("host_id")),
				IncludeMembers: includeMembers,
			}),
			envelope.DecodeControlListClustersResponse,
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

		items := make([]clusterItem, 0, len(payload.Clusters))
		for _, item := range payload.Clusters {
			items = append(items, mapControlClusterItem(item))
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

func clusterDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		clusterID := strings.TrimSpace(chi.URLParam(r, "id"))
		if clusterID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "cluster id is required")
			return
		}
		includeMembers, err := parseOptionalBoolQuery(r, "include_members")
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		if !includeMembers {
			includeMembers = true
		}

		subject := deps.Config.NATS.Subjects.ControlClustersGet
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetClusterRequest(middleware.GetRequestID(r.Context()), clusterID, includeMembers),
			envelope.DecodeControlGetClusterResponse,
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
			"item":       mapControlClusterDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func clusterCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body clusterUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		metadataJSON, err := marshalOptionalRawJSON(body.MetadataJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}

		subject := deps.Config.NATS.Subjects.ControlClustersCreate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeCreateClusterRequest(envelope.CreateClusterRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				Name:          strings.TrimSpace(body.Name),
				Slug:          strings.TrimSpace(body.Slug),
				Description:   strings.TrimSpace(body.Description),
				IsActive:      body.IsActive,
				MetadataJSON:  metadataJSON,
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "cluster created")),
			}),
			envelope.DecodeControlGetClusterResponse,
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
			"item":       mapControlClusterDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func clusterUpdateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		clusterID := strings.TrimSpace(chi.URLParam(r, "id"))
		if clusterID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "cluster id is required")
			return
		}
		var body clusterUpsertRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		metadataJSON, err := marshalOptionalRawJSON(body.MetadataJSON)
		if err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}

		subject := deps.Config.NATS.Subjects.ControlClustersUpdate
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeUpdateClusterRequest(envelope.UpdateClusterRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				ClusterID:     clusterID,
				Name:          strings.TrimSpace(body.Name),
				Slug:          strings.TrimSpace(body.Slug),
				Description:   strings.TrimSpace(body.Description),
				IsActive:      body.IsActive,
				MetadataJSON:  metadataJSON,
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "cluster updated")),
			}),
			envelope.DecodeControlGetClusterResponse,
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
			"item":       mapControlClusterDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func clusterAddHostHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		clusterID := strings.TrimSpace(chi.URLParam(r, "id"))
		if clusterID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "cluster id is required")
			return
		}
		var body clusterHostMutationBody
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		if strings.TrimSpace(body.HostID) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host_id is required")
			return
		}

		subject := deps.Config.NATS.Subjects.ControlClustersAddHost
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeClusterHostMutationRequest(envelope.ClusterHostMutationRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				ClusterID:     clusterID,
				HostID:        strings.TrimSpace(body.HostID),
				Audit:         controlAuditContextFromRequest(r, firstNonEmpty(strings.TrimSpace(body.Reason), "cluster host added")),
			}),
			envelope.DecodeControlGetClusterResponse,
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
			"item":       mapControlClusterDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func clusterRemoveHostHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		clusterID := strings.TrimSpace(chi.URLParam(r, "id"))
		hostID := strings.TrimSpace(chi.URLParam(r, "hostId"))
		if clusterID == "" || hostID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "cluster id and host id are required")
			return
		}

		subject := deps.Config.NATS.Subjects.ControlClustersRemoveHost
		payload, reply, err := requestControlEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeClusterHostMutationRequest(envelope.ClusterHostMutationRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				ClusterID:     clusterID,
				HostID:        hostID,
				Audit:         controlAuditContextFromRequest(r, "cluster host removed"),
			}),
			envelope.DecodeControlGetClusterResponse,
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
			"item":       mapControlClusterDetailsItem(payload),
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func mapControlClusterItem(item envelope.ControlCluster) clusterItem {
	return clusterItem{
		ClusterID:    item.ClusterID,
		Name:         item.Name,
		Slug:         item.Slug,
		Description:  item.Description,
		IsActive:     item.IsActive,
		CreatedAt:    item.CreatedAt,
		UpdatedAt:    item.UpdatedAt,
		CreatedBy:    item.CreatedBy,
		UpdatedBy:    item.UpdatedBy,
		MetadataJSON: parseJSONString(item.MetadataJSON),
		HostCount:    item.HostCount,
		AgentCount:   item.AgentCount,
	}
}

func mapControlClusterDetailsItem(item envelope.ControlClusterDetails) clusterItem {
	out := mapControlClusterItem(item.Cluster)
	out.Hosts = make([]clusterHostBindingItem, 0, len(item.Hosts))
	for _, host := range item.Hosts {
		out.Hosts = append(out.Hosts, clusterHostBindingItem{
			ClusterHostID: host.ClusterHostID,
			HostID:        host.HostID,
			Hostname:      host.Hostname,
			CreatedAt:     host.CreatedAt,
		})
	}
	out.Agents = make([]clusterAgentBindingItem, 0, len(item.Agents))
	for _, agent := range item.Agents {
		out.Agents = append(out.Agents, clusterAgentBindingItem{
			ClusterAgentID: agent.ClusterAgentID,
			AgentID:        agent.AgentID,
			CreatedAt:      agent.CreatedAt,
		})
	}
	return out
}
