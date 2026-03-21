//go:build legacy

package opensearch

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/model"
)

type pendingMessage struct {
	msg   *nats.Msg
	event model.Event
}

type Indexer struct {
	baseURL       string
	indexPrefix   string
	username      string
	password      string
	httpClient    *http.Client
	logger        *zap.Logger
	flushSize     int
	flushInterval time.Duration
	cacheTTL      time.Duration

	mu          sync.Mutex
	pending     []pendingMessage
	knownIndex  map[string]time.Time
	flushTicker *time.Ticker
	closed      chan struct{}
}

func New(cfg config.OpenSearchConfig, logger *zap.Logger) (*Indexer, error) {
	idx := &Indexer{
		baseURL:       strings.TrimRight(cfg.URL, "/"),
		indexPrefix:   cfg.IndexPrefix,
		username:      cfg.Username,
		password:      cfg.Password,
		httpClient:    &http.Client{Timeout: 15 * time.Second},
		logger:        logger,
		flushSize:     cfg.FlushSize,
		flushInterval: cfg.FlushInterval,
		cacheTTL:      cfg.IndexCacheTTL,
		knownIndex:    make(map[string]time.Time),
		closed:        make(chan struct{}),
	}
	if idx.flushSize <= 0 {
		idx.flushSize = 200
	}
	if idx.flushInterval <= 0 {
		idx.flushInterval = 2 * time.Second
	}
	idx.flushTicker = time.NewTicker(idx.flushInterval)
	go idx.flushLoop()
	return idx, nil
}

func (i *Indexer) HandleNATS(ctx context.Context, msg *nats.Msg) {
	var event model.Event
	if err := json.Unmarshal(msg.Data, &event); err != nil {
		i.logger.Error("failed to decode message", zap.Error(err))
		_ = msg.Term()
		return
	}
	if err := i.enqueue(ctx, msg, event); err != nil {
		i.logger.Error("failed to enqueue event", zap.Error(err), zap.String("event_id", event.ID))
		_ = msg.Nak()
	}
}

func (i *Indexer) enqueue(ctx context.Context, msg *nats.Msg, event model.Event) error {
	i.mu.Lock()
	i.pending = append(i.pending, pendingMessage{msg: msg, event: event})
	shouldFlush := len(i.pending) >= i.flushSize
	i.mu.Unlock()
	if shouldFlush {
		return i.Flush(ctx)
	}
	return nil
}

func (i *Indexer) Flush(ctx context.Context) error {
	i.mu.Lock()
	batch := append([]pendingMessage(nil), i.pending...)
	i.pending = nil
	i.mu.Unlock()
	if len(batch) == 0 {
		return nil
	}
	if err := i.flushBatch(ctx, batch); err != nil {
		i.mu.Lock()
		i.pending = append(batch, i.pending...)
		i.mu.Unlock()
		return err
	}
	return nil
}

func (i *Indexer) flushBatch(ctx context.Context, batch []pendingMessage) error {
	indices := make(map[string]struct{})
	for _, item := range batch {
		indices[i.indexName(item.event.Timestamp)] = struct{}{}
	}
	for index := range indices {
		if err := i.ensureIndex(ctx, index); err != nil {
			return err
		}
	}
	var body bytes.Buffer
	for _, item := range batch {
		index := i.indexName(item.event.Timestamp)
		meta, _ := json.Marshal(map[string]any{"index": map[string]any{"_index": index, "_id": item.event.ID}})
		doc, _ := json.Marshal(item.event)
		body.Write(meta)
		body.WriteByte('\n')
		body.Write(doc)
		body.WriteByte('\n')
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, i.baseURL+"/_bulk", &body)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/x-ndjson")
	i.applyAuth(req)
	resp, err := i.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("bulk index request: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		payload, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("bulk index status=%d body=%s", resp.StatusCode, string(payload))
	}
	var raw struct {
		Errors bool `json:"errors"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&raw); err != nil && err != io.EOF {
		return fmt.Errorf("decode bulk response: %w", err)
	}
	if raw.Errors {
		return fmt.Errorf("bulk index returned item errors")
	}
	for _, item := range batch {
		_ = item.msg.Ack()
	}
	return nil
}

func (i *Indexer) ensureIndex(ctx context.Context, index string) error {
	now := time.Now()
	i.mu.Lock()
	if expiry, ok := i.knownIndex[index]; ok && now.Before(expiry) {
		i.mu.Unlock()
		return nil
	}
	i.mu.Unlock()

	req, err := http.NewRequestWithContext(ctx, http.MethodHead, i.baseURL+"/"+index, nil)
	if err != nil {
		return err
	}
	i.applyAuth(req)
	resp, err := i.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		mapping := strings.NewReader(`{"settings":{"index":{"number_of_shards":1,"number_of_replicas":0}},"mappings":{"properties":{"timestamp":{"type":"date"},"severity":{"type":"keyword"},"service":{"type":"keyword"},"host":{"type":"keyword"},"agent_id":{"type":"keyword"},"fingerprint":{"type":"keyword"}}}}`)
		createReq, err := http.NewRequestWithContext(ctx, http.MethodPut, i.baseURL+"/"+index, mapping)
		if err != nil {
			return err
		}
		createReq.Header.Set("Content-Type", "application/json")
		i.applyAuth(createReq)
		createResp, err := i.httpClient.Do(createReq)
		if err != nil {
			return err
		}
		defer createResp.Body.Close()
		if createResp.StatusCode >= 400 && createResp.StatusCode != http.StatusBadRequest {
			payload, _ := io.ReadAll(createResp.Body)
			return fmt.Errorf("create index status=%d body=%s", createResp.StatusCode, string(payload))
		}
	}
	i.mu.Lock()
	i.knownIndex[index] = now.Add(i.cacheTTL)
	i.mu.Unlock()
	return nil
}

func (i *Indexer) Ping(ctx context.Context) bool {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, i.baseURL, nil)
	if err != nil {
		return false
	}
	i.applyAuth(req)
	resp, err := i.httpClient.Do(req)
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode < http.StatusBadRequest
}

func (i *Indexer) Close(ctx context.Context) error {
	select {
	case <-i.closed:
		return nil
	default:
		close(i.closed)
	}
	if i.flushTicker != nil {
		i.flushTicker.Stop()
	}
	return i.Flush(ctx)
}

func (i *Indexer) flushLoop() {
	for {
		select {
		case <-i.closed:
			return
		case <-i.flushTicker.C:
			if err := i.Flush(context.Background()); err != nil {
				i.logger.Error("opensearch flush failed", zap.Error(err))
			}
		}
	}
}

func (i *Indexer) BaseURL() string     { return i.baseURL }
func (i *Indexer) IndexPrefix() string { return i.indexPrefix }

func (i *Indexer) applyAuth(req *http.Request) {
	if i.username != "" {
		req.SetBasicAuth(i.username, i.password)
	}
}

func (i *Indexer) indexName(ts time.Time) string {
	return fmt.Sprintf("%s-%s", i.indexPrefix, ts.UTC().Format("2006.01.02"))
}
