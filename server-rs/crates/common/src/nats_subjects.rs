pub const AGENTS_ENROLL_REQUEST: &str = "agents.enroll.request";
pub const AGENTS_POLICY_FETCH: &str = "agents.policy.fetch";
pub const AGENTS_HEARTBEAT: &str = "agents.heartbeat";
pub const AGENTS_DIAGNOSTICS: &str = "agents.diagnostics";
pub const AGENTS_LIST: &str = "agents.list";
pub const AGENTS_GET: &str = "agents.get";
pub const AGENTS_DIAGNOSTICS_GET: &str = "agents.diagnostics.get";
pub const AGENTS_POLICY_GET: &str = "agents.policy.get";
pub const AGENTS_BOOTSTRAP_TOKEN_ISSUE: &str = "agents.bootstrap-token.issue";

pub const CONTROL_POLICIES_LIST: &str = "control.policies.list";
pub const CONTROL_POLICIES_GET: &str = "control.policies.get";
pub const CONTROL_POLICIES_CREATE: &str = "control.policies.create";
pub const CONTROL_POLICIES_UPDATE: &str = "control.policies.update";
pub const CONTROL_POLICIES_REVISIONS: &str = "control.policies.revisions";
pub const CONTROL_HOSTS_LIST: &str = "control.hosts.list";
pub const CONTROL_HOSTS_GET: &str = "control.hosts.get";
pub const CONTROL_HOSTS_CREATE: &str = "control.hosts.create";
pub const CONTROL_HOSTS_UPDATE: &str = "control.hosts.update";
pub const CONTROL_HOST_GROUPS_LIST: &str = "control.host-groups.list";
pub const CONTROL_HOST_GROUPS_GET: &str = "control.host-groups.get";
pub const CONTROL_HOST_GROUPS_CREATE: &str = "control.host-groups.create";
pub const CONTROL_HOST_GROUPS_UPDATE: &str = "control.host-groups.update";
pub const CONTROL_HOST_GROUPS_ADD_MEMBER: &str = "control.host-groups.add-member";
pub const CONTROL_HOST_GROUPS_REMOVE_MEMBER: &str = "control.host-groups.remove-member";
pub const CONTROL_CREDENTIALS_LIST: &str = "control.credentials.list";
pub const CONTROL_CREDENTIALS_GET: &str = "control.credentials.get";
pub const CONTROL_CREDENTIALS_CREATE: &str = "control.credentials.create";

pub const DEPLOYMENTS_JOBS_CREATE: &str = "deployments.jobs.create";
pub const DEPLOYMENTS_JOBS_GET: &str = "deployments.jobs.get";
pub const DEPLOYMENTS_JOBS_LIST: &str = "deployments.jobs.list";
pub const DEPLOYMENTS_JOBS_RETRY: &str = "deployments.jobs.retry";
pub const DEPLOYMENTS_JOBS_CANCEL: &str = "deployments.jobs.cancel";
pub const DEPLOYMENTS_JOBS_STATUS: &str = "deployments.jobs.status";
pub const DEPLOYMENTS_JOBS_STEP: &str = "deployments.jobs.step";
pub const DEPLOYMENTS_PLAN_CREATE: &str = "deployments.plan.create";

pub const QUERY_LOGS_SEARCH: &str = "query.logs.search";
pub const QUERY_LOGS_GET: &str = "query.logs.get";
pub const QUERY_LOGS_CONTEXT: &str = "query.logs.context";
pub const QUERY_LOGS_HISTOGRAM: &str = "query.logs.histogram";
pub const QUERY_LOGS_SEVERITY: &str = "query.logs.severity";
pub const QUERY_LOGS_TOP_HOSTS: &str = "query.logs.top_hosts";
pub const QUERY_LOGS_TOP_SERVICES: &str = "query.logs.top_services";
pub const QUERY_LOGS_HEATMAP: &str = "query.logs.heatmap";
pub const QUERY_LOGS_TOP_PATTERNS: &str = "query.logs.top_patterns";
pub const QUERY_DASHBOARDS_OVERVIEW: &str = "query.dashboards.overview";

pub const ALERTS_LIST: &str = "alerts.list";
pub const ALERTS_GET: &str = "alerts.get";
pub const ALERTS_RULES_CREATE: &str = "alerts.rules.create";
pub const ALERTS_RULES_UPDATE: &str = "alerts.rules.update";

