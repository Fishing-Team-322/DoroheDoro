package httpapi

import (
	"errors"
	"net/http"
	"strings"

	"github.com/go-chi/chi/v5"

	"github.com/example/dorohedoro/internal/middleware"
)

type statusError interface {
	error
	HTTPStatus() int
	ErrorCode() string
}

func hostAgentStatusHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.AgentStatus == nil {
			middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "agent status service is not ready")
			return
		}
		hostID := strings.TrimSpace(chi.URLParam(r, "id"))
		if hostID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host id is required")
			return
		}
		view, err := deps.AgentStatus.GetHostAgentStatus(r.Context(), hostID)
		if err != nil {
			writeAgentStatusError(w, r, err)
			return
		}
		middleware.WriteJSON(w, http.StatusOK, view)
	}
}

func hostAgentDiagnosticsHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.AgentStatus == nil {
			middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "agent status service is not ready")
			return
		}
		hostID := strings.TrimSpace(chi.URLParam(r, "id"))
		if hostID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "host id is required")
			return
		}
		view, err := deps.AgentStatus.GetHostDiagnostics(r.Context(), hostID)
		if err != nil {
			writeAgentStatusError(w, r, err)
			return
		}
		middleware.WriteJSON(w, http.StatusOK, view)
	}
}

func clusterAgentsOverviewHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.AgentStatus == nil {
			middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "agent status service is not ready")
			return
		}
		clusterID := strings.TrimSpace(chi.URLParam(r, "id"))
		if clusterID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "cluster id is required")
			return
		}
		view, err := deps.AgentStatus.GetClusterAgentsOverview(r.Context(), clusterID)
		if err != nil {
			writeAgentStatusError(w, r, err)
			return
		}
		middleware.WriteJSON(w, http.StatusOK, view)
	}
}

func deploymentTimelineHandler(deps RouterDeps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if deps.AgentStatus == nil {
			middleware.WriteError(w, r, http.StatusServiceUnavailable, "unavailable", "agent status service is not ready")
			return
		}
		jobID := strings.TrimSpace(chi.URLParam(r, "id"))
		if jobID == "" {
			middleware.WriteError(w, r, http.StatusBadRequest, "invalid_argument", "deployment job id is required")
			return
		}
		view, err := deps.AgentStatus.GetDeploymentTimeline(r.Context(), jobID)
		if err != nil {
			writeAgentStatusError(w, r, err)
			return
		}
		middleware.WriteJSON(w, http.StatusOK, view)
	}
}

func writeAgentStatusError(w http.ResponseWriter, r *http.Request, err error) {
	var reqErr statusError
	if errors.As(err, &reqErr) {
		middleware.WriteError(w, r, reqErr.HTTPStatus(), reqErr.ErrorCode(), err.Error())
		return
	}
	middleware.WriteTransportError(w, r, err)
}
