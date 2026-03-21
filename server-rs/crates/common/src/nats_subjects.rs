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
pub const CONTROL_CLUSTERS_LIST: &str = "control.clusters.list";
pub const CONTROL_CLUSTERS_GET: &str = "control.clusters.get";
pub const CONTROL_CLUSTERS_CREATE: &str = "control.clusters.create";
pub const CONTROL_CLUSTERS_UPDATE: &str = "control.clusters.update";
pub const CONTROL_CLUSTERS_ADD_HOST: &str = "control.clusters.add-host";
pub const CONTROL_CLUSTERS_REMOVE_HOST: &str = "control.clusters.remove-host";
pub const CONTROL_ROLES_LIST: &str = "control.roles.list";
pub const CONTROL_ROLES_GET: &str = "control.roles.get";
pub const CONTROL_ROLES_CREATE: &str = "control.roles.create";
pub const CONTROL_ROLES_UPDATE: &str = "control.roles.update";
pub const CONTROL_ROLES_PERMISSIONS_GET: &str = "control.roles.permissions.get";
pub const CONTROL_ROLES_PERMISSIONS_SET: &str = "control.roles.permissions.set";
pub const CONTROL_ROLE_BINDINGS_LIST: &str = "control.role-bindings.list";
pub const CONTROL_ROLE_BINDINGS_CREATE: &str = "control.role-bindings.create";
pub const CONTROL_ROLE_BINDINGS_DELETE: &str = "control.role-bindings.delete";
pub const CONTROL_INTEGRATIONS_LIST: &str = "control.integrations.list";
pub const CONTROL_INTEGRATIONS_GET: &str = "control.integrations.get";
pub const CONTROL_INTEGRATIONS_CREATE: &str = "control.integrations.create";
pub const CONTROL_INTEGRATIONS_UPDATE: &str = "control.integrations.update";
pub const CONTROL_INTEGRATIONS_BIND: &str = "control.integrations.bind";
pub const CONTROL_INTEGRATIONS_UNBIND: &str = "control.integrations.unbind";

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
pub const QUERY_LOGS_ANOMALIES: &str = "query.logs.anomalies";
pub const QUERY_DASHBOARDS_OVERVIEW: &str = "query.dashboards.overview";

pub const TICKETS_LIST: &str = "tickets.list";
pub const TICKETS_GET: &str = "tickets.get";
pub const TICKETS_CREATE: &str = "tickets.create";
pub const TICKETS_ASSIGN: &str = "tickets.assign";
pub const TICKETS_UNASSIGN: &str = "tickets.unassign";
pub const TICKETS_COMMENT_ADD: &str = "tickets.comment.add";
pub const TICKETS_STATUS_CHANGE: &str = "tickets.status.change";
pub const TICKETS_CLOSE: &str = "tickets.close";

pub const ANOMALIES_RULES_LIST: &str = "anomalies.rules.list";
pub const ANOMALIES_RULES_GET: &str = "anomalies.rules.get";
pub const ANOMALIES_RULES_CREATE: &str = "anomalies.rules.create";
pub const ANOMALIES_RULES_UPDATE: &str = "anomalies.rules.update";
pub const ANOMALIES_INSTANCES_LIST: &str = "anomalies.instances.list";
pub const ANOMALIES_INSTANCES_GET: &str = "anomalies.instances.get";

pub const ALERTS_LIST: &str = "alerts.list";
pub const ALERTS_GET: &str = "alerts.get";
pub const ALERTS_RULES_LIST: &str = "alerts.rules.list";
pub const ALERTS_RULES_GET: &str = "alerts.rules.get";
pub const ALERTS_RULES_CREATE: &str = "alerts.rules.create";
pub const ALERTS_RULES_UPDATE: &str = "alerts.rules.update";

pub const NOTIFICATIONS_DISPATCH_REQUESTED: &str = "notifications.dispatch.requested.v1";
pub const NOTIFICATIONS_TELEGRAM_DISPATCH_REQUESTED: &str =
    "notifications.telegram.dispatch.requested.v1";
pub const NOTIFICATIONS_TELEGRAM_DISPATCH_SUCCEEDED: &str =
    "notifications.telegram.dispatch.succeeded.v1";
pub const NOTIFICATIONS_TELEGRAM_DISPATCH_FAILED: &str =
    "notifications.telegram.dispatch.failed.v1";
pub const NOTIFICATIONS_TELEGRAM_HEALTHCHECK_REQUESTED: &str =
    "notifications.telegram.healthcheck.requested.v1";
pub const NOTIFICATIONS_TELEGRAM_HEALTHCHECK_RESULT: &str =
    "notifications.telegram.healthcheck.result.v1";

