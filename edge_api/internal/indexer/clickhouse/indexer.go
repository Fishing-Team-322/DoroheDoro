//go:build legacy

package clickhouse

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"sort"
	"strconv"
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

type AnalyticsQueryParams struct {
	From  string
	To    string
	Limit int
}

type HistogramBucket struct {
	Bucket string `json:"bucket"`
	Count  uint64 `json:"count"`
}

type CountRow struct {
	Key   string `json:"key"`
	Count uint64 `json:"count"`
}

type Indexer struct {
	baseURL       string
	database      string
	table         string
	httpClient    *http.Client
	logger        *zap.Logger
	flushSize     int
	flushInterval time.Duration

	mu          sync.Mutex
	pending     []pendingMessage
	flushTicker *time.Ticker
	closed      chan struct{}
}

func New(ctx context.Context, cfg config.ClickHouseConfig, logger *zap.Logger) (*Indexer, error) {
	baseURL := strings.TrimRight(cfg.DSN, "/")
	if strings.HasPrefix(baseURL, "clickhouse://") {
		baseURL = strings.Replace(baseURL, "clickhouse://", "http://", 1)
	}
	idx := &Indexer{
		baseURL:       baseURL,
		database:      cfg.Database,
		table:         cfg.Table,
		httpClient:    &http.Client{Timeout: 20 * time.Second},
		logger:        logger,
		flushSize:     cfg.FlushSize,
		flushInterval: cfg.FlushInterval,
		closed:        make(chan struct{}),
	}
	if idx.flushSize <= 0 {
		idx.flushSize = 200
	}
	if idx.flushInterval <= 0 {
		idx.flushInterval = 2 * time.Second
	}
	if err := idx.ensureSchema(ctx); err != nil {
		return nil, err
	}
	idx.flushTicker = time.NewTicker(idx.flushInterval)
	go idx.flushLoop()
	return idx, nil
}

func (i *Indexer) ensureSchema(ctx context.Context) error {
	stmts := []string{
		fmt.Sprintf("CREATE DATABASE IF NOT EXISTS %s", i.database),
		fmt.Sprintf(`CREATE TABLE IF NOT EXISTS %s.%s (
			timestamp DateTime64(3, 'UTC'),
			id String,
			host String,
			agent_id String,
			source_type String,
			source String,
			service String,
			severity String,
			message String,
			fingerprint String,
			labels_json String,
			fields_json String
		) ENGINE = MergeTree ORDER BY (timestamp, host, service, severity, id)`, i.database, i.table),
	}
	for _, stmt := range stmts {
		if _, err := i.execSQL(ctx, stmt, nil); err != nil {
			return err
		}
	}
	return nil
}

