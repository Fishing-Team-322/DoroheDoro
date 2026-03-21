package config

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/example/dorohedoro/internal/natsbridge/subjects"
)

type Config struct {
	ServiceName string
	Version     string
	LogLevel    string
	Public      PublicConfig
	HTTP        HTTPConfig
	GRPC        GRPCConfig
	NATS        NATSConfig
	Timeouts    TimeoutConfig
	Limits      LimitsConfig
	Auth        AuthConfig
	Stream      StreamConfig
}

type PublicConfig struct {
	BaseURL       string
	EdgeURL       string
	AgentGRPCAddr string
}

type HTTPConfig struct {
	ListenAddr         string
	TLSCert            string
	TLSKey             string
	CORSAllowedOrigins []string
}

type GRPCConfig struct {
	ListenAddr           string
	TLSCert              string
	TLSKey               string
	ClientCA             string
	MTLSEnabled          bool
	AllowInsecureDevMode bool
	MaxRecvBytes         int
	MaxSendBytes         int
	Keepalive            time.Duration
}

type NATSConfig struct {
	URL            string
	RequestTimeout time.Duration
	Subjects       subjects.Registry
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
	HTTPStubEnabled   bool
	SessionCookieName string
	CSRFCookieName    string
	CookieSecure      bool
	SessionTTL        time.Duration
	DevUser           DevAuthUser
}

type DevAuthUser struct {
	Login       string
	Email       string
	Password    string
	UserID      string
	Role        string
	DisplayName string
}

type StreamConfig struct {
	HeartbeatInterval time.Duration
	RetryInterval     time.Duration
}

