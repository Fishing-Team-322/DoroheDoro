package agentstatus

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

func (s *Service) newFreshness() model.AgentDataFreshness {
	return model.AgentDataFreshness{
		GeneratedAt: s.now().Format(time.RFC3339),
		Sections:    map[string]model.AgentDataFreshnessSection{},
	}
}

func (s *Service) observeSection(sections map[string]model.AgentDataFreshnessSection, name, observedAt string, staleAfter time.Duration, note string) {
	if sections == nil || strings.TrimSpace(name) == "" {
		return
	}
	observedAt = strings.TrimSpace(observedAt)
	if observedAt == "" {
		s.observeMissing(sections, name, firstNonEmpty(note, "данные не поступили"))
		return
	}
	timestamp := parseTimestamp(observedAt)
	if timestamp.IsZero() {
		sections[name] = model.AgentDataFreshnessSection{
			Status:     "stale",
			ObservedAt: observedAt,
			Note:       firstNonEmpty(note, "не удалось распарсить временную метку"),
		}
		return
	}
	age := s.now().Sub(timestamp)
	status := "fresh"
	if staleAfter > 0 && age > staleAfter {
		status = "stale"
	}
	sections[name] = model.AgentDataFreshnessSection{
		Status:     status,
		ObservedAt: timestamp.Format(time.RFC3339),
		AgeSec:     int64(age.Seconds()),
		Note:       note,
	}
}

func (s *Service) observeMissing(sections map[string]model.AgentDataFreshnessSection, name, note string) {
	if sections == nil || strings.TrimSpace(name) == "" {
		return
	}
	sections[name] = model.AgentDataFreshnessSection{
		Status: "missing",
		Note:   note,
	}
}

func (s *Service) isNoLogs(lastLogSeenAt string) bool {
	lastSeen := parseTimestamp(lastLogSeenAt)
	if lastSeen.IsZero() {
		return true
	}
	return s.now().Sub(lastSeen) > s.settings.NoLogsWindow
}

func normalizeTimelinePhase(stepName string, status int32) string {
	normalized := strings.ToLower(strings.TrimSpace(stepName))
	switch {
	case normalized == "", normalized == "plan.created":
		return ""
	case strings.Contains(normalized, "inventory"), strings.Contains(normalized, "workspace.prepared"):
		return "inventory_rendered"
	case strings.Contains(normalized, "manifest"), strings.Contains(normalized, "vault.secrets.resolved"):
		return "manifest_resolved"
	case strings.Contains(normalized, "ansible.started"), strings.Contains(normalized, "runner.started"):
		return "ansible_started"
	case strings.Contains(normalized, "host.connected"), strings.Contains(normalized, "ssh.connected"):
		return "host_connected"
	case strings.Contains(normalized, "artifact"), strings.Contains(normalized, "image.selected"):
		return "image_selected"
	case strings.Contains(normalized, "image.pulled"), strings.Contains(normalized, "image_pull"):
		return "image_pulled"
	case strings.Contains(normalized, "unit.rendered"), strings.Contains(normalized, "systemd"), strings.Contains(normalized, "unit_rendered"):
		return "unit_rendered"
	case strings.Contains(normalized, "service.restarted"), strings.Contains(normalized, "service_restart"):
		return "service_restarted"
	case strings.Contains(normalized, "health_check.started"), strings.Contains(normalized, "health.started"):
		return "health_check_started"
	case strings.Contains(normalized, "health_check.passed"):
		return "health_check_passed"
	case strings.Contains(normalized, "health_check.failed"):
		return "health_check_failed"
	case strings.Contains(normalized, "rollback.started"):
		return "rollback_started"
	case strings.Contains(normalized, "rollback.succeeded"):
		return "rollback_succeeded"
	case strings.Contains(normalized, "rollback.failed"):
		return "rollback_failed"
	case strings.Contains(normalized, "runner.completed"), strings.Contains(normalized, "execution.summary"):
		return "service_restarted"
	case strings.Contains(normalized, "runner.cancelled") && status == 4:
		return "rollback_failed"
	default:
		return ""
	}
}

func parseJSONString(value string) any {
	value = strings.TrimSpace(value)
	if value == "" {
		return nil
	}
	var out any
	if err := json.Unmarshal([]byte(value), &out); err != nil {
		return value
	}
	return out
}

func extractArtifactRef(steps []envelope.DeploymentStepSummary) string {
	for _, step := range steps {
		payload := parseJSONString(step.PayloadJSON)
		if ref := nestedString(payload, "artifact_ref", "image_ref", "image", "ref", "artifact.ref", "artifact.image", "artifact.name", "artifact.reference"); ref != "" {
			return ref
		}
	}
	return ""
}

func nestedString(value any, keys ...string) string {
	switch node := value.(type) {
	case map[string]any:
		for _, key := range keys {
			if strings.Contains(key, ".") {
				if resolved := nestedStringPath(node, strings.Split(key, ".")); resolved != "" {
					return resolved
				}
			}
			if child, ok := node[key]; ok {
				if resolved := stringField(child); resolved != "" {
					return resolved
				}
				if resolved := nestedString(child, keys...); resolved != "" {
					return resolved
				}
			}
		}
		for _, child := range node {
			if resolved := nestedString(child, keys...); resolved != "" {
				return resolved
			}
		}
	case []any:
		for _, child := range node {
			if resolved := nestedString(child, keys...); resolved != "" {
				return resolved
			}
		}
	}
	return ""
}

