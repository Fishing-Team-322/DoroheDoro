package subjects

type Registry struct {
	AgentsEnrollRequest  string
	AgentsPolicyFetch    string
	AgentsHeartbeat      string
	AgentsDiagnostics    string
	AgentsRegistryList   string
	AgentsRegistryGet    string
	AgentsDiagnosticsGet string

	ControlPoliciesList      string
	ControlPoliciesGet       string
	ControlPoliciesCreate    string
	ControlPoliciesUpdate    string
	ControlPoliciesRevisions string

	ControlHostsList   string
	ControlHostsGet    string
	ControlHostsCreate string
	ControlHostsUpdate string

	ControlHostGroupsList         string
	ControlHostGroupsGet          string
	ControlHostGroupsCreate       string
	ControlHostGroupsUpdate       string
	ControlHostGroupsAddMember    string
	ControlHostGroupsRemoveMember string

	ControlCredentialsList   string
	ControlCredentialsGet    string
	ControlCredentialsCreate string

	ControlClustersList   string
	ControlClustersGet    string
	ControlClustersCreate string
	ControlClustersUpdate string

	ControlRolesList   string
	ControlRolesGet    string
	ControlRolesCreate string
	ControlRolesUpdate string

	ControlPermissionsList   string
	ControlPermissionsGet    string
	ControlPermissionsCreate string
	ControlPermissionsUpdate string

	IntegrationsList   string
	IntegrationsGet    string
	IntegrationsCreate string
	IntegrationsUpdate string

	TicketsList   string
	TicketsGet    string
	TicketsCreate string
	TicketsUpdate string

	AnomaliesList   string
	AnomaliesGet    string
	AnomaliesCreate string
	AnomaliesUpdate string

	DeploymentsJobsCreate string
	DeploymentsJobsGet    string
	DeploymentsJobsList   string
	DeploymentsJobsRetry  string
	DeploymentsJobsCancel string
	DeploymentsJobsStatus string
	DeploymentsJobsStep   string
	DeploymentsPlanCreate string

	QueryLogsSearch         string
	QueryLogsGet            string
	QueryLogsContext        string
	QueryLogsHistogram      string
	QueryLogsSeverity       string
	QueryLogsTopHosts       string
	QueryLogsTopServices    string
	QueryLogsHeatmap        string
	QueryLogsTopPatterns    string
	QueryLogsAnomalies      string
	QueryDashboardsOverview string

	AlertsList        string
	AlertsGet         string
	AlertsRulesCreate string
	AlertsRulesUpdate string

	AuditList string

	LogsIngestRaw string

	StreamLogs        string
	StreamDeployments string
	StreamAlerts      string
	StreamAgents      string
	StreamClusters    string
	StreamTickets     string
	StreamAnomalies   string
}

func Defaults() Registry {
	return Registry{
		AgentsEnrollRequest:  "agents.enroll.request",
		AgentsPolicyFetch:    "agents.policy.fetch",
		AgentsHeartbeat:      "agents.heartbeat",
		AgentsDiagnostics:    "agents.diagnostics",
		AgentsRegistryList:   "agents.registry.list",
		AgentsRegistryGet:    "agents.registry.get",
		AgentsDiagnosticsGet: "agents.diagnostics.get",

		ControlPoliciesList:      "control.policies.list",
		ControlPoliciesGet:       "control.policies.get",
		ControlPoliciesCreate:    "control.policies.create",
		ControlPoliciesUpdate:    "control.policies.update",
		ControlPoliciesRevisions: "control.policies.revisions",

		ControlHostsList:   "control.hosts.list",
		ControlHostsGet:    "control.hosts.get",
		ControlHostsCreate: "control.hosts.create",
		ControlHostsUpdate: "control.hosts.update",

		ControlHostGroupsList:         "control.host-groups.list",
		ControlHostGroupsGet:          "control.host-groups.get",
		ControlHostGroupsCreate:       "control.host-groups.create",
		ControlHostGroupsUpdate:       "control.host-groups.update",
		ControlHostGroupsAddMember:    "control.host-groups.add-member",
		ControlHostGroupsRemoveMember: "control.host-groups.remove-member",

		ControlCredentialsList:   "control.credentials.list",
		ControlCredentialsGet:    "control.credentials.get",
		ControlCredentialsCreate: "control.credentials.create",

		ControlClustersList:   "control.clusters.list",
		ControlClustersGet:    "control.clusters.get",
		ControlClustersCreate: "control.clusters.create",
		ControlClustersUpdate: "control.clusters.update",

		ControlRolesList:   "control.roles.list",
		ControlRolesGet:    "control.roles.get",
		ControlRolesCreate: "control.roles.create",
		ControlRolesUpdate: "control.roles.update",

		ControlPermissionsList:   "control.permissions.list",
		ControlPermissionsGet:    "control.permissions.get",
		ControlPermissionsCreate: "control.permissions.create",
		ControlPermissionsUpdate: "control.permissions.update",

		IntegrationsList:   "integrations.list",
		IntegrationsGet:    "integrations.get",
		IntegrationsCreate: "integrations.create",
		IntegrationsUpdate: "integrations.update",

		TicketsList:   "tickets.list",
		TicketsGet:    "tickets.get",
		TicketsCreate: "tickets.create",
		TicketsUpdate: "tickets.update",

		AnomaliesList:   "anomalies.list",
		AnomaliesGet:    "anomalies.get",
		AnomaliesCreate: "anomalies.create",
		AnomaliesUpdate: "anomalies.update",

		DeploymentsJobsCreate: "deployments.jobs.create",
		DeploymentsJobsGet:    "deployments.jobs.get",
		DeploymentsJobsList:   "deployments.jobs.list",
		DeploymentsJobsRetry:  "deployments.jobs.retry",
		DeploymentsJobsCancel: "deployments.jobs.cancel",
		DeploymentsJobsStatus: "deployments.jobs.status",
		DeploymentsJobsStep:   "deployments.jobs.step",
		DeploymentsPlanCreate: "deployments.plan.create",

		QueryLogsSearch:         "query.logs.search",
		QueryLogsGet:            "query.logs.get",
		QueryLogsContext:        "query.logs.context",
		QueryLogsHistogram:      "query.logs.histogram",
		QueryLogsSeverity:       "query.logs.severity",
		QueryLogsTopHosts:       "query.logs.top_hosts",
		QueryLogsTopServices:    "query.logs.top_services",
		QueryLogsHeatmap:        "query.logs.heatmap",
		QueryLogsTopPatterns:    "query.logs.top_patterns",
		QueryLogsAnomalies:      "query.logs.anomalies",
		QueryDashboardsOverview: "query.dashboards.overview",

		AlertsList:        "alerts.list",
		AlertsGet:         "alerts.get",
		AlertsRulesCreate: "alerts.rules.create",
		AlertsRulesUpdate: "alerts.rules.update",

		AuditList: "audit.list",

		LogsIngestRaw: "logs.ingest.raw",

		StreamLogs:        "ui.stream.logs",
		StreamDeployments: "ui.stream.deployments",
		StreamAlerts:      "ui.stream.alerts",
		StreamAgents:      "ui.stream.agents",
		StreamClusters:    "ui.stream.clusters",
		StreamTickets:     "ui.stream.tickets",
		StreamAnomalies:   "ui.stream.anomalies",
	}
}
