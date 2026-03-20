package grpcapi

import (
	"context"
	"strings"

	"go.uber.org/zap"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"

	edgev1 "github.com/example/dorohedoro/contracts/proto"
	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
)

type Server struct {
	edgev1.UnimplementedAgentIngressServiceServer
	cfg    config.Config
	bridge *natsbridge.Bridge
	logger *zap.Logger
}

func New(cfg config.Config, bridge *natsbridge.Bridge, logger *zap.Logger) *Server {
	return &Server{cfg: cfg, bridge: bridge, logger: logger}
}

func (s *Server) Enroll(ctx context.Context, req *edgev1.EnrollRequest) (*edgev1.EnrollResponse, error) {
	if req == nil || strings.TrimSpace(req.GetEnrollmentToken()) == "" || strings.TrimSpace(req.GetHost()) == "" {
		return nil, status.Error(codes.InvalidArgument, "enrollment_token and host are required")
	}
	var resp edgev1.EnrollResponse
	if err := s.bridge.Request(ctx, s.cfg.NATS.Subjects.AgentsEnrollRequest, req, &resp); err != nil {
		return nil, status.Error(codes.Unavailable, err.Error())
	}
	return &resp, nil
}

func (s *Server) FetchPolicy(ctx context.Context, req *edgev1.FetchPolicyRequest) (*edgev1.FetchPolicyResponse, error) {
	if req == nil || strings.TrimSpace(req.GetAgentId()) == "" {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	var resp edgev1.FetchPolicyResponse
	if err := s.bridge.Request(ctx, s.cfg.NATS.Subjects.AgentsPolicyFetch, req, &resp); err != nil {
		return nil, status.Error(codes.Unavailable, err.Error())
	}
	return &resp, nil
}

func (s *Server) SendHeartbeat(ctx context.Context, req *edgev1.HeartbeatRequest) (*edgev1.Ack, error) {
	if req == nil || auth.RequireAgent(ctx, req.GetAgentId()) != nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if err := s.bridge.Publish(ctx, s.cfg.NATS.Subjects.AgentsHeartbeat, req); err != nil {
		return nil, status.Error(codes.Unavailable, err.Error())
	}
	return &edgev1.Ack{Accepted: true, RequestId: middleware.GetRequestID(ctx), Message: "published"}, nil
}

func (s *Server) SendDiagnostics(ctx context.Context, req *edgev1.DiagnosticsRequest) (*edgev1.Ack, error) {
	if req == nil || auth.RequireAgent(ctx, req.GetAgentId()) != nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if err := s.bridge.Publish(ctx, s.cfg.NATS.Subjects.AgentsDiagnostics, req); err != nil {
		return nil, status.Error(codes.Unavailable, err.Error())
	}
	return &edgev1.Ack{Accepted: true, RequestId: middleware.GetRequestID(ctx), Message: "published"}, nil
}

func (s *Server) IngestLogs(ctx context.Context, req *edgev1.IngestLogsRequest) (*edgev1.IngestLogsResponse, error) {
	if req == nil || auth.RequireAgent(ctx, req.GetAgentId()) != nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if len(req.GetEvents()) == 0 {
		return nil, status.Error(codes.InvalidArgument, "events are required")
	}
	if len(req.GetEvents()) > s.cfg.Limits.AgentLogBatchSize {
		return nil, status.Error(codes.InvalidArgument, "batch size limit exceeded")
	}
	if err := s.bridge.Publish(ctx, s.cfg.NATS.Subjects.LogsIngestRaw, req); err != nil {
		return nil, status.Error(codes.Unavailable, err.Error())
	}
	return &edgev1.IngestLogsResponse{Accepted: true, AcceptedCount: int32(len(req.GetEvents())), RequestId: middleware.GetRequestID(ctx)}, nil
}
