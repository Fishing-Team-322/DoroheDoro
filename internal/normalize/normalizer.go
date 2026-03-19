package normalize

import (
	"crypto/sha1"
	"encoding/hex"
	"fmt"
	"strings"
	"time"

	"github.com/google/uuid"

	"github.com/example/dorohedoro/internal/model"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type Normalizer struct{}

func New() *Normalizer { return &Normalizer{} }

func (n *Normalizer) NormalizeBatch(batch *logsv1.LogBatch) ([]model.Event, []string) {
	result := make([]model.Event, 0, len(batch.GetEvents()))
	errs := make([]string, 0)
	for i, item := range batch.GetEvents() {
		event, err := n.Normalize(batch, item)
		if err != nil {
			errs = append(errs, fmt.Sprintf("event[%d]: %v", i, err))
			continue
		}
		result = append(result, event)
	}
	return result, errs
}

func (n *Normalizer) Normalize(batch *logsv1.LogBatch, item *logsv1.LogEvent) (model.Event, error) {
	message := strings.TrimSpace(item.GetMessage())
	if message == "" {
		return model.Event{}, ErrEmptyMessage
	}

	ts := time.UnixMilli(item.GetTimestampUnixMs()).UTC()
	if item.GetTimestampUnixMs() == 0 {
		ts = time.Now().UTC()
	}
	labels := cloneLabels(item.GetLabels())
	return model.Event{
		ID:          uuid.NewString(),
		Timestamp:   ts,
		Host:        defaultString(batch.GetHost(), "unknown-host"),
		AgentID:     defaultString(batch.GetAgentId(), "unknown-agent"),
		SourceType:  normalizeSourceType(item.GetSourceType()),
		Source:      defaultString(item.GetSource(), "unknown-source"),
		Service:     defaultString(item.GetService(), "unknown-service"),
		Severity:    normalizeSeverity(item.GetSeverity()),
		Message:     message,
		Fingerprint: fingerprint(message),
		Labels:      labels,
		Fields: map[string]any{
			"sent_at_unix_ms": batch.GetSentAtUnixMs(),
		},
		Raw: defaultString(item.GetRaw(), message),
	}, nil
}

func normalizeSeverity(value string) string {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "trace", "debug":
		return "debug"
	case "info", "notice":
		return "info"
	case "warn", "warning":
		return "warn"
	case "err", "error":
		return "error"
	case "fatal", "critical", "crit", "panic":
		return "fatal"
	default:
		return "info"
	}
}

func normalizeSourceType(value string) string {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "journald":
		return "journald"
	default:
		return "file"
	}
}

func fingerprint(message string) string {
	sum := sha1.Sum([]byte(message))
	return hex.EncodeToString(sum[:])
}

func defaultString(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return value
}

func cloneLabels(src map[string]string) map[string]string {
	if len(src) == 0 {
		return nil
	}
	out := make(map[string]string, len(src))
	for k, v := range src {
		out[k] = v
	}
	return out
}
