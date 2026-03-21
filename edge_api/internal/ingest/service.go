//go:build legacy

package ingest

import (
	"context"
	"fmt"
	"strings"
	"time"

	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/bus"
	"github.com/example/dorohedoro/internal/diagnostics"
	"github.com/example/dorohedoro/internal/enrollment"
	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/normalize"
	"github.com/example/dorohedoro/internal/policy"
	"github.com/example/dorohedoro/internal/stream"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type Service struct {
	normalizer         *normalize.Normalizer
	bus                *bus.JetStream
	streamHub          *stream.Hub
	logger             *zap.Logger
	diagnostics        *diagnostics.Store
	enrollmentStore    *enrollment.Store
	policyStore        *policy.Store
	allowUnknownAgents bool
}

func NewService(normalizer *normalize.Normalizer, bus *bus.JetStream, streamHub *stream.Hub, logger *zap.Logger, diagnosticsStore *diagnostics.Store, enrollmentStore *enrollment.Store, policyStore *policy.Store, allowUnknownAgents bool) *Service {
	return &Service{
		normalizer:         normalizer,
		bus:                bus,
		streamHub:          streamHub,
		logger:             logger,
		diagnostics:        diagnosticsStore,
		enrollmentStore:    enrollmentStore,
		policyStore:        policyStore,
		allowUnknownAgents: allowUnknownAgents,
	}
}

func (s *Service) IngestBatch(ctx context.Context, batch *logsv1.LogBatch) (accepted int, rejected int, errs []string, events []model.Event) {
	agentID := strings.TrimSpace(batch.GetAgentId())
	if agentID == "" {
		agentID = "unknown-agent"
	}
	if _, ok := s.enrollmentStore.Get(agentID); !ok && !s.allowUnknownAgents {
		return 0, len(batch.GetEvents()), []string{"agent is not enrolled"}, nil
	}
	if _, ok := s.enrollmentStore.Get(agentID); !ok && s.allowUnknownAgents {
		defPolicy := s.policyStore.Default()
		s.enrollmentStore.Upsert(enrollment.Agent{
			AgentID:        agentID,
			Host:           defaultHost(batch.GetHost()),
			EnrolledAt:     time.Now().UTC(),
			PolicyRevision: defPolicy.Revision,
		})
		s.diagnostics.EnsureAgent(agentID, defaultHost(batch.GetHost()), defPolicy.Revision, time.Now().UTC())
	}

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
	policyRevision := s.policyStore.Get(agentID).Revision
	lastErr := ""
	if len(errs) > 0 {
		lastErr = strings.Join(errs, "; ")
	}
	s.diagnostics.RecordIngest(agentID, defaultHost(batch.GetHost()), policyRevision, accepted, rejected, lastErr)
	s.logger.Info("ingested batch", zap.String("agent_id", agentID), zap.String("host", batch.GetHost()), zap.Int("accepted", accepted), zap.Int("rejected", rejected))
	return accepted, rejected, errs, events
}

func defaultHost(host string) string {
	if strings.TrimSpace(host) == "" {
		return "unknown-host"
	}
	return strings.TrimSpace(host)
}
