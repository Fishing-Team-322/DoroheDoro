package query

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/example/dorohedoro/internal/model"
)

type Searcher struct {
	baseURL     string
	indexPrefix string
	username    string
	password    string
	httpClient  *http.Client
}

type SearchParams struct {
	Query    string
	From     string
	To       string
	Host     string
	Service  string
	Severity string
	Limit    int
	Offset   int
}

type SearchResult struct {
	Items  []model.Event `json:"items"`
	Total  int           `json:"total"`
	TookMS int           `json:"took_ms"`
}

func NewSearcher(baseURL, indexPrefix, username, password string) *Searcher {
	return &Searcher{baseURL: strings.TrimRight(baseURL, "/"), indexPrefix: indexPrefix, username: username, password: password, httpClient: &http.Client{Timeout: 10 * time.Second}}
}

func (s *Searcher) Search(ctx context.Context, params SearchParams) (SearchResult, error) {
	return s.search(ctx, buildQuery(params))
}

func (s *Searcher) GetContext(ctx context.Context, id string) (SearchResult, error) {
	return s.search(ctx, map[string]any{
		"size": 10,
		"sort": []any{map[string]any{"timestamp": map[string]any{"order": "asc"}}},
		"query": map[string]any{"bool": map[string]any{"should": []any{
			map[string]any{"term": map[string]any{"id.keyword": id}},
			map[string]any{"term": map[string]any{"id": id}},
		}, "minimum_should_match": 1}},
	})
}

func (s *Searcher) search(ctx context.Context, query map[string]any) (SearchResult, error) {
	payload, err := json.Marshal(query)
	if err != nil {
		return SearchResult{}, err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, fmt.Sprintf("%s/%s/_search", s.baseURL, s.indexPattern()), bytes.NewReader(payload))
	if err != nil {
		return SearchResult{}, err
	}
	req.Header.Set("Content-Type", "application/json")
	s.applyAuth(req)
	resp, err := s.httpClient.Do(req)
	if err != nil {
		return SearchResult{}, err
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		b, _ := io.ReadAll(resp.Body)
		return SearchResult{}, fmt.Errorf("opensearch search status=%d body=%s", resp.StatusCode, string(b))
	}
	var raw struct {
		Took int `json:"took"`
		Hits struct {
			Total struct {
				Value int `json:"value"`
			} `json:"total"`
			Hits []struct {
				Source model.Event `json:"_source"`
			} `json:"hits"`
		} `json:"hits"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&raw); err != nil {
		return SearchResult{}, err
	}
	items := make([]model.Event, 0, len(raw.Hits.Hits))
	for _, hit := range raw.Hits.Hits {
		items = append(items, hit.Source)
	}
	return SearchResult{Items: items, Total: raw.Hits.Total.Value, TookMS: raw.Took}, nil
}

func buildQuery(params SearchParams) map[string]any {
	must := make([]any, 0, 1)
	filter := make([]any, 0, 4)
	if strings.TrimSpace(params.Query) != "" {
		must = append(must, map[string]any{"simple_query_string": map[string]any{"query": params.Query, "fields": []string{"message^3", "raw", "service", "host"}}})
	} else {
		must = append(must, map[string]any{"match_all": map[string]any{}})
	}
	if params.Host != "" {
		filter = append(filter, termFilter("host.keyword", params.Host, "host", params.Host))
	}
	if params.Service != "" {
		filter = append(filter, termFilter("service.keyword", params.Service, "service", params.Service))
	}
	if params.Severity != "" {
		filter = append(filter, termFilter("severity.keyword", params.Severity, "severity", params.Severity))
	}
	if params.From != "" || params.To != "" {
		rangeQuery := map[string]any{}
		if ts, ok := parseTime(params.From); ok {
			rangeQuery["gte"] = ts
		}
		if ts, ok := parseTime(params.To); ok {
			rangeQuery["lte"] = ts
		}
		filter = append(filter, map[string]any{"range": map[string]any{"timestamp": rangeQuery}})
	}
	limit := params.Limit
	if limit <= 0 || limit > 500 {
		limit = 100
	}
	if params.Offset < 0 {
		params.Offset = 0
	}
	return map[string]any{"from": params.Offset, "size": limit, "sort": []any{map[string]any{"timestamp": map[string]any{"order": "desc"}}}, "query": map[string]any{"bool": map[string]any{"must": must, "filter": filter}}}
}

func termFilter(keywordField, value, fallbackField, fallbackValue string) map[string]any {
	return map[string]any{"bool": map[string]any{"should": []any{
		map[string]any{"term": map[string]any{keywordField: value}},
		map[string]any{"term": map[string]any{fallbackField: fallbackValue}},
	}, "minimum_should_match": 1}}
}

func parseTime(value string) (string, bool) {
	if strings.TrimSpace(value) == "" {
		return "", false
	}
	if epochMillis, err := strconv.ParseInt(value, 10, 64); err == nil {
		return time.UnixMilli(epochMillis).UTC().Format(time.RFC3339), true
	}
	if parsed, err := time.Parse(time.RFC3339, value); err == nil {
		return parsed.UTC().Format(time.RFC3339), true
	}
	return "", false
}

func (s *Searcher) indexPattern() string { return fmt.Sprintf("%s-*", s.indexPrefix) }

func (s *Searcher) Ping(ctx context.Context) bool {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, s.baseURL, nil)
	if err != nil {
		return false
	}
	s.applyAuth(req)
	resp, err := s.httpClient.Do(req)
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode < http.StatusBadRequest
}

func (s *Searcher) applyAuth(req *http.Request) {
	if s.username != "" {
		req.SetBasicAuth(s.username, s.password)
	}
}
