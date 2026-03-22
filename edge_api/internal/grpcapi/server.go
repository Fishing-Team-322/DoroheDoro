package grpcapi

import (
	"context"
	"errors"
	"strings"

	"go.uber.org/zap"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"

	edgev1 "github.com/example/dorohedoro/contracts/proto"
	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
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
	tlsIdentity, err := auth.RequireTLSIdentity(ctx, s.cfg.GRPC.MTLSEnabled)
	if err != nil {
		return nil, err
	}
	reply, err := s.requestAgentReply(ctx, s.cfg.NATS.Subjects.AgentsEnrollRequest, envelope.EncodeEnrollRequest(envelope.EnrollRequest{
		CorrelationID:   middleware.GetRequestID(ctx),
		BootstrapToken:  req.GetEnrollmentToken(),
		Hostname:        req.GetHost(),
		Version:         strings.TrimSpace(req.Labels["version"]),
		Metadata:        req.Labels,
		ExistingAgentID: strings.TrimSpace(req.GetExistingAgentId()),
		TLSIdentity:     tlsIdentity,
	}))
	if err != nil {
		return nil, err
	}
	payload, err := envelope.DecodeEnrollResponse(reply.Payload)
	if err != nil {
		return nil, status.Error(codes.Internal, "invalid upstream enroll payload")
	}
	return &edgev1.EnrollResponse{
		AgentId:        payload.AgentID,
		Status:         payload.Status,
		IssuedAtUnixMs: payload.RespondedAtUnixMs,
		RequestId:      firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(ctx)),
	}, nil
}

func (s *Server) FetchPolicy(ctx context.Context, req *edgev1.FetchPolicyRequest) (*edgev1.FetchPolicyResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if err := auth.RequireAgentWithMTLS(ctx, req.GetAgentId(), s.cfg.GRPC.MTLSEnabled); err != nil {
		return nil, err
	}
	if strings.TrimSpace(req.GetAgentId()) == "" {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	reply, err := s.requestAgentReply(ctx, s.cfg.NATS.Subjects.AgentsPolicyFetch, envelope.EncodeFetchPolicyRequest(envelope.FetchPolicyRequest{
		CorrelationID: middleware.GetRequestID(ctx),
		AgentID:       req.GetAgentId(),
	}))
	if err != nil {
		return nil, err
	}
	payload, err := envelope.DecodeFetchPolicyResponse(reply.Payload)
	if err != nil {
		return nil, status.Error(codes.Internal, "invalid upstream policy payload")
	}
	return &edgev1.FetchPolicyResponse{
		Policy: &edgev1.PolicyPayload{
			PolicyId: payload.PolicyID,
			Revision: payload.PolicyRevision,
			BodyJSON: payload.PolicyBodyJSON,
		},
		Changed:   strings.TrimSpace(req.GetCurrentRevision()) != strings.TrimSpace(payload.PolicyRevision),
		RequestId: firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(ctx)),
	}, nil
}

func (s *Server) SendHeartbeat(ctx context.Context, req *edgev1.HeartbeatRequest) (*edgev1.Ack, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if err := auth.RequireAgentWithMTLS(ctx, req.GetAgentId(), s.cfg.GRPC.MTLSEnabled); err != nil {
		return nil, err
	}
	if err := s.bridge.Publish(ctx, s.cfg.NATS.Subjects.AgentsHeartbeat, envelope.EncodeHeartbeatPayload(envelope.HeartbeatPayload{
		AgentID:      req.GetAgentId(),
		Hostname:     req.Host,
		Version:      req.GetVersion(),
		Status:       req.Status,
		HostMetadata: req.HostMetadata,
		SentAtUnixMs: req.SentAtUnixMs,
	})); err != nil {
		return nil, mapBridgeError(err)
	}
	return &edgev1.Ack{Accepted: true, RequestId: middleware.GetRequestID(ctx), Message: "published"}, nil
}

