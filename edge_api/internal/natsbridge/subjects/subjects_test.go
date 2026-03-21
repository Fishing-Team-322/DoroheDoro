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
	if registry.AgentsRegistryList != "agents.registry.list" {
		t.Fatalf("unexpected agents list subject: %s", registry.AgentsRegistryList)
	}
	if registry.AgentsRegistryGet != "agents.registry.get" {
		t.Fatalf("unexpected agents get subject: %s", registry.AgentsRegistryGet)
	}
	if registry.AgentsDiagnosticsGet != "agents.diagnostics.get" {
		t.Fatalf("unexpected diagnostics get subject: %s", registry.AgentsDiagnosticsGet)
	}
	if registry.ControlPoliciesList != "control.policies.list" {
		t.Fatalf("unexpected policies list subject: %s", registry.ControlPoliciesList)
	}
	if registry.ControlPoliciesGet != "control.policies.get" {
		t.Fatalf("unexpected policies get subject: %s", registry.ControlPoliciesGet)
	}
	if registry.ControlPoliciesRevisions != "control.policies.revisions" {
		t.Fatalf("unexpected policies revisions subject: %s", registry.ControlPoliciesRevisions)
	}
}
