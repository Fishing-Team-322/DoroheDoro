use serde::{Deserialize, Serialize};

use crate::config::SecurityScanConfig;

pub const SECURITY_POSTURE_REPORT_EVENT: &str = "security.posture.report.v1";
pub const SECURITY_POSTURE_SCAN_FAILED_EVENT: &str = "security.posture.scan.failed.v1";
pub const SECURITY_POSTURE_SCAN_SKIPPED_EVENT: &str = "security.posture.scan.skipped.v1";
pub const SECURITY_POSTURE_RULES_LOADED_EVENT: &str = "security.posture.rules.loaded.v1";
pub const SECURITY_SCHEMA_VERSION: &str = "v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecuritySeverity {
    Info,
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityFindingSummary {
    pub total: u32,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub info: u32,
}

impl SecurityFindingSummary {
    pub fn observe(&mut self, severity: SecuritySeverity) {
        self.total = self.total.saturating_add(1);
        match severity {
            SecuritySeverity::Critical => self.critical = self.critical.saturating_add(1),
            SecuritySeverity::High => self.high = self.high.saturating_add(1),
            SecuritySeverity::Medium => self.medium = self.medium.saturating_add(1),
            SecuritySeverity::Low => self.low = self.low.saturating_add(1),
            SecuritySeverity::Info => self.info = self.info.saturating_add(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityFinding {
    pub finding_id: String,
    #[serde(default)]
    pub finding_fingerprint: Option<String>,
    pub category: String,
    pub severity: SecuritySeverity,
    pub title: String,
    pub detail: String,
    pub asset_type: String,
    pub asset_name: String,
    pub observed_value: Option<String>,
    pub expected_value: Option<String>,
    pub check_id: String,
    #[serde(default)]
    pub remediation: Option<String>,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityPortState {
    pub port_id: String,
    pub protocol: String,
    pub listen_address: String,
    pub port: u16,
    pub exposure: String,
    #[serde(default)]
    pub inode: Option<u64>,
    #[serde(default)]
    pub process_id: Option<u32>,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub executable_path: Option<String>,
    #[serde(default)]
    pub service_unit: Option<String>,
    #[serde(default)]
    pub owner_uid: Option<u32>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityAssetVersion {
    pub asset_id: String,
    pub requested_name: String,
    pub resolved_name: Option<String>,
    pub source_kind: String,
    pub locator: Option<String>,
    pub installed: bool,
    pub version: Option<String>,
    pub min_secure_version: Option<String>,
    pub evaluation: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityMisconfigurationCheck {
    pub check_id: String,
    pub check_name: String,
    pub target: String,
    pub status: String,
    pub severity: SecuritySeverity,
    pub detail: String,
    #[serde(default)]
    pub observed_value: Option<String>,
    #[serde(default)]
    pub expected_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityRuntimeIssue {
    pub issue_id: String,
    pub issue_kind: String,
    pub severity: SecuritySeverity,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityPostureReport {
    pub schema_version: String,
    pub event_name: String,
    pub report_id: String,
    pub created_at: String,
    pub agent_id: String,
    pub hostname: String,
    pub profile: String,
    pub interval_sec: u64,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub status: String,
    pub port_states: Vec<SecurityPortState>,
    pub asset_versions: Vec<SecurityAssetVersion>,
    pub misconfig_checks: Vec<SecurityMisconfigurationCheck>,
    pub runtime_health: Vec<SecurityRuntimeIssue>,
    pub findings: Vec<SecurityFinding>,
    pub summary: SecurityFindingSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityScanFailureEvent {
    pub schema_version: String,
    pub event_name: String,
    pub event_id: String,
    pub created_at: String,
    pub agent_id: String,
    pub hostname: String,
    pub profile: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_kind: String,
    pub error_code: String,
    pub error_message: String,
    pub retry_backoff_sec: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityScanSkippedEvent {
    pub schema_version: String,
    pub event_name: String,
    pub event_id: String,
    pub created_at: String,
    pub agent_id: String,
    pub hostname: String,
    pub profile: String,
    pub reason_code: String,
    pub reason_message: String,
    pub retry_after_sec: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityRulesLoadedEvent {
    pub schema_version: String,
    pub event_name: String,
    pub event_id: String,
    pub created_at: String,
    pub agent_id: String,
    pub hostname: String,
    pub source_path: String,
    pub rules_digest: String,
    pub package_rule_count: usize,
    pub watchlist: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SecurityRulesFile {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub packages: Vec<SecurityPackageRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SecurityPackageRule {
    pub name: String,
    pub min_secure_version: String,
    #[serde(default)]
    pub severity: SecuritySeverity,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityPostureStatusSnapshot {
    pub enabled: bool,
    pub profile: String,
    pub interval_sec: u64,
    pub jitter_sec: u64,
    pub timeout_sec: u64,
    pub last_started_at: Option<i64>,
    pub last_finished_at: Option<i64>,
    pub last_status: String,
    pub last_status_reason: Option<String>,
    pub last_report_id: Option<String>,
    pub last_delivery_status: Option<String>,
    pub last_delivery_error: Option<String>,
    pub last_rules_loaded_at: Option<i64>,
    pub last_rules_digest: Option<String>,
    pub last_report_path: Option<String>,
    pub backoff_until: Option<i64>,
    pub consecutive_failures: u32,
    pub summary: SecurityFindingSummary,
}

impl Default for SecurityPostureStatusSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: "balanced".to_string(),
            interval_sec: 0,
            jitter_sec: 0,
            timeout_sec: 0,
            last_started_at: None,
            last_finished_at: None,
            last_status: "never_run".to_string(),
            last_status_reason: None,
            last_report_id: None,
            last_delivery_status: None,
            last_delivery_error: None,
            last_rules_loaded_at: None,
            last_rules_digest: None,
            last_report_path: None,
            backoff_until: None,
            consecutive_failures: 0,
            summary: SecurityFindingSummary::default(),
        }
    }
}

impl SecurityPostureStatusSnapshot {
    pub fn from_record(config: &SecurityScanConfig, record: &SecurityScanStateRecord) -> Self {
        Self {
            enabled: config.enabled,
            profile: config.profile.as_str().to_string(),
            interval_sec: config.interval_sec,
            jitter_sec: config.jitter_sec,
            timeout_sec: config.timeout_sec,
            last_started_at: record.last_started_at_unix_ms,
            last_finished_at: record.last_finished_at_unix_ms,
            last_status: record
                .last_status
                .clone()
                .unwrap_or_else(|| "never_run".to_string()),
            last_status_reason: record.last_status_reason.clone(),
            last_report_id: record.last_report_id.clone(),
            last_delivery_status: record.last_delivery_status.clone(),
            last_delivery_error: record.last_delivery_error.clone(),
            last_rules_loaded_at: record.last_rules_loaded_at_unix_ms,
            last_rules_digest: record.last_rules_digest.clone(),
            last_report_path: record.last_report_path.clone(),
            backoff_until: record.backoff_until_unix_ms,
            consecutive_failures: record.consecutive_failures,
            summary: record.summary.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityScanStateRecord {
    pub last_started_at_unix_ms: Option<i64>,
    pub last_finished_at_unix_ms: Option<i64>,
    pub last_status: Option<String>,
    pub last_status_reason: Option<String>,
    pub last_report_id: Option<String>,
    pub last_delivery_status: Option<String>,
    pub last_delivery_error: Option<String>,
    pub last_rules_loaded_at_unix_ms: Option<i64>,
    pub last_rules_digest: Option<String>,
    pub last_report_path: Option<String>,
    pub backoff_until_unix_ms: Option<i64>,
    pub consecutive_failures: u32,
    pub summary: SecurityFindingSummary,
    pub updated_at_unix_ms: i64,
}

impl Default for SecurityScanStateRecord {
    fn default() -> Self {
        Self {
            last_started_at_unix_ms: None,
            last_finished_at_unix_ms: None,
            last_status: None,
            last_status_reason: None,
            last_report_id: None,
            last_delivery_status: None,
            last_delivery_error: None,
            last_rules_loaded_at_unix_ms: None,
            last_rules_digest: None,
            last_report_path: None,
            backoff_until_unix_ms: None,
            consecutive_failures: 0,
            summary: SecurityFindingSummary::default(),
            updated_at_unix_ms: 0,
        }
    }
}

fn default_schema_version() -> String {
    SECURITY_SCHEMA_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::{SecurityFindingSummary, SecurityPostureStatusSnapshot, SecuritySeverity};

    #[test]
    fn finding_summary_counts_by_severity() {
        let mut summary = SecurityFindingSummary::default();
        summary.observe(SecuritySeverity::Critical);
        summary.observe(SecuritySeverity::High);
        summary.observe(SecuritySeverity::Medium);
        summary.observe(SecuritySeverity::Low);
        summary.observe(SecuritySeverity::Info);

        assert_eq!(summary.total, 5);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.high, 1);
        assert_eq!(summary.medium, 1);
        assert_eq!(summary.low, 1);
        assert_eq!(summary.info, 1);
    }

    #[test]
    fn security_status_defaults_to_never_run() {
        let snapshot = SecurityPostureStatusSnapshot::default();
        assert_eq!(snapshot.last_status, "never_run");
        assert!(!snapshot.enabled);
        assert_eq!(snapshot.summary.total, 0);
    }
}
