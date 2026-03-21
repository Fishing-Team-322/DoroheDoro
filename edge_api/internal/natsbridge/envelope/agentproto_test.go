package envelope

import "testing"

func TestAgentReplyEnvelopeRoundTrip(t *testing.T) {
	encoded := EncodeAgentReplyEnvelope(AgentReplyEnvelope{
		Status:        "ok",
		Code:          "ok",
		Message:       "",
		Payload:       []byte{1, 2, 3},
		CorrelationID: "corr-1",
	})

	decoded, err := DecodeAgentReplyEnvelope(encoded)
	if err != nil {
		t.Fatalf("decode envelope: %v", err)
	}

	if decoded.Status != "ok" || decoded.Code != "ok" || decoded.CorrelationID != "corr-1" {
		t.Fatalf("unexpected decoded envelope: %+v", decoded)
	}
	if len(decoded.Payload) != 3 {
		t.Fatalf("unexpected payload size: %d", len(decoded.Payload))
	}
}

func TestFetchPolicyResponseRoundTrip(t *testing.T) {
	payload := EncodeAgentReplyEnvelope(AgentReplyEnvelope{
		Status:        "ok",
		Code:          "ok",
		CorrelationID: "corr-2",
		Payload: func() []byte {
			return EncodeFetchPolicyResponseFixture(FetchPolicyResponse{
				AgentID:           "agent-1",
				PolicyID:          "policy-1",
				PolicyRevision:    "rev-1",
				PolicyBodyJSON:    "{\"sources\":[\"file\"]}",
				Status:            "assigned",
				RespondedAtUnixMs: 123,
			})
		}(),
	})

	envelope, err := DecodeAgentReplyEnvelope(payload)
	if err != nil {
		t.Fatalf("decode envelope: %v", err)
	}
	response, err := DecodeFetchPolicyResponse(envelope.Payload)
	if err != nil {
		t.Fatalf("decode fetch policy response: %v", err)
	}
	if response.AgentID != "agent-1" || response.PolicyRevision != "rev-1" {
		t.Fatalf("unexpected response: %+v", response)
	}
}

func EncodeFetchPolicyResponseFixture(response FetchPolicyResponse) []byte {
	var out []byte
	out = appendStringField(out, 1, response.AgentID)
	out = appendStringField(out, 2, response.PolicyID)
	out = appendStringField(out, 3, response.PolicyRevision)
	out = appendStringField(out, 4, response.PolicyBodyJSON)
	out = appendStringField(out, 5, response.Status)
	out = appendInt64Field(out, 6, response.RespondedAtUnixMs)
	return out
}
