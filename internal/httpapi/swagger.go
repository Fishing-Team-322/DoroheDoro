package httpapi

import (
	"time"

	"github.com/example/dorohedoro/internal/diagnostics"
	chindexer "github.com/example/dorohedoro/internal/indexer/clickhouse"
	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/policy"
	"github.com/example/dorohedoro/internal/query"
)

type statusResponse struct {
	Status string `json:"status"`
}

type errorResponse struct {
	Error string `json:"error"`
}

type agentsListResponse struct {
	Items []diagnostics.AgentStatus `json:"items"`
}

type agentDiagnosticsResponse struct {
	Agent       diagnostics.AgentStatus `json:"agent"`
	Policy      policy.Policy           `json:"policy"`
	RuntimeTime time.Time               `json:"runtime_time"`
}

type policyResponse struct {
	AgentID          string        `json:"agent_id"`
	CurrentRevision  string        `json:"current_revision"`
	Policy           policy.Policy `json:"policy"`
	Changed          bool          `json:"changed"`
	ServedAt         time.Time     `json:"served_at"`
	IngestTLSMode    string        `json:"ingest_tls_mode"`
	MTLSEnabled      bool          `json:"mtls_enabled"`
	MTLSTODOScaffold string        `json:"mtls_todo_scaffold"`
}

type histogramResponse struct {
	Items []chindexer.HistogramBucket `json:"items"`
}

type countsResponse struct {
	Items []chindexer.CountRow `json:"items"`
}

type healthDoc struct{}

// swaggerHealth godoc
// @Summary Liveness probe
// @Description Returns the current liveness state of the HTTP API.
// @Tags health
// @Produce json
// @Success 200 {object} statusResponse
// @Router /healthz [get]
func swaggerHealth() {}

// swaggerReady godoc
// @Summary Readiness probe
// @Description Returns ready when the search backend is reachable; otherwise reports not-ready.
// @Tags health
// @Produce json
// @Success 200 {object} statusResponse
// @Failure 503 {object} statusResponse
// @Router /readyz [get]
func swaggerReady() {}

// swaggerSearchLogs godoc
// @Summary Search logs
// @Description Searches indexed log events with optional filters and pagination.
// @Tags logs
// @Produce json
// @Param q query string false "Free-text query"
// @Param from query string false "RFC3339 timestamp or epoch milliseconds"
// @Param to query string false "RFC3339 timestamp or epoch milliseconds"
// @Param host query string false "Host filter"
// @Param service query string false "Service filter"
// @Param severity query string false "Severity filter"
// @Param limit query int false "Result limit" default(100)
// @Param offset query int false "Result offset" default(0)
// @Success 200 {object} query.SearchResult
// @Failure 502 {object} errorResponse
// @Router /api/v1/logs/search [get]
func swaggerSearchLogs() {}

// swaggerLogContext godoc
// @Summary Get log context
// @Description Returns an anchor event and nearby events from the same host and, when available, the same service.
// @Tags logs
// @Produce json
// @Param id path string true "Event ID"
// @Success 200 {object} query.ContextResult
// @Failure 502 {object} errorResponse
// @Router /api/v1/logs/{id}/context [get]
func swaggerLogContext() {}

// swaggerAgents godoc
// @Summary List agents
// @Description Lists known agents and their current diagnostic status.
// @Tags agents
// @Produce json
// @Success 200 {object} agentsListResponse
// @Router /api/v1/agents [get]
func swaggerAgents() {}

// swaggerAgent godoc
// @Summary Get agent
// @Description Returns a single agent diagnostic record.
// @Tags agents
// @Produce json
// @Param id path string true "Agent ID"
// @Success 200 {object} diagnostics.AgentStatus
// @Failure 404 {object} errorResponse
// @Router /api/v1/agents/{id} [get]
func swaggerAgent() {}

// swaggerAgentDiagnostics godoc
// @Summary Get agent diagnostics
// @Description Returns combined diagnostic and policy information for a single agent.
// @Tags agents
// @Produce json
// @Param id path string true "Agent ID"
// @Success 200 {object} agentDiagnosticsResponse
// @Failure 404 {object} errorResponse
// @Router /api/v1/agents/{id}/diagnostics [get]
func swaggerAgentDiagnostics() {}

// swaggerPolicy godoc
// @Summary Get effective policy
// @Description Returns the effective policy currently assigned to an agent.
// @Tags policy
// @Produce json
// @Param agent_id query string true "Agent ID"
// @Param current_revision query string false "Agent's currently applied revision"
// @Success 200 {object} policyResponse
// @Failure 400 {object} errorResponse
// @Router /api/v1/policy [get]
func swaggerPolicy() {}

// swaggerAnalyticsHistogram godoc
// @Summary Get event histogram
// @Description Returns per-minute event counts from ClickHouse analytics when enabled.
// @Tags analytics
// @Produce json
// @Param from query string false "RFC3339 timestamp or epoch milliseconds"
// @Param to query string false "RFC3339 timestamp or epoch milliseconds"
// @Param limit query int false "Unused by histogram endpoint"
// @Success 200 {object} histogramResponse
// @Failure 502 {object} errorResponse
// @Failure 503 {object} errorResponse
// @Router /api/v1/analytics/histogram [get]
func swaggerAnalyticsHistogram() {}

// swaggerAnalyticsSeverity godoc
// @Summary Get severity aggregation
// @Description Returns event counts grouped by severity from ClickHouse analytics when enabled.
// @Tags analytics
// @Produce json
// @Param from query string false "RFC3339 timestamp or epoch milliseconds"
// @Param to query string false "RFC3339 timestamp or epoch milliseconds"
// @Param limit query int false "Optional limit"
// @Success 200 {object} countsResponse
// @Failure 502 {object} errorResponse
// @Failure 503 {object} errorResponse
// @Router /api/v1/analytics/severity [get]
func swaggerAnalyticsSeverity() {}

// swaggerAnalyticsTopHosts godoc
// @Summary Get top hosts
// @Description Returns the most active hosts from ClickHouse analytics when enabled.
// @Tags analytics
// @Produce json
// @Param from query string false "RFC3339 timestamp or epoch milliseconds"
// @Param to query string false "RFC3339 timestamp or epoch milliseconds"
// @Param limit query int false "Maximum number of rows" default(10)
// @Success 200 {object} countsResponse
// @Failure 502 {object} errorResponse
// @Failure 503 {object} errorResponse
// @Router /api/v1/analytics/top-hosts [get]
func swaggerAnalyticsTopHosts() {}

// swaggerAnalyticsTopServices godoc
// @Summary Get top services
// @Description Returns the most active services from ClickHouse analytics when enabled.
// @Tags analytics
// @Produce json
// @Param from query string false "RFC3339 timestamp or epoch milliseconds"
// @Param to query string false "RFC3339 timestamp or epoch milliseconds"
// @Param limit query int false "Maximum number of rows" default(10)
// @Success 200 {object} countsResponse
// @Failure 502 {object} errorResponse
// @Failure 503 {object} errorResponse
// @Router /api/v1/analytics/top-services [get]
func swaggerAnalyticsTopServices() {}

var (
	_ = model.Event{}
)
