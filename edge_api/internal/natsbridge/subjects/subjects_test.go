package subjects

import "testing"

func TestDefaultsMatchServerRSEnrollmentSubjects(t *testing.T) {
	registry := Defaults()

	if registry.AgentsEnrollRequest != "agents.enroll.request" {
		t.Fatalf("unexpected enroll subject: %s", registry.AgentsEnrollRequest)
	}
	if registry.AgentsPolicyFetch != "agents.policy.fetch" {
		t.Fatalf("unexpected policy subject: %s", registry.AgentsPolicyFetch)
	}
	if registry.AgentsHeartbeat != "agents.heartbeat" {
		t.Fatalf("unexpected heartbeat subject: %s", registry.AgentsHeartbeat)
	}
	if registry.AgentsDiagnostics != "agents.diagnostics" {
		t.Fatalf("unexpected diagnostics subject: %s", registry.AgentsDiagnostics)
	}
}
