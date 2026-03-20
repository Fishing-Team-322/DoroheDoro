package httpapi

import (
	"encoding/json"
	"errors"
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"
	"go.uber.org/zap"

	apidocs "github.com/example/dorohedoro/docs"
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

<<<<<<< HEAD:defay1x9/internal/httpapi/router.go
	registerDocsRoutes(r)

	r.Get("/healthz", func(w http.ResponseWriter, r *http.Request) {
		middleware.WriteJSON(w, http.StatusOK, map[string]string{"status": "ok"})
=======
	r.Get("/", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]any{
			"service":       "edge-api",
			"status":        "ok",
			"docs":          "/docs",
			"openapi":       "/openapi.json",
			"health":        "/health",
			"readiness":     "/ready",
			"grpc_listener": deps.GRPCListenAddr,
		})
>>>>>>> origin/main:edge_api/internal/httpapi/router.go
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
			middleware.WriteJSON(w, http.StatusOK, map[string]any{
				"user": auth.Context(r.Context()),
				"auth": map[string]any{"mode": "stub", "rbac": "todo"},
			})
		})
		api.Get("/agents", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.AgentsList, nil, deps.Logger))
		api.Get("/agents/{id}", func(w http.ResponseWriter, r *http.Request) {
			id := strings.TrimSpace(chi.URLParam(r, "id"))
			if id == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.AgentsGet, map[string]string{"id": id}, deps.Logger, http.StatusOK)
		})
		api.Get("/agents/{id}/diagnostics", func(w http.ResponseWriter, r *http.Request) {
			id := strings.TrimSpace(chi.URLParam(r, "id"))
			if id == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "agent id is required")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.AgentDiagnosticsGet, map[string]string{"id": id}, deps.Logger, http.StatusOK)
		})

		api.Get("/policies", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.PoliciesList, nil, deps.Logger))
		api.Get("/policies/{id}", func(w http.ResponseWriter, r *http.Request) {
			id := strings.TrimSpace(chi.URLParam(r, "id"))
			if id == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy id is required")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.PoliciesGet, map[string]string{"id": id}, deps.Logger, http.StatusOK)
		})

		api.Post("/deployments", func(w http.ResponseWriter, r *http.Request) {
			var req transport.DeploymentCreateRequest
			if err := decodeJSONBody(r, &req); err != nil {
				writeDecodeError(w, r, err)
				return
			}
			if strings.TrimSpace(req.PolicyID) == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy_id is required")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.DeploymentsCreate, req, deps.Logger, http.StatusAccepted)
		})
		api.Get("/deployments/{id}", func(w http.ResponseWriter, r *http.Request) {
			id := strings.TrimSpace(chi.URLParam(r, "id"))
			if id == "" {
				middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment id is required")
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.DeploymentsGet, map[string]string{"id": id}, deps.Logger, http.StatusOK)
		})
		api.Get("/deployments", requestReplyJSON(deps.Bridge, deps.Config.NATS.Subjects.DeploymentsList, nil, deps.Logger))

		api.Post("/logs/search", func(w http.ResponseWriter, r *http.Request) {
			var req transport.LogSearchRequest
			if err := decodeJSONBody(r, &req); err != nil {
				writeDecodeError(w, r, err)
				return
			}
			callRequestReply(w, r, deps.Bridge, deps.Config.NATS.Subjects.LogsSearch, req, deps.Logger, http.StatusOK)
		})
		api.Get("/logs/histogram", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsHistogram, deps.Logger))
		api.Get("/logs/severity", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsSeverity, deps.Logger))
		api.Get("/logs/top-hosts", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsTopHosts, deps.Logger))
		api.Get("/logs/top-services", queryRequestReply(deps.Bridge, deps.Config.NATS.Subjects.LogsTopServices, deps.Logger))
		api.Get("/stream/logs", func(w http.ResponseWriter, r *http.Request) {
			deps.Stream.ServeLogs(w, r, deps.Config.NATS.Subjects.UIStreamLogs)
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
		middleware.WriteTransportError(w, r, err)
		return
	}
	w.Header().Set("X-NATS-Subject", subject)
	middleware.WriteJSON(w, successStatus, resp)
}

func decodeJSONBody(r *http.Request, dst any) error {
	defer r.Body.Close()
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(dst); err != nil {
		return err
	}
	if decoder.More() {
		return errors.New("multiple JSON documents are not allowed")
	}
	return nil
}

func writeDecodeError(w http.ResponseWriter, r *http.Request, err error) {
	if err == nil {
		return
	}
	middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "invalid JSON body")
}
