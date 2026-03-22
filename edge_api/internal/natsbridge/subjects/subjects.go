package subjects

type Registry struct {
	AgentsEnrollRequest       string
	AgentsBootstrapTokenIssue string
	AgentsPolicyFetch         string
	AgentsHeartbeat           string
	AgentsDiagnostics         string
	AgentsList                string
	AgentsGet                 string
	AgentsDiagnosticsGet      string
	AgentsPolicyGet           string

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

	ControlClustersList       string
	ControlClustersGet        string
	ControlClustersCreate     string
	ControlClustersUpdate     string
	ControlClustersAddHost    string
	ControlClustersRemoveHost string

	ControlRolesList           string
	ControlRolesGet            string
	ControlRolesCreate         string
	ControlRolesUpdate         string
	ControlRolesPermissionsGet string
	ControlRolesPermissionsSet string
	ControlRoleBindingsList    string
	ControlRoleBindingsCreate  string
	ControlRoleBindingsDelete  string

	ControlIntegrationsList   string
	ControlIntegrationsGet    string
	ControlIntegrationsCreate string
	ControlIntegrationsUpdate string
	ControlIntegrationsBind   string
	ControlIntegrationsUnbind string

	TicketsList         string
	TicketsGet          string
	TicketsCreate       string
	TicketsAssign       string
	TicketsUnassign     string
	TicketsCommentAdd   string
	TicketsStatusChange string
	TicketsClose        string

	AnomalyRulesList     string
	AnomalyRulesGet      string
	AnomalyRulesCreate   string
	AnomalyRulesUpdate   string
	AnomalyInstancesList string
	AnomalyInstancesGet  string

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
	AlertsRulesList   string
	AlertsRulesGet    string
	AlertsRulesCreate string
	AlertsRulesUpdate string

	AuditList         string
	AuditEventsAppend string

	NotificationsDispatchRequested          string
	NotificationsTelegramDispatchRequested  string
	NotificationsTelegramDispatchSucceeded  string
	NotificationsTelegramDispatchFailed     string
	NotificationsTelegramHealthcheckRequest string
	NotificationsTelegramHealthcheckResult  string

	LogsIngestRaw        string
	LogsIngestNormalized string

	StreamLogs   string
	StreamAlerts string
	StreamAgents string
}

func Defaults() Registry {
	return Registry{
		AgentsEnrollRequest:       "agents.enroll.request",
		AgentsBootstrapTokenIssue: "agents.bootstrap-token.issue",
		AgentsPolicyFetch:         "agents.policy.fetch",
		AgentsHeartbeat:           "agents.heartbeat",
		AgentsDiagnostics:         "agents.diagnostics",
		AgentsList:                "agents.list",
		AgentsGet:                 "agents.get",
		AgentsDiagnosticsGet:      "agents.diagnostics.get",
		AgentsPolicyGet:           "agents.policy.get",

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

		ControlClustersList:       "control.clusters.list",
		ControlClustersGet:        "control.clusters.get",
		ControlClustersCreate:     "control.clusters.create",
		ControlClustersUpdate:     "control.clusters.update",
		ControlClustersAddHost:    "control.clusters.add-host",
		ControlClustersRemoveHost: "control.clusters.remove-host",

		ControlRolesList:           "control.roles.list",
		ControlRolesGet:            "control.roles.get",
		ControlRolesCreate:         "control.roles.create",
		ControlRolesUpdate:         "control.roles.update",
		ControlRolesPermissionsGet: "control.roles.permissions.get",
		ControlRolesPermissionsSet: "control.roles.permissions.set",
		ControlRoleBindingsList:    "control.role-bindings.list",
		ControlRoleBindingsCreate:  "control.role-bindings.create",
		ControlRoleBindingsDelete:  "control.role-bindings.delete",

		ControlIntegrationsList:   "control.integrations.list",
		ControlIntegrationsGet:    "control.integrations.get",
		ControlIntegrationsCreate: "control.integrations.create",
		ControlIntegrationsUpdate: "control.integrations.update",
		ControlIntegrationsBind:   "control.integrations.bind",
		ControlIntegrationsUnbind: "control.integrations.unbind",

		TicketsList:         "tickets.list",
		TicketsGet:          "tickets.get",
		TicketsCreate:       "tickets.create",
		TicketsAssign:       "tickets.assign",
		TicketsUnassign:     "tickets.unassign",
		TicketsCommentAdd:   "tickets.comment.add",
		TicketsStatusChange: "tickets.status.change",
		TicketsClose:        "tickets.close",

		AnomalyRulesList:     "anomalies.rules.list",
		AnomalyRulesGet:      "anomalies.rules.get",
		AnomalyRulesCreate:   "anomalies.rules.create",
		AnomalyRulesUpdate:   "anomalies.rules.update",
		AnomalyInstancesList: "anomalies.instances.list",
		AnomalyInstancesGet:  "anomalies.instances.get",

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
		AlertsRulesList:   "alerts.rules.list",
		AlertsRulesGet:    "alerts.rules.get",
		AlertsRulesCreate: "alerts.rules.create",
		AlertsRulesUpdate: "alerts.rules.update",

		AuditList:         "audit.list",
		AuditEventsAppend: "audit.events.append",

		NotificationsDispatchRequested:          "notifications.dispatch.requested.v1",
		NotificationsTelegramDispatchRequested:  "notifications.telegram.dispatch.requested.v1",
		NotificationsTelegramDispatchSucceeded:  "notifications.telegram.dispatch.succeeded.v1",
		NotificationsTelegramDispatchFailed:     "notifications.telegram.dispatch.failed.v1",
		NotificationsTelegramHealthcheckRequest: "notifications.telegram.healthcheck.requested.v1",
		NotificationsTelegramHealthcheckResult:  "notifications.telegram.healthcheck.result.v1",

		LogsIngestRaw:        "logs.ingest.raw",
		LogsIngestNormalized: "logs.ingest.normalized",

		StreamLogs:   "ui.stream.logs",
		StreamAlerts: "ui.stream.alerts",
		StreamAgents: "ui.stream.agents",
	}
}