func Load() (Config, error) {
	defaultSubjects := subjects.Defaults()
	devLogin := env("DEV_TEST_LOGIN", "admin")
	devEmail := env("DEV_TEST_EMAIL", "admin@example.com")

	cfg := Config{
		ServiceName: env("SERVICE_NAME", "edge-api"),
		Version:     env("SERVICE_VERSION", "dev"),
		LogLevel:    env("LOG_LEVEL", "info"),
		Public: PublicConfig{
			BaseURL:       env("PUBLIC_BASE_URL", "http://localhost:3000"),
			EdgeURL:       env("EDGE_PUBLIC_URL", "http://localhost:8080"),
			AgentGRPCAddr: env("AGENT_PUBLIC_GRPC_ADDR", "localhost:9090"),
		},
		HTTP: HTTPConfig{
			ListenAddr:         env("HTTP_LISTEN_ADDR", ":8080"),
			TLSCert:            os.Getenv("HTTP_TLS_CERT_FILE"),
			TLSKey:             os.Getenv("HTTP_TLS_KEY_FILE"),
			CORSAllowedOrigins: envCSV("CORS_ALLOWED_ORIGINS", []string{"http://localhost:3000"}),
		},
		GRPC: GRPCConfig{
			ListenAddr:           envAny([]string{"AGENT_GRPC_LISTEN_ADDR", "GRPC_LISTEN_ADDR"}, ":9090"),
			TLSCert:              envAny([]string{"AGENT_TLS_CERT_FILE", "GRPC_TLS_CERT_FILE"}, ""),
			TLSKey:               envAny([]string{"AGENT_TLS_KEY_FILE", "GRPC_TLS_KEY_FILE"}, ""),
			ClientCA:             envAny([]string{"AGENT_TLS_CLIENT_CA_FILE", "GRPC_CLIENT_CA_FILE"}, ""),
			MTLSEnabled:          envBoolAny([]string{"AGENT_MTLS_ENABLED", "GRPC_MTLS_ENABLED"}, false),
			AllowInsecureDevMode: envBool("AGENT_ALLOW_INSECURE_DEV_MODE", false),
			MaxRecvBytes:         envInt("GRPC_MAX_RECV_BYTES", 4<<20),
			MaxSendBytes:         envInt("GRPC_MAX_SEND_BYTES", 4<<20),
			Keepalive:            parseDuration(env("GRPC_KEEPALIVE", "30s"), 30*time.Second),
		},
		NATS: NATSConfig{
			URL:            env("NATS_URL", "nats://localhost:4222"),
			RequestTimeout: parseDuration(env("NATS_REQUEST_TIMEOUT", "3s"), 3*time.Second),
			Subjects: subjects.Registry{
				AgentsEnrollRequest:           env("SUBJECT_AGENTS_ENROLL_REQUEST", defaultSubjects.AgentsEnrollRequest),
				AgentsPolicyFetch:             env("SUBJECT_AGENTS_POLICY_FETCH", defaultSubjects.AgentsPolicyFetch),
				AgentsHeartbeat:               env("SUBJECT_AGENTS_HEARTBEAT", defaultSubjects.AgentsHeartbeat),
				AgentsDiagnostics:             env("SUBJECT_AGENTS_DIAGNOSTICS", defaultSubjects.AgentsDiagnostics),
				AgentsList:                    env("SUBJECT_AGENTS_LIST", defaultSubjects.AgentsList),
				AgentsGet:                     env("SUBJECT_AGENTS_GET", defaultSubjects.AgentsGet),
				AgentsDiagnosticsGet:          env("SUBJECT_AGENTS_DIAGNOSTICS_GET", defaultSubjects.AgentsDiagnosticsGet),
				AgentsPolicyGet:               env("SUBJECT_AGENTS_POLICY_GET", defaultSubjects.AgentsPolicyGet),
				ControlPoliciesList:           env("SUBJECT_CONTROL_POLICIES_LIST", defaultSubjects.ControlPoliciesList),
				ControlPoliciesGet:            env("SUBJECT_CONTROL_POLICIES_GET", defaultSubjects.ControlPoliciesGet),
				ControlPoliciesCreate:         env("SUBJECT_CONTROL_POLICIES_CREATE", defaultSubjects.ControlPoliciesCreate),
				ControlPoliciesUpdate:         env("SUBJECT_CONTROL_POLICIES_UPDATE", defaultSubjects.ControlPoliciesUpdate),
				ControlPoliciesRevisions:      env("SUBJECT_CONTROL_POLICIES_REVISIONS", defaultSubjects.ControlPoliciesRevisions),
				ControlHostsList:              env("SUBJECT_CONTROL_HOSTS_LIST", defaultSubjects.ControlHostsList),
				ControlHostsGet:               env("SUBJECT_CONTROL_HOSTS_GET", defaultSubjects.ControlHostsGet),
				ControlHostsCreate:            env("SUBJECT_CONTROL_HOSTS_CREATE", defaultSubjects.ControlHostsCreate),
				ControlHostsUpdate:            env("SUBJECT_CONTROL_HOSTS_UPDATE", defaultSubjects.ControlHostsUpdate),
				ControlHostGroupsList:         env("SUBJECT_CONTROL_HOST_GROUPS_LIST", defaultSubjects.ControlHostGroupsList),
				ControlHostGroupsGet:          env("SUBJECT_CONTROL_HOST_GROUPS_GET", defaultSubjects.ControlHostGroupsGet),
				ControlHostGroupsCreate:       env("SUBJECT_CONTROL_HOST_GROUPS_CREATE", defaultSubjects.ControlHostGroupsCreate),
				ControlHostGroupsUpdate:       env("SUBJECT_CONTROL_HOST_GROUPS_UPDATE", defaultSubjects.ControlHostGroupsUpdate),
				ControlHostGroupsAddMember:    env("SUBJECT_CONTROL_HOST_GROUPS_ADD_MEMBER", defaultSubjects.ControlHostGroupsAddMember),
				ControlHostGroupsRemoveMember: env("SUBJECT_CONTROL_HOST_GROUPS_REMOVE_MEMBER", defaultSubjects.ControlHostGroupsRemoveMember),
				ControlCredentialsList:        env("SUBJECT_CONTROL_CREDENTIALS_LIST", defaultSubjects.ControlCredentialsList),
				ControlCredentialsGet:         env("SUBJECT_CONTROL_CREDENTIALS_GET", defaultSubjects.ControlCredentialsGet),
				ControlCredentialsCreate:      env("SUBJECT_CONTROL_CREDENTIALS_CREATE", defaultSubjects.ControlCredentialsCreate),
				ControlClustersList:           env("SUBJECT_CONTROL_CLUSTERS_LIST", defaultSubjects.ControlClustersList),
				ControlClustersGet:            env("SUBJECT_CONTROL_CLUSTERS_GET", defaultSubjects.ControlClustersGet),
				ControlClustersCreate:         env("SUBJECT_CONTROL_CLUSTERS_CREATE", defaultSubjects.ControlClustersCreate),
				ControlClustersUpdate:         env("SUBJECT_CONTROL_CLUSTERS_UPDATE", defaultSubjects.ControlClustersUpdate),
				ControlClustersAddHost:        env("SUBJECT_CONTROL_CLUSTERS_ADD_HOST", defaultSubjects.ControlClustersAddHost),
				ControlClustersRemoveHost:     env("SUBJECT_CONTROL_CLUSTERS_REMOVE_HOST", defaultSubjects.ControlClustersRemoveHost),
				ControlRolesList:              env("SUBJECT_CONTROL_ROLES_LIST", defaultSubjects.ControlRolesList),
				ControlRolesGet:               env("SUBJECT_CONTROL_ROLES_GET", defaultSubjects.ControlRolesGet),
				ControlRolesCreate:            env("SUBJECT_CONTROL_ROLES_CREATE", defaultSubjects.ControlRolesCreate),
				ControlRolesUpdate:            env("SUBJECT_CONTROL_ROLES_UPDATE", defaultSubjects.ControlRolesUpdate),
				ControlRolesPermissionsGet:    env("SUBJECT_CONTROL_ROLES_PERMISSIONS_GET", defaultSubjects.ControlRolesPermissionsGet),
				ControlRolesPermissionsSet:    env("SUBJECT_CONTROL_ROLES_PERMISSIONS_SET", defaultSubjects.ControlRolesPermissionsSet),
				ControlRoleBindingsList:       env("SUBJECT_CONTROL_ROLE_BINDINGS_LIST", defaultSubjects.ControlRoleBindingsList),
				ControlRoleBindingsCreate:     env("SUBJECT_CONTROL_ROLE_BINDINGS_CREATE", defaultSubjects.ControlRoleBindingsCreate),
				ControlRoleBindingsDelete:     env("SUBJECT_CONTROL_ROLE_BINDINGS_DELETE", defaultSubjects.ControlRoleBindingsDelete),
				ControlIntegrationsList:       env("SUBJECT_CONTROL_INTEGRATIONS_LIST", defaultSubjects.ControlIntegrationsList),
				ControlIntegrationsGet:        env("SUBJECT_CONTROL_INTEGRATIONS_GET", defaultSubjects.ControlIntegrationsGet),
				ControlIntegrationsCreate:     env("SUBJECT_CONTROL_INTEGRATIONS_CREATE", defaultSubjects.ControlIntegrationsCreate),
				ControlIntegrationsUpdate:     env("SUBJECT_CONTROL_INTEGRATIONS_UPDATE", defaultSubjects.ControlIntegrationsUpdate),
				ControlIntegrationsBind:       env("SUBJECT_CONTROL_INTEGRATIONS_BIND", defaultSubjects.ControlIntegrationsBind),
				ControlIntegrationsUnbind:     env("SUBJECT_CONTROL_INTEGRATIONS_UNBIND", defaultSubjects.ControlIntegrationsUnbind),
				TicketsList:                   env("SUBJECT_TICKETS_LIST", defaultSubjects.TicketsList),
				TicketsGet:                    env("SUBJECT_TICKETS_GET", defaultSubjects.TicketsGet),
				TicketsCreate:                 env("SUBJECT_TICKETS_CREATE", defaultSubjects.TicketsCreate),
				TicketsAssign:                 env("SUBJECT_TICKETS_ASSIGN", defaultSubjects.TicketsAssign),
				TicketsUnassign:               env("SUBJECT_TICKETS_UNASSIGN", defaultSubjects.TicketsUnassign),
				TicketsCommentAdd:             env("SUBJECT_TICKETS_COMMENT_ADD", defaultSubjects.TicketsCommentAdd),
				TicketsStatusChange:           env("SUBJECT_TICKETS_STATUS_CHANGE", defaultSubjects.TicketsStatusChange),
				TicketsClose:                  env("SUBJECT_TICKETS_CLOSE", defaultSubjects.TicketsClose),
				AnomalyRulesList:              env("SUBJECT_ANOMALIES_RULES_LIST", defaultSubjects.AnomalyRulesList),
				AnomalyRulesGet:               env("SUBJECT_ANOMALIES_RULES_GET", defaultSubjects.AnomalyRulesGet),
				AnomalyRulesCreate:            env("SUBJECT_ANOMALIES_RULES_CREATE", defaultSubjects.AnomalyRulesCreate),
				AnomalyRulesUpdate:            env("SUBJECT_ANOMALIES_RULES_UPDATE", defaultSubjects.AnomalyRulesUpdate),
				AnomalyInstancesList:          env("SUBJECT_ANOMALIES_INSTANCES_LIST", defaultSubjects.AnomalyInstancesList),
				AnomalyInstancesGet:           env("SUBJECT_ANOMALIES_INSTANCES_GET", defaultSubjects.AnomalyInstancesGet),
				DeploymentsJobsCreate:         env("SUBJECT_DEPLOYMENTS_JOBS_CREATE", defaultSubjects.DeploymentsJobsCreate),
				DeploymentsJobsGet:            env("SUBJECT_DEPLOYMENTS_JOBS_GET", defaultSubjects.DeploymentsJobsGet),
				DeploymentsJobsList:           env("SUBJECT_DEPLOYMENTS_JOBS_LIST", defaultSubjects.DeploymentsJobsList),
				DeploymentsJobsRetry:          env("SUBJECT_DEPLOYMENTS_JOBS_RETRY", defaultSubjects.DeploymentsJobsRetry),
				DeploymentsJobsCancel:         env("SUBJECT_DEPLOYMENTS_JOBS_CANCEL", defaultSubjects.DeploymentsJobsCancel),
				DeploymentsJobsStatus:         env("SUBJECT_DEPLOYMENTS_JOBS_STATUS", defaultSubjects.DeploymentsJobsStatus),
				DeploymentsJobsStep:           env("SUBJECT_DEPLOYMENTS_JOBS_STEP", defaultSubjects.DeploymentsJobsStep),
				DeploymentsPlanCreate:         env("SUBJECT_DEPLOYMENTS_PLAN_CREATE", defaultSubjects.DeploymentsPlanCreate),
				QueryLogsSearch:               env("SUBJECT_QUERY_LOGS_SEARCH", defaultSubjects.QueryLogsSearch),
				QueryLogsGet:                  env("SUBJECT_QUERY_LOGS_GET", defaultSubjects.QueryLogsGet),
				QueryLogsContext:              env("SUBJECT_QUERY_LOGS_CONTEXT", defaultSubjects.QueryLogsContext),
				QueryLogsHistogram:            env("SUBJECT_QUERY_LOGS_HISTOGRAM", defaultSubjects.QueryLogsHistogram),
				QueryLogsSeverity:             env("SUBJECT_QUERY_LOGS_SEVERITY", defaultSubjects.QueryLogsSeverity),
				QueryLogsTopHosts:             env("SUBJECT_QUERY_LOGS_TOP_HOSTS", defaultSubjects.QueryLogsTopHosts),
				QueryLogsTopServices:          env("SUBJECT_QUERY_LOGS_TOP_SERVICES", defaultSubjects.QueryLogsTopServices),
				QueryLogsHeatmap:              env("SUBJECT_QUERY_LOGS_HEATMAP", defaultSubjects.QueryLogsHeatmap),
				QueryLogsTopPatterns:          env("SUBJECT_QUERY_LOGS_TOP_PATTERNS", defaultSubjects.QueryLogsTopPatterns),
				QueryLogsAnomalies:            env("SUBJECT_QUERY_LOGS_ANOMALIES", defaultSubjects.QueryLogsAnomalies),
				QueryDashboardsOverview:       env("SUBJECT_QUERY_DASHBOARDS_OVERVIEW", defaultSubjects.QueryDashboardsOverview),
				AlertsList:                    env("SUBJECT_ALERTS_LIST", defaultSubjects.AlertsList),
				AlertsGet:                     env("SUBJECT_ALERTS_GET", defaultSubjects.AlertsGet),
				AlertsRulesList:               env("SUBJECT_ALERTS_RULES_LIST", defaultSubjects.AlertsRulesList),
				AlertsRulesGet:                env("SUBJECT_ALERTS_RULES_GET", defaultSubjects.AlertsRulesGet),
				AlertsRulesCreate:             env("SUBJECT_ALERTS_RULES_CREATE", defaultSubjects.AlertsRulesCreate),
				AlertsRulesUpdate:             env("SUBJECT_ALERTS_RULES_UPDATE", defaultSubjects.AlertsRulesUpdate),
				AuditList:                     env("SUBJECT_AUDIT_LIST", defaultSubjects.AuditList),
				AuditEventsAppend:             env("SUBJECT_AUDIT_EVENTS_APPEND", defaultSubjects.AuditEventsAppend),
				LogsIngestRaw:                 env("SUBJECT_LOGS_INGEST_RAW", defaultSubjects.LogsIngestRaw),
				LogsIngestNormalized:          env("SUBJECT_LOGS_INGEST_NORMALIZED", defaultSubjects.LogsIngestNormalized),
				StreamLogs:                    env("SUBJECT_UI_STREAM_LOGS", defaultSubjects.StreamLogs),
				StreamDeployments:             env("SUBJECT_UI_STREAM_DEPLOYMENTS", defaultSubjects.StreamDeployments),
				StreamAlerts:                  env("SUBJECT_UI_STREAM_ALERTS", defaultSubjects.StreamAlerts),
				StreamAgents:                  env("SUBJECT_UI_STREAM_AGENTS", defaultSubjects.StreamAgents),
				StreamClusters:                env("SUBJECT_UI_STREAM_CLUSTERS", defaultSubjects.StreamClusters),
				StreamTickets:                 env("SUBJECT_UI_STREAM_TICKETS", defaultSubjects.StreamTickets),
				StreamAnomalies:               env("SUBJECT_UI_STREAM_ANOMALIES", defaultSubjects.StreamAnomalies),
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
			HTTPStubEnabled:   envBool("HTTP_AUTH_STUB_ENABLED", true),
			SessionCookieName: env("SESSION_COOKIE_NAME", "session_token"),
			CSRFCookieName:    env("CSRF_COOKIE_NAME", "csrf_token"),
			CookieSecure:      envBoolWithFallback(false, "COOKIE_SECURE", "SESSION_COOKIE_SECURE"),
			SessionTTL:        parseDuration(env("SESSION_TTL", "12h"), 12*time.Hour),
			DevUser: DevAuthUser{
				Login:       devLogin,
				Email:       devEmail,
				Password:    env("DEV_TEST_PASSWORD", "admin123"),
				UserID:      env("DEV_TEST_USER_ID", "dev-user-1"),
				Role:        env("DEV_TEST_ROLE", "admin"),
				DisplayName: env("DEV_TEST_DISPLAY_NAME", humanizeIdentifier(devLogin)),
			},
		},
		Stream: StreamConfig{
			HeartbeatInterval: parseDuration(env("STREAM_HEARTBEAT_INTERVAL", "25s"), 25*time.Second),
			RetryInterval:     parseDuration(env("STREAM_RETRY_INTERVAL", "5s"), 5*time.Second),
		},
	}

	if cfg.HTTP.ListenAddr == "" || cfg.GRPC.ListenAddr == "" || cfg.NATS.URL == "" {
		return Config{}, fmt.Errorf("HTTP_LISTEN_ADDR, AGENT_GRPC_LISTEN_ADDR and NATS_URL are required")
	}
	if cfg.Limits.HTTPBodyBytes <= 0 {
		cfg.Limits.HTTPBodyBytes = 1 << 20
	}
	if cfg.Limits.AgentLogBatchSize <= 0 {
		cfg.Limits.AgentLogBatchSize = 1000
	}
	if cfg.Auth.SessionTTL <= 0 {
		cfg.Auth.SessionTTL = 12 * time.Hour
	}
	if strings.TrimSpace(cfg.Auth.DevUser.Login) == "" {
		cfg.Auth.DevUser.Login = "admin"
	}
	if strings.TrimSpace(cfg.Auth.DevUser.Email) == "" {
		cfg.Auth.DevUser.Email = cfg.Auth.DevUser.Login + "@example.com"
	}
	if strings.TrimSpace(cfg.Auth.DevUser.DisplayName) == "" {
		cfg.Auth.DevUser.DisplayName = humanizeIdentifier(cfg.Auth.DevUser.Login)
	}
	if cfg.GRPC.MTLSEnabled {
		if cfg.GRPC.TLSCert == "" || cfg.GRPC.TLSKey == "" || cfg.GRPC.ClientCA == "" {
			return Config{}, fmt.Errorf("AGENT_MTLS_ENABLED requires AGENT_TLS_CERT_FILE, AGENT_TLS_KEY_FILE and AGENT_TLS_CLIENT_CA_FILE")
		}
	}
	if !cfg.GRPC.MTLSEnabled && !cfg.GRPC.AllowInsecureDevMode && (cfg.GRPC.TLSCert == "" || cfg.GRPC.TLSKey == "") {
		return Config{}, fmt.Errorf("agent gRPC transport requires TLS cert/key unless AGENT_ALLOW_INSECURE_DEV_MODE=true is set explicitly")
	}
	return cfg, nil
}

func env(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func envAny(keys []string, fallback string) string {
	for _, key := range keys {
		if v := strings.TrimSpace(os.Getenv(key)); v != "" {
			return v
		}
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

func envBoolAny(keys []string, fallback bool) bool {
	for _, key := range keys {
		v := os.Getenv(key)
		if v == "" {
			continue
		}
		parsed, err := strconv.ParseBool(v)
		if err == nil {
			return parsed
		}
	}
	return fallback
}

func envBoolWithFallback(fallback bool, keys ...string) bool {
	for _, key := range keys {
		v := os.Getenv(key)
		if v == "" {
			continue
		}
		parsed, err := strconv.ParseBool(v)
		if err == nil {
			return parsed
		}
	}
	return fallback
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

func humanizeIdentifier(value string) string {
	parts := strings.Fields(strings.NewReplacer(".", " ", "-", " ", "_", " ").Replace(strings.TrimSpace(value)))
	if len(parts) == 0 {
		return "Demo User"
	}
	for i, part := range parts {
		if part == "" {
			continue
		}
		parts[i] = strings.ToUpper(part[:1]) + part[1:]
	}
	return strings.Join(parts, " ")
}
