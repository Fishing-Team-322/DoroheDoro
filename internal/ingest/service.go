package ingest

import (
	"context"
	"fmt"

	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/bus"
	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/normalize"
	"github.com/example/dorohedoro/internal/stream"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type Service struct {
	normalizer *normalize.Normalizer
	bus        *bus.JetStream
	streamHub  *stream.Hub
	logger     *zap.Logger
}

func NewService(normalizer *normalize.Normalizer, bus *bus.JetStream, streamHub *stream.Hub, logger *zap.Logger) *Service {
	return &Service{normalizer: normalizer, bus: bus, streamHub: streamHub, logger: logger}
}

func (s *Service) IngestBatch(ctx context.Context, batch *logsv1.LogBatch) (accepted int, rejected int, errs []string, events []model.Event) {
	normalized, errs := s.normalizer.NormalizeBatch(batch)
	for _, event := range normalized {
		if err := s.bus.PublishEvent(ctx, event); err != nil {
			rejected++
			errs = append(errs, fmt.Sprintf("publish event %s: %v", event.ID, err))
			continue
		}
		accepted++
		events = append(events, event)
		s.streamHub.Broadcast(event)
	}
	rejected += len(batch.GetEvents()) - len(normalized)
	s.logger.Info("ingested batch", zap.String("agent_id", batch.GetAgentId()), zap.String("host", batch.GetHost()), zap.Int("accepted", accepted), zap.Int("rejected", rejected))
	return accepted, rejected, errs, events
}
