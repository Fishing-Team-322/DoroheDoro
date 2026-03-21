package natsbridge

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/nats-io/nats.go"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/config"
)

type ErrorKind string

const (
	ErrorKindUnavailable ErrorKind = "unavailable"
	ErrorKindTimeout     ErrorKind = "timeout"
	ErrorKindBadResponse ErrorKind = "bad_response"
	ErrorKindEncode      ErrorKind = "encode"
)

type BridgeError struct {
	Kind    ErrorKind
	Subject string
	Err     error
}

func (e *BridgeError) Error() string {
	if e == nil {
		return ""
	}
	return fmt.Sprintf("nats %s on %s", e.Kind, e.Subject)
}

func (e *BridgeError) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.Err
}

type Bridge struct {
	conn   *nats.Conn
	cfg    config.NATSConfig
	logger *zap.Logger
}

func New(cfg config.NATSConfig, logger *zap.Logger) (*Bridge, error) {
	conn, err := nats.Connect(
		cfg.URL,
		nats.Name("edge-api"),
		nats.Timeout(5*time.Second),
		nats.ReconnectWait(time.Second),
		nats.DisconnectErrHandler(func(c *nats.Conn, err error) {
			if logger != nil {
				logger.Warn("nats disconnected", zap.Error(err))
			}
		}),
		nats.ReconnectHandler(func(c *nats.Conn) {
			if logger != nil {
				logger.Info("nats reconnected", zap.String("server", c.ConnectedUrl()))
			}
		}),
		nats.ClosedHandler(func(c *nats.Conn) {
			if logger != nil {
				logger.Warn("nats connection closed", zap.Error(c.LastError()))
			}
		}),
	)
	if err != nil {
		return nil, fmt.Errorf("connect nats: %w", err)
	}
	return &Bridge{conn: conn, cfg: cfg, logger: logger}, nil
}

func (b *Bridge) Close() {
	if b.conn != nil {
		_ = b.conn.Drain()
		b.conn.Close()
	}
}

func (b *Bridge) Ready() bool {
	if b == nil || b.conn == nil || b.conn.Status() != nats.CONNECTED {
		return false
	}
	return b.conn.FlushTimeout(250*time.Millisecond) == nil
}

func (b *Bridge) Request(ctx context.Context, subject string, payload []byte) (*nats.Msg, error) {
	requestCtx, cancel := b.withTimeout(ctx)
	defer cancel()
	msg := &nats.Msg{Subject: subject, Data: payload, Header: nats.Header{}}
	msg.Header.Set("X-Request-ID", RequestIDFromContext(requestCtx))
	reply, err := b.conn.RequestMsgWithContext(requestCtx, msg)
	if err != nil {
		return nil, classify(subject, err)
	}
	return reply, nil
}

func (b *Bridge) Publish(ctx context.Context, subject string, payload []byte) error {
	msg := &nats.Msg{Subject: subject, Data: payload, Header: nats.Header{}}
	msg.Header.Set("X-Request-ID", RequestIDFromContext(ctx))
	if err := b.conn.PublishMsg(msg); err != nil {
		return classify(subject, err)
	}
	return nil
}

func (b *Bridge) RequestJSON(ctx context.Context, subject string, payload any, out any) error {
	data, err := marshalJSON(payload)
	if err != nil {
		return &BridgeError{Kind: ErrorKindEncode, Subject: subject, Err: err}
	}
	reply, err := b.Request(ctx, subject, data)
	if err != nil {
		return err
	}
	if out == nil || len(reply.Data) == 0 {
		return nil
	}
	if err := json.Unmarshal(reply.Data, out); err != nil {
		return &BridgeError{Kind: ErrorKindBadResponse, Subject: subject, Err: err}
	}
	return nil
}

func (b *Bridge) PublishJSON(ctx context.Context, subject string, payload any) error {
	data, err := marshalJSON(payload)
	if err != nil {
		return &BridgeError{Kind: ErrorKindEncode, Subject: subject, Err: err}
	}
	return b.Publish(ctx, subject, data)
}

func (b *Bridge) Subscribe(subject string, ch chan *nats.Msg) (*nats.Subscription, error) {
	sub, err := b.conn.ChanSubscribe(subject, ch)
	if err != nil {
		return nil, classify(subject, err)
	}
	return sub, nil
}

func (b *Bridge) withTimeout(ctx context.Context) (context.Context, context.CancelFunc) {
	if _, ok := ctx.Deadline(); ok || b.cfg.RequestTimeout <= 0 {
		return ctx, func() {}
	}
	return context.WithTimeout(ctx, b.cfg.RequestTimeout)
}

func marshalJSON(payload any) ([]byte, error) {
	if payload == nil {
		return []byte(`{}`), nil
	}
	data, err := json.Marshal(payload)
	if err != nil {
		return nil, fmt.Errorf("marshal nats payload: %w", err)
	}
	return data, nil
}

func classify(subject string, err error) error {
	if err == nil {
		return nil
	}
	switch {
	case errors.Is(err, context.DeadlineExceeded):
		return &BridgeError{Kind: ErrorKindTimeout, Subject: subject, Err: err}
	case strings.Contains(strings.ToLower(err.Error()), "no responders"):
		return &BridgeError{Kind: ErrorKindUnavailable, Subject: subject, Err: err}
	default:
		return &BridgeError{Kind: ErrorKindUnavailable, Subject: subject, Err: err}
	}
}

type requestIDKey string

const requestIDKeyValue requestIDKey = "nats-request-id"

func WithRequestID(ctx context.Context, requestID string) context.Context {
	return context.WithValue(ctx, requestIDKeyValue, strings.TrimSpace(requestID))
}

func RequestIDFromContext(ctx context.Context) string {
	if ctx == nil {
		return uuid.NewString()
	}
	if requestID, ok := ctx.Value(requestIDKeyValue).(string); ok && strings.TrimSpace(requestID) != "" {
		return requestID
	}
	return uuid.NewString()
}
