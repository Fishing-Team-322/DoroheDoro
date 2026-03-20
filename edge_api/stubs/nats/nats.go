package nats

import (
	"context"
	"errors"
	"sync"
	"time"
)

type Status int

const CONNECTED Status = 1

type Header map[string][]string

func (h Header) Set(key, value string) { h[key] = []string{value} }

type Msg struct {
	Subject string
	Data    []byte
	Header  Header
	Reply   string
}

type Subscription struct {
	subject string
	ch      chan *Msg
	conn    *Conn
}

func (s *Subscription) Unsubscribe() error {
	s.conn.mu.Lock()
	defer s.conn.mu.Unlock()
	delete(s.conn.subscribers, s.subject)
	return nil
}

type Option func(*Conn)

func Name(string) Option                 { return func(*Conn) {} }
func ReconnectWait(time.Duration) Option { return func(*Conn) {} }

type Conn struct {
	mu          sync.RWMutex
	subscribers map[string]chan *Msg
	status      Status
}

func Connect(url string, opts ...Option) (*Conn, error) {
	c := &Conn{subscribers: map[string]chan *Msg{}, status: CONNECTED}
	for _, opt := range opts {
		opt(c)
	}
	return c, nil
}

func (c *Conn) Status() Status { return c.status }
func (c *Conn) Drain() error   { return nil }
func (c *Conn) Close()         {}

func (c *Conn) PublishMsg(msg *Msg) error {
	c.mu.RLock()
	ch := c.subscribers[msg.Subject]
	c.mu.RUnlock()
	if ch != nil {
		select {
		case ch <- msg:
		default:
		}
	}
	return nil
}

func (c *Conn) RequestMsgWithContext(ctx context.Context, msg *Msg) (*Msg, error) {
	// Stub transport: if someone published a canned response into reply subject it can be consumed later.
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}
	return &Msg{Subject: msg.Subject, Data: []byte(`{"accepted":true,"status":"stub","items":[],"request_id":"stub"}`), Header: Header{}}, nil
}

func (c *Conn) ChanSubscribe(subject string, ch chan *Msg) (*Subscription, error) {
	if ch == nil {
		return nil, errors.New("nil channel")
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	c.subscribers[subject] = ch
	return &Subscription{subject: subject, ch: ch, conn: c}, nil
}
