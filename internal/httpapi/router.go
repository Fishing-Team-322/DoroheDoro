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
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/query"
	"github.com/example/dorohedoro/internal/stream"
)

type RouterDeps struct {
	Searcher *query.Searcher
	Hub      *stream.Hub
	Logger   *zap.Logger
	Ready    func(context.Context) bool
}

func NewRouter(deps RouterDeps) http.Handler {
	r := chi.NewRouter()
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
