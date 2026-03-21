package stream

import (
	"encoding/json"
	"errors"
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
	Mapper  func([]byte) ([]byte, error)
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
	g.ServeMany(w, r, []StreamRequest{req})
}

func (g *Gateway) ServeMany(w http.ResponseWriter, r *http.Request, requests []StreamRequest) {
	if len(requests) == 0 {
		middleware.WriteError(w, r, http.StatusInternalServerError, "internal", "stream request is not configured")
		return
	}
	controller := http.NewResponseController(w)
	if err := controller.Flush(); err != nil && errors.Is(err, http.ErrNotSupported) {
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

	type streamEnvelope struct {
		event string
		data  []byte
	}
	envelopes := make(chan streamEnvelope, 64)
	subs := make([]*nats.Subscription, 0, len(requests))
	for _, req := range requests {
		ch := make(chan *nats.Msg, 16)
		sub, err := g.bridge.Subscribe(req.Subject, ch)
		if err != nil {
			for _, subscribed := range subs {
				_ = subscribed.Unsubscribe()
			}
			middleware.WriteTransportError(w, r, err)
			return
		}
		subs = append(subs, sub)
		go func(req StreamRequest, ch <-chan *nats.Msg) {
			for {
				select {
				case <-r.Context().Done():
					return
				case msg := <-ch:
					if msg == nil {
						return
					}
					payload := msg.Data
					if req.Mapper != nil {
						decoded, err := req.Mapper(msg.Data)
						if err != nil {
							continue
						}
						payload = decoded
					}
					select {
					case envelopes <- streamEnvelope{event: req.Event, data: payload}:
					case <-r.Context().Done():
						return
					}
				}
			}
		}(req, ch)
	}
	defer func() {
		for _, sub := range subs {
			_ = sub.Unsubscribe()
		}
	}()

	eventID := 0
	fmt.Fprintf(w, "retry: %d\n", g.cfg.RetryInterval.Milliseconds())
	fmt.Fprintf(w, "event: ready\nid: %d\ndata: {\"request_id\":%q,\"subjects\":%s}\n\n", eventID, middleware.GetRequestID(r.Context()), subjectsJSON(requests))
	_ = controller.Flush()

	heartbeat := time.NewTicker(g.cfg.HeartbeatInterval)
	defer heartbeat.Stop()

	for {
		select {
		case <-r.Context().Done():
			return
		case <-heartbeat.C:
			fmt.Fprintf(w, ": heartbeat %s\n\n", time.Now().UTC().Format(time.RFC3339))
			_ = controller.Flush()
		case msg := <-envelopes:
			if msg.data == nil {
				return
			}
			if !matchesFilter(msg.data, filters) {
				continue
			}
			eventID++
			fmt.Fprintf(w, "event: %s\nid: %d\ndata: %s\n\n", msg.event, eventID, msg.data)
			_ = controller.Flush()
		}
	}
}

func subjectsJSON(requests []StreamRequest) string {
	subjects := make([]string, 0, len(requests))
	for _, req := range requests {
		subjects = append(subjects, req.Subject)
	}
	data, err := json.Marshal(subjects)
	if err != nil {
		return "[]"
	}
	return string(data)
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
