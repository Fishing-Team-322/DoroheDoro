package middleware

import (
	"context"
	"encoding/json"
	"net"
	"net/http"
	"runtime/debug"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/natsbridge"
)

type statusWriter struct {
	http.ResponseWriter
	status int
}

func (w *statusWriter) WriteHeader(status int) {
	w.status = status
	w.ResponseWriter.WriteHeader(status)
}

type contextKey string

const requestIDKey contextKey = "request-id"

func Chain(h http.Handler, middlewares ...func(http.Handler) http.Handler) http.Handler {
	for i := len(middlewares) - 1; i >= 0; i-- {
		h = middlewares[i](h)
	}
	return h
}

func RequestID(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestID := strings.TrimSpace(r.Header.Get("X-Request-ID"))
		if requestID == "" {
			requestID = uuid.NewString()
		}
		ctx := context.WithValue(r.Context(), requestIDKey, requestID)
		ctx = natsbridge.WithRequestID(ctx, requestID)
		w.Header().Set("X-Request-ID", requestID)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

func Timeout(timeout time.Duration) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.TimeoutHandler(next, timeout, `{"error":{"code":"deadline_exceeded","message":"request timeout"}}`)
	}
}

func Recovery(logger *zap.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			defer func() {
				if rec := recover(); rec != nil {
					logger.Error("http panic recovered", zap.Any("panic", rec), zap.ByteString("stack", debug.Stack()), zap.String("request_id", GetRequestID(r.Context())))
					WriteError(w, r, http.StatusInternalServerError, "internal", "internal server error")
				}
			}()
			next.ServeHTTP(w, r)
		})
	}
}

func MaxBodyBytes(limit int64) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			r.Body = http.MaxBytesReader(w, r.Body, limit)
			next.ServeHTTP(w, r)
		})
	}
}

func RateLimitHooks(rps, burst int) func(http.Handler) http.Handler {
	if rps <= 0 || burst <= 0 {
		return func(next http.Handler) http.Handler { return next }
	}
	type bucket struct {
		tokens float64
		last   time.Time
	}
	var (
		mu      sync.Mutex
		buckets = map[string]*bucket{}
	)
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			key := clientIP(r)
			now := time.Now()
			mu.Lock()
			b := buckets[key]
			if b == nil {
				b = &bucket{tokens: float64(burst), last: now}
				buckets[key] = b
			}
			elapsed := now.Sub(b.last).Seconds()
			b.tokens += elapsed * float64(rps)
			if b.tokens > float64(burst) {
				b.tokens = float64(burst)
			}
			b.last = now
			if b.tokens < 1 {
				mu.Unlock()
				WriteError(w, r, http.StatusTooManyRequests, "resource_exhausted", "rate limit hook triggered")
				return
			}
			b.tokens--
			mu.Unlock()
			next.ServeHTTP(w, r)
		})
	}
}

func AccessLog(logger *zap.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			started := time.Now()
			sw := &statusWriter{ResponseWriter: w, status: http.StatusOK}
			next.ServeHTTP(sw, r)
			logger.Info("http access",
				zap.String("request_id", GetRequestID(r.Context())),
				zap.String("method", r.Method),
				zap.String("path", r.URL.Path),
				zap.Int("status", sw.status),
				zap.Duration("latency", time.Since(started)),
				zap.String("remote_addr", clientIP(r)),
				zap.String("agent_id", maskedValue(r.Header.Get("X-Agent-ID"))),
				zap.String("subject", maskedValue(r.Header.Get("X-NATS-Subject"))),
			)
		})
	}
}

func WriteJSON(w http.ResponseWriter, status int, payload any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(payload)
}

func WriteError(w http.ResponseWriter, r *http.Request, status int, code, message string) {
	WriteJSON(w, status, model.ErrorEnvelope{Error: model.ErrorBody{Code: code, Message: message, RequestID: GetRequestID(r.Context())}})
}

func GetRequestID(ctx context.Context) string {
	if v, ok := ctx.Value(requestIDKey).(string); ok && v != "" {
		return v
	}
	return ""
}

func clientIP(r *http.Request) string {
	if forwarded := strings.TrimSpace(r.Header.Get("X-Forwarded-For")); forwarded != "" {
		parts := strings.Split(forwarded, ",")
		return strings.TrimSpace(parts[0])
	}
	host, _, err := net.SplitHostPort(strings.TrimSpace(r.RemoteAddr))
	if err == nil {
		return host
	}
	return r.RemoteAddr
}

func maskedValue(v string) string {
	v = strings.TrimSpace(v)
	if len(v) <= 4 {
		return v
	}
	return v[:2] + "***" + v[len(v)-2:]
}
