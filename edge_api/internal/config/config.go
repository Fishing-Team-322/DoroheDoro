package config

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"
)

type Config struct {
	ServiceName string
	LogLevel    string
	HTTP        HTTPConfig
	GRPC        GRPCConfig
	NATS        NATSConfig
	Timeouts    TimeoutConfig
	Limits      LimitsConfig
	Auth        AuthConfig
	Stream      StreamConfig
}

type HTTPConfig struct {
	ListenAddr         string
	TLSCert            string
	TLSKey             string
	CORSAllowedOrigins []string
}

type GRPCConfig struct {
	ListenAddr   string
	TLSCert      string
	TLSKey       string
	MTLSEnabled  bool
	ClientCA     string
	MaxRecvBytes int
	MaxSendBytes int
	Keepalive    time.Duration
}

type NATSConfig struct {
	URL            string
	RequestTimeout time.Duration
	Subjects       Subjects
}

type Subjects struct {
	AgentsEnrollRequest string
	AgentsPolicyFetch   string
	AgentsHeartbeat     string
	AgentsDiagnostics   string
	LogsIngestRaw       string
	DeploymentsCreate   string
	DeploymentsGet      string
	DeploymentsList     string
	LogsSearch          string
	LogsHistogram       string
	LogsSeverity        string
	LogsTopHosts        string
	LogsTopServices     string
	AgentsList          string
	AgentsGet           string
	AgentDiagnosticsGet string
	PoliciesList        string
	PoliciesGet         string
	UIStreamLogs        string
}

type TimeoutConfig struct {
	HTTP time.Duration
	GRPC time.Duration
}

type LimitsConfig struct {
	HTTPBodyBytes     int64
	RateLimitRPS      int
	RateLimitBurst    int
	AgentLogBatchSize int
}

type AuthConfig struct {
	HTTPStubEnabled     bool
	MTLSHookEnabled     bool
	SessionCookieName   string
	CSRFCookieName      string
	SessionCookieSecure bool
	SessionTTL          time.Duration
}

type StreamConfig struct {
	HeartbeatInterval time.Duration
	RetryInterval     time.Duration
}

