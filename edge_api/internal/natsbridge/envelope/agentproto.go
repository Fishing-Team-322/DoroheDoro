package envelope

import (
	"fmt"
	"sort"

	"google.golang.org/protobuf/encoding/protowire"
)

type AgentReplyEnvelope struct {
	Status        string
	Code          string
	Message       string
	Payload       []byte
	CorrelationID string
}

type EnrollRequest struct {
	CorrelationID   string
	BootstrapToken  string
	Hostname        string
	Version         string
	Metadata        map[string]string
	ExistingAgentID string
	TLSIdentity     string
}

type EnrollResponse struct {
	AgentID           string
	PolicyID          string
	PolicyRevision    string
	PolicyBodyJSON    string
	Status            string
	RespondedAtUnixMs int64
}

type FetchPolicyRequest struct {
	CorrelationID string
	AgentID       string
}

type FetchPolicyResponse struct {
	AgentID           string
	PolicyID          string
	PolicyRevision    string
	PolicyBodyJSON    string
	Status            string
	RespondedAtUnixMs int64
}

type IssueBootstrapTokenRequest struct {
	CorrelationID    string
	PolicyID         string
	PolicyRevisionID string
	RequestedBy      string
	ExpiresAtUnixMs  int64
}

type IssueBootstrapTokenResponse struct {
	TokenID          string
	BootstrapToken   string
	PolicyID         string
	PolicyRevisionID string
	ExpiresAtUnixMs  int64
	CreatedAtUnixMs  int64
}

type ListAgentsRequest struct {
	CorrelationID string
}

type AgentPolicyBinding struct {
	PolicyID          string
	PolicyRevisionID  string
	PolicyRevision    string
	AssignedAt        string
	PolicyName        string
	PolicyDescription string
}

type AgentSummary struct {
	AgentID         string
	Hostname        string
	Version         string
	Status          string
	LastSeenAt      string
	EffectivePolicy *AgentPolicyBinding
}

type ListAgentsResponse struct {
	Agents []AgentSummary
}

type AgentDetail struct {
	AgentID         string
	Hostname        string
	Version         string
	Status          string
	Metadata        map[string]string
	FirstSeenAt     string
	LastSeenAt      string
	EffectivePolicy *AgentPolicyBinding
}

type GetAgentRequest struct {
	CorrelationID string
	AgentID       string
}

type GetAgentDiagnosticsRequest struct {
	CorrelationID string
	AgentID       string
}

type DiagnosticsSnapshot struct {
	AgentID     string
	PayloadJSON string
	CreatedAt   string
}

type GetAgentPolicyRequest struct {
	CorrelationID string
	AgentID       string
}

type GetAgentPolicyResponse struct {
	Policy *AgentPolicyBinding
}

type HeartbeatPayload struct {
	AgentID      string
	Hostname     string
	Version      string
	Status       string
	HostMetadata map[string]string
	SentAtUnixMs int64
}

type DiagnosticsPayload struct {
	AgentID      string
	PayloadJSON  string
	SentAtUnixMs int64
}

func EncodeAgentReplyEnvelope(envelope AgentReplyEnvelope) []byte {
	var out []byte
	out = appendStringField(out, 1, envelope.Status)
	out = appendStringField(out, 2, envelope.Code)
	out = appendStringField(out, 3, envelope.Message)
	out = appendBytesField(out, 4, envelope.Payload)
	out = appendStringField(out, 5, envelope.CorrelationID)
	return out
}

func DecodeAgentReplyEnvelope(data []byte) (AgentReplyEnvelope, error) {
	var out AgentReplyEnvelope
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Status = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Code = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Message = decoded
		case 4:
			decoded, err := consumeBytes(kind, value)
			if err != nil {
				return err
			}
			out.Payload = decoded
		case 5:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.CorrelationID = decoded
		}
		return nil
	})
	return out, err
}

func EncodeEnrollRequest(request EnrollRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.BootstrapToken)
	out = appendStringField(out, 3, request.Hostname)
	out = appendStringField(out, 4, request.Version)
	out = appendStringMapField(out, 5, request.Metadata)
	out = appendStringField(out, 6, request.ExistingAgentID)
	out = appendStringField(out, 7, request.TLSIdentity)
	return out
}

