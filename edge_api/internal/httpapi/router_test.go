package httpapi

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/natsbridge/subjects"
)

func TestReadyzReflectsBridgeState(t *testing.T) {
	router := NewRouter(RouterDeps{
		Config:  testRouterConfig(),
		Logger:  zap.NewNop(),
		ReadyFn: func() bool { return false },
	})

	response := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/readyz", nil)
	router.ServeHTTP(response, request)

	if response.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503, got %d body=%s", response.Code, response.Body.String())
	}
}

func TestRuntimeRoutesExposeSubjectMetadataWhenBridgeMissing(t *testing.T) {
	router := NewRouter(RouterDeps{
		Config: testRouterConfig(),
		Logger: zap.NewNop(),
	})

	response := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/api/v1/logs/histogram", nil)
	router.ServeHTTP(response, request)

	if response.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503, got %d body=%s", response.Code, response.Body.String())
	}
	if got := response.Header().Get("X-NATS-Subject"); got != "query.logs.histogram" {
		t.Fatalf("expected X-NATS-Subject query.logs.histogram, got %q", got)
	}
}

func TestAlertRoutesExposeSubjectMetadataWhenBridgeMissing(t *testing.T) {
	router := NewRouter(RouterDeps{
		Config: testRouterConfig(),
		Logger: zap.NewNop(),
	})

	response := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/api/v1/alerts", nil)
	router.ServeHTTP(response, request)

	if response.Code != http.StatusServiceUnavailable {
		t.Fatalf("expected 503, got %d body=%s", response.Code, response.Body.String())
	}
	if got := response.Header().Get("X-NATS-Subject"); got != "alerts.list" {
		t.Fatalf("expected X-NATS-Subject alerts.list, got %q", got)
	}
}

func testRouterConfig() config.Config {
	return config.Config{
		ServiceName: "edge-api",
		Version:     "test",
		Limits: config.LimitsConfig{
			HTTPBodyBytes: 1 << 20,
		},
		Auth: config.AuthConfig{
			HTTPStubEnabled:   true,
			SessionCookieName: "session_token",
			CSRFCookieName:    "csrf_token",
		},
		NATS: config.NATSConfig{
			Subjects: subjects.Defaults(),
		},
	}
}
