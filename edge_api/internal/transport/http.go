package transport

type DeploymentCreateRequest struct {
	PolicyID string            `json:"policy_id"`
	AgentIDs []string          `json:"agent_ids,omitempty"`
	Params   map[string]string `json:"params,omitempty"`
}

type LogSearchRequest struct {
	Query    string            `json:"query"`
	From     string            `json:"from,omitempty"`
	To       string            `json:"to,omitempty"`
	AgentID  string            `json:"agent_id,omitempty"`
	Host     string            `json:"host,omitempty"`
	Service  string            `json:"service,omitempty"`
	Severity string            `json:"severity,omitempty"`
	Limit    int               `json:"limit,omitempty"`
	Cursor   string            `json:"cursor,omitempty"`
	Filters  map[string]string `json:"filters,omitempty"`
}
