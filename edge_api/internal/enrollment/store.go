//go:build legacy

package enrollment

import (
	"fmt"
	"sync"
	"time"

	"github.com/google/uuid"

	"github.com/example/dorohedoro/internal/policy"
)

type Agent struct {
	AgentID        string            `json:"agent_id"`
	Host           string            `json:"host"`
	Metadata       map[string]string `json:"metadata,omitempty"`
	EnrolledAt     time.Time         `json:"enrolled_at"`
	PolicyRevision string            `json:"policy_revision"`
}

type EnrollRequest struct {
	BootstrapToken string            `json:"bootstrap_token"`
	Host           string            `json:"host"`
	Metadata       map[string]string `json:"metadata,omitempty"`
}

type EnrollResponse struct {
	AgentID       string        `json:"agent_id"`
	Policy        policy.Policy `json:"policy"`
	Ingest        IngestConfig  `json:"ingest"`
	MTLSTODO      string        `json:"mtls_todo,omitempty"`
	EnrolledAt    time.Time     `json:"enrolled_at"`
	BootstrapMode string        `json:"bootstrap_mode"`
}

type IngestConfig struct {
	GRPCAddress string `json:"grpc_address"`
	TLSMode     string `json:"tls_mode"`
}

type Store struct {
	mu             sync.RWMutex
	bootstrapToken string
	agents         map[string]Agent
}

func NewStore(bootstrapToken string) *Store {
	return &Store{bootstrapToken: bootstrapToken, agents: make(map[string]Agent)}
}

func (s *Store) Enroll(req EnrollRequest, assigned policy.Policy, grpcAddr, tlsMode string) (EnrollResponse, error) {
	if req.BootstrapToken != s.bootstrapToken {
		return EnrollResponse{}, fmt.Errorf("invalid bootstrap token")
	}
	now := time.Now().UTC()
	agent := Agent{
		AgentID:        "agent-" + uuid.NewString(),
		Host:           defaultHost(req.Host),
		Metadata:       cloneMap(req.Metadata),
		EnrolledAt:     now,
		PolicyRevision: assigned.Revision,
	}
	s.mu.Lock()
	s.agents[agent.AgentID] = agent
	s.mu.Unlock()
	return EnrollResponse{
		AgentID:       agent.AgentID,
		Policy:        assigned,
		Ingest:        IngestConfig{GRPCAddress: grpcAddr, TLSMode: tlsMode},
		MTLSTODO:      "dev mode uses insecure gRPC today; add mTLS cert bootstrap and verification before production",
		EnrolledAt:    now,
		BootstrapMode: "dev-token",
	}, nil
}

func (s *Store) Get(agentID string) (Agent, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	agent, ok := s.agents[agentID]
	if !ok {
		return Agent{}, false
	}
	agent.Metadata = cloneMap(agent.Metadata)
	return agent, true
}

func (s *Store) Upsert(agent Agent) {
	s.mu.Lock()
	defer s.mu.Unlock()
	agent.Metadata = cloneMap(agent.Metadata)
	s.agents[agent.AgentID] = agent
}

func defaultHost(v string) string {
	if v == "" {
		return "unknown-host"
	}
	return v
}

func cloneMap(src map[string]string) map[string]string {
	if len(src) == 0 {
		return nil
	}
	out := make(map[string]string, len(src))
	for k, v := range src {
		out[k] = v
	}
	return out
}
