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

		api.Get("/policies", policiesListHandler(deps))
		api.Get("/policies/{id}", policyDetailHandler(deps))
		api.Post("/policies", runtimeUnavailable(deps.Config.NATS.Subjects.ControlPoliciesCreate, "policy create is served by server-rs control-plane runtime"))
		api.Patch("/policies/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.ControlPoliciesUpdate, "policy update is served by server-rs control-plane runtime"))
		api.Get("/policies/{id}/revisions", policyRevisionsHandler(deps))

		api.Get("/hosts", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostsList, "inventory control-plane is served by server-rs runtime"))
		api.Post("/hosts", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostsCreate, "inventory control-plane is served by server-rs runtime"))
		api.Get("/hosts/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostsGet, "inventory control-plane is served by server-rs runtime"))
		api.Patch("/hosts/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostsUpdate, "inventory control-plane is served by server-rs runtime"))

		api.Get("/host-groups", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostGroupsList, "inventory control-plane is served by server-rs runtime"))
		api.Post("/host-groups", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostGroupsCreate, "inventory control-plane is served by server-rs runtime"))
		api.Get("/host-groups/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostGroupsGet, "inventory control-plane is served by server-rs runtime"))
		api.Patch("/host-groups/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.ControlHostGroupsUpdate, "inventory control-plane is served by server-rs runtime"))

		api.Get("/credentials", runtimeUnavailable(deps.Config.NATS.Subjects.ControlCredentialsList, "credentials metadata is served by server-rs runtime"))
		api.Post("/credentials", runtimeUnavailable(deps.Config.NATS.Subjects.ControlCredentialsCreate, "credentials metadata is served by server-rs runtime"))
		api.Get("/credentials/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.ControlCredentialsGet, "credentials metadata is served by server-rs runtime"))

		api.Post("/deployments", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsCreate, "deployment-plane is served by server-rs runtime"))
		api.Get("/deployments", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsList, "deployment-plane is served by server-rs runtime"))
		api.Get("/deployments/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsGet, "deployment-plane is served by server-rs runtime"))
		api.Get("/deployments/{id}/steps", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsStep, "deployment-plane is served by server-rs runtime"))
		api.Get("/deployments/{id}/targets", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsStatus, "deployment target status is served by server-rs runtime"))
		api.Post("/deployments/{id}/retry", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsRetry, "deployment-plane is served by server-rs runtime"))
		api.Post("/deployments/{id}/cancel", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsJobsCancel, "deployment-plane is served by server-rs runtime"))
		api.Post("/deployments/plan", runtimeUnavailable(deps.Config.NATS.Subjects.DeploymentsPlanCreate, "deployment planning is served by server-rs runtime"))

		api.Post("/logs/search", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsSearch, "query-plane is served by server-rs runtime"))
		api.Get("/logs/{eventId}", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsGet, "query-plane is served by server-rs runtime"))
		api.Post("/logs/context", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsContext, "query-plane is served by server-rs runtime"))
		api.Get("/logs/histogram", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsHistogram, "query-plane is served by server-rs runtime"))
		api.Get("/logs/severity", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsSeverity, "query-plane is served by server-rs runtime"))
		api.Get("/logs/top-hosts", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsTopHosts, "query-plane is served by server-rs runtime"))
		api.Get("/logs/top-services", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsTopServices, "query-plane is served by server-rs runtime"))
		api.Get("/logs/heatmap", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsHeatmap, "query-plane is served by server-rs runtime"))
		api.Get("/logs/top-patterns", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsTopPatterns, "query-plane is served by server-rs runtime"))
		api.Get("/logs/anomalies", runtimeUnavailable(deps.Config.NATS.Subjects.QueryLogsAnomalies, "query-plane is served by server-rs runtime"))
		api.Get("/dashboards/overview", runtimeUnavailable(deps.Config.NATS.Subjects.QueryDashboardsOverview, "dashboard query-plane is served by server-rs runtime"))

		api.Get("/alerts", runtimeUnavailable(deps.Config.NATS.Subjects.AlertsList, "alert-plane is served by server-rs runtime"))
		api.Get("/alerts/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.AlertsGet, "alert-plane is served by server-rs runtime"))
		api.Post("/alerts", runtimeUnavailable(deps.Config.NATS.Subjects.AlertsRulesCreate, "alert-plane is served by server-rs runtime"))
		api.Patch("/alerts/{id}", runtimeUnavailable(deps.Config.NATS.Subjects.AlertsRulesUpdate, "alert-plane is served by server-rs runtime"))

		api.Get("/audit", runtimeUnavailable(deps.Config.NATS.Subjects.AuditList, "audit-plane is served by server-rs runtime"))

		api.Get("/stream/logs", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamLogs, Event: "log"})
		})
		api.Get("/stream/deployments", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamDeployments, Event: "deployment"})
		})
		api.Get("/stream/alerts", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamAlerts, Event: "alert"})
		})
		api.Get("/stream/agents", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.Serve(w, r, stream.StreamRequest{Subject: deps.Config.NATS.Subjects.StreamAgents, Event: "agent"})
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
		replyMsg, err := deps.Bridge.Request(r.Context(), deps.Config.NATS.Subjects.AgentsPolicyFetch, envelope.EncodeFetchPolicyRequest(envelope.FetchPolicyRequest{
			CorrelationID: middleware.GetRequestID(r.Context()),
			AgentID:       agentID,
		}))
		if err != nil {
			deps.Logger.Error("nats request failed", zap.String("subject", deps.Config.NATS.Subjects.AgentsPolicyFetch), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteTransportError(w, r, err)
			return
		}

		reply, err := envelope.DecodeAgentReplyEnvelope(replyMsg.Data)
		if err != nil {
			deps.Logger.Error("decode upstream agent reply failed", zap.String("subject", deps.Config.NATS.Subjects.AgentsPolicyFetch), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "invalid upstream response")
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		payload, err := envelope.DecodeFetchPolicyResponse(reply.Payload)
		if err != nil {
			deps.Logger.Error("decode upstream policy payload failed", zap.String("subject", deps.Config.NATS.Subjects.AgentsPolicyFetch), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
			middleware.WriteError(w, r, http.StatusBadGateway, "internal", "invalid upstream response")
			return
		}

		w.Header().Set("X-NATS-Subject", deps.Config.NATS.Subjects.AgentsPolicyFetch)
		middleware.WriteJSON(w, http.StatusOK, map[string]any{
			"agent_id": payload.AgentID,
			"policy": map[string]any{
				"id":        payload.PolicyID,
				"revision":  payload.PolicyRevision,
				"body_json": payload.PolicyBodyJSON,
				"status":    payload.Status,
			},
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}

func agentsListHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		subject := deps.Config.NATS.Subjects.AgentsRegistryList
		items, reply, err := requestJSONEnvelope[[]agentRegistryItem](
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

func agentDetailHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agentID := strings.TrimSpace(chi.URLParam(r, "id"))
		if agentID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AgentsRegistryGet
		item, reply, err := requestJSONEnvelope[agentRegistryItem](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]string{"correlation_id": middleware.GetRequestID(r.Context()), "agent_id": agentID},
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

func agentDiagnosticsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agentID := strings.TrimSpace(chi.URLParam(r, "id"))
		if agentID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
			return
		}
		subject := deps.Config.NATS.Subjects.AgentsDiagnosticsGet
		item, reply, err := requestJSONEnvelope[agentDiagnosticsItem](
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			map[string]string{"correlation_id": middleware.GetRequestID(r.Context()), "agent_id": agentID},
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
