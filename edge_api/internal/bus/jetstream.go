//go:build legacy

package bus

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/model"
)

type JetStream struct {
	conn   *nats.Conn
	js     nats.JetStreamContext
	cfg    config.NATSConfig
	logger *zap.Logger
}

func New(ctx context.Context, cfg config.NATSConfig, logger *zap.Logger) (*JetStream, error) {
	conn, err := nats.Connect(cfg.URL, nats.Name("dorohedoro-server"))
	if err != nil {
		return nil, fmt.Errorf("connect nats: %w", err)
	}
	js, err := conn.JetStream()
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("jetstream: %w", err)
	}
	bus := &JetStream{conn: conn, js: js, cfg: cfg, logger: logger}
	if err := bus.ensureStream(ctx); err != nil {
		conn.Close()
		return nil, err
	}
	return bus, nil
}

func (b *JetStream) ensureStream(ctx context.Context) error {
	_, err := b.js.StreamInfo(b.cfg.StreamName, nats.Context(ctx))
	if err == nil {
		return nil
	}
	_, err = b.js.AddStream(&nats.StreamConfig{
		Name:      b.cfg.StreamName,
		Subjects:  []string{b.cfg.Subject},
		Storage:   nats.FileStorage,
		Retention: nats.LimitsPolicy,
		MaxAge:    72 * time.Hour,
	}, nats.Context(ctx))
	if err != nil && err != nats.ErrStreamNameAlreadyInUse {
		return fmt.Errorf("add stream: %w", err)
	}
	return nil
}

func (b *JetStream) PublishEvent(ctx context.Context, event model.Event) error {
	payload, err := json.Marshal(event)
	if err != nil {
		return fmt.Errorf("marshal event: %w", err)
	}
	_, err = b.js.Publish(b.cfg.Subject, payload, nats.Context(ctx))
	if err != nil {
		return fmt.Errorf("publish event: %w", err)
	}
	return nil
}

func (b *JetStream) SubscribeDurable(ctx context.Context, consumer string, handler nats.MsgHandler) (*nats.Subscription, error) {
	return b.js.Subscribe(b.cfg.Subject, handler,
		nats.ManualAck(),
		nats.Durable(consumer),
		nats.DeliverAll(),
		nats.AckExplicit(),
		nats.BindStream(b.cfg.StreamName),
		nats.Context(ctx),
	)
}

func (b *JetStream) Close() {
	if b.conn != nil {
		_ = b.conn.Drain()
		b.conn.Close()
	}
}