func Load() (Config, error) {
	cfg := Config{
		ServiceName: env("SERVICE_NAME", "edge-api"),
		LogLevel:    env("LOG_LEVEL", "info"),
		HTTP: HTTPConfig{
			ListenAddr:         env("HTTP_LISTEN_ADDR", ":8080"),
			TLSCert:            os.Getenv("HTTP_TLS_CERT_FILE"),
			TLSKey:             os.Getenv("HTTP_TLS_KEY_FILE"),
			CORSAllowedOrigins: envCSV("CORS_ALLOWED_ORIGINS", []string{"http://localhost:3000"}),
		},
		GRPC: GRPCConfig{
			ListenAddr:   env("GRPC_LISTEN_ADDR", ":9090"),
			TLSCert:      os.Getenv("GRPC_TLS_CERT_FILE"),
			TLSKey:       os.Getenv("GRPC_TLS_KEY_FILE"),
			MTLSEnabled:  envBool("GRPC_MTLS_ENABLED", false),
			ClientCA:     os.Getenv("GRPC_CLIENT_CA_FILE"),
			MaxRecvBytes: envInt("GRPC_MAX_RECV_BYTES", 4<<20),
			MaxSendBytes: envInt("GRPC_MAX_SEND_BYTES", 4<<20),
			Keepalive:    parseDuration(env("GRPC_KEEPALIVE", "30s"), 30*time.Second),
		},
		NATS: NATSConfig{
			URL:            env("NATS_URL", "nats://localhost:4222"),
			RequestTimeout: parseDuration(env("NATS_REQUEST_TIMEOUT", "3s"), 3*time.Second),
			Subjects: Subjects{
				AgentsEnrollRequest: env("SUBJECT_AGENTS_ENROLL_REQUEST", "agents.enroll.request"),
				AgentsPolicyFetch:   env("SUBJECT_AGENTS_POLICY_FETCH", "agents.policy.fetch"),
				AgentsHeartbeat:     env("SUBJECT_AGENTS_HEARTBEAT", "agents.heartbeat"),
				AgentsDiagnostics:   env("SUBJECT_AGENTS_DIAGNOSTICS", "agents.diagnostics"),
				LogsIngestRaw:       env("SUBJECT_LOGS_INGEST_RAW", "logs.ingest.raw"),
				DeploymentsCreate:   env("SUBJECT_DEPLOYMENTS_CREATE", "deployments.jobs.create"),
				DeploymentsGet:      env("SUBJECT_DEPLOYMENTS_GET", "deployments.jobs.get"),
				DeploymentsList:     env("SUBJECT_DEPLOYMENTS_LIST", "deployments.jobs.list"),
				LogsSearch:          env("SUBJECT_QUERY_LOGS_SEARCH", "query.logs.search"),
				LogsHistogram:       env("SUBJECT_QUERY_LOGS_HISTOGRAM", "query.logs.histogram"),
				LogsSeverity:        env("SUBJECT_QUERY_LOGS_SEVERITY", "query.logs.severity"),
				LogsTopHosts:        env("SUBJECT_QUERY_LOGS_TOP_HOSTS", "query.logs.top_hosts"),
				LogsTopServices:     env("SUBJECT_QUERY_LOGS_TOP_SERVICES", "query.logs.top_services"),
				AgentsList:          env("SUBJECT_AGENTS_LIST", "agents.list"),
				AgentsGet:           env("SUBJECT_AGENTS_GET", "agents.get"),
				AgentDiagnosticsGet: env("SUBJECT_AGENTS_DIAGNOSTICS_GET", "agents.diagnostics.get"),
				PoliciesList:        env("SUBJECT_POLICIES_LIST", "policies.list"),
				PoliciesGet:         env("SUBJECT_POLICIES_GET", "policies.get"),
				UIStreamLogs:        env("SUBJECT_UI_STREAM_LOGS", "ui.stream.logs"),
			},
		},
		Timeouts: TimeoutConfig{
			HTTP: parseDuration(env("HTTP_REQUEST_TIMEOUT", "15s"), 15*time.Second),
			GRPC: parseDuration(env("GRPC_REQUEST_TIMEOUT", "15s"), 15*time.Second),
		},
		Limits: LimitsConfig{
			HTTPBodyBytes:     envInt64("HTTP_MAX_BODY_BYTES", 1<<20),
			RateLimitRPS:      envInt("RATE_LIMIT_RPS", 0),
			RateLimitBurst:    envInt("RATE_LIMIT_BURST", 0),
			AgentLogBatchSize: envInt("AGENT_LOG_BATCH_SIZE", 1000),
		},
		Auth: AuthConfig{
			HTTPStubEnabled:     envBool("HTTP_AUTH_STUB_ENABLED", true),
			MTLSHookEnabled:     envBool("GRPC_MTLS_HOOK_ENABLED", false),
			SessionCookieName:   env("SESSION_COOKIE_NAME", "session_token"),
			CSRFCookieName:      env("CSRF_COOKIE_NAME", "csrf_token"),
			SessionCookieSecure: envBool("SESSION_COOKIE_SECURE", false),
			SessionTTL:          parseDuration(env("SESSION_TTL", "12h"), 12*time.Hour),
		},
		Stream: StreamConfig{
			HeartbeatInterval: parseDuration(env("STREAM_HEARTBEAT_INTERVAL", "25s"), 25*time.Second),
			RetryInterval:     parseDuration(env("STREAM_RETRY_INTERVAL", "5s"), 5*time.Second),
		},
	}
	if cfg.HTTP.ListenAddr == "" || cfg.GRPC.ListenAddr == "" || cfg.NATS.URL == "" {
		return Config{}, fmt.Errorf("HTTP_LISTEN_ADDR, GRPC_LISTEN_ADDR and NATS_URL are required")
	}
	if cfg.Limits.HTTPBodyBytes <= 0 {
		cfg.Limits.HTTPBodyBytes = 1 << 20
	}
	if cfg.Limits.AgentLogBatchSize <= 0 {
		cfg.Limits.AgentLogBatchSize = 1000
	}
	return cfg, nil
}

func env(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func envBool(key string, fallback bool) bool {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	parsed, err := strconv.ParseBool(v)
	if err != nil {
		return fallback
	}
	return parsed
}

func envInt(key string, fallback int) int {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	parsed, err := strconv.Atoi(v)
	if err != nil {
		return fallback
	}
	return parsed
}

func envInt64(key string, fallback int64) int64 {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	parsed, err := strconv.ParseInt(v, 10, 64)
	if err != nil {
		return fallback
	}
	return parsed
}

func parseDuration(v string, fallback time.Duration) time.Duration {
	d, err := time.ParseDuration(v)
	if err != nil {
		return fallback
	}
	return d
}

func envCSV(key string, fallback []string) []string {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	parts := strings.Split(v, ",")
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if part != "" {
			result = append(result, part)
		}
	}
	if len(result) == 0 {
		return fallback
	}
	return result
}
