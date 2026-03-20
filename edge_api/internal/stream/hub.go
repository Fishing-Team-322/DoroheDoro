package stream

import (
	"strings"
	"sync"

	"github.com/example/dorohedoro/internal/model"
)

type Filter struct {
	Host     string
	Service  string
	Severity string
}

type subscriber struct {
	ch     chan model.Event
	filter Filter
}

type Hub struct {
	mu          sync.RWMutex
	subscribers map[*subscriber]struct{}
	bufferSize  int
}

func NewHub(bufferSize int) *Hub {
	return &Hub{subscribers: make(map[*subscriber]struct{}), bufferSize: bufferSize}
}

func (h *Hub) Subscribe(filter Filter) (<-chan model.Event, func()) {
	sub := &subscriber{ch: make(chan model.Event, h.bufferSize), filter: filter}
	h.mu.Lock()
	h.subscribers[sub] = struct{}{}
	h.mu.Unlock()
	return sub.ch, func() {
		h.mu.Lock()
		if _, ok := h.subscribers[sub]; ok {
			delete(h.subscribers, sub)
			close(sub.ch)
		}
		h.mu.Unlock()
	}
}

func (h *Hub) Broadcast(event model.Event) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	for sub := range h.subscribers {
		if !matches(sub.filter, event) {
			continue
		}
		select {
		case sub.ch <- event:
		default:
		}
	}
}

func matches(filter Filter, event model.Event) bool {
	if filter.Host != "" && !strings.EqualFold(filter.Host, event.Host) {
		return false
	}
	if filter.Service != "" && !strings.EqualFold(filter.Service, event.Service) {
		return false
	}
	if filter.Severity != "" && !strings.EqualFold(filter.Severity, event.Severity) {
		return false
	}
	return true
}
