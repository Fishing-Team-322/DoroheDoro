package config

import (
	"fmt"
	"os"
	"strconv"
	"time"
)

type Config struct {
	LogLevel   string
	HTTP       HTTPConfig
	GRPC       GRPCConfig
	NATS       NATSConfig
	OpenSearch OpenSearchConfig
	ClickHouse ClickHouseConfig
	Stream     StreamConfig
	Enrollment EnrollmentConfig
	Policy     PolicyConfig
	Ingest     IngestConfig
}

type HTTPConfig struct{ ListenAddr string }
type GRPCConfig struct{ ListenAddr string }

type NATSConfig struct {
	URL               string
	StreamName        string
	Subject           string
	IndexerConsumer   string
	AnalyticsConsumer string
}

type OpenSearchConfig struct {
	URL           string
	IndexPrefix   string
	Username      string
	Password      string
	FlushSize     int
	FlushInterval time.Duration
	IndexCacheTTL time.Duration
	ContextWindow time.Duration
	ContextBefore int
	ContextAfter  int
}

type ClickHouseConfig struct {
	Enabled       bool
	DSN           string
	Database      string
	Table         string
	FlushSize     int
	FlushInterval time.Duration
}

type StreamConfig struct{ BufferSize int }

type EnrollmentConfig struct {
	DevBootstrapToken string
	MTLSEnabled       bool
	TLSMode           string
}

type PolicyConfig struct {
	DefaultRevision   string
	DefaultBatchSize  int
	DefaultBatchWait  time.Duration
	DefaultSourceType string
}

type IngestConfig struct {
	AllowUnknownAgents bool
}

func Load() (Config, error) {
	cfg := Config{
		LogLevel: env("LOG_LEVEL", "info"),
		HTTP:     HTTPConfig{ListenAddr: env("HTTP_LISTEN_ADDR", ":8080")},
		GRPC:     GRPCConfig{ListenAddr: env("GRPC_LISTEN_ADDR", ":9090")},
		NATS: NATSConfig{
			URL:               env("NATS_URL", "nats://localhost:4222"),
			StreamName:        env("NATS_STREAM_NAME", "LOGS"),
			Subject:           env("NATS_SUBJECT", "logs.normalized"),
			IndexerConsumer:   env("NATS_INDEXER_CONSUMER", "logs-opensearch-indexer"),
			AnalyticsConsumer: env("NATS_ANALYTICS_CONSUMER", "logs-clickhouse-indexer"),
		},
		OpenSearch: OpenSearchConfig{
			URL:           env("OPENSEARCH_URL", "http://localhost:9200"),
			IndexPrefix:   env("OPENSEARCH_INDEX_PREFIX", "logs"),
			Username:      os.Getenv("OPENSEARCH_USERNAME"),
			Password:      os.Getenv("OPENSEARCH_PASSWORD"),
			FlushSize:     envInt("OPENSEARCH_BULK_FLUSH_SIZE", 200),
			FlushInterval: ParseDuration(env("OPENSEARCH_BULK_FLUSH_INTERVAL", "2s"), 2*time.Second),
			IndexCacheTTL: ParseDuration(env("OPENSEARCH_INDEX_CACHE_TTL", "10m"), 10*time.Minute),
			ContextWindow: ParseDuration(env("OPENSEARCH_CONTEXT_WINDOW", "15m"), 15*time.Minute),
			ContextBefore: envInt("OPENSEARCH_CONTEXT_BEFORE", 10),
			ContextAfter:  envInt("OPENSEARCH_CONTEXT_AFTER", 10),
		},
		ClickHouse: ClickHouseConfig{
			Enabled:       envBool("CLICKHOUSE_ENABLED", false),
			DSN:           env("CLICKHOUSE_DSN", "http://localhost:8123"),
			Database:      env("CLICKHOUSE_DATABASE", "default"),
			Table:         env("CLICKHOUSE_TABLE", "logs_analytics"),
			FlushSize:     envInt("CLICKHOUSE_FLUSH_SIZE", 200),
			FlushInterval: ParseDuration(env("CLICKHOUSE_FLUSH_INTERVAL", "2s"), 2*time.Second),
		},
		Stream: StreamConfig{BufferSize: envInt("WS_BUFFER_SIZE", 256)},
		Enrollment: EnrollmentConfig{
			DevBootstrapToken: env("ENROLLMENT_TOKEN", "dev-bootstrap-token"),
			MTLSEnabled:       envBool("INGEST_MTLS_ENABLED", false),
			TLSMode:           env("INGEST_TLS_MODE", "disabled"),
		},
		Policy: PolicyConfig{
			DefaultRevision:   env("DEFAULT_POLICY_REVISION", "rev-1"),
			DefaultBatchSize:  envInt("DEFAULT_POLICY_BATCH_SIZE", 100),
			DefaultBatchWait:  ParseDuration(env("DEFAULT_POLICY_BATCH_WAIT", "5s"), 5*time.Second),
			DefaultSourceType: env("DEFAULT_POLICY_SOURCE_TYPE", "file"),
		},
		Ingest: IngestConfig{AllowUnknownAgents: envBool("INGEST_ALLOW_UNKNOWN_AGENTS", true)},
	}

	if cfg.Stream.BufferSize <= 0 {
		return Config{}, fmt.Errorf("WS_BUFFER_SIZE must be positive")
	}
	if cfg.OpenSearch.FlushSize <= 0 {
		cfg.OpenSearch.FlushSize = 200
	}
	if cfg.ClickHouse.FlushSize <= 0 {
		cfg.ClickHouse.FlushSize = 200
	}
	if cfg.Policy.DefaultBatchSize <= 0 {
		cfg.Policy.DefaultBatchSize = 100
	}
	if cfg.OpenSearch.ContextBefore < 0 {
		cfg.OpenSearch.ContextBefore = 10
	}
	if cfg.OpenSearch.ContextAfter < 0 {
		cfg.OpenSearch.ContextAfter = 10
	}
	return cfg, nil
}

func env(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}

func envBool(key string, fallback bool) bool {
	value := os.Getenv(key)
	if value == "" {
		return fallback
	}
	parsed, err := strconv.ParseBool(value)
	if err != nil {
		return fallback
	}
	return parsed
}

func envInt(key string, fallback int) int {
	value := os.Getenv(key)
	if value == "" {
		return fallback
	}
	parsed, err := strconv.Atoi(value)
	if err != nil {
		return fallback
	}
	return parsed
}

func ParseDuration(value string, fallback time.Duration) time.Duration {
	if value == "" {
		return fallback
	}
	d, err := time.ParseDuration(value)
	if err != nil {
		return fallback
	}
	return d
}
