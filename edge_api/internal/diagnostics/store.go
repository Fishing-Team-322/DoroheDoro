//go:build legacy

package diagnostics

import (
	"sort"
	"sync"
	"time"
)

type AgentStatus struct {
	AgentID        string    `json:"agent_id"`
	Host           string    `json:"host"`
	EnrolledAt     time.Time `json:"enrolled_at"`
	LastSeen       time.Time `json:"last_seen"`
	PolicyRevision string    `json:"policy_revision"`
	SentBatches    int64     `json:"sent_batches"`
	AcceptedEvents int64     `json:"accepted_events"`
	RejectedEvents int64     `json:"rejected_events"`
	LastError      string    `json:"last_error,omitempty"`
	Health         string    `json:"health"`
	Status         string    `json:"status"`
}

type Store struct {
	mu     sync.RWMutex
	agents map[string]AgentStatus
}

func NewStore() *Store {
	return &Store{agents: make(map[string]AgentStatus)}
}

func (s *Store) EnsureAgent(agentID, host, policyRevision string, enrolledAt time.Time) AgentStatus {
	s.mu.Lock()
	defer s.mu.Unlock()
	st, ok := s.agents[agentID]
	if !ok {
		st = AgentStatus{AgentID: agentID, EnrolledAt: enrolledAt, Status: "enrolled", Health: "healthy"}
	}
	if host != "" {
		st.Host = host
	}
	if policyRevision != "" {
		st.PolicyRevision = policyRevision
	}
	if !enrolledAt.IsZero() && st.EnrolledAt.IsZero() {
		st.EnrolledAt = enrolledAt
	}
	if st.Health == "" {
		st.Health = "healthy"
	}
	if st.Status == "" {
		st.Status = "enrolled"
	}
	s.agents[agentID] = st
	return st
}

func (s *Store) RecordIngest(agentID, host, policyRevision string, accepted, rejected int, lastErr string) AgentStatus {
	now := time.Now().UTC()
	s.mu.Lock()
	defer s.mu.Unlock()
	st, ok := s.agents[agentID]
	if !ok {
		st = AgentStatus{AgentID: agentID, EnrolledAt: now}
	}
	if host != "" {
		st.Host = host
	}
	if policyRevision != "" {
		st.PolicyRevision = policyRevision
	}
	if st.EnrolledAt.IsZero() {
		st.EnrolledAt = now
	}
	st.LastSeen = now
	st.SentBatches++
	st.AcceptedEvents += int64(accepted)
	st.RejectedEvents += int64(rejected)
	st.LastError = lastErr
	if lastErr != "" {
		st.Health = "degraded"
		st.Status = "error"
	} else {
		st.Health = "healthy"
		st.Status = "active"
	}
	s.agents[agentID] = st
	return st
}

func (s *Store) List() []AgentStatus {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]AgentStatus, 0, len(s.agents))
	for _, st := range s.agents {
		out = append(out, st)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].AgentID < out[j].AgentID })
	return out
}

func (s *Store) Get(agentID string) (AgentStatus, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	st, ok := s.agents[agentID]
	return st, ok
}
