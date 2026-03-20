package auth

import (
	"context"
	"net/http"
	"strings"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"

	"github.com/example/dorohedoro/internal/model"
)

type contextKey string

const authContextKey contextKey = "auth-context"

type Hooks struct{}

func (Hooks) HTTPMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ctx := WithContext(r.Context(), FromHTTP(r))
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

func (Hooks) GRPCUnaryInterceptor(
	ctx context.Context,
	req any,
	info *grpc.UnaryServerInfo,
	handler grpc.UnaryHandler,
) (any, error) {
	ac, err := FromGRPC(ctx)
	if err != nil {
		return nil, err
	}
	return handler(WithContext(ctx, ac), req)
}

func WithContext(ctx context.Context, ac model.AuthContext) context.Context {
	return context.WithValue(ctx, authContextKey, ac)
}

func Context(ctx context.Context) model.AuthContext {
	if ac, ok := ctx.Value(authContextKey).(model.AuthContext); ok {
		return ac
	}
	return model.AuthContext{Subject: "anonymous", Role: "stub"}
}

func FromHTTP(r *http.Request) model.AuthContext {
	subject := strings.TrimSpace(r.Header.Get("X-Subject"))
	if subject == "" {
		subject = "web-user"
	}
	return model.AuthContext{
		Subject: subject,
		Role:    valueOr(r.Header.Get("X-Role"), "stub"),
		AgentID: strings.TrimSpace(r.Header.Get("X-Agent-ID")),
	}
}

func FromGRPC(ctx context.Context) (model.AuthContext, error) {
	md, _ := metadata.FromIncomingContext(ctx)
	subject := first(md.Get("x-subject"))
	if subject == "" {
		subject = "agent"
	}
	return model.AuthContext{
		Subject: subject,
		Role:    valueOr(first(md.Get("x-role")), "stub"),
		AgentID: first(md.Get("x-agent-id")),
	}, nil
}

func RequireAgent(ctx context.Context, agentID string) error {
	if strings.TrimSpace(agentID) == "" {
		return status.Error(codes.Unauthenticated, "agent_id is required")
	}
	return nil
}

func first(values []string) string {
	if len(values) == 0 {
		return ""
	}
	return strings.TrimSpace(values[0])
}

func valueOr(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return strings.TrimSpace(value)
}
