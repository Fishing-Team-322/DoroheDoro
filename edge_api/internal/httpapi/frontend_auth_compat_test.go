package httpapi

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/go-chi/chi/v5"

	"github.com/example/dorohedoro/internal/config"
)

func TestCompatAuthLoginAllowsCSRFWithoutExistingSession(t *testing.T) {
	handler := newCompatAuthTestHandler()
	router := chi.NewRouter()
	handler.Register(router)

	csrfResp := httptest.NewRecorder()
	csrfReq := httptest.NewRequest(http.MethodGet, "/auth/csrf", nil)
	router.ServeHTTP(csrfResp, csrfReq)

	if csrfResp.Code != http.StatusOK {
		t.Fatalf("expected csrf status 200, got %d", csrfResp.Code)
	}

	csrfCookie := cookieByName(csrfResp.Result().Cookies(), handler.cfg.CSRFCookieName)
	if csrfCookie == nil || csrfCookie.Value == "" {
		t.Fatalf("expected csrf cookie %q to be set", handler.cfg.CSRFCookieName)
	}

	loginBody, err := json.Marshal(map[string]string{
		"identifier": handler.cfg.DevUser.Login,
		"password":   handler.cfg.DevUser.Password,
	})
	if err != nil {
		t.Fatalf("marshal login body: %v", err)
	}

	loginResp := httptest.NewRecorder()
	loginReq := httptest.NewRequest(http.MethodPost, "/auth/login", bytes.NewReader(loginBody))
	loginReq.Header.Set("Content-Type", "application/json")
	loginReq.Header.Set("X-CSRF-Token", csrfCookie.Value)
	loginReq.AddCookie(csrfCookie)
	router.ServeHTTP(loginResp, loginReq)

	if loginResp.Code != http.StatusOK {
		t.Fatalf("expected login status 200, got %d body=%s", loginResp.Code, loginResp.Body.String())
	}

	sessionCookie := cookieByName(loginResp.Result().Cookies(), handler.cfg.SessionCookieName)
	if sessionCookie == nil || sessionCookie.Value == "" {
		t.Fatalf("expected session cookie %q to be set", handler.cfg.SessionCookieName)
	}

	meResp := httptest.NewRecorder()
	meReq := httptest.NewRequest(http.MethodGet, "/auth/me", nil)
	meReq.AddCookie(sessionCookie)
	router.ServeHTTP(meResp, meReq)

	if meResp.Code != http.StatusOK {
		t.Fatalf("expected /auth/me status 200, got %d body=%s", meResp.Code, meResp.Body.String())
	}
}

func TestCompatAuthProtectedMutationsStillRequireSessionAndCSRF(t *testing.T) {
	handler := newCompatAuthTestHandler()
	router := chi.NewRouter()
	handler.Register(router)

	tests := []struct {
		name   string
		method string
		target string
		body   []byte
		want   int
	}{
		{
			name:   "logout",
			method: http.MethodPost,
			target: "/auth/logout",
			want:   http.StatusForbidden,
		},
		{
			name:   "profile update",
			method: http.MethodPatch,
			target: "/profile",
			body:   []byte(`{"displayName":"Admin Smoke"}`),
			want:   http.StatusUnauthorized,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resp := httptest.NewRecorder()
			req := httptest.NewRequest(tt.method, tt.target, bytes.NewReader(tt.body))
			if len(tt.body) > 0 {
				req.Header.Set("Content-Type", "application/json")
			}
			req.AddCookie(&http.Cookie{Name: handler.cfg.CSRFCookieName, Value: "csrf-only-token"})
			req.Header.Set("X-CSRF-Token", "csrf-only-token")
			router.ServeHTTP(resp, req)

			if resp.Code != tt.want {
				t.Fatalf("expected status %d, got %d body=%s", tt.want, resp.Code, resp.Body.String())
			}
		})
	}
}

func newCompatAuthTestHandler() *compatAuthHandler {
	return newCompatAuthHandler(config.Config{
		Auth: config.AuthConfig{
			HTTPStubEnabled:   true,
			SessionCookieName: "session_token",
			CSRFCookieName:    "csrf_token",
			CookieSecure:      false,
			SessionTTL:        24 * time.Hour,
			DevUser: config.DevAuthUser{
				Login:       "admin",
				Email:       "admin@example.com",
				Password:    "admin123",
				UserID:      "dev-user-1",
				Role:        "admin",
				DisplayName: "Admin",
			},
		},
	})
}

func cookieByName(cookies []*http.Cookie, name string) *http.Cookie {
	for _, cookie := range cookies {
		if cookie.Name == name {
			return cookie
		}
	}
	return nil
}
