use std::fmt;

use common::{json::NormalizedLogEvent, AppError, AppResult};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::AnomalyRuleRecord;

#[derive(Debug, Clone)]
pub enum ParsedAnomalyRule {
    RareFingerprint(RareFingerprintRule),
    Threshold(ThresholdRule),
    Baseline(BaselineRule),
}

impl ParsedAnomalyRule {
    pub fn id(&self) -> Uuid {
        match self {
            ParsedAnomalyRule::RareFingerprint(rule) => rule.record.id,
            ParsedAnomalyRule::Threshold(rule) => rule.record.id,
            ParsedAnomalyRule::Baseline(rule) => rule.record.id,
        }
    }

    pub fn cluster_id(&self) -> Option<Uuid> {
        match self {
            ParsedAnomalyRule::RareFingerprint(rule) => rule.record.cluster_id(),
            ParsedAnomalyRule::Threshold(rule) => rule.record.cluster_id(),
            ParsedAnomalyRule::Baseline(rule) => rule.record.cluster_id(),
        }
    }

    pub fn severity(&self) -> &str {
        match self {
            ParsedAnomalyRule::RareFingerprint(rule) => &rule.severity,
            ParsedAnomalyRule::Threshold(rule) => &rule.severity,
            ParsedAnomalyRule::Baseline(rule) => &rule.severity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RareFingerprintRule {
    pub record: AnomalyRuleRecord,
    pub severity: String,
    pub filter: LogFilterConfig,
    pub window_minutes: u32,
    pub max_count: u64,
    pub scope: RareFingerprintScope,
}

impl RareFingerprintRule {
    pub fn matches(&self, event: &NormalizedLogEvent) -> bool {
        self.filter.matches(event)
    }

    pub fn resolved_filter(&self, event: &NormalizedLogEvent) -> ResolvedLogFilter {
        let host_override = match self.scope {
            RareFingerprintScope::PerHost => Some(event.host.as_str()),
            RareFingerprintScope::Global => None,
        };
        self.filter.resolve(host_override)
    }

    pub fn dedupe_key(&self, event: &NormalizedLogEvent) -> String {
        let host_key = match self.scope {
            RareFingerprintScope::PerHost => event.host.as_str(),
            RareFingerprintScope::Global => "global",
        };
        format!("{}::{}::{}", self.record.id, host_key, event.fingerprint)
    }
}

#[derive(Debug, Clone)]
pub struct ThresholdRule {
    pub record: AnomalyRuleRecord,
    pub severity: String,
    pub filter: LogFilterConfig,
    pub window_minutes: u32,
    pub threshold: u64,
}

impl ThresholdRule {
    pub fn dedupe_key(&self) -> String {
        format!("{}::threshold", self.record.id)
    }
}

#[derive(Debug, Clone)]
pub struct BaselineRule {
    pub record: AnomalyRuleRecord,
    pub severity: String,
    pub filter: LogFilterConfig,
    pub window_minutes: u32,
    pub baseline_minutes: u32,
    pub multiplier: f64,
    pub min_count: u64,
    pub min_baseline_total: u64,
}

impl BaselineRule {
    pub fn baseline_windows(&self) -> u32 {
        (self.baseline_minutes / self.window_minutes).max(1)
    }

    pub fn dedupe_key(&self) -> String {
        format!("{}::baseline", self.record.id)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RareFingerprintScope {
    Global,
    PerHost,
}

impl Default for RareFingerprintScope {
    fn default() -> Self {
        RareFingerprintScope::PerHost
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LogFilterConfig {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

impl LogFilterConfig {
    pub fn as_resolved(&self) -> ResolvedLogFilter {
        ResolvedLogFilter {
            host: self.host.clone(),
            service: self.service.clone(),
            severity: self.severity.clone(),
            query: self.query.clone(),
            fingerprint: None,
        }
    }

    pub fn matches(&self, event: &NormalizedLogEvent) -> bool {
        if let Some(host) = self.host.as_deref() {
            if !host.eq_ignore_ascii_case(&event.host) {
                return false;
            }
        }
        if let Some(service) = self.service.as_deref() {
            if !service.eq_ignore_ascii_case(&event.service) {
                return false;
            }
        }
        if let Some(severity) = self.severity.as_deref() {
            if !severity.eq_ignore_ascii_case(&event.severity) {
                return false;
            }
        }
        if let Some(query) = self.query.as_deref() {
            if !event
                .message
                .to_ascii_lowercase()
                .contains(&query.to_ascii_lowercase())
            {
                return false;
            }
        }
        true
    }

    pub fn resolve(&self, host_override: Option<&str>) -> ResolvedLogFilter {
        let mut resolved = self.as_resolved();
        if resolved.host.is_none() {
            resolved.host = host_override.map(|value| value.to_string());
        }
        resolved
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedLogFilter {
    pub host: Option<String>,
    pub service: Option<String>,
    pub severity: Option<String>,
    pub query: Option<String>,
    pub fingerprint: Option<String>,
}

impl ResolvedLogFilter {
    pub fn with_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.fingerprint = Some(fingerprint.into());
        self
    }
}

#[derive(Debug, Clone, Copy)]
enum RuleKind {
    RareFingerprint,
    Threshold,
    Baseline,
}

impl TryFrom<&str> for RuleKind {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "rare_fingerprint" => Ok(RuleKind::RareFingerprint),
            "threshold" => Ok(RuleKind::Threshold),
            "baseline" => Ok(RuleKind::Baseline),
            other => Err(AppError::invalid_argument(format!(
                "unsupported anomaly rule kind: {other}"
            ))),
        }
    }
}

pub fn parse_anomaly_rule(record: AnomalyRuleRecord) -> AppResult<ParsedAnomalyRule> {
    match RuleKind::try_from(record.kind.as_str())? {
        RuleKind::RareFingerprint => {
            let raw: RareFingerprintConfig = serde_json::from_value(record.config_json.clone())
                .map_err(|error| {
                    AppError::invalid_argument(format!(
                        "invalid rare fingerprint config for {}: {error}",
                        record.id
                    ))
                })?;
            Ok(ParsedAnomalyRule::RareFingerprint(RareFingerprintRule {
                record,
                severity: normalize_severity(&raw.severity),
                filter: raw.filter,
                window_minutes: raw.window_minutes.max(1),
                max_count: raw.max_count.max(1),
                scope: raw.scope.unwrap_or_default(),
            }))
        }
        RuleKind::Threshold => {
            let raw: ThresholdConfig =
                serde_json::from_value(record.config_json.clone()).map_err(|error| {
                    AppError::invalid_argument(format!(
                        "invalid threshold config for {}: {error}",
                        record.id
                    ))
                })?;
            if raw.threshold == 0 {
                return Err(AppError::invalid_argument(format!(
                    "threshold anomaly rule {} must have threshold > 0",
                    record.id
                )));
            }
            Ok(ParsedAnomalyRule::Threshold(ThresholdRule {
                record,
                severity: normalize_severity(&raw.severity),
                filter: raw.filter,
                window_minutes: raw.window_minutes.max(1),
                threshold: raw.threshold,
            }))
        }
        RuleKind::Baseline => {
            let raw: BaselineConfig =
                serde_json::from_value(record.config_json.clone()).map_err(|error| {
                    AppError::invalid_argument(format!(
                        "invalid baseline config for {}: {error}",
                        record.id
                    ))
                })?;
            if raw.baseline_minutes < raw.window_minutes {
                return Err(AppError::invalid_argument(format!(
                    "baseline_minutes must be >= window_minutes for anomaly rule {}",
                    record.id
                )));
            }
            Ok(ParsedAnomalyRule::Baseline(BaselineRule {
                record,
                severity: normalize_severity(&raw.severity),
                filter: raw.filter,
                window_minutes: raw.window_minutes.max(1),
                baseline_minutes: raw.baseline_minutes.max(raw.window_minutes.max(1)),
                multiplier: raw.multiplier.max(1.0),
                min_count: raw.min_count,
                min_baseline_total: raw.min_baseline_total,
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct RareFingerprintConfig {
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default = "default_rare_window_minutes")]
    window_minutes: u32,
    #[serde(default = "default_rare_threshold")]
    max_count: u64,
    #[serde(default)]
    filter: LogFilterConfig,
    #[serde(default)]
    scope: Option<RareFingerprintScope>,
}

#[derive(Debug, Deserialize)]
struct ThresholdConfig {
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default = "default_window_minutes")]
    window_minutes: u32,
    threshold: u64,
    #[serde(default)]
    filter: LogFilterConfig,
}

#[derive(Debug, Deserialize)]
struct BaselineConfig {
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default = "default_window_minutes")]
    window_minutes: u32,
    #[serde(default = "default_baseline_minutes")]
    baseline_minutes: u32,
    #[serde(default = "default_multiplier")]
    multiplier: f64,
    #[serde(default)]
    min_count: u64,
    #[serde(default)]
    min_baseline_total: u64,
    #[serde(default)]
    filter: LogFilterConfig,
}

fn default_severity() -> String {
    "medium".to_string()
}

fn default_window_minutes() -> u32 {
    5
}

fn default_rare_window_minutes() -> u32 {
    60
}

fn default_baseline_minutes() -> u32 {
    60
}

fn default_rare_threshold() -> u64 {
    1
}

fn default_multiplier() -> f64 {
    3.0
}

fn normalize_severity(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        default_severity()
    } else {
        normalized.to_ascii_lowercase()
    }
}

impl fmt::Display for RuleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleKind::RareFingerprint => write!(f, "rare_fingerprint"),
            RuleKind::Threshold => write!(f, "threshold"),
            RuleKind::Baseline => write!(f, "baseline"),
        }
    }
}
