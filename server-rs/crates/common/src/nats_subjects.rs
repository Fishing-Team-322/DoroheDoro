pub const AGENTS_ENROLL_REQUEST: &str = "agents.enroll.request";
pub const AGENTS_POLICY_FETCH: &str = "agents.policy.fetch";
pub const AGENTS_HEARTBEAT: &str = "agents.heartbeat";
pub const AGENTS_DIAGNOSTICS: &str = "agents.diagnostics";
pub const AGENTS_REGISTRY_LIST: &str = "agents.registry.list";
pub const AGENTS_REGISTRY_GET: &str = "agents.registry.get";
pub const AGENTS_DIAGNOSTICS_GET: &str = "agents.diagnostics.get";

pub const CONTROL_POLICIES_LIST: &str = "control.policies.list";
pub const CONTROL_POLICIES_GET: &str = "control.policies.get";
pub const CONTROL_POLICIES_REVISIONS: &str = "control.policies.revisions";

#[cfg(test)]
mod tests {
    use super::{
        AGENTS_DIAGNOSTICS, AGENTS_DIAGNOSTICS_GET, AGENTS_ENROLL_REQUEST, AGENTS_HEARTBEAT,
        AGENTS_POLICY_FETCH, AGENTS_REGISTRY_GET, AGENTS_REGISTRY_LIST, CONTROL_POLICIES_GET,
        CONTROL_POLICIES_LIST, CONTROL_POLICIES_REVISIONS,
    };

    #[test]
    fn subjects_match_contract() {
        assert_eq!(AGENTS_ENROLL_REQUEST, "agents.enroll.request");
        assert_eq!(AGENTS_POLICY_FETCH, "agents.policy.fetch");
        assert_eq!(AGENTS_HEARTBEAT, "agents.heartbeat");
        assert_eq!(AGENTS_DIAGNOSTICS, "agents.diagnostics");
        assert_eq!(AGENTS_REGISTRY_LIST, "agents.registry.list");
        assert_eq!(AGENTS_REGISTRY_GET, "agents.registry.get");
        assert_eq!(AGENTS_DIAGNOSTICS_GET, "agents.diagnostics.get");
        assert_eq!(CONTROL_POLICIES_LIST, "control.policies.list");
        assert_eq!(CONTROL_POLICIES_GET, "control.policies.get");
        assert_eq!(CONTROL_POLICIES_REVISIONS, "control.policies.revisions");
    }
}
