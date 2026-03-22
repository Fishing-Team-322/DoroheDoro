package httpapi

import (
	"net/http"
	"strings"

	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

type bootstrapTokenCreateRequest struct {
	PolicyID         string `json:"policy_id"`
	PolicyRevisionID string `json:"policy_revision_id"`
	RequestedBy      string `json:"requested_by"`
	ExpiresAtUnixMs  int64  `json:"expires_at_unix_ms"`
}

func agentBootstrapTokenCreateHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var body bootstrapTokenCreateRequest
		if err := decodeJSONBody(r, &body); err != nil {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", err.Error())
			return
		}
		if strings.TrimSpace(body.PolicyID) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy_id is required")
			return
		}
		if strings.TrimSpace(body.PolicyRevisionID) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "policy_revision_id is required")
			return
		}
		if strings.TrimSpace(body.RequestedBy) == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "requested_by is required")
			return
		}
		if body.ExpiresAtUnixMs == 0 {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "expires_at_unix_ms is required")
			return
		}

		subject := deps.Config.NATS.Subjects.AgentsBootstrapTokenIssue
		payload, reply, err := requestAgentEnvelope(
			r.Context(),
			deps.Bridge,
			deps.Logger,
			subject,
			envelope.EncodeIssueBootstrapTokenRequest(envelope.IssueBootstrapTokenRequest{
				CorrelationID:    middleware.GetRequestID(r.Context()),
				PolicyID:         strings.TrimSpace(body.PolicyID),
				PolicyRevisionID: strings.TrimSpace(body.PolicyRevisionID),
				RequestedBy:      strings.TrimSpace(body.RequestedBy),
				ExpiresAtUnixMs:  body.ExpiresAtUnixMs,
			}),
			envelope.DecodeIssueBootstrapTokenResponse,
		)
		if err != nil {
			middleware.WriteTransportError(w, r, err)
			return
		}
		if strings.EqualFold(reply.Status, "error") {
			writeAgentReplyError(w, r, reply)
			return
		}

		w.Header().Set("X-NATS-Subject", subject)
		middleware.WriteJSON(w, http.StatusCreated, map[string]any{
			"item": map[string]any{
				"token_id":           payload.TokenID,
				"bootstrap_token":    payload.BootstrapToken,
				"policy_id":          payload.PolicyID,
				"policy_revision_id": payload.PolicyRevisionID,
				"expires_at_unix_ms": payload.ExpiresAtUnixMs,
				"created_at_unix_ms": payload.CreatedAtUnixMs,
			},
			"request_id": firstNonEmpty(reply.CorrelationID, middleware.GetRequestID(r.Context())),
		})
	}
}