func (i *Indexer) HandleNATS(ctx context.Context, msg *nats.Msg) {
	var event model.Event
	if err := json.Unmarshal(msg.Data, &event); err != nil {
		i.logger.Error("failed to decode analytics message", zap.Error(err))
		_ = msg.Term()
		return
	}
	if err := i.enqueue(ctx, msg, event); err != nil {
		i.logger.Error("failed to enqueue analytics event", zap.Error(err), zap.String("event_id", event.ID))
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
	if err := i.insertBatch(ctx, batch); err != nil {
		i.mu.Lock()
		i.pending = append(batch, i.pending...)
		i.mu.Unlock()
		return err
	}
	return nil
}

func (i *Indexer) insertBatch(ctx context.Context, batch []pendingMessage) error {
	var payload bytes.Buffer
	for _, item := range batch {
		labelsJSON, _ := json.Marshal(item.event.Labels)
		fieldsJSON, _ := json.Marshal(item.event.Fields)
		row := map[string]any{
			"timestamp":   item.event.Timestamp.UTC().Format("2006-01-02 15:04:05.000"),
			"id":          item.event.ID,
			"host":        item.event.Host,
			"agent_id":    item.event.AgentID,
			"source_type": item.event.SourceType,
			"source":      item.event.Source,
			"service":     item.event.Service,
			"severity":    item.event.Severity,
			"message":     item.event.Message,
			"fingerprint": item.event.Fingerprint,
			"labels_json": string(labelsJSON),
			"fields_json": string(fieldsJSON),
		}
		encoded, _ := json.Marshal(row)
		payload.Write(encoded)
		payload.WriteByte('\n')
	}
	insertSQL := fmt.Sprintf("INSERT INTO %s.%s FORMAT JSONEachRow", i.database, i.table)
	if _, err := i.execSQL(ctx, insertSQL, &payload); err != nil {
		return err
	}
	for _, item := range batch {
		_ = item.msg.Ack()
	}
	return nil
}

func (i *Indexer) Histogram(ctx context.Context, params AnalyticsQueryParams) ([]HistogramBucket, error) {
	query := fmt.Sprintf(`SELECT formatDateTime(toStartOfMinute(timestamp), '%%Y-%%m-%%dT%%H:%%M:00Z') AS bucket, count() AS count FROM %s.%s %s GROUP BY bucket ORDER BY bucket ASC FORMAT JSONEachRow`, i.database, i.table, whereClause(params))
	rows, err := i.queryRows(ctx, query)
	if err != nil {
		return nil, err
	}
	out := make([]HistogramBucket, 0, len(rows))
	for _, row := range rows {
		out = append(out, HistogramBucket{Bucket: fmt.Sprint(row["bucket"]), Count: toUint64(row["count"])})
	}
	return out, nil
}

func (i *Indexer) Severity(ctx context.Context, params AnalyticsQueryParams) ([]CountRow, error) {
	query := fmt.Sprintf(`SELECT severity AS key, count() AS count FROM %s.%s %s GROUP BY severity ORDER BY count DESC FORMAT JSONEachRow`, i.database, i.table, whereClause(params))
	return i.countQuery(ctx, query)
}

func (i *Indexer) TopHosts(ctx context.Context, params AnalyticsQueryParams) ([]CountRow, error) {
	query := fmt.Sprintf(`SELECT host AS key, count() AS count FROM %s.%s %s GROUP BY host ORDER BY count DESC LIMIT %d FORMAT JSONEachRow`, i.database, i.table, whereClause(params), normalizeLimit(params.Limit, 10))
	return i.countQuery(ctx, query)
}

func (i *Indexer) TopServices(ctx context.Context, params AnalyticsQueryParams) ([]CountRow, error) {
	query := fmt.Sprintf(`SELECT service AS key, count() AS count FROM %s.%s %s GROUP BY service ORDER BY count DESC LIMIT %d FORMAT JSONEachRow`, i.database, i.table, whereClause(params), normalizeLimit(params.Limit, 10))
	return i.countQuery(ctx, query)
}

func (i *Indexer) countQuery(ctx context.Context, query string) ([]CountRow, error) {
	rows, err := i.queryRows(ctx, query)
	if err != nil {
		return nil, err
	}
	out := make([]CountRow, 0, len(rows))
	for _, row := range rows {
		out = append(out, CountRow{Key: fmt.Sprint(row["key"]), Count: toUint64(row["count"])})
	}
	sort.Slice(out, func(a, b int) bool { return out[a].Count > out[b].Count })
	return out, nil
}

func (i *Indexer) queryRows(ctx context.Context, query string) ([]map[string]any, error) {
	data, err := i.execSQL(ctx, query, nil)
	if err != nil {
		return nil, err
	}
	trimmed := strings.TrimSpace(string(data))
	if trimmed == "" {
		return nil, nil
	}
	lines := strings.Split(trimmed, "\n")
	rows := make([]map[string]any, 0, len(lines))
	for _, line := range lines {
		if strings.TrimSpace(line) == "" {
			continue
		}
		var row map[string]any
		if err := json.Unmarshal([]byte(line), &row); err != nil {
			return nil, err
		}
		rows = append(rows, row)
	}
	return rows, nil
}

func (i *Indexer) execSQL(ctx context.Context, query string, body io.Reader) ([]byte, error) {
	var reqBody bytes.Buffer
	if _, err := reqBody.WriteString(query); err != nil {
		return nil, err
	}
	if body != nil {
		data, err := io.ReadAll(body)
		if err != nil {
			return nil, err
		}
		if len(data) > 0 {
			reqBody.WriteByte('\n')
			reqBody.Write(data)
		}
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, i.baseURL+"/?database="+url.QueryEscape(i.database), &reqBody)
	if err != nil {
		return nil, err
	}
	resp, err := i.httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	payload, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("clickhouse status=%d body=%s", resp.StatusCode, string(payload))
	}
	return payload, nil
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
				i.logger.Error("clickhouse flush failed", zap.Error(err))
			}
		}
	}
}

func whereClause(params AnalyticsQueryParams) string {
	filters := make([]string, 0, 2)
	if from, ok := parseAnalyticsTime(params.From); ok {
		filters = append(filters, fmt.Sprintf("timestamp >= toDateTime64('%s', 3, 'UTC')", from))
	}
	if to, ok := parseAnalyticsTime(params.To); ok {
		filters = append(filters, fmt.Sprintf("timestamp <= toDateTime64('%s', 3, 'UTC')", to))
	}
	if len(filters) == 0 {
		return ""
	}
	return "WHERE " + strings.Join(filters, " AND ")
}

func parseAnalyticsTime(value string) (string, bool) {
	if strings.TrimSpace(value) == "" {
		return "", false
	}
	if epochMillis, err := strconv.ParseInt(value, 10, 64); err == nil {
		return time.UnixMilli(epochMillis).UTC().Format("2006-01-02 15:04:05.000"), true
	}
	if ts, err := time.Parse(time.RFC3339, value); err == nil {
		return ts.UTC().Format("2006-01-02 15:04:05.000"), true
	}
	return "", false
}

func normalizeLimit(limit, fallback int) int {
	if limit <= 0 || limit > 100 {
		return fallback
	}
	return limit
}

func toUint64(v any) uint64 {
	switch vv := v.(type) {
	case float64:
		return uint64(vv)
	case int:
		return uint64(vv)
	case uint64:
		return vv
	case json.Number:
		n, _ := vv.Int64()
		return uint64(n)
	default:
		return 0
	}
}
