package httpapi

import (
	"context"
	"encoding/json"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/gorilla/websocket"
	httpSwagger "github.com/swaggo/http-swagger/v2"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/diagnostics"
	"github.com/example/dorohedoro/internal/enrollment"
	chindexer "github.com/example/dorohedoro/internal/indexer/clickhouse"
	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/policy"
	"github.com/example/dorohedoro/internal/query"
	"github.com/example/dorohedoro/internal/stream"
)

type RouterDeps struct {
	Searcher         *query.Searcher
	Analytics        *chindexer.Indexer
	Hub              *stream.Hub
	Logger           *zap.Logger
	Ready            func(context.Context) bool
	Enrollment       *enrollment.Store
	Policy           *policy.Store
	Diagnostics      *diagnostics.Store
	GRPCListenAddr   string
	EnrollmentConfig config.EnrollmentConfig
}

func NewRouter(deps RouterDeps) http.Handler {
	r := chi.NewRouter()
	r.Get("/swagger/*", httpSwagger.Handler(httpSwagger.URL("/swagger/doc.json")))
	upgrader := websocket.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}

	r.Get("/healthz", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	r.Get("/readyz", func(w http.ResponseWriter, r *http.Request) {
		if deps.Ready != nil && deps.Ready(r.Context()) {
			writeJSON(w, http.StatusOK, map[string]string{"status": "ready"})
			return
		}
		writeJSON(w, http.StatusServiceUnavailable, map[string]string{"status": "not-ready"})
	})

	r.Route("/api/v1", func(api chi.Router) {
		api.Post("/enroll", func(w http.ResponseWriter, r *http.Request) {
			var req enrollment.EnrollRequest
			if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
				writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
				return
			}
			assigned := deps.Policy.Default()
			resp, err := deps.Enrollment.Enroll(req, assigned, deps.GRPCListenAddr, deps.EnrollmentConfig.TLSMode)
			if err != nil {
				writeJSON(w, http.StatusUnauthorized, map[string]string{"error": err.Error()})
				return
			}
			deps.Policy.Assign(resp.AgentID, assigned)
			deps.Diagnostics.EnsureAgent(resp.AgentID, req.Host, assigned.Revision, resp.EnrolledAt)
			writeJSON(w, http.StatusOK, resp)
		})

		api.Get("/policy", func(w http.ResponseWriter, r *http.Request) {
			agentID := strings.TrimSpace(r.URL.Query().Get("agent_id"))
			currentRevision := strings.TrimSpace(r.URL.Query().Get("current_revision"))
			if agentID == "" {
				writeJSON(w, http.StatusBadRequest, map[string]string{"error": "agent_id is required"})
				return
			}
			assigned := deps.Policy.Get(agentID)
			writeJSON(w, http.StatusOK, map[string]any{
				"agent_id":           agentID,
				"current_revision":   currentRevision,
				"policy":             assigned,
				"changed":            assigned.Revision != currentRevision,
				"served_at":          time.Now().UTC(),
				"ingest_tls_mode":    deps.EnrollmentConfig.TLSMode,
				"mtls_enabled":       deps.EnrollmentConfig.MTLSEnabled,
				"mtls_todo_scaffold": "server currently runs dev-mode insecure gRPC; add certificate issuance and verification before production",
			})
		})

		api.Get("/logs/search", func(w http.ResponseWriter, r *http.Request) {
			params := query.SearchParams{
				Query:    r.URL.Query().Get("q"),
				From:     r.URL.Query().Get("from"),
				To:       r.URL.Query().Get("to"),
				Host:     r.URL.Query().Get("host"),
				Service:  r.URL.Query().Get("service"),
				Severity: r.URL.Query().Get("severity"),
				Limit:    atoiDefault(r.URL.Query().Get("limit"), 100),
				Offset:   atoiDefault(r.URL.Query().Get("offset"), 0),
			}
			result, err := deps.Searcher.Search(r.Context(), params)
			if err != nil {
				deps.Logger.Error("search failed", zap.Error(err))
				writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
				return
			}
			writeJSON(w, http.StatusOK, result)
		})
		api.Get("/logs/{id}/context", func(w http.ResponseWriter, r *http.Request) {
			result, err := deps.Searcher.GetContext(r.Context(), chi.URLParam(r, "id"))
			if err != nil {
				deps.Logger.Error("context query failed", zap.Error(err))
				writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
				return
			}
			writeJSON(w, http.StatusOK, result)
		})

		api.Get("/agents", func(w http.ResponseWriter, r *http.Request) {
			writeJSON(w, http.StatusOK, map[string]any{"items": deps.Diagnostics.List()})
		})
		api.Get("/agents/{id}", func(w http.ResponseWriter, r *http.Request) {
			id := chi.URLParam(r, "id")
			if st, ok := deps.Diagnostics.Get(id); ok {
				writeJSON(w, http.StatusOK, st)
				return
			}
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "agent not found"})
		})
		api.Get("/agents/{id}/diagnostics", func(w http.ResponseWriter, r *http.Request) {
			id := chi.URLParam(r, "id")
			st, ok := deps.Diagnostics.Get(id)
			if !ok {
				writeJSON(w, http.StatusNotFound, map[string]string{"error": "agent not found"})
				return
			}
			writeJSON(w, http.StatusOK, map[string]any{
				"agent":        st,
				"policy":       deps.Policy.Get(id),
				"runtime_time": time.Now().UTC(),
			})
		})

		api.Get("/analytics/histogram", func(w http.ResponseWriter, r *http.Request) {
			if deps.Analytics == nil {
				writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "clickhouse analytics disabled"})
				return
			}
			result, err := deps.Analytics.Histogram(r.Context(), analyticsParams(r))
			if err != nil {
				writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
				return
			}
			writeJSON(w, http.StatusOK, map[string]any{"items": result})
		})
		api.Get("/analytics/severity", func(w http.ResponseWriter, r *http.Request) {
			if deps.Analytics == nil {
				writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "clickhouse analytics disabled"})
				return
			}
			result, err := deps.Analytics.Severity(r.Context(), analyticsParams(r))
			if err != nil {
				writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
				return
			}
			writeJSON(w, http.StatusOK, map[string]any{"items": result})
		})
		api.Get("/analytics/top-hosts", func(w http.ResponseWriter, r *http.Request) {
			if deps.Analytics == nil {
				writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "clickhouse analytics disabled"})
				return
			}
			result, err := deps.Analytics.TopHosts(r.Context(), analyticsParams(r))
			if err != nil {
				writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
				return
			}
			writeJSON(w, http.StatusOK, map[string]any{"items": result})
		})
		api.Get("/analytics/top-services", func(w http.ResponseWriter, r *http.Request) {
			if deps.Analytics == nil {
				writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "clickhouse analytics disabled"})
				return
			}
			result, err := deps.Analytics.TopServices(r.Context(), analyticsParams(r))
			if err != nil {
				writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
				return
			}
			writeJSON(w, http.StatusOK, map[string]any{"items": result})
		})

		api.Get("/stream/ws", func(w http.ResponseWriter, r *http.Request) {
			conn, err := upgrader.Upgrade(w, r, nil)
			if err != nil {
				return
			}
			filter := stream.Filter{Host: strings.TrimSpace(r.URL.Query().Get("host")), Service: strings.TrimSpace(r.URL.Query().Get("service")), Severity: strings.TrimSpace(r.URL.Query().Get("severity"))}
			ch, unsubscribe := deps.Hub.Subscribe(filter)
			defer unsubscribe()
			defer conn.Close()
			ctx, cancel := context.WithCancel(r.Context())
			defer cancel()
			go func() {
				for {
					if _, _, err := conn.ReadMessage(); err != nil {
						cancel()
						return
					}
				}
			}()
			ticker := time.NewTicker(30 * time.Second)
			defer ticker.Stop()
			for {
				select {
				case <-ctx.Done():
					return
				case <-ticker.C:
					_ = conn.WriteControl(websocket.PingMessage, []byte("ping"), time.Now().Add(5*time.Second))
				case event, ok := <-ch:
					if !ok {
						return
					}
					if err := conn.WriteJSON(struct {
						Type  string      `json:"type"`
						Event model.Event `json:"event"`
					}{Type: "event", Event: event}); err != nil {
						return
					}
				}
			}
		})
	})

	return r
}

func analyticsParams(r *http.Request) chindexer.AnalyticsQueryParams {
	return chindexer.AnalyticsQueryParams{
		From:  r.URL.Query().Get("from"),
		To:    r.URL.Query().Get("to"),
		Limit: atoiDefault(r.URL.Query().Get("limit"), 10),
	}
}

func writeJSON(w http.ResponseWriter, status int, payload any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(payload)
}

func atoiDefault(value string, fallback int) int {
	parsed, err := strconv.Atoi(value)
	if err != nil {
		return fallback
	}
	return parsed
}
