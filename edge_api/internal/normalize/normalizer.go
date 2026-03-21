//go:build legacy

package normalize

import (
	"crypto/sha1"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"regexp"
	"strings"
	"time"

	"github.com/google/uuid"

	"github.com/example/dorohedoro/internal/model"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type Normalizer struct{}

var (
	uuidPattern   = regexp.MustCompile(`\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b`)
	ipv4Pattern   = regexp.MustCompile(`\b(?:\d{1,3}\.){3}\d{1,3}\b`)
	numberPattern = regexp.MustCompile(`\b\d+\b`)
)

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
	fields := map[string]any{
		"sent_at_unix_ms": batch.GetSentAtUnixMs(),
	}
	service := strings.TrimSpace(item.GetService())
	severity := strings.TrimSpace(item.GetSeverity())
	sourceType := strings.TrimSpace(item.GetSourceType())
	raw := defaultString(item.GetRaw(), message)

	if parsed, ok := parseJSONPayload(message); ok {
		mergeFields(fields, parsed)
		if v := firstString(parsed, "message", "msg", "log"); v != "" {
			message = v
		}
		if v := firstString(parsed, "severity", "level", "log.level"); v != "" {
			severity = v
		}
		if v := firstString(parsed, "service", "service.name", "app", "logger"); v != "" {
			service = v
		}
		if v := firstString(parsed, "source_type", "sourceType", "input.type"); v != "" {
			sourceType = v
		}
		if v := firstString(parsed, "host", "hostname"); v != "" && batch.GetHost() == "" {
			fields["parsed_host"] = v
		}
		if parsedTS, ok := firstTime(parsed, "timestamp", "ts", "@timestamp", "time"); ok {
			ts = parsedTS.UTC()
		}
	}

	hostValue := stringFromFields(fields, "parsed_host", "host", "hostname")
	if hostValue == "" {
		hostValue = "unknown-host"
	}

	event := model.Event{
		ID:          uuid.NewString(),
		Timestamp:   ts,
		Host:        defaultString(batch.GetHost(), hostValue),
		AgentID:     defaultString(batch.GetAgentId(), "unknown-agent"),
		SourceType:  normalizeSourceType(defaultString(sourceType, detectSourceType(message, raw, item.GetSource()))),
		Source:      defaultString(item.GetSource(), "unknown-source"),
		Service:     normalizeService(defaultString(service, detectService(message, item.GetSource(), labels, fields))),
		Severity:    normalizeSeverity(defaultString(severity, detectSeverity(message, fields))),
		Message:     message,
		Fingerprint: fingerprint(message),
		Labels:      labels,
		Fields:      fields,
		Raw:         raw,
	}
	return event, nil
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
	case "":
		return "unknown"
	default:
		return "unknown"
	}
}

func normalizeSourceType(value string) string {
	switch strings.ToLower(strings.TrimSpace(value)) {
	case "journald", "journal":
		return "journald"
	case "syslog":
		return "syslog"
	case "container", "docker", "kubernetes":
		return "container"
	case "http":
		return "http"
	case "":
		return "file"
	default:
		return strings.ToLower(strings.TrimSpace(value))
	}
}

func normalizeService(value string) string {
	value = strings.TrimSpace(strings.ToLower(value))
	if value == "" {
		return "unknown-service"
	}
	return value
}

func fingerprint(message string) string {
	normalized := strings.ToLower(strings.TrimSpace(message))
	normalized = uuidPattern.ReplaceAllString(normalized, "<uuid>")
	normalized = ipv4Pattern.ReplaceAllString(normalized, "<ip>")
	normalized = numberPattern.ReplaceAllString(normalized, "<num>")
	sum := sha1.Sum([]byte(normalized))
	return hex.EncodeToString(sum[:])
}

func defaultString(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return strings.TrimSpace(value)
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

func parseJSONPayload(message string) (map[string]any, bool) {
	trimmed := strings.TrimSpace(message)
	if !strings.HasPrefix(trimmed, "{") || !strings.HasSuffix(trimmed, "}") {
		return nil, false
	}
	var out map[string]any
	if err := json.Unmarshal([]byte(trimmed), &out); err != nil {
		return nil, false
	}
	return out, true
}

func mergeFields(dst, src map[string]any) {
	for k, v := range src {
		dst[k] = v
	}
}

func firstString(fields map[string]any, keys ...string) string {
	for _, key := range keys {
		if v, ok := fields[key]; ok {
			switch vv := v.(type) {
			case string:
				if strings.TrimSpace(vv) != "" {
					return strings.TrimSpace(vv)
				}
			}
		}
	}
	return ""
}

func firstTime(fields map[string]any, keys ...string) (time.Time, bool) {
	for _, key := range keys {
		v, ok := fields[key]
		if !ok {
			continue
		}
		switch vv := v.(type) {
		case string:
			if ts, err := time.Parse(time.RFC3339, vv); err == nil {
				return ts, true
			}
		case float64:
			ms := int64(vv)
			if ms > 0 {
				if ms > 1_000_000_000_000 {
					return time.UnixMilli(ms), true
				}
				return time.Unix(ms, 0), true
			}
		}
	}
	return time.Time{}, false
}

func stringFromFields(fields map[string]any, keys ...string) string {
	for _, key := range keys {
		if v := firstString(fields, key); v != "" {
			return v
		}
	}
	return ""
}

func detectSeverity(message string, fields map[string]any) string {
	if v := firstString(fields, "severity", "level"); v != "" {
		return v
	}
	lower := strings.ToLower(message)
	switch {
	case strings.Contains(lower, " fatal ") || strings.HasPrefix(lower, "fatal") || strings.Contains(lower, "panic") || strings.Contains(lower, "critical"):
		return "fatal"
	case strings.Contains(lower, " error ") || strings.HasPrefix(lower, "error") || strings.Contains(lower, "failed"):
		return "error"
	case strings.Contains(lower, " warn ") || strings.HasPrefix(lower, "warn") || strings.Contains(lower, "warning") || strings.Contains(lower, "timed out"):
		return "warn"
	case strings.Contains(lower, " debug ") || strings.HasPrefix(lower, "debug"):
		return "debug"
	case strings.Contains(lower, " info ") || strings.HasPrefix(lower, "info"):
		return "info"
	default:
		return "unknown"
	}
}

func detectService(message, source string, labels map[string]string, fields map[string]any) string {
	if service := labels["service"]; strings.TrimSpace(service) != "" {
		return service
	}
	if service := firstString(fields, "service", "service_name", "service.name", "app"); service != "" {
		return service
	}
	if strings.TrimSpace(source) != "" {
		source = strings.TrimSpace(source)
		if idx := strings.LastIndex(source, "/"); idx >= 0 && idx < len(source)-1 {
			source = source[idx+1:]
		}
		source = strings.TrimSuffix(source, ".log")
		source = strings.TrimSuffix(source, ".service")
		if source != "" {
			return source
		}
	}
	lower := strings.ToLower(message)
	for _, candidate := range []string{"nginx", "sshd", "kernel", "postgres", "postgresql", "redis", "clickhouse"} {
		if strings.Contains(lower, candidate) {
			return candidate
		}
	}
	return "unknown-service"
}

func detectSourceType(message, raw, source string) string {
	combined := strings.ToLower(message + " " + raw + " " + source)
	switch {
	case strings.Contains(combined, "journald") || strings.Contains(combined, "systemd"):
		return "journald"
	case strings.Contains(combined, "stdout") || strings.Contains(combined, "container") || strings.Contains(combined, "kube"):
		return "container"
	case strings.Contains(source, "/"):
		return "file"
	default:
		return "file"
	}
}
