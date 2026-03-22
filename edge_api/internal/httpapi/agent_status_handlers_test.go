package httpapi

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/model"
)

type stubAgentStatusReader struct {
	hostStatus      model.HostAgentStatusView
	hostStatusErr   error
	diagnostics     model.HostAgentDiagnosticsView
	clusterOverview model.ClusterAgentsOverviewView
	timeline        model.DeploymentTimelineView
}

func (s *stubAgentStatusReader) GetHostAgentStatus(_ context.Context, _ string) (model.HostAgentStatusView, error) {
	return s.hostStatus, s.hostStatusErr
}

func (s *stubAgentStatusReader) GetHostDiagnostics(_ context.Context, _ string) (model.HostAgentDiagnosticsView, error) {
	return s.diagnostics, nil
}

func (s *stubAgentStatusReader) GetClusterAgentsOverview(_ context.Context, _ string) (model.ClusterAgentsOverviewView, error) {
	return s.clusterOverview, nil
}

func (s *stubAgentStatusReader) GetDeploymentTimeline(_ context.Context, _ string) (model.DeploymentTimelineView, error) {
	return s.timeline, nil
}

func (s *stubAgentStatusReader) MapAgentStreamEventJSON(data []byte) ([]byte, error) {
	return data, nil
}

func (s *stubAgentStatusReader) MapDeploymentStepEventJSON(data []byte) ([]byte, error) {
	return data, nil
}

type stubStatusErr struct {
	status int
	code   string
	msg    string
}

func (e stubStatusErr) Error() string     { return e.msg }
func (e stubStatusErr) HTTPStatus() int   { return e.status }
func (e stubStatusErr) ErrorCode() string { return e.code }

func TestHostAgentStatusRouteReturnsAggregatedPayload(t *testing.T) {
	reader := &stubAgentStatusReader{
		hostStatus: model.HostAgentStatusView{
			HostID:               "host-1",
			Hostname:             "node-1",
			DeploymentStatus:     "succeeded",
			EnrollmentStatus:     "enrolled",
			HeartbeatStatus:      "healthy",
			DoctorStatus:         "pass",
			PrimaryFailureDomain: "unknown",
			HumanHint:            "ok",
			SuggestedNextStep:    "none",
			DataFreshness: model.AgentDataFreshness{
				GeneratedAt: "2026-03-22T12:00:00Z",
			},
		},
	}
	router := NewRouter(RouterDeps{
		Config:      testRouterConfig(),
		Logger:      zap.NewNop(),
		ReadyFn:     func() bool { return true },
		AgentStatus: reader,
	})

	response := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/api/v1/hosts/host-1/agent-status", nil)
	router.ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d body=%s", response.Code, response.Body.String())
	}

	var payload model.HostAgentStatusView
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if payload.HostID != "host-1" || payload.HeartbeatStatus != "healthy" {
		t.Fatalf("unexpected payload: %+v", payload)
	}
}

func TestHostAgentStatusRouteMapsServiceErrors(t *testing.T) {
	reader := &stubAgentStatusReader{
		hostStatusErr: stubStatusErr{
			status: http.StatusNotFound,
			code:   "not_found",
			msg:    "host not found",
		},
	}
	router := NewRouter(RouterDeps{
		Config:      testRouterConfig(),
		Logger:      zap.NewNop(),
		ReadyFn:     func() bool { return true },
		AgentStatus: reader,
	})

	response := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/api/v1/hosts/missing/agent-status", nil)
	router.ServeHTTP(response, request)

	if response.Code != http.StatusNotFound {
		t.Fatalf("expected 404, got %d body=%s", response.Code, response.Body.String())
	}
	if got := response.Header().Get("Content-Type"); got != "application/json" {
		t.Fatalf("expected application/json, got %q", got)
	}
}
