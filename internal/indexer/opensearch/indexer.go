package opensearch

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/model"
)

type Indexer struct {
	baseURL     string
	indexPrefix string
	username    string
	password    string
	httpClient  *http.Client
	logger      *zap.Logger
}

func New(cfg config.OpenSearchConfig, logger *zap.Logger) (*Indexer, error) {
	return &Indexer{
		baseURL:     strings.TrimRight(cfg.URL, "/"),
		indexPrefix: cfg.IndexPrefix,
		username:    cfg.Username,
		password:    cfg.Password,
		httpClient:  &http.Client{Timeout: 10 * time.Second},
		logger:      logger,
	}, nil
}

func (i *Indexer) IndexDocument(ctx context.Context, event model.Event) error {
	index := i.indexName(event.Timestamp)
	if err := i.ensureIndex(ctx, index); err != nil {
		return err
	}
	var body bytes.Buffer
	meta, _ := json.Marshal(map[string]any{"index": map[string]any{"_index": index, "_id": event.ID}})
	doc, _ := json.Marshal(event)
	body.Write(meta)
	body.WriteByte('\n')
	body.Write(doc)
	body.WriteByte('\n')
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
	return nil
}

func (i *Indexer) HandleNATS(ctx context.Context, msg *nats.Msg) {
	var event model.Event
	if err := json.Unmarshal(msg.Data, &event); err != nil {
		i.logger.Error("failed to decode message", zap.Error(err))
		_ = msg.Term()
		return
	}
	if err := i.IndexDocument(ctx, event); err != nil {
		i.logger.Error("failed to index event", zap.Error(err), zap.String("event_id", event.ID))
		_ = msg.Nak()
		return
	}
	_ = msg.Ack()
}

func (i *Indexer) ensureIndex(ctx context.Context, index string) error {
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
	if resp.StatusCode == http.StatusOK {
		return nil
	}
	mapping := strings.NewReader(`{"settings":{"index":{"number_of_shards":1,"number_of_replicas":0}},"mappings":{"properties":{"timestamp":{"type":"date"}}}}`)
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
