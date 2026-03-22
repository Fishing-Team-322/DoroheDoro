package agentstatus

import (
	"strings"
	"testing"
	"time"

	"github.com/example/dorohedoro/internal/model"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

func TestBuildHostStatusViewMarksStaleHeartbeatAsNetwork(t *testing.T) {
	now := time.Date(2026, 3, 22, 12, 0, 0, 0, time.UTC)
	service := &Service{
		settings: Settings{
			HeartbeatStaleAfter: 2 * time.Minute,
			NoLogsWindow:        15 * time.Minute,
		},
		now: func() time.Time { return now },
	}

	lastRead := now.Add(-30 * time.Second).UnixMilli()
	policyRevision := "rev-1"
	rm := hostReadModel{
		host: envelope.ControlHost{
			HostID:   "host-1",
			Hostname: "node-1",
			Labels:   map[string]string{},
		},
		agentDetail: &envelope.AgentDetail{
			AgentID:     "agent-1",
			Status:      "enrolled",
			FirstSeenAt: now.Add(-1 * time.Hour).Format(time.RFC3339),
			LastSeenAt:  now.Add(-10 * time.Minute).Format(time.RFC3339),
		},
		deployment: hostDeploymentInfo{
			CurrentJob: &envelope.DeploymentJobSummary{
				JobID:  "job-1",
				Status: 4,
			},
			LastSuccessfulDeployAt: now.Add(-20 * time.Minute).Format(time.RFC3339),
		},
		diagnostics: parsedDiagnostics{
			CreatedAt: now.Add(-1 * time.Minute).Format(time.RFC3339),
			Snapshot: &diagnosticsSnapshot{
				RuntimeStatus: "healthy",
				ActiveSources: 1,
				SpoolEnabled:  true,
				PolicyState: diagnosticsPolicyState{
					CurrentPolicyRevision: &policyRevision,
				},
				ConnectivityState: diagnosticsConnectivityState{
					TLSEnabled: true,
				},
				SourceStatuses: []diagnosticsSourceStatus{{
					SourceID:   "syslog",
					Path:       "/var/log/syslog",
					Status:     "healthy",
					LastReadAt: &lastRead,
				}},
				IdentityStatus: diagnosticsIdentityStatus{
					Status: "enrolled",
				},
			},
		},
		freshness: map[string]model.AgentDataFreshnessSection{},
	}

	view := service.buildHostStatusView(rm)

	if view.HeartbeatStatus != "stale" {
		t.Fatalf("expected stale heartbeat, got %q", view.HeartbeatStatus)
	}
	if view.PrimaryFailureDomain != "network" {
		t.Fatalf("expected network failure domain, got %q", view.PrimaryFailureDomain)
	}
	if !strings.Contains(view.HumanHint, "heartbeat") {
		t.Fatalf("expected heartbeat hint, got %q", view.HumanHint)
	}
}

func TestBuildHostStatusViewMapsArtifactDeployFailure(t *testing.T) {
	service := &Service{
		settings: Settings{HeartbeatStaleAfter: 2 * time.Minute},
		now:      func() time.Time { return time.Date(2026, 3, 22, 12, 0, 0, 0, time.UTC) },
	}
	rm := hostReadModel{
		host: envelope.ControlHost{
			HostID:   "host-2",
			Hostname: "node-2",
			Labels:   map[string]string{},
		},
		deployment: hostDeploymentInfo{
			CurrentJob: &envelope.DeploymentJobSummary{
				JobID:        "job-2",
				Status:       5,
				CurrentPhase: "ansible.artifact_unresolved",
				ExecutorKind: 2,
				UpdatedAt:    time.Date(2026, 3, 22, 11, 55, 0, 0, time.UTC).Format(time.RFC3339),
				CreatedAt:    time.Date(2026, 3, 22, 11, 50, 0, 0, time.UTC).Format(time.RFC3339),
			},
			LastFailedDeployAt: time.Date(2026, 3, 22, 11, 56, 0, 0, time.UTC).Format(time.RFC3339),
		},
		freshness: map[string]model.AgentDataFreshnessSection{},
	}

	view := service.buildHostStatusView(rm)

	if view.DeploymentStatus != "failed" {
		t.Fatalf("expected failed deployment status, got %q", view.DeploymentStatus)
	}
	if view.PrimaryFailureDomain != "deployment" {
		t.Fatalf("expected deployment failure domain, got %q", view.PrimaryFailureDomain)
	}
	if !strings.Contains(strings.ToLower(view.HumanHint), "artifact") && !strings.Contains(strings.ToLower(view.HumanHint), "registry") {
		t.Fatalf("expected artifact-related hint, got %q", view.HumanHint)
	}
}

func TestNormalizeTimelinePhase(t *testing.T) {
	cases := map[string]string{
		"ansible.workspace.prepared": "inventory_rendered",
		"vault.secrets.resolved":     "manifest_resolved",
		"runner.completed":           "service_restarted",
		"artifact.unresolved":        "image_selected",
	}
	for stepName, want := range cases {
		if got := normalizeTimelinePhase(stepName, 3); got != want {
			t.Fatalf("normalizeTimelinePhase(%q) = %q, want %q", stepName, got, want)
		}
	}
}
