package httpapi

import (
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	apidocs "github.com/example/dorohedoro/docs"
	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
	"github.com/example/dorohedoro/internal/stream"
)

type RouterDeps struct {
	Config  config.Config
	Bridge  *natsbridge.Bridge
	Stream  *stream.Gateway
	Logger  *zap.Logger
	Auth    auth.Hooks
	ReadyFn func() bool
}

func NewRouter(deps RouterDeps) http.Handler {
	r := chi.NewRouter()
	r.Use(middleware.RequestID)
	r.Use(middleware.Recovery(deps.Logger))
	r.Use(middleware.MaxBodyBytes(deps.Config.Limits.HTTPBodyBytes))
	r.Use(middleware.RateLimitHooks(deps.Config.Limits.RateLimitRPS, deps.Config.Limits.RateLimitBurst))
	r.Use(middleware.Timeout(deps.Config.Timeouts.HTTP))

	compatAuth := newCompatAuthHandler(deps.Config)
	r.Use(compatAuth.corsMiddleware)
	r.Use(middleware.AccessLog(deps.Logger))
	r.Use(deps.Auth.HTTPMiddleware)

	registerDocsRoutes(r)
	compatAuth.Register(r)

	r.Get("/", func(w http.ResponseWriter, r *http.Request) {
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"service":   deps.Config.ServiceName,
			"version":   deps.Config.Version,
			"status":    "ok",
			"docs":      "/docs",
			"openapi":   "/openapi.json",
			"health":    "/healthz",
			"readiness": "/readyz",
			"grpc":      deps.Config.GRPC.ListenAddr,
		})
	})
	r.Get("/healthz", func(w http.ResponseWriter, r *http.Request) {
		middleware.WriteJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	r.Get("/readyz", func(w http.ResponseWriter, r *http.Request) {
		if deps.ReadyFn != nil && deps.ReadyFn() {
			middleware.WriteJSON(w, http.StatusOK, map[string]string{"status": "ready"})
			return
		}
		middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "edge bridge is not ready")
	})
	r.Get("/version", func(w http.ResponseWriter, r *http.Request) {
		middleware.WriteJSON(w, http.StatusOK, map[string]string{"version": deps.Config.Version})
	})

	r.Route("/api/v1", func(api chi.Router) {
		api.Get("/me", func(w http.ResponseWriter, r *http.Request) {
			middleware.WriteJSON(w, http.StatusOK, map[string]any{
				"user": auth.Context(r.Context()),
				"auth": map[string]any{"mode": "stub", "rbac": "todo"},
			})
		})

		api.Get("/auth/me", compatAuth.handleCurrentSession)
		api.Post("/auth/login", compatAuth.handleLogin)
		api.Post("/auth/logout", compatAuth.handleLogout)

		api.Get("/agents", agentsListHandler(deps))
		api.Get("/agents/{id}", agentDetailHandler(deps))
		api.Get("/agents/{id}/diagnostics", agentDiagnosticsHandler(deps))
		api.Get("/agents/{id}/policy", agentPolicyHandler(deps))

		api.Get("/policies", controlPoliciesListHandler(deps))
		api.Get("/policies/{id}", controlPolicyDetailHandler(deps))
		api.Post("/policies", policyCreateHandler(deps))
		api.Patch("/policies/{id}", policyUpdateHandler(deps))
		api.Get("/policies/{id}/revisions", controlPolicyRevisionsHandler(deps))

		api.Get("/hosts", hostsListHandler(deps))
		api.Post("/hosts", hostCreateHandler(deps))
		api.Get("/hosts/{id}", hostDetailHandler(deps))
		api.Patch("/hosts/{id}", hostUpdateHandler(deps))

		api.Get("/host-groups", hostGroupsListHandler(deps))
		api.Post("/host-groups", hostGroupCreateHandler(deps))
		api.Get("/host-groups/{id}", hostGroupDetailHandler(deps))
		api.Patch("/host-groups/{id}", hostGroupUpdateHandler(deps))
		api.Post("/host-groups/{id}/members", hostGroupAddMemberHandler(deps))
		api.Delete("/host-groups/{id}/members/{hostId}", hostGroupRemoveMemberHandler(deps))

		api.Get("/credentials", credentialsListHandler(deps))
		api.Post("/credentials", credentialCreateHandler(deps))
		api.Get("/credentials/{id}", credentialDetailHandler(deps))

		api.Post("/deployments", deploymentCreateHandler(deps))
		api.Get("/deployments", deploymentsListHandler(deps))
		api.Get("/deployments/{id}", deploymentDetailHandler(deps))
		api.Get("/deployments/{id}/steps", deploymentStepsHandler(deps))
		api.Get("/deployments/{id}/targets", deploymentTargetsHandler(deps))
		api.Post("/deployments/{id}/retry", deploymentRetryHandler(deps))
		api.Post("/deployments/{id}/cancel", deploymentCancelHandler(deps))
		api.Post("/deployments/plan", deploymentPlanHandler(deps))

		api.Post("/logs/search", logsSearchHandler(deps))
		api.Get("/logs/{eventId}", logDetailHandler(deps))
		api.Post("/logs/context", logsContextHandler(deps))
		api.Get("/logs/histogram", logsHistogramHandler(deps))
		api.Get("/logs/severity", logsSeverityHandler(deps))
		api.Get("/logs/top-hosts", logsTopHostsHandler(deps))
		api.Get("/logs/top-services", logsTopServicesHandler(deps))
		api.Get("/logs/heatmap", logsHeatmapHandler(deps))
		api.Get("/logs/top-patterns", logsTopPatternsHandler(deps))
		api.Get("/logs/anomalies", logsAnomaliesHandler(deps))
		api.Get("/dashboards/overview", dashboardOverviewHandler(deps))

		api.Get("/alerts", alertsListHandler(deps))
		api.Get("/alerts/rules", alertRulesListHandler(deps))
		api.Get("/alerts/rules/{id}", alertRuleDetailHandler(deps))
		api.Get("/alerts/{id}", alertDetailHandler(deps))
		api.Post("/alerts", alertRuleCreateHandler(deps))
		api.Patch("/alerts/{id}", alertRuleUpdateHandler(deps))

		api.Get("/audit", auditListHandler(deps))

		api.Get("/clusters", clustersListHandler(deps))
		api.Get("/clusters/{id}", clusterDetailHandler(deps))
		api.Post("/clusters", clusterCreateHandler(deps))
		api.Patch("/clusters/{id}", clusterUpdateHandler(deps))
		api.Post("/clusters/{id}/hosts", clusterAddHostHandler(deps))
		api.Delete("/clusters/{id}/hosts/{hostId}", clusterRemoveHostHandler(deps))

		api.Get("/roles", rolesListHandler(deps))
		api.Get("/roles/{id}", roleDetailHandler(deps))
		api.Post("/roles", roleCreateHandler(deps))
		api.Patch("/roles/{id}", roleUpdateHandler(deps))
		api.Get("/roles/{id}/permissions", rolePermissionsGetHandler(deps))
		api.Put("/roles/{id}/permissions", rolePermissionsSetHandler(deps))
		api.Get("/role-bindings", roleBindingsListHandler(deps))
		api.Post("/role-bindings", roleBindingCreateHandler(deps))
		api.Delete("/role-bindings/{id}", roleBindingDeleteHandler(deps))

		api.Get("/integrations", integrationsListHandler(deps))
		api.Get("/integrations/{id}", integrationDetailHandler(deps))
		api.Post("/integrations", integrationCreateHandler(deps))
		api.Patch("/integrations/{id}", integrationUpdateHandler(deps))
		api.Post("/integrations/{id}/bindings", integrationBindHandler(deps))
		api.Delete("/integrations/{id}/bindings/{bindingId}", integrationUnbindHandler(deps))

		api.Get("/tickets", ticketsListHandler(deps))
		api.Get("/tickets/{id}", ticketDetailHandler(deps))
		api.Post("/tickets", ticketCreateHandler(deps))
		api.Post("/tickets/{id}/assign", ticketAssignHandler(deps))
		api.Post("/tickets/{id}/unassign", ticketUnassignHandler(deps))
		api.Post("/tickets/{id}/comments", ticketCommentAddHandler(deps))
		api.Post("/tickets/{id}/status", ticketStatusHandler(deps))
		api.Post("/tickets/{id}/close", ticketCloseHandler(deps))

		api.Get("/anomalies/rules", anomalyRulesListHandler(deps))
		api.Get("/anomalies/rules/{id}", anomalyRuleDetailHandler(deps))
		api.Post("/anomalies/rules", anomalyRuleCreateHandler(deps))
		api.Patch("/anomalies/rules/{id}", anomalyRuleUpdateHandler(deps))
		api.Get("/anomalies/instances", anomalyInstancesListHandler(deps))
		api.Get("/anomalies/instances/{id}", anomalyInstanceDetailHandler(deps))

		api.Get("/stream/logs", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamLogs, Event: "log"})
		})
		api.Get("/stream/deployments", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.ServeMany(w, r, []stream.StreamRequest{
				{Subject: deps.Config.NATS.Subjects.DeploymentsJobsStatus, Event: "status", Mapper: envelope.DecodeDeploymentStatusEventJSON},
				{Subject: deps.Config.NATS.Subjects.DeploymentsJobsStep, Event: "step", Mapper: envelope.DecodeDeploymentStepEventJSON},
			})
		})
		api.Get("/stream/alerts", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamAlerts, Event: "alert"})
		})
		api.Get("/stream/agents", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamAgents, Event: "agent"})
		})
		api.Get("/stream/clusters", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamClusters, Event: "cluster"})
		})
		api.Get("/stream/tickets", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamTickets, Event: "ticket"})
		})
		api.Get("/stream/anomalies", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamAnomalies, Event: "anomaly"})
		})
	})

	return r
}

