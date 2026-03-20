package httpapi

import (
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/go-chi/chi/v5"

	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/middleware"
)

type compatUser struct {
	ID          string `json:"id"`
	Email       string `json:"email"`
	Login       string `json:"login"`
	DisplayName string `json:"displayName"`
	UpdatedAt   string `json:"updatedAt,omitempty"`
}

type compatSessionPayload struct {
	User      compatUser `json:"user"`
	CSRFToken string     `json:"csrfToken,omitempty"`
	ExpiresAt string     `json:"expiresAt,omitempty"`
}

type compatSession struct {
	Token     string
	User      compatUser
	CSRFToken string
	ExpiresAt time.Time
}

type compatProfileUpdateRequest struct {
	DisplayName string `json:"displayName"`
}

type compatLoginRequest struct {
	Identifier string `json:"identifier"`
	Email      string `json:"email"`
	Login      string `json:"login"`
	Password   string `json:"password"`
}

type compatAuthHandler struct {
	cfg     config.AuthConfig
	store   *compatSessionStore
	now     func() time.Time
	origins map[string]struct{}
}

type compatSessionStore struct {
	mu       sync.RWMutex
	sessions map[string]compatSession
}

func newCompatAuthHandler(cfg config.Config) *compatAuthHandler {
	return &compatAuthHandler{
		cfg:     cfg.Auth,
		store:   &compatSessionStore{sessions: make(map[string]compatSession)},
		now:     time.Now,
		origins: makeOriginSet(cfg.HTTP.CORSAllowedOrigins),
	}
}

func (h *compatAuthHandler) Register(r chi.Router) {
	h.registerOptions(r, "/auth/csrf")
	h.registerOptions(r, "/auth/login")
	h.registerOptions(r, "/auth/logout")
	h.registerOptions(r, "/auth/me")
	h.registerOptions(r, "/profile")
	r.Get("/auth/csrf", h.handleCSRF)
	r.Post("/auth/login", h.handleLogin)
	r.Post("/auth/logout", h.handleLogout)
	r.Get("/auth/me", h.handleCurrentSession)
	r.Patch("/profile", h.handleProfileUpdate)
}

func (h *compatAuthHandler) registerOptions(r chi.Router, pattern string) {
	r.Options(pattern, func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNoContent)
	})
}

func (h *compatAuthHandler) handleCSRF(w http.ResponseWriter, r *http.Request) {
	csrfToken := randomToken(32)
	if session, ok := h.sessionFromRequest(r); ok {
		session.CSRFToken = csrfToken
		h.store.save(session)
		h.setSessionCookie(w, session)
	}
	h.setCSRFCookie(w, csrfToken)
	middleware.WriteJSON(w, http.StatusOK, map[string]string{"csrfToken": csrfToken})
}

func (h *compatAuthHandler) handleLogin(w http.ResponseWriter, r *http.Request) {
	if err := h.validateCSRF(r); err != nil {
		writeCompatError(w, http.StatusForbidden, "forbidden", err.Error())
		return
	}

	var req compatLoginRequest
	if err := decodeJSONBody(r, &req); err != nil {
		writeCompatError(w, http.StatusBadRequest, "invalid_argument", "invalid JSON body")
		return
	}

	identifier := firstNonEmpty(req.Identifier, req.Email, req.Login)
	identifier = strings.TrimSpace(identifier)
	password := strings.TrimSpace(req.Password)
	if identifier == "" || password == "" {
		writeCompatError(w, http.StatusBadRequest, "invalid_argument", "identifier and password are required")
		return
	}

	user := buildCompatUser(identifier, h.now())
	csrfToken := randomToken(32)
	session := compatSession{
		Token:     randomToken(32),
		User:      user,
		CSRFToken: csrfToken,
		ExpiresAt: h.now().Add(h.cfg.SessionTTL),
	}
	h.store.save(session)
	h.setSessionCookie(w, session)
	h.setCSRFCookie(w, csrfToken)
	middleware.WriteJSON(w, http.StatusOK, session.payload())
}

func (h *compatAuthHandler) handleLogout(w http.ResponseWriter, r *http.Request) {
	if err := h.validateCSRF(r); err != nil {
		writeCompatError(w, http.StatusForbidden, "forbidden", err.Error())
		return
	}

	if session, ok := h.sessionFromRequest(r); ok {
		h.store.delete(session.Token)
	}

	h.clearSessionCookie(w)
	h.clearCSRFCookie(w)
	middleware.WriteJSON(w, http.StatusOK, map[string]bool{"success": true})
}