func DecodeEnrollResponse(data []byte) (EnrollResponse, error) {
	var out EnrollResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.AgentID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyID = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyRevision = decoded
		case 4:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyBodyJSON = decoded
		case 5:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Status = decoded
		case 6:
			decoded, err := consumeInt64(kind, value)
			if err != nil {
				return err
			}
			out.RespondedAtUnixMs = decoded
		}
		return nil
	})
	return out, err
}

func EncodeFetchPolicyRequest(request FetchPolicyRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.AgentID)
	return out
}

func EncodeIssueBootstrapTokenRequest(request IssueBootstrapTokenRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.PolicyID)
	out = appendStringField(out, 3, request.PolicyRevisionID)
	out = appendStringField(out, 4, request.RequestedBy)
	out = appendInt64Field(out, 5, request.ExpiresAtUnixMs)
	return out
}

func EncodeListAgentsRequest(request ListAgentsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	return out
}

func EncodeGetAgentRequest(request GetAgentRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.AgentID)
	return out
}

func EncodeGetAgentDiagnosticsRequest(request GetAgentDiagnosticsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.AgentID)
	return out
}

func EncodeGetAgentPolicyRequest(request GetAgentPolicyRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.AgentID)
	return out
}

func DecodeFetchPolicyResponse(data []byte) (FetchPolicyResponse, error) {
	var out FetchPolicyResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.AgentID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyID = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyRevision = decoded
		case 4:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyBodyJSON = decoded
		case 5:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Status = decoded
		case 6:
			decoded, err := consumeInt64(kind, value)
			if err != nil {
				return err
			}
			out.RespondedAtUnixMs = decoded
		}
		return nil
	})
	return out, err
}

func DecodeIssueBootstrapTokenResponse(data []byte) (IssueBootstrapTokenResponse, error) {
	var out IssueBootstrapTokenResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.TokenID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.BootstrapToken = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyID = decoded
		case 4:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyRevisionID = decoded
		case 5:
			decoded, err := consumeInt64(kind, value)
			if err != nil {
				return err
			}
			out.ExpiresAtUnixMs = decoded
		case 6:
			decoded, err := consumeInt64(kind, value)
			if err != nil {
				return err
			}
			out.CreatedAtUnixMs = decoded
		}
		return nil
	})
	return out, err
}

func DecodeAgentPolicyBinding(data []byte) (AgentPolicyBinding, error) {
	var out AgentPolicyBinding
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyRevisionID = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyRevision = decoded
		case 4:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.AssignedAt = decoded
		case 5:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyName = decoded
		case 6:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PolicyDescription = decoded
		}
		return nil
	})
	return out, err
}

func DecodeAgentSummary(data []byte) (AgentSummary, error) {
	var out AgentSummary
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.AgentID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Hostname = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Version = decoded
		case 4:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Status = decoded
		case 5:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.LastSeenAt = decoded
		case 6:
			decoded, err := consumeBytes(kind, value)
			if err != nil {
				return err
			}
			binding, err := DecodeAgentPolicyBinding(decoded)
			if err != nil {
				return err
			}
			out.EffectivePolicy = &binding
		}
		return nil
	})
	return out, err
}

func DecodeListAgentsResponse(data []byte) (ListAgentsResponse, error) {
	var out ListAgentsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		if num != 1 {
			return nil
		}
		decoded, err := consumeBytes(kind, value)
		if err != nil {
			return err
		}
		agent, err := DecodeAgentSummary(decoded)
		if err != nil {
			return err
		}
		out.Agents = append(out.Agents, agent)
		return nil
	})
	return out, err
}

func DecodeAgentDetail(data []byte) (AgentDetail, error) {
	var out AgentDetail
	out.Metadata = map[string]string{}
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.AgentID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Hostname = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Version = decoded
		case 4:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.Status = decoded
		case 5:
			decoded, err := consumeBytes(kind, value)
			if err != nil {
				return err
			}
			key, mapValue, err := decodeStringMapEntry(decoded)
			if err != nil {
				return err
			}
			out.Metadata[key] = mapValue
		case 6:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.FirstSeenAt = decoded
		case 7:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.LastSeenAt = decoded
		case 8:
			decoded, err := consumeBytes(kind, value)
			if err != nil {
				return err
			}
			binding, err := DecodeAgentPolicyBinding(decoded)
			if err != nil {
				return err
			}
			out.EffectivePolicy = &binding
		}
		return nil
	})
	return out, err
}

