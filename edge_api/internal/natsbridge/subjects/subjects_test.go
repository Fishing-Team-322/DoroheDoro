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
	if registry.ControlHostsList != "control.hosts.list" {
		t.Fatalf("unexpected hosts list subject: %s", registry.ControlHostsList)
	}
	if registry.ControlHostsCreate != "control.hosts.create" {
		t.Fatalf("unexpected hosts create subject: %s", registry.ControlHostsCreate)
	}
	if registry.ControlHostGroupsList != "control.host-groups.list" {
		t.Fatalf("unexpected host groups list subject: %s", registry.ControlHostGroupsList)
	}
	if registry.ControlCredentialsCreate != "control.credentials.create" {
		t.Fatalf("unexpected credentials create subject: %s", registry.ControlCredentialsCreate)
	}
	if registry.DeploymentsJobsCreate != "deployments.jobs.create" {
		t.Fatalf("unexpected deployments create subject: %s", registry.DeploymentsJobsCreate)
	}
	if registry.DeploymentsJobsGet != "deployments.jobs.get" {
		t.Fatalf("unexpected deployments get subject: %s", registry.DeploymentsJobsGet)
	}
	if registry.DeploymentsJobsList != "deployments.jobs.list" {
		t.Fatalf("unexpected deployments list subject: %s", registry.DeploymentsJobsList)
	}
	if registry.DeploymentsJobsStatus != "deployments.jobs.status" {
		t.Fatalf("unexpected deployments status subject: %s", registry.DeploymentsJobsStatus)
	}
	if registry.DeploymentsJobsStep != "deployments.jobs.step" {
		t.Fatalf("unexpected deployments step subject: %s", registry.DeploymentsJobsStep)
	}
	if registry.DeploymentsPlanCreate != "deployments.plan.create" {
		t.Fatalf("unexpected deployments plan subject: %s", registry.DeploymentsPlanCreate)
	}
}