func (h *compatAuthHandler) handleCurrentSession(w http.ResponseWriter, r *http.Request) {
	session, ok := h.sessionFromRequest(r)
	if !ok {
		writeCompatError(w, http.StatusUnauthorized, "unauthorized", "authentication required")
		return
	}

	if session.ExpiresAt.Before(h.now()) {
		h.store.delete(session.Token)
		h.clearSessionCookie(w)
		writeCompatError(w, http.StatusUnauthorized, "unauthorized", "session expired")
		return
	}

	middleware.WriteJSON(w, http.StatusOK, session.payload())
}

func (h *compatAuthHandler) handleProfileUpdate(w http.ResponseWriter, r *http.Request) {
	session, ok := h.sessionFromRequest(r)
	if !ok {
		writeCompatError(w, http.StatusUnauthorized, "unauthorized", "authentication required")
		return
	}
	if err := h.validateCSRF(r); err != nil {
		writeCompatError(w, http.StatusForbidden, "forbidden", err.Error())
		return
	}

	var req compatProfileUpdateRequest
	if err := decodeJSONBody(r, &req); err != nil {
		writeCompatError(w, http.StatusBadRequest, "invalid_argument", "invalid JSON body")
		return
	}
	if strings.TrimSpace(req.DisplayName) == "" {
		writeCompatError(w, http.StatusBadRequest, "invalid_argument", "displayName is required")
		return
	}

	session.User.DisplayName = strings.TrimSpace(req.DisplayName)
	session.User.UpdatedAt = h.now().UTC().Format(time.RFC3339)
	session.CSRFToken = randomToken(32)
	h.store.save(session)
	h.setSessionCookie(w, session)
	h.setCSRFCookie(w, session.CSRFToken)
	middleware.WriteJSON(w, http.StatusOK, session.payload())
}

func (h *compatAuthHandler) corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		origin := strings.TrimSpace(r.Header.Get("Origin"))
		if origin != "" && h.isAllowedOrigin(origin) {
			w.Header().Set("Access-Control-Allow-Origin", origin)
			w.Header().Set("Access-Control-Allow-Credentials", "true")
			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Accept, Content-Type, X-CSRF-Token, X-Request-ID, X-Subject, X-Role, X-Agent-ID")
			w.Header().Set("Vary", "Origin")
		}
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func (h *compatAuthHandler) validateCSRF(r *http.Request) error {
	cookie, err := r.Cookie(h.cfg.CSRFCookieName)
	if err != nil || strings.TrimSpace(cookie.Value) == "" {
		return errCSRFInvalid("missing csrf cookie")
	}
	header := strings.TrimSpace(r.Header.Get("X-CSRF-Token"))
	if header == "" {
		return errCSRFInvalid("missing csrf header")
	}
	if header != strings.TrimSpace(cookie.Value) {
		return errCSRFInvalid("invalid csrf token")
	}
	if session, ok := h.sessionFromRequest(r); ok && strings.TrimSpace(session.CSRFToken) != "" && session.CSRFToken != header {
		return errCSRFInvalid("invalid csrf token")
	}
	return nil
}

func (h *compatAuthHandler) sessionFromRequest(r *http.Request) (compatSession, bool) {
	cookie, err := r.Cookie(h.cfg.SessionCookieName)
	if err != nil {
		return compatSession{}, false
	}
	return h.store.get(strings.TrimSpace(cookie.Value), h.now)
}

func (h *compatAuthHandler) isAllowedOrigin(origin string) bool {
	if len(h.origins) == 0 {
		return false
	}
	_, ok := h.origins[origin]
	return ok
}

func (h *compatAuthHandler) setSessionCookie(w http.ResponseWriter, session compatSession) {
	http.SetCookie(w, &http.Cookie{
		Name:     h.cfg.SessionCookieName,
		Value:    session.Token,
		Path:     "/",
		HttpOnly: true,
		Secure:   h.cfg.SessionCookieSecure,
		SameSite: http.SameSiteLaxMode,
		Expires:  session.ExpiresAt,
		MaxAge:   int(time.Until(session.ExpiresAt).Seconds()),
	})
}

func (h *compatAuthHandler) clearSessionCookie(w http.ResponseWriter) {
	http.SetCookie(w, &http.Cookie{
		Name:     h.cfg.SessionCookieName,
		Value:    "",
		Path:     "/",
		HttpOnly: true,
		Secure:   h.cfg.SessionCookieSecure,
		SameSite: http.SameSiteLaxMode,
		MaxAge:   -1,
		Expires:  time.Unix(0, 0),
	})
}