func DecodeDiagnosticsSnapshot(data []byte) (DiagnosticsSnapshot, error) {
	var out DiagnosticsSnapshot
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.AgentID = decoded
		case 2:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.PayloadJSON = decoded
		case 3:
			decoded, err := consumeString(kind, value)
			if err != nil {
				return err
			}
			out.CreatedAt = decoded
		}
		return nil
	})
	return out, err
}

func DecodeGetAgentPolicyResponse(data []byte) (GetAgentPolicyResponse, error) {
	var out GetAgentPolicyResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, value []byte) error {
		if num != 1 {
			return nil
		}
		decoded, err := consumeBytes(kind, value)
		if err != nil {
			return err
		}
		binding, err := DecodeAgentPolicyBinding(decoded)
		if err != nil {
			return err
		}
		out.Policy = &binding
		return nil
	})
	return out, err
}

func EncodeHeartbeatPayload(payload HeartbeatPayload) []byte {
	var out []byte
	out = appendStringField(out, 1, payload.AgentID)
	out = appendStringField(out, 2, payload.Hostname)
	out = appendStringField(out, 3, payload.Version)
	out = appendStringField(out, 4, payload.Status)
	out = appendStringMapField(out, 5, payload.HostMetadata)
	out = appendInt64Field(out, 6, payload.SentAtUnixMs)
	return out
}

func EncodeDiagnosticsPayload(payload DiagnosticsPayload) []byte {
	var out []byte
	out = appendStringField(out, 1, payload.AgentID)
	out = appendStringField(out, 2, payload.PayloadJSON)
	out = appendInt64Field(out, 3, payload.SentAtUnixMs)
	return out
}

func appendStringField(dst []byte, num protowire.Number, value string) []byte {
	if value == "" {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.BytesType)
	return protowire.AppendString(dst, value)
}

func appendBytesField(dst []byte, num protowire.Number, value []byte) []byte {
	if len(value) == 0 {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.BytesType)
	return protowire.AppendBytes(dst, value)
}

func appendInt64Field(dst []byte, num protowire.Number, value int64) []byte {
	if value == 0 {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.VarintType)
	return protowire.AppendVarint(dst, uint64(value))
}

func appendStringMapField(dst []byte, num protowire.Number, values map[string]string) []byte {
	if len(values) == 0 {
		return dst
	}
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	for _, key := range keys {
		var entry []byte
		entry = appendStringField(entry, 1, key)
		entry = appendStringField(entry, 2, values[key])
		dst = appendBytesField(dst, num, entry)
	}
	return dst
}

func walkFields(data []byte, handle func(num protowire.Number, kind protowire.Type, value []byte) error) error {
	for len(data) > 0 {
		num, kind, tagLen := protowire.ConsumeTag(data)
		if tagLen < 0 {
			return fmt.Errorf("consume tag: %w", protowire.ParseError(tagLen))
		}
		data = data[tagLen:]
		valueLen := protowire.ConsumeFieldValue(num, kind, data)
		if valueLen < 0 {
			return fmt.Errorf("consume field %d: %w", num, protowire.ParseError(valueLen))
		}
		if err := handle(num, kind, data[:valueLen]); err != nil {
			return err
		}
		data = data[valueLen:]
	}
	return nil
}

func consumeString(kind protowire.Type, value []byte) (string, error) {
	if kind != protowire.BytesType {
		return "", fmt.Errorf("expected bytes field, got %v", kind)
	}
	decoded, n := protowire.ConsumeString(value)
	if n < 0 {
		return "", protowire.ParseError(n)
	}
	return decoded, nil
}

func consumeBytes(kind protowire.Type, value []byte) ([]byte, error) {
	if kind != protowire.BytesType {
		return nil, fmt.Errorf("expected bytes field, got %v", kind)
	}
	decoded, n := protowire.ConsumeBytes(value)
	if n < 0 {
		return nil, protowire.ParseError(n)
	}
	return decoded, nil
}

func consumeInt64(kind protowire.Type, value []byte) (int64, error) {
	if kind != protowire.VarintType {
		return 0, fmt.Errorf("expected varint field, got %v", kind)
	}
	decoded, n := protowire.ConsumeVarint(value)
	if n < 0 {
		return 0, protowire.ParseError(n)
	}
	return int64(decoded), nil
}
