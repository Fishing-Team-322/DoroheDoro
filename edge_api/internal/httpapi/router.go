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

		api.Get("/agents", notImplemented("control-plane agent registry is served by server-rs runtime"))
		api.Get("/agents/{id}", notImplemented("control-plane agent detail is served by server-rs runtime"))
		api.Get("/agents/{id}/diagnostics", notImplemented("agent diagnostics query is served by server-rs runtime"))
		api.Get("/agents/{id}/policy", agentPolicyHandler(deps))

		api.Get("/policies", notImplemented("policy control-plane is served by server-rs runtime"))
		api.Get("/policies/{id}", notImplemented("policy control-plane is served by server-rs runtime"))
		api.Post("/policies", notImplemented("policy control-plane is served by server-rs runtime"))
		api.Patch("/policies/{id}", notImplemented("policy control-plane is served by server-rs runtime"))
		api.Get("/policies/{id}/revisions", notImplemented("policy control-plane is served by server-rs runtime"))

		api.Get("/hosts", notImplemented("inventory control-plane is served by server-rs runtime"))
		api.Post("/hosts", notImplemented("inventory control-plane is served by server-rs runtime"))
		api.Get("/hosts/{id}", notImplemented("inventory control-plane is served by server-rs runtime"))
		api.Patch("/hosts/{id}", notImplemented("inventory control-plane is served by server-rs runtime"))

		api.Get("/host-groups", notImplemented("inventory control-plane is served by server-rs runtime"))
		api.Post("/host-groups", notImplemented("inventory control-plane is served by server-rs runtime"))
		api.Get("/host-groups/{id}", notImplemented("inventory control-plane is served by server-rs runtime"))
		api.Patch("/host-groups/{id}", notImplemented("inventory control-plane is served by server-rs runtime"))

		api.Get("/credentials", notImplemented("credentials metadata is served by server-rs runtime"))
		api.Post("/credentials", notImplemented("credentials metadata is served by server-rs runtime"))
		api.Get("/credentials/{id}", notImplemented("credentials metadata is served by server-rs runtime"))

		api.Post("/deployments", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Get("/deployments", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Get("/deployments/{id}", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Get("/deployments/{id}/steps", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Get("/deployments/{id}/targets", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Post("/deployments/{id}/retry", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Post("/deployments/{id}/cancel", notImplemented("deployment-plane is served by server-rs runtime"))
		api.Post("/deployments/plan", notImplemented("deployment-plane is served by server-rs runtime"))

		api.Post("/logs/search", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/{eventId}", notImplemented("query-plane is served by server-rs runtime"))
		api.Post("/logs/context", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/histogram", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/severity", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/top-hosts", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/top-services", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/heatmap", notImplemented("query-plane is served by server-rs runtime"))
		api.Get("/logs/top-patterns", notImplemented("query-plane is served by server-rs runtime"))

		api.Get("/alerts", notImplemented("alert-plane is served by server-rs runtime"))
		api.Get("/alerts/{id}", notImplemented("alert-plane is served by server-rs runtime"))
		api.Post("/alerts", notImplemented("alert-plane is served by server-rs runtime"))
		api.Patch("/alerts/{id}", notImplemented("alert-plane is served by server-rs runtime"))

		api.Get("/audit", notImplemented("audit-plane is served by server-rs runtime"))

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

func notImplemented(message string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		middleware.WriteError(w, r, http.StatusNotImplemented, "not_implemented", message)
	}
}