func (h *compatAuthHandler) setCSRFCookie(w http.ResponseWriter, token string) {
	http.SetCookie(w, &http.Cookie{
		Name:     h.cfg.CSRFCookieName,
		Value:    token,
		Path:     "/",
		HttpOnly: false,
		Secure:   h.cfg.SessionCookieSecure,
		SameSite: http.SameSiteLaxMode,
		MaxAge:   int(h.cfg.SessionTTL.Seconds()),
		Expires:  h.now().Add(h.cfg.SessionTTL),
	})
}

func (h *compatAuthHandler) clearCSRFCookie(w http.ResponseWriter) {
	http.SetCookie(w, &http.Cookie{
		Name:     h.cfg.CSRFCookieName,
		Value:    "",
		Path:     "/",
		HttpOnly: false,
		Secure:   h.cfg.SessionCookieSecure,
		SameSite: http.SameSiteLaxMode,
		MaxAge:   -1,
		Expires:  time.Unix(0, 0),
	})
}

func (s *compatSessionStore) save(session compatSession) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.sessions[session.Token] = session
}

func (s *compatSessionStore) get(token string, now func() time.Time) (compatSession, bool) {
	s.mu.RLock()
	session, ok := s.sessions[token]
	s.mu.RUnlock()
	if !ok {
		return compatSession{}, false
	}
	if session.ExpiresAt.Before(now()) {
		s.delete(token)
		return compatSession{}, false
	}
	return session, true
}

func (s *compatSessionStore) delete(token string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.sessions, token)
}

func (s compatSession) payload() compatSessionPayload {
	return compatSessionPayload{
		User:      s.User,
		CSRFToken: s.CSRFToken,
		ExpiresAt: s.ExpiresAt.UTC().Format(time.RFC3339),
	}
}

func buildCompatUser(identifier string, now time.Time) compatUser {
	identifier = strings.TrimSpace(identifier)
	login := identifier
	email := identifier
	if strings.Contains(identifier, "@") {
		login = strings.SplitN(identifier, "@", 2)[0]
	} else {
		email = identifier + "@example.test"
	}
	login = sanitizeIdentifier(login)
	displayName := humanizeIdentifier(login)
	if displayName == "" {
		displayName = "Demo User"
	}
	return compatUser{
		ID:          "user_" + login,
		Email:       email,
		Login:       login,
		DisplayName: displayName,
		UpdatedAt:   now.UTC().Format(time.RFC3339),
	}
}

func sanitizeIdentifier(value string) string {
	value = strings.ToLower(strings.TrimSpace(value))
	value = strings.ReplaceAll(value, " ", "-")
	var b strings.Builder
	for _, ch := range value {
		switch {
		case ch >= 'a' && ch <= 'z':
			b.WriteRune(ch)
		case ch >= '0' && ch <= '9':
			b.WriteRune(ch)
		case ch == '-', ch == '_', ch == '.':
			b.WriteRune(ch)
		}
	}
	if b.Len() == 0 {
		return "demo-user"
	}
	return b.String()
}

func humanizeIdentifier(value string) string {
	parts := strings.Fields(strings.NewReplacer(".", " ", "-", " ", "_", " ").Replace(strings.TrimSpace(value)))
	if len(parts) == 0 {
		return ""
	}
	for i, part := range parts {
		if part == "" {
			continue
		}
		parts[i] = strings.ToUpper(part[:1]) + part[1:]
	}
	return strings.Join(parts, " ")
}

func makeOriginSet(origins []string) map[string]struct{} {
	result := make(map[string]struct{}, len(origins))
	for _, origin := range origins {
		origin = strings.TrimSpace(origin)
		if origin == "" {
			continue
		}
		result[origin] = struct{}{}
	}
	return result
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if strings.TrimSpace(value) != "" {
			return value
		}
	}
	return ""
}

func randomToken(size int) string {
	buf := make([]byte, size)
	if _, err := rand.Read(buf); err != nil {
		panic(err)
	}
	return base64.RawURLEncoding.EncodeToString(buf)
}

type csrfError string

func (e csrfError) Error() string { return string(e) }

func errCSRFInvalid(message string) error {
	return csrfError(message)
}

func writeCompatError(w http.ResponseWriter, status int, code, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{
		"code":    code,
		"message": message,
	})
}