type agentRegistryItem struct {
	AgentID      string         `json:"agent_id"`
	Hostname     string         `json:"hostname"`
	Status       string         `json:"status"`
	Version      string         `json:"version,omitempty"`
	MetadataJSON map[string]any `json:"metadata_json,omitempty"`
	FirstSeenAt  string         `json:"first_seen_at"`
	LastSeenAt   string         `json:"last_seen_at"`
}

type agentDiagnosticsItem struct {
	AgentID     string `json:"agent_id"`
	PayloadJSON any    `json:"payload_json"`
	CreatedAt   string `json:"created_at"`
}

type policyItem struct {
	PolicyID       string `json:"policy_id"`
	Name           string `json:"name"`
	Description    string `json:"description,omitempty"`
	IsActive       bool   `json:"is_active"`
	CreatedAt      string `json:"created_at"`
	UpdatedAt      string `json:"updated_at"`
	LatestRevision string `json:"latest_revision,omitempty"`
	LatestBodyJSON any    `json:"latest_body_json,omitempty"`
}

type policyRevisionItem struct {
	PolicyRevisionID string `json:"policy_revision_id"`
	Revision         string `json:"revision"`
	BodyJSON         any    `json:"body_json"`
	CreatedAt        string `json:"created_at"`
}

func registerDocsRoutes(r chi.Router) {
	r.Get("/docs", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/docs/index.html", http.StatusTemporaryRedirect)
	})
	r.Get("/docs/", func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "/docs/index.html", http.StatusTemporaryRedirect)
	})
	r.Get("/openapi.json", serveEmbeddedFile("openapi.json", "application/json; charset=utf-8"))
	r.Get("/openapi.yaml", serveEmbeddedFile("openapi.yaml", "application/yaml; charset=utf-8"))
	r.Get("/docs/index.html", serveEmbeddedFile("ui/index.html", "text/html; charset=utf-8"))
	r.Get("/docs/openapi-explorer.js", serveEmbeddedFile("ui/openapi-explorer.js", "application/javascript; charset=utf-8"))
	r.Get("/docs/openapi-explorer.css", serveEmbeddedFile("ui/openapi-explorer.css", "text/css; charset=utf-8"))
}

