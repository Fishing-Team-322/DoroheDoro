use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverallStatus {
    Pass,
    Warn,
    Fail,
    Healthy,
    WarmingUp,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportCheck {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    pub category: String,
    #[serde(default)]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
    pub check_count: usize,
    pub warning_count: usize,
    pub failure_count: usize,
    pub overall_status: OverallStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationalReport {
    pub report_kind: String,
    pub config_path: String,
    pub generated_at: String,
    pub summary: ReportSummary,
    pub checks: Vec<ReportCheck>,
}

impl OperationalReport {
    pub fn has_failures(&self) -> bool {
        self.summary.failure_count > 0
    }

    pub fn print_text(&self) {
        println!("doro-agent {}", self.report_kind);
        println!("config: {}", self.config_path);
        println!("generated_at: {}", self.generated_at);
        for check in &self.checks {
            println!(
                "[{}] {} ({}): {}",
                check.status.label(),
                check.name,
                check.category,
                check.detail
            );
            if let Some(hint) = &check.hint {
                println!("  hint: {hint}");
            }
        }
        println!(
            "summary: {} checks, {} warning(s), {} failure(s), overall={}",
            self.summary.check_count,
            self.summary.warning_count,
            self.summary.failure_count,
            self.summary.overall_status.label()
        );
    }

    pub fn print_json(&self) -> Result<(), serde_json::Error> {
        println!("{}", serde_json::to_string_pretty(self)?);
        Ok(())
    }
}

impl CheckStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

impl OverallStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Healthy => "healthy",
            Self::WarmingUp => "warming_up",
            Self::Unhealthy => "unhealthy",
        }
    }
}

impl ReportCheck {
    pub fn new(
        name: impl Into<String>,
        status: CheckStatus,
        category: impl Into<String>,
        detail: impl Into<String>,
        hint: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            status,
            detail: detail.into(),
            category: category.into(),
            hint,
        }
    }
}
