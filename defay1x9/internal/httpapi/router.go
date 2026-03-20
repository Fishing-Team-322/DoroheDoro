package httpapi

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
	"github.com/example/dorohedoro/internal/stream"
	"github.com/example/dorohedoro/internal/transport"
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
	r.Use(middleware.AccessLog(deps.Logger))
	r.Use(deps.Auth.HTTPMiddleware)

	r.Get("/healthz", func(w http.ResponseWriter, r *http.Request) {
		middleware.WriteJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	r.Get("/readyz", func(w http.ResponseWriter, r *http.Request) {
		if deps.ReadyFn != nil && deps.ReadyFn() {
			middleware.WriteJSON(w, http.StatusOK, map[string]string{"status": "ready"})
			return
		}
		middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "nats bridge is not ready")
	})

	r.Route("/api/v1", func(api chi.Router) {
		api.Get("/me", func(w http.ResponseWriter, r *http.Request) {
			middleware.WriteJSON(w, http.StatusOK, auth.Context(r.Context()))
		})
		api.Get("/agents", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.AgentsList, nil, deps.Logger))
		api.Get("/agents/{id}", func(w http.ResponseWriter, r *http.Request) {
			requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.AgentsGet, map[string]string{"id": chi.URLParam(r, "id")}, deps.Logger).ServeHTTP(w, r)
		})
		api.Get("/agents/{id}/diagnostics", func(w http.ResponseWriter, r *http.Request) {
			requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.AgentDiagnosticsGet, map[string]string{"id": chi.URLParam(r, "id")}, deps.Logger).ServeHTTP(w, r)
		})

		api.Get("/policies", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.PoliciesList, nil, deps.Logger))
		api.Get("/policies/{id}", func(w http.ResponseWriter, r *http.Request) {
			requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.PoliciesGet, map[string]string{"id": chi.URLParam(r, "id")}, deps.Logger).ServeHTTP(w, r)
		})

		api.Post("/deployments", func(w http.ResponseWriter, r *http.Request) {
			var req transport.DeploymentCreateRequest
			if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "invalid JSON body")
				return
			}
			if strings.TrimSpace(req.PolicyID) == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment policy_id is required")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.DeploymentsCreate, req, deps.Logger, http.StatusAccepted)
		})
		api.Get("/deployments/{id}", func(w http.ResponseWriter, r *http.Request) {
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.DeploymentsStatus, map[string]string{"id": chi.URLParam(r, "id")}, deps.Logger, http.StatusOK)
		})
		api.Get("/deployments", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.DeploymentsList, nil, deps.Logger))

		api.Post("/logs/search", func(w http.ResponseWriter, r *http.Request) {
			var req transport.LogSearchRequest
			if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "invalid JSON body")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.LogsSearch, req, deps.Logger, http.StatusOK)
		})
		api.Get("/logs/histogram", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsHistogram, deps.Logger))
		api.Get("/logs/severity", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsSeverity, deps.Logger))
		api.Get("/logs/top-hosts", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsTopHosts, deps.Logger))
		api.Get("/logs/top-services", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsTopServices, deps.Logger))

		api.Get("/alerts", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.AlertsList, nil, deps.Logger))
		api.Get("/alerts/{id}", func(w http.ResponseWriter, r *http.Request) {
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.AlertsGet, map[string]string{"id": chi.URLParam(r, "id")}, deps.Logger, http.StatusOK)
		})

		api.Get("/stream/logs", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.ServeLogs(w, r, deps.Config.NATS.Subjects.UIStreamLogs)
		})
	})

	return r
}

func requestReplyJSON(bridge *natsbridge.Bridge, subject string, payload any, logger *zap.Logger) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		callRequestReply(w, r, bridge, subject, payload, logger, http.StatusOK)
	}
}

func queryRequestReply(bridge *natsbridge.Bridge, subject string, logger *zap.Logger) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		payload := map[string]string{}
		for key, values := range r.URL.Query() {
			if len(values) > 0 {
				payload[key] = values[0]
			}
		}
		callRequestReply(w, r, bridge, subject, payload, logger, http.StatusOK)
	}
}

func callRequestReply(w http.ResponseWriter, r *http.Request, bridge *natsbridge.Bridge, subject string, payload any, logger *zap.Logger, successStatus int) {
	var resp any
	if err := bridge.Request(r.Context(), subject, payload, &resp); err != nil {
		logger.Error("nats request failed", zap.String("subject", subject), zap.String("request_id", middleware.GetRequestID(r.Context())), zap.Error(err))
		middleware.WriteError(w, r, http.StatusBadGateway, "unavailable", err.Error())
		return
	}
	w.Header().Set("X-NATS-Subject", subject)
	middleware.WriteJSON(w, successStatus, resp)
}
