package model

type ErrorEnvelope struct {
	Error ErrorBody `json:"error"`
}

type ErrorBody struct {
	Code      string `json:"code"`
	Message   string `json:"message"`
	RequestID string `json:"request_id,omitempty"`
}

type AuthContext struct {
	Subject string `json:"subject"`
	Role    string `json:"role"`
	AgentID string `json:"agent_id,omitempty"`
}

type StreamEnvelope struct {
	Type    string `json:"type"`
	Subject string `json:"subject,omitempty"`
	Payload any    `json:"payload"`
}