func serveEmbeddedFile(name, contentType string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		payload, err := apidocs.Files.ReadFile(name)
		if err != nil {
			middleware.WriteError(w, r, http.StatusInternalServerError, "internal", "embedded docs asset is missing")
			return
		}
		w.Header().Set("Content-Type", contentType)
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write(payload)
	}
}

func agentPolicyHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agentID := strings.TrimSpace(chi.URLParam(r, "id"))
		if agentID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
			return
		}
		replyMsg, err := deps.Bridge.Request(r.Context(), deps.Config.NATS.Subjects.AgentsPolicyGet, envelope.EncodeGetAgentPolicyRequest(envelope.GetAgentPolicyRequest{
			CorrelationID: middleware.GetRequestID(r.Context()),
			AgentID:       agentID,
		}))
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", deps.Config.NATS.Subjects.AgentsPolicyGet), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}

		reply, err := envelope.DecodeAgentReplyEnvelope(replyMsg.Data)
		if err != nil {
			deps.Logger.Error("decode upstream agent reply failed", zap.String("subject", deps.Config.NATS.Subjects.AgentsPolicyGet), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "invalid upstream response")
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		payload, err := envelope.DecodeGetAgentPolicyResponse(reply.Payload)
		if err != nil {
			deps.Logger.Error("decode upstream policy payload failed", zap.String("subject", deps.Config.NATS.Subjects.AgentsPolicyGet), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "invalid upstream response")
			return
		}

		w.Header().Set("X-NATS-Subject", deps.Config.NATS.Subjects.AgentsPolicyGet)
		policy := map[string]any{}
		if payload.Policy != nil {
			policy["id"] = payload.Policy.PolicyID
			policy["revision_id"] = payload.Policy.PolicyRevisionID
			policy["revision"] = payload.Policy.PolicyRevision
			policy["assigned_at"] = payload.Policy.AssignedAt
			policy["name"] = payload.Policy.PolicyName
			policy["description"] = payload.Policy.PolicyDescription
		}
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"agent_id":   agentID,
			"policy":     policy,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func agentsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.AgentsList
		payload, reply, err := requestAgentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeListAgentsRequest(envelope.ListAgentsRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
			}),
			envelope.DecodeListAgentsResponse,
		)
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		items := make([]agentRegistryItem, 0, len(payload.Agents))
		for _, agent := range payload.Agents {
			items = append(items, agentRegistryItem{
				AgentID:      agent.AgentID,
				Hostname:     agent.Hostname,
				Status:       agent.Status,
				Version:      agent.Version,
				LastSeenAt:   agent.LastSeenAt,
				MetadataJSON: map[string]any{},
			})
		}
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func agentDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agentID := strings.TrimSpace(chi.URLParam(r, "id"))
		if agentID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AgentsGet
		item, reply, err := requestAgentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetAgentRequest(envelope.GetAgentRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				AgentID:       agentID,
			}),
			envelope.DecodeAgentDetail,
		)
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item": map[string]any{
				"agent_id":         item.AgentID,
				"hostname":         item.Hostname,
				"status":           item.Status,
				"version":          item.Version,
				"metadata_json":    item.Metadata,
				"first_seen_at":    item.FirstSeenAt,
				"last_seen_at":     item.LastSeenAt,
				"effective_policy": mapPolicyBinding(item.EffectivePolicy),
			},
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func agentDiagnosticsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agentID := strings.TrimSpace(chi.URLParam(r, "id"))
		if agentID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AgentsDiagnosticsGet
		item, reply, err := requestAgentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeGetAgentDiagnosticsRequest(envelope.GetAgentDiagnosticsRequest{
				CorrelationID: middleware.GetRequestID(r.Context()),
				AgentID:       agentID,
			}),
			envelope.DecodeDiagnosticsSnapshot,
		)
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item": map[string]any{
				"agent_id":     item.AgentID,
				"payload_json": item.PayloadJSON,
				"created_at":   item.CreatedAt,
			},
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func mapPolicyBinding(binding *envelope.AgentPolicyBinding) map[string]any {
	if binding == nil {
		return nil
	}
	return map[string]any{
		"policy_id":          binding.PolicyID,
		"policy_revision_id": binding.PolicyRevisionID,
		"policy_revision":    binding.PolicyRevision,
		"assigned_at":        binding.AssignedAt,
		"policy_name":        binding.PolicyName,
		"policy_description": binding.PolicyDescription,
	}
}

func policiesListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.ControlPoliciesList
		items, reply, err := requestJSONEnvelope[[]policyItem](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]string{"correlation_id": middleware.GetRequestID(r.Context())},
		)
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func policyDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		policyID := strings.TrimSpace(chi.URLParam(r, "id"))
		if policyID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlPoliciesGet
		item, reply, err := requestJSONEnvelope[policyItem](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]string{"correlation_id": middleware.GetRequestID(r.Context()), "policy_id": policyID},
		)
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"item":       item,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func policyRevisionsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		policyID := strings.TrimSpace(chi.URLParam(r, "id"))
		if policyID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.ControlPoliciesRevisions
		items, reply, err := requestJSONEnvelope[[]policyRevisionItem](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]string{"correlation_id": middleware.GetRequestID(r.Context()), "policy_id": policyID},
		)
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}
		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"items":      items,
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func writeAgentReplyError(w http.ResponseWriter, r *http.Request, reply envelope.AgentReplyEnvelope) {
	message := firstNonEmpty(reply.Message, "upstream request failed")
	switch strings.TrimSpace(reply.Code) {
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

func runtimeUnavailable(subject, message string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if strings.TrimSpace(subject) != "" {
			w.Header().Set("X-NATS-Subject", subject)
		}
		w.Header().Set("X-Boundary-State", "awaiting-runtime")
		middleware.WriteError(w, r, http.StatusNotImplemented, "not_implemented", message)
	}
}
