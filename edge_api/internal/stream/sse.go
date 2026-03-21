package stream

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
)

type Gateway struct {
	bridge *natsbridge.Bridge
	cfg    config.StreamConfig
}

type StreamRequest struct {
	Subject string
	Event   string
}

type streamFilter struct {
	Host     string
	Service  string
	Severity string
}

func NewGateway(bridge *natsbridge.Bridge, cfg config.StreamConfig) *Gateway {
	return &Gateway{bridge: bridge, cfg: cfg}
}

func (g *Gateway) Serve(w http.ResponseWriter, r *http.Request, req StreamRequest) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		middleware.WriteError(w, r, http.StatusInternalServerError, "internal", "streaming is not supported")
		return
	}
	filters := streamFilter{
		Host:     strings.TrimSpace(r.URL.Query().Get("host")),
		Service:  strings.TrimSpace(r.URL.Query().Get("service")),
		Severity: strings.TrimSpace(r.URL.Query().Get("severity")),
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.Header().Set("X-Accel-Buffering", "no")

	ch := make(chan *nats.Msg, 64)
	sub, err := g.bridge.Subscribe(req.Subject, ch)
	if err != nil {
		middleware.WriteTransportError(w, r, err)
		return
	}
	defer sub.Unsubscribe()

	eventID := 0
	fmt.Fprintf(w, "retry: %d\n", g.cfg.RetryInterval.Milliseconds())
	fmt.Fprintf(w, "event: ready\nid: %d\ndata: {\"request_id\":%q,\"subject\":%q,\"event\":%q}\n\n", eventID, middleware.GetRequestID(r.Context()), req.Subject, req.Event)
	flusher.Flush()

	heartbeat := time.NewTicker(g.cfg.HeartbeatInterval)
	defer heartbeat.Stop()

	for {
		select {
		case <-r.Context().Done():
			return
		case <-heartbeat.C:
			fmt.Fprintf(w, ": heartbeat %s\n\n", time.Now().UTC().Format(time.RFC3339))
			flusher.Flush()
		case msg := <-ch:
			if msg == nil {
				return
			}
			if !matchesFilter(msg.Data, filters) {
				continue
			}
			eventID++
			fmt.Fprintf(w, "event: %s\nid: %d\ndata: %s\n\n", req.Event, eventID, msg.Data)
			flusher.Flush()
		}
	}
}

func matchesFilter(data []byte, filter streamFilter) bool {
	if filter.Host == "" && filter.Service == "" && filter.Severity == "" {
		return true
	}
	var payload map[string]any
	if err := json.Unmarshal(data, &payload); err != nil {
		return true
	}
	return matchField(payload, "host", filter.Host) && matchField(payload, "service", filter.Service) && matchField(payload, "severity", filter.Severity)
}

func matchField(payload map[string]any, key, expected string) bool {
	if expected == "" {
		return true
	}
	if strings.EqualFold(stringValue(payload[key]), expected) {
		return true
	}
	if nested, ok := payload["payload"].(map[string]any); ok && strings.EqualFold(stringValue(nested[key]), expected) {
		return true
	}
	return false
}

func stringValue(value any) string {
	switch v := value.(type) {
	case string:
		return strings.TrimSpace(v)
	default:
		return ""
	}
}
