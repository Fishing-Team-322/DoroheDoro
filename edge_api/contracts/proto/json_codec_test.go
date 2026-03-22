package proto

import (
	"strings"
	"testing"
)

func TestJSONCodecMarshalPreservesFalseBoolFields(t *testing.T) {
	payload, err := JSONCodec{}.Marshal(FetchPolicyResponse{
		Policy: &PolicyPayload{
			PolicyId: "policy-1",
			Revision: "rev-1",
			BodyJSON: `{"sources":["/tmp/doro-agent-bootstrap.log"]}`,
		},
		Changed: false,
	})
	if err != nil {
		t.Fatalf("marshal fetch policy response: %v", err)
	}

	text := string(payload)
	if !strings.Contains(text, `"changed":false`) {
		t.Fatalf("expected changed=false in payload, got %s", text)
	}
}
