pub const AGENTS_ENROLL_REQUEST: &str = "agents.enroll.request";
pub const AGENTS_POLICY_FETCH: &str = "agents.policy.fetch";
pub const AGENTS_HEARTBEAT: &str = "agents.heartbeat";
pub const AGENTS_DIAGNOSTICS: &str = "agents.diagnostics";
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subjects_match_contract() {
        assert_eq!(AGENTS_ENROLL_REQUEST, "agents.enroll.request");
        assert_eq!(AGENTS_POLICY_FETCH, "agents.policy.fetch");
        assert_eq!(AGENTS_HEARTBEAT, "agents.heartbeat");
        assert_eq!(AGENTS_DIAGNOSTICS, "agents.diagnostics");
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
    }
}
