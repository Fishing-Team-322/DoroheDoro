package stream

import (
	"context"
	"fmt"
	"net/http"
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

func NewGateway(bridge *natsbridge.Bridge, cfg config.StreamConfig) *Gateway {
	return &Gateway{bridge: bridge, cfg: cfg}
}

func (g *Gateway) ServeLogs(w http.ResponseWriter, r *http.Request, subject string) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		middleware.WriteError(w, r, http.StatusInternalServerError, "internal", "streaming is not supported")
		return
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	ch := make(chan *nats.Msg, 64)
	sub, err := g.bridge.Subscribe(subject, ch)
	if err != nil {
		middleware.WriteError(w, r, http.StatusBadGateway, "unavailable", err.Error())
		return
	}
	defer sub.Unsubscribe()
	fmt.Fprintf(w, "event: ready\ndata: {\"request_id\":%q}\n\n", middleware.GetRequestID(r.Context()))
	flusher.Flush()
	heartbeat := time.NewTicker(g.cfg.HeartbeatInterval)
	defer heartbeat.Stop()
	for {
		select {
		case <-r.Context().Done():
			return
		case <-heartbeat.C:
			fmt.Fprint(w, ": heartbeat\n\n")
			flusher.Flush()
		case msg := <-ch:
			if msg == nil {
				return
			}
			fmt.Fprintf(w, "event: message\ndata: %s\n\n", msg.Data)
			flusher.Flush()
		}
	}
}

func (g *Gateway) WaitUntilReady(ctx context.Context) bool {
	for {
		if g.bridge.Ready() {
			return true
		}
		select {
		case <-ctx.Done():
			return false
		case <-time.After(100 * time.Millisecond):
		}
	}
}
