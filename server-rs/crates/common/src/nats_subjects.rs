pub const AGENTS_ENROLL_REQUEST: &str = "agents.enroll.request";
pub const AGENTS_POLICY_FETCH: &str = "agents.policy.fetch";
pub const AGENTS_HEARTBEAT: &str = "agents.heartbeat";
pub const AGENTS_DIAGNOSTICS: &str = "agents.diagnostics";

#[cfg(test)]
mod tests {
    use super::{AGENTS_DIAGNOSTICS, AGENTS_ENROLL_REQUEST, AGENTS_HEARTBEAT, AGENTS_POLICY_FETCH};

    #[test]
    fn subjects_match_contract() {
        assert_eq!(AGENTS_ENROLL_REQUEST, "agents.enroll.request");
        assert_eq!(AGENTS_POLICY_FETCH, "agents.policy.fetch");
        assert_eq!(AGENTS_HEARTBEAT, "agents.heartbeat");
        assert_eq!(AGENTS_DIAGNOSTICS, "agents.diagnostics");
    }
}
