package grpcapi

import (
	"context"

	"github.com/google/uuid"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"

	"github.com/example/dorohedoro/internal/ingest"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type Server struct {
	logsv1.UnimplementedIngestionServiceServer
	service *ingest.Service
}

func New(service *ingest.Service) *Server { return &Server{service: service} }

func (s *Server) IngestBatch(ctx context.Context, batch *logsv1.LogBatch) (*logsv1.IngestResponse, error) {
	if batch == nil {
		return nil, status.Error(codes.InvalidArgument, "batch is required")
	}
	accepted, rejected, errs, _ := s.service.IngestBatch(ctx, batch)
	return &logsv1.IngestResponse{
		AcceptedCount: int32(accepted),
		RejectedCount: int32(rejected),
		Errors:        errs,
		RequestId:     uuid.NewString(),
	}, nil
}