func nestedStringPath(value map[string]any, parts []string) string {
	current := any(value)
	for _, part := range parts {
		node, ok := current.(map[string]any)
		if !ok {
			return ""
		}
		current, ok = node[part]
		if !ok {
			return ""
		}
	}
	return stringField(current)
}

func stringField(value any) string {
	switch typed := value.(type) {
	case string:
		return strings.TrimSpace(typed)
	case json.Number:
		return typed.String()
	case float64:
		if typed == float64(int64(typed)) {
			return strconv.FormatInt(int64(typed), 10)
		}
		return strconv.FormatFloat(typed, 'f', -1, 64)
	case bool:
		return strconv.FormatBool(typed)
	default:
		return ""
	}
}

func parseTimestamp(value string) time.Time {
	value = strings.TrimSpace(value)
	if value == "" {
		return time.Time{}
	}
	if digits, err := strconv.ParseInt(value, 10, 64); err == nil {
		switch {
		case len(value) >= 13:
			return time.UnixMilli(digits).UTC()
		case len(value) >= 10:
			return time.Unix(digits, 0).UTC()
		}
	}
	layouts := []string{
		time.RFC3339Nano,
		time.RFC3339,
		time.DateTime,
		"2006-01-02 15:04:05.999999999",
		"2006-01-02 15:04:05",
		"2006-01-02",
	}
	for _, layout := range layouts {
		if parsed, err := time.Parse(layout, value); err == nil {
			return parsed.UTC()
		}
	}
	return time.Time{}
}

func unixMillisToRFC3339(value *int64) string {
	if value == nil || *value == 0 {
		return ""
	}
	if *value > 1_000_000_000_000 {
		return time.UnixMilli(*value).UTC().Format(time.RFC3339)
	}
	return time.Unix(*value, 0).UTC().Format(time.RFC3339)
}

func ptrValue(value *string) string {
	if value == nil {
		return ""
	}
	return strings.TrimSpace(*value)
}

func appendMissing(values []string, extra ...string) []string {
	if len(extra) == 0 {
		return values
	}
	seen := make(map[string]struct{}, len(values))
	for _, value := range values {
		seen[value] = struct{}{}
	}
	for _, value := range extra {
		value = strings.TrimSpace(value)
		if value == "" {
			continue
		}
		if _, ok := seen[value]; ok {
			continue
		}
		seen[value] = struct{}{}
		values = append(values, value)
	}
	return values
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		value = strings.TrimSpace(value)
		if value != "" {
			return value
		}
	}
	return ""
}

func badResponse(subject string, err error) error {
	return &requestError{
		StatusCode: 502,
		Code:       "bad_gateway",
		Message:    fmt.Sprintf("invalid upstream response from %s", subject),
		Err:        err,
	}
}

func invalidArgument(message string) error {
	return &requestError{StatusCode: 400, Code: "invalid_argument", Message: message}
}

func notFoundError(message string) error {
	return &requestError{StatusCode: 404, Code: "not_found", Message: message}
}

func unavailableError(message string) error {
	return &requestError{StatusCode: 503, Code: "unavailable", Message: message}
}

func mapReplyError(code, message string) error {
	code = strings.TrimSpace(code)
	message = firstNonEmpty(message, "upstream request failed")
	switch code {
	case "invalid_argument":
		return &requestError{StatusCode: 400, Code: code, Message: message}
	case "unauthenticated":
		return &requestError{StatusCode: 401, Code: code, Message: message}
	case "permission_denied":
		return &requestError{StatusCode: 403, Code: code, Message: message}
	case "not_found":
		return &requestError{StatusCode: 404, Code: code, Message: message}
	case "conflict", "already_exists":
		return &requestError{StatusCode: 409, Code: code, Message: message}
	case "unavailable":
		return &requestError{StatusCode: 503, Code: code, Message: message}
	default:
		return &requestError{StatusCode: 502, Code: firstNonEmpty(code, "bad_gateway"), Message: message}
	}
}

func enrollmentStatusFromAgent(agent envelope.AgentDetail) string {
	status := strings.ToLower(strings.TrimSpace(agent.Status))
	switch status {
	case "", "enrolled":
		if strings.TrimSpace(agent.AgentID) != "" {
			return "enrolled"
		}
		return "not_enrolled"
	case "pending", "enrolling":
		return "pending"
	default:
		if strings.TrimSpace(agent.AgentID) != "" {
			return "enrolled"
		}
		return "not_enrolled"
	}
}

func deploymentJobStatusLabel(value int32) string {
	switch value {
	case 1:
		return "queued"
	case 2:
		return "running"
	case 3:
		return "partial_success"
	case 4:
		return "succeeded"
	case 5:
		return "failed"
	case 6:
		return "cancelled"
	default:
		return "unknown"
	}
}

func deploymentTargetStatusLabel(value int32) string {
	switch value {
	case 1:
		return "pending"
	case 2:
		return "running"
	case 3:
		return "succeeded"
	case 4:
		return "failed"
	case 5:
		return "cancelled"
	default:
		return "unknown"
	}
}

func deploymentStepStatusLabel(value int32) string {
	switch value {
	case 1:
		return "pending"
	case 2:
		return "running"
	case 3:
		return "succeeded"
	case 4:
		return "failed"
	case 5:
		return "skipped"
	default:
		return "unknown"
	}
}

func deploymentExecutorKindLabel(value int32) string {
	switch value {
	case 1:
		return "mock"
	case 2:
		return "ansible"
	default:
		return "unknown"
	}
}
