package model

import "time"

type Event struct {
	ID          string            `json:"id"`
	Timestamp   time.Time         `json:"timestamp"`
	Host        string            `json:"host"`
	AgentID     string            `json:"agent_id"`
	SourceType  string            `json:"source_type"`
	Source      string            `json:"source"`
	Service     string            `json:"service"`
	Severity    string            `json:"severity"`
	Message     string            `json:"message"`
	Fingerprint string            `json:"fingerprint"`
	Labels      map[string]string `json:"labels,omitempty"`
	Fields      map[string]any    `json:"fields,omitempty"`
	Raw         string            `json:"raw,omitempty"`
}
