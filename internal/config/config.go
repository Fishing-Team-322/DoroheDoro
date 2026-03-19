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
}

type HTTPConfig struct{ ListenAddr string }
type GRPCConfig struct{ ListenAddr string }

type NATSConfig struct {
	URL             string
	StreamName      string
	Subject         string
	IndexerConsumer string
}

type OpenSearchConfig struct {
	URL         string
	IndexPrefix string
	Username    string
	Password    string
}

type ClickHouseConfig struct {
	Enabled  bool
	DSN      string
	Database string
	Table    string
}

type StreamConfig struct{ BufferSize int }

func Load() (Config, error) {
	cfg := Config{
		LogLevel: env("LOG_LEVEL", "info"),
		HTTP:     HTTPConfig{ListenAddr: env("HTTP_LISTEN_ADDR", ":8080")},
		GRPC:     GRPCConfig{ListenAddr: env("GRPC_LISTEN_ADDR", ":9090")},
		NATS: NATSConfig{
			URL:             env("NATS_URL", "nats://localhost:4222"),
			StreamName:      env("NATS_STREAM_NAME", "LOGS"),
			Subject:         env("NATS_SUBJECT", "logs.normalized"),
			IndexerConsumer: env("NATS_INDEXER_CONSUMER", "logs-opensearch-indexer"),
		},
		OpenSearch: OpenSearchConfig{
			URL:         env("OPENSEARCH_URL", "http://localhost:9200"),
			IndexPrefix: env("OPENSEARCH_INDEX_PREFIX", "logs"),
			Username:    os.Getenv("OPENSEARCH_USERNAME"),
			Password:    os.Getenv("OPENSEARCH_PASSWORD"),
		},
		ClickHouse: ClickHouseConfig{
			Enabled:  envBool("CLICKHOUSE_ENABLED", false),
			DSN:      env("CLICKHOUSE_DSN", "clickhouse://default:@localhost:9000/default"),
			Database: env("CLICKHOUSE_DATABASE", "default"),
			Table:    env("CLICKHOUSE_TABLE", "logs_analytics"),
		},
		Stream: StreamConfig{BufferSize: envInt("WS_BUFFER_SIZE", 256)},
	}

	if cfg.Stream.BufferSize <= 0 {
		return Config{}, fmt.Errorf("WS_BUFFER_SIZE must be positive")
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
