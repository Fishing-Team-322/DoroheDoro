package httpapi

import (
	"encoding/json"
	"net/http"
	"strconv"
	"strings"

	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

func parsePagingRequest(r *http.Request) (envelope.PagingRequest, error) {
	var paging envelope.PagingRequest

	if raw := strings.TrimSpace(r.URL.Query().Get("limit")); raw != "" {
		value, err := strconv.ParseUint(raw, 10, 32)
		if err != nil {
			return paging, httpError("limit must be an unsigned integer")
		}
		paging.Limit = uint32(value)
	}
	if raw := strings.TrimSpace(r.URL.Query().Get("offset")); raw != "" {
		value, err := strconv.ParseUint(raw, 10, 64)
		if err != nil {
			return paging, httpError("offset must be an unsigned integer")
		}
		paging.Offset = value
	}
	paging.Query = strings.TrimSpace(r.URL.Query().Get("query"))
	return paging, nil
}

func parseOptionalBoolQuery(r *http.Request, name string) (bool, error) {
	raw := strings.TrimSpace(r.URL.Query().Get(name))
	if raw == "" {
		return false, nil
	}
	value, err := strconv.ParseBool(raw)
	if err != nil {
		return false, httpError(name + " must be a boolean")
	}
	return value, nil
}

func controlAuditContextFromRequest(r *http.Request, reason string) envelope.AuditContext {
	ac := auth.Context(r.Context())
	actorID := strings.TrimSpace(ac.Subject)
	if actorID == "" || actorID == "anonymous" {
		actorID = firstNonEmpty(strings.TrimSpace(ac.AgentID), "web-user")
	}
	actorType := "user"
	if strings.TrimSpace(ac.AgentID) != "" || strings.EqualFold(strings.TrimSpace(ac.Role), "agent") {
		actorType = "agent"
	}
	return envelope.AuditContext{
		ActorID:   actorID,
		ActorType: actorType,
		RequestID: middleware.GetRequestID(r.Context()),
		Reason:    strings.TrimSpace(reason),
	}
}

func marshalOptionalRawJSON(raw json.RawMessage) (string, error) {
	if len(raw) == 0 {
		return "", nil
	}
	var value any
	if err := json.Unmarshal(raw, &value); err != nil {
		return "", err
	}
	encoded, err := json.Marshal(value)
	if err != nil {
		return "", err
	}
	return string(encoded), nil
}