pub const AUDIT_LIST: &str = "audit.list";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subjects_match_contract() {
        assert_eq!(AGENTS_ENROLL_REQUEST, "agents.enroll.request");
        assert_eq!(AGENTS_POLICY_FETCH, "agents.policy.fetch");
        assert_eq!(AGENTS_HEARTBEAT, "agents.heartbeat");
        assert_eq!(AGENTS_DIAGNOSTICS, "agents.diagnostics");
        assert_eq!(AGENTS_LIST, "agents.list");
        assert_eq!(AGENTS_GET, "agents.get");
        assert_eq!(AGENTS_DIAGNOSTICS_GET, "agents.diagnostics.get");
        assert_eq!(AGENTS_POLICY_GET, "agents.policy.get");
        assert_eq!(AGENTS_BOOTSTRAP_TOKEN_ISSUE, "agents.bootstrap-token.issue");
        assert_eq!(CONTROL_POLICIES_LIST, "control.policies.list");
        assert_eq!(CONTROL_POLICIES_GET, "control.policies.get");
        assert_eq!(CONTROL_POLICIES_CREATE, "control.policies.create");
        assert_eq!(CONTROL_POLICIES_UPDATE, "control.policies.update");
        assert_eq!(CONTROL_POLICIES_REVISIONS, "control.policies.revisions");
        assert_eq!(CONTROL_HOSTS_LIST, "control.hosts.list");
        assert_eq!(CONTROL_HOSTS_GET, "control.hosts.get");
        assert_eq!(CONTROL_HOSTS_CREATE, "control.hosts.create");
        assert_eq!(CONTROL_HOSTS_UPDATE, "control.hosts.update");
        assert_eq!(CONTROL_HOST_GROUPS_LIST, "control.host-groups.list");
        assert_eq!(CONTROL_HOST_GROUPS_GET, "control.host-groups.get");
        assert_eq!(CONTROL_HOST_GROUPS_CREATE, "control.host-groups.create");
        assert_eq!(CONTROL_HOST_GROUPS_UPDATE, "control.host-groups.update");
        assert_eq!(
            CONTROL_HOST_GROUPS_ADD_MEMBER,
            "control.host-groups.add-member"
        );
        assert_eq!(
            CONTROL_HOST_GROUPS_REMOVE_MEMBER,
            "control.host-groups.remove-member"
        );
        assert_eq!(CONTROL_CREDENTIALS_LIST, "control.credentials.list");
        assert_eq!(CONTROL_CREDENTIALS_GET, "control.credentials.get");
        assert_eq!(CONTROL_CREDENTIALS_CREATE, "control.credentials.create");
        assert_eq!(DEPLOYMENTS_JOBS_CREATE, "deployments.jobs.create");
        assert_eq!(DEPLOYMENTS_JOBS_GET, "deployments.jobs.get");
        assert_eq!(DEPLOYMENTS_JOBS_LIST, "deployments.jobs.list");
        assert_eq!(DEPLOYMENTS_JOBS_RETRY, "deployments.jobs.retry");
        assert_eq!(DEPLOYMENTS_JOBS_CANCEL, "deployments.jobs.cancel");
        assert_eq!(DEPLOYMENTS_JOBS_STATUS, "deployments.jobs.status");
        assert_eq!(DEPLOYMENTS_JOBS_STEP, "deployments.jobs.step");
        assert_eq!(DEPLOYMENTS_PLAN_CREATE, "deployments.plan.create");
        assert_eq!(QUERY_LOGS_SEARCH, "query.logs.search");
        assert_eq!(QUERY_LOGS_GET, "query.logs.get");
        assert_eq!(QUERY_LOGS_CONTEXT, "query.logs.context");
        assert_eq!(QUERY_LOGS_HISTOGRAM, "query.logs.histogram");
        assert_eq!(QUERY_LOGS_SEVERITY, "query.logs.severity");
        assert_eq!(QUERY_LOGS_TOP_HOSTS, "query.logs.top_hosts");
        assert_eq!(QUERY_LOGS_TOP_SERVICES, "query.logs.top_services");
        assert_eq!(QUERY_LOGS_HEATMAP, "query.logs.heatmap");
        assert_eq!(QUERY_LOGS_TOP_PATTERNS, "query.logs.top_patterns");
        assert_eq!(QUERY_DASHBOARDS_OVERVIEW, "query.dashboards.overview");
        assert_eq!(ALERTS_LIST, "alerts.list");
        assert_eq!(ALERTS_GET, "alerts.get");
        assert_eq!(ALERTS_RULES_CREATE, "alerts.rules.create");
        assert_eq!(ALERTS_RULES_UPDATE, "alerts.rules.update");
        assert_eq!(AUDIT_LIST, "audit.list");
    }
}
