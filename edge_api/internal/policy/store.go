package policy

import (
	"sync"
	"time"
)

type Policy struct {
	Revision   string            `json:"revision"`
	Sources    []string          `json:"sources"`
	Labels     map[string]string `json:"labels,omitempty"`
	BatchSize  int               `json:"batch_size"`
	BatchWait  string            `json:"batch_wait"`
	SourceType string            `json:"source_type"`
	UpdatedAt  time.Time         `json:"updated_at"`
}

type Store struct {
	mu       sync.RWMutex
	def      Policy
	agentMap map[string]Policy
}

func NewStore(def Policy) *Store {
	return &Store{def: def, agentMap: make(map[string]Policy)}
}

func (s *Store) Default() Policy {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return clonePolicy(s.def)
}

func (s *Store) Get(agentID string) Policy {
	s.mu.RLock()
	defer s.mu.RUnlock()
	if p, ok := s.agentMap[agentID]; ok {
		return clonePolicy(p)
	}
	return clonePolicy(s.def)
}

func (s *Store) Assign(agentID string, p Policy) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.agentMap[agentID] = clonePolicy(p)
}

func clonePolicy(in Policy) Policy {
	out := in
	if len(in.Sources) > 0 {
		out.Sources = append([]string(nil), in.Sources...)
	}
	if len(in.Labels) > 0 {
		out.Labels = make(map[string]string, len(in.Labels))
		for k, v := range in.Labels {
			out.Labels[k] = v
		}
	}
	return out
}