func (s *Server) SendDiagnostics(ctx context.Context, req *edgev1.DiagnosticsRequest) (*edgev1.Ack, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if err := auth.RequireAgentWithMTLS(ctx, req.GetAgentId(), s.cfg.GRPC.MTLSEnabled); err != nil {
		return nil, err
	}
	if err := s.bridge.Publish(ctx, s.cfg.NATS.Subjects.AgentsDiagnostics, envelope.EncodeDiagnosticsPayload(envelope.DiagnosticsPayload{
		AgentID:      req.GetAgentId(),
		PayloadJSON:  req.PayloadJSON,
		SentAtUnixMs: req.SentAtUnixMs,
	})); err != nil {
		return nil, mapBridgeError(err)
	}
	return &edgev1.Ack{Accepted: true, RequestId: middleware.GetRequestID(ctx), Message: "published"}, nil
}

func (s *Server) IngestLogs(ctx context.Context, req *edgev1.IngestLogsRequest) (*edgev1.IngestLogsResponse, error) {
	if req == nil {
		return nil, status.Error(codes.InvalidArgument, "agent_id is required")
	}
	if err := auth.RequireAgentWithMTLS(ctx, req.GetAgentId(), s.cfg.GRPC.MTLSEnabled); err != nil {
		return nil, err
	}
	if len(req.GetEvents()) == 0 {
		return nil, status.Error(codes.InvalidArgument, "events are required")
	}
	if len(req.GetEvents()) > s.cfg.Limits.AgentLogBatchSize {
		return nil, status.Error(codes.InvalidArgument, "batch size limit exceeded")
	}
	if err := s.bridge.PublishJSON(ctx, s.cfg.NATS.Subjects.LogsIngestRaw, req); err != nil {
		return nil, mapBridgeError(err)
	}
	return &edgev1.IngestLogsResponse{Accepted: true, AcceptedCount: int32(len(req.GetEvents())), RequestId: middleware.GetRequestID(ctx)}, nil
}

func (s *Server) requestAgentReply(ctx context.Context, subject string, payload []byte) (envelope.AgentReplyEnvelope, error) {
	msg, err := s.bridge.Request(ctx, subject, payload)
	if err != nil {
		return envelope.AgentReplyEnvelope{}, mapBridgeError(err)
	}
	reply, err := envelope.DecodeAgentReplyEnvelope(msg.Data)
	if err != nil {
		s.logger.Error("decode upstream agent reply failed", zap.String("subject", subject), zap.Error(err))
		return envelope.AgentReplyEnvelope{}, status.Error(codes.Internal, "invalid upstream response envelope")
	}
	if strings.EqualFold(reply.Status, "error") {
		return envelope.AgentReplyEnvelope{}, mapAgentReplyError(reply)
	}
	return reply, nil
}

func mapAgentReplyError(reply envelope.AgentReplyEnvelope) error {
	message := firstNonEmpty(reply.Message, "upstream request failed")
	switch strings.TrimSpace(reply.Code) {
	case "invalid_argument":
		return status.Error(codes.InvalidArgument, message)
	case "unauthenticated":
		return status.Error(codes.Unauthenticated, message)
	case "permission_denied":
		return status.Error(codes.PermissionDenied, message)
	case "not_found":
		return status.Error(codes.NotFound, message)
	case "unavailable":
		return status.Error(codes.Unavailable, message)
	default:
		return status.Error(codes.Internal, message)
	}
}

func mapBridgeError(err error) error {
	if errors.Is(err, context.DeadlineExceeded) {
		return status.Error(codes.Unavailable, "upstream request timeout")
	}
	var bridgeErr *natsbridge.BridgeError
	if errors.As(err, &bridgeErr) {
		switch bridgeErr.Kind {
		case natsbridge.ErrorKindTimeout:
			return status.Error(codes.Unavailable, "upstream request timeout")
		case natsbridge.ErrorKindBadResponse:
			return status.Error(codes.Internal, "invalid upstream response")
		default:
			return status.Error(codes.Unavailable, "edge bridge unavailable")
		}
	}
	return status.Error(codes.Internal, "internal server error")
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if strings.TrimSpace(value) != "" {
			return strings.TrimSpace(value)
		}
	}
	return ""
}