pub const AUDIT_LIST: &str = "audit.list";
pub const AUDIT_EVENTS_APPEND: &str = "audit.events.append";
pub const LOGS_INGEST_RAW: &str = "logs.ingest.raw";
pub const LOGS_INGEST_NORMALIZED: &str = "logs.ingest.normalized";
pub const UI_STREAM_LOGS: &str = "ui.stream.logs";
pub const UI_STREAM_ALERTS: &str = "ui.stream.alerts";
pub const UI_STREAM_AGENTS: &str = "ui.stream.agents";

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
        assert_eq!(CONTROL_CLUSTERS_LIST, "control.clusters.list");
        assert_eq!(CONTROL_CLUSTERS_GET, "control.clusters.get");
        assert_eq!(CONTROL_CLUSTERS_CREATE, "control.clusters.create");
        assert_eq!(CONTROL_CLUSTERS_UPDATE, "control.clusters.update");
        assert_eq!(CONTROL_CLUSTERS_ADD_HOST, "control.clusters.add-host");
        assert_eq!(CONTROL_CLUSTERS_REMOVE_HOST, "control.clusters.remove-host");
        assert_eq!(CONTROL_ROLES_LIST, "control.roles.list");
        assert_eq!(CONTROL_ROLES_GET, "control.roles.get");
        assert_eq!(CONTROL_ROLES_CREATE, "control.roles.create");
        assert_eq!(CONTROL_ROLES_UPDATE, "control.roles.update");
        assert_eq!(
            CONTROL_ROLES_PERMISSIONS_GET,
            "control.roles.permissions.get"
        );
        assert_eq!(
            CONTROL_ROLES_PERMISSIONS_SET,
            "control.roles.permissions.set"
        );
        assert_eq!(CONTROL_ROLE_BINDINGS_LIST, "control.role-bindings.list");
        assert_eq!(CONTROL_ROLE_BINDINGS_CREATE, "control.role-bindings.create");
        assert_eq!(CONTROL_ROLE_BINDINGS_DELETE, "control.role-bindings.delete");
        assert_eq!(CONTROL_INTEGRATIONS_LIST, "control.integrations.list");
        assert_eq!(CONTROL_INTEGRATIONS_GET, "control.integrations.get");
        assert_eq!(CONTROL_INTEGRATIONS_CREATE, "control.integrations.create");
        assert_eq!(CONTROL_INTEGRATIONS_UPDATE, "control.integrations.update");
        assert_eq!(CONTROL_INTEGRATIONS_BIND, "control.integrations.bind");
        assert_eq!(CONTROL_INTEGRATIONS_UNBIND, "control.integrations.unbind");
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
        assert_eq!(QUERY_LOGS_ANOMALIES, "query.logs.anomalies");
        assert_eq!(QUERY_DASHBOARDS_OVERVIEW, "query.dashboards.overview");
        assert_eq!(TICKETS_LIST, "tickets.list");
        assert_eq!(TICKETS_GET, "tickets.get");
        assert_eq!(TICKETS_CREATE, "tickets.create");
        assert_eq!(TICKETS_ASSIGN, "tickets.assign");
        assert_eq!(TICKETS_UNASSIGN, "tickets.unassign");
        assert_eq!(TICKETS_COMMENT_ADD, "tickets.comment.add");
        assert_eq!(TICKETS_STATUS_CHANGE, "tickets.status.change");
        assert_eq!(TICKETS_CLOSE, "tickets.close");
        assert_eq!(ANOMALIES_RULES_LIST, "anomalies.rules.list");
        assert_eq!(ANOMALIES_RULES_GET, "anomalies.rules.get");
        assert_eq!(ANOMALIES_RULES_CREATE, "anomalies.rules.create");
        assert_eq!(ANOMALIES_RULES_UPDATE, "anomalies.rules.update");
        assert_eq!(ANOMALIES_INSTANCES_LIST, "anomalies.instances.list");
        assert_eq!(ANOMALIES_INSTANCES_GET, "anomalies.instances.get");
        assert_eq!(ALERTS_LIST, "alerts.list");
        assert_eq!(ALERTS_GET, "alerts.get");
        assert_eq!(ALERTS_RULES_LIST, "alerts.rules.list");
        assert_eq!(ALERTS_RULES_GET, "alerts.rules.get");
        assert_eq!(ALERTS_RULES_CREATE, "alerts.rules.create");
        assert_eq!(ALERTS_RULES_UPDATE, "alerts.rules.update");
        assert_eq!(
            NOTIFICATIONS_DISPATCH_REQUESTED,
            "notifications.dispatch.requested.v1"
        );
        assert_eq!(
            NOTIFICATIONS_TELEGRAM_DISPATCH_REQUESTED,
            "notifications.telegram.dispatch.requested.v1"
        );
        assert_eq!(
            NOTIFICATIONS_TELEGRAM_DISPATCH_SUCCEEDED,
            "notifications.telegram.dispatch.succeeded.v1"
        );
        assert_eq!(
            NOTIFICATIONS_TELEGRAM_DISPATCH_FAILED,
            "notifications.telegram.dispatch.failed.v1"
        );
        assert_eq!(
            NOTIFICATIONS_TELEGRAM_HEALTHCHECK_REQUESTED,
            "notifications.telegram.healthcheck.requested.v1"
        );
        assert_eq!(
            NOTIFICATIONS_TELEGRAM_HEALTHCHECK_RESULT,
            "notifications.telegram.healthcheck.result.v1"
        );
        assert_eq!(AUDIT_LIST, "audit.list");
        assert_eq!(AUDIT_EVENTS_APPEND, "audit.events.append");
        assert_eq!(LOGS_INGEST_RAW, "logs.ingest.raw");
        assert_eq!(LOGS_INGEST_NORMALIZED, "logs.ingest.normalized");
        assert_eq!(UI_STREAM_LOGS, "ui.stream.logs");
        assert_eq!(UI_STREAM_ALERTS, "ui.stream.alerts");
        assert_eq!(UI_STREAM_AGENTS, "ui.stream.agents");
    }
}
