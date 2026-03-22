package grpcapi

import (
	"context"
	"testing"

	"go.uber.org/zap"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"

	edgev1 "github.com/example/dorohedoro/contracts/proto"
	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/config"
)

func TestFetchPolicyReturnsAuthErrors(t *testing.T) {
	server := New(config.Config{
		GRPC: config.GRPCConfig{MTLSEnabled: true},
	}, nil, zap.NewNop())

	_, err := server.FetchPolicy(context.Background(), &edgev1.FetchPolicyRequest{AgentId: "agent-1"})
	if status.Code(err) != codes.Unauthenticated {
		t.Fatalf("expected unauthenticated, got %v", err)
	}

	_, err = server.FetchPolicy(context.Background(), &edgev1.FetchPolicyRequest{AgentId: ""})
	if status.Code(err) != codes.InvalidArgument {
		t.Fatalf("expected invalid argument, got %v", err)
	}

	ctx := auth.WithTLSIdentity(context.Background(), auth.AgentTLSIdentity{CommonName: "agent-1"})
	_, err = server.FetchPolicy(ctx, &edgev1.FetchPolicyRequest{AgentId: "agent-2"})
	if status.Code(err) != codes.PermissionDenied {
		t.Fatalf("expected permission denied, got %v", err)
	}
}

func TestSendHeartbeatRejectsMismatchedTLSIdentity(t *testing.T) {
	server := New(config.Config{
		GRPC: config.GRPCConfig{MTLSEnabled: true},
	}, nil, zap.NewNop())
	ctx := auth.WithTLSIdentity(context.Background(), auth.AgentTLSIdentity{CommonName: "agent-1"})

	_, err := server.SendHeartbeat(ctx, &edgev1.HeartbeatRequest{
		AgentId:      "agent-2",
		Host:         "demo-host",
		SentAtUnixMs: 1,
		Status:       "online",
	})
	if status.Code(err) != codes.PermissionDenied {
		t.Fatalf("expected permission denied, got %v", err)
	}
}

func TestSendDiagnosticsRejectsMismatchedTLSIdentity(t *testing.T) {
	server := New(config.Config{
		GRPC: config.GRPCConfig{MTLSEnabled: true},
	}, nil, zap.NewNop())
	ctx := auth.WithTLSIdentity(context.Background(), auth.AgentTLSIdentity{CommonName: "agent-1"})

	_, err := server.SendDiagnostics(ctx, &edgev1.DiagnosticsRequest{
		AgentId:      "agent-2",
		Host:         "demo-host",
		SentAtUnixMs: 1,
		PayloadJSON:  "{}",
	})
	if status.Code(err) != codes.PermissionDenied {
		t.Fatalf("expected permission denied, got %v", err)
	}
}

func TestIngestLogsRejectsMismatchedTLSIdentity(t *testing.T) {
	server := New(config.Config{
		GRPC: config.GRPCConfig{MTLSEnabled: true},
		Limits: config.LimitsConfig{
			AgentLogBatchSize: 100,
		},
	}, nil, zap.NewNop())
	ctx := auth.WithTLSIdentity(context.Background(), auth.AgentTLSIdentity{CommonName: "agent-1"})

	_, err := server.IngestLogs(ctx, &edgev1.IngestLogsRequest{
		AgentId:      "agent-2",
		Host:         "demo-host",
		SentAtUnixMs: 1,
		Events: []*edgev1.AgentLog{
			{Message: "test"},
		},
	})
	if status.Code(err) != codes.PermissionDenied {
		t.Fatalf("expected permission denied, got %v", err)
	}
}
