use std::collections::BTreeSet;

use common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::models::IntegrationModel;

pub const TELEGRAM_INTEGRATION_KIND: &str = "telegram_bot";
pub const TELEGRAM_PARSE_MODE_HTML: &str = "HTML";
pub const TELEGRAM_PARSE_MODE_PLAIN: &str = "plain";
pub const TELEGRAM_TEMPLATE_VERSION_V1: &str = "v1";

const TELEGRAM_ALLOWED_INPUT_KEYS: &[&str] = &[
    "bot_name",
    "parse_mode",
    "secret_ref",
    "default_chat_id",
    "message_template_version",
    "delivery_enabled",
    "masked_secret_ref",
    "has_secret_ref",
    "token",
];

const TELEGRAM_ALLOWED_EVENT_TYPES: &[&str] = &[
    "alerts.firing",
    "alerts.resolved",
    "anomalies.detected",
    "anomalies.resolved",
    "security.finding.opened",
    "security.finding.resolved",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelegramIntegrationConfig {
    pub bot_name: String,
    pub parse_mode: String,
    pub secret_ref: String,
    pub default_chat_id: Option<String>,
    pub message_template_version: String,
    pub delivery_enabled: bool,
}

impl TelegramIntegrationConfig {
    pub fn from_value(value: &Value, integration_name: &str) -> AppResult<Self> {
        let object = value.as_object().ok_or_else(|| {
            AppError::invalid_argument("telegram_bot config_json must be a JSON object")
        })?;

        validate_telegram_keys(object)?;
        reject_raw_token(object)?;

        let bot_name = optional_string(object, "bot_name")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| integration_name.trim().to_string());
        if bot_name.is_empty() {
            return Err(AppError::invalid_argument(
                "telegram_bot config_json.bot_name is required",
            ));
        }

        let secret_ref = required_string(
            object,
            "secret_ref",
            "telegram_bot config_json.secret_ref is required",
        )?;
        let parse_mode = normalize_parse_mode(optional_string(object, "parse_mode").as_deref())?;
        let default_chat_id =
            optional_string(object, "default_chat_id").filter(|value| !value.is_empty());
        let template_version = optional_string(object, "message_template_version")
            .unwrap_or_else(|| TELEGRAM_TEMPLATE_VERSION_V1.to_string());
        if template_version != TELEGRAM_TEMPLATE_VERSION_V1 {
            return Err(AppError::invalid_argument(
                "telegram_bot config_json.message_template_version must be `v1`",
            ));
        }
        let delivery_enabled = optional_bool(object, "delivery_enabled")?.unwrap_or(true);

        Ok(Self {
            bot_name,
            parse_mode,
            secret_ref,
            default_chat_id,
            message_template_version: template_version,
            delivery_enabled,
        })
    }

    pub fn to_storage_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("bot_name".to_string(), Value::String(self.bot_name.clone()));
        object.insert(
            "parse_mode".to_string(),
            Value::String(self.parse_mode.clone()),
        );
        object.insert(
            "secret_ref".to_string(),
            Value::String(self.secret_ref.clone()),
        );
        object.insert(
            "message_template_version".to_string(),
            Value::String(self.message_template_version.clone()),
        );
        object.insert(
            "delivery_enabled".to_string(),
            Value::Bool(self.delivery_enabled),
        );
        if let Some(default_chat_id) = &self.default_chat_id {
            object.insert(
                "default_chat_id".to_string(),
                Value::String(default_chat_id.clone()),
            );
        }
        Value::Object(object)
    }

    pub fn to_sanitized_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("bot_name".to_string(), Value::String(self.bot_name.clone()));
        object.insert(
            "parse_mode".to_string(),
            Value::String(self.parse_mode.clone()),
        );
        object.insert(
            "message_template_version".to_string(),
            Value::String(self.message_template_version.clone()),
        );
        object.insert(
            "delivery_enabled".to_string(),
            Value::Bool(self.delivery_enabled),
        );
        object.insert("has_secret_ref".to_string(), Value::Bool(true));
        object.insert(
            "masked_secret_ref".to_string(),
            Value::String(mask_secret_ref(&self.secret_ref)),
        );
        if let Some(default_chat_id) = &self.default_chat_id {
            object.insert(
                "default_chat_id".to_string(),
                Value::String(default_chat_id.clone()),
            );
        }
        Value::Object(object)
    }
}

pub fn allowed_telegram_event_types() -> &'static [&'static str] {
    TELEGRAM_ALLOWED_EVENT_TYPES
}

pub fn normalize_telegram_config(
    kind: &str,
    integration_name: &str,
    config: &Value,
) -> AppResult<Value> {
    if kind != TELEGRAM_INTEGRATION_KIND {
        return Ok(config.clone());
    }
    TelegramIntegrationConfig::from_value(config, integration_name)
        .map(|value| value.to_storage_value())
}

pub fn merge_existing_telegram_config(
    kind: &str,
    integration_name: &str,
    existing: &Value,
    update: &Value,
) -> Value {
    if kind != TELEGRAM_INTEGRATION_KIND {
        return update.clone();
    }

    let Some(update_object) = update.as_object() else {
        return update.clone();
    };

    let has_secret_ref = optional_string(update_object, "secret_ref")
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    if has_secret_ref {
        return update.clone();
    }

    let Some(secret_ref) = TelegramIntegrationConfig::from_value(existing, integration_name)
        .ok()
        .map(|config| config.secret_ref)
        .filter(|value| !value.is_empty())
    else {
        return update.clone();
    };

    let mut merged = update_object.clone();
    merged.insert("secret_ref".to_string(), Value::String(secret_ref));
    Value::Object(merged)
}

pub fn sanitize_integration_model(mut integration: IntegrationModel) -> IntegrationModel {
    if integration.kind != TELEGRAM_INTEGRATION_KIND {
        return integration;
    }

    integration.config_json =
        sanitize_telegram_config_value(&integration.config_json, &integration.name);
    integration
}

pub fn sanitize_telegram_config_value(config: &Value, integration_name: &str) -> Value {
    match TelegramIntegrationConfig::from_value(config, integration_name) {
        Ok(config) => config.to_sanitized_value(),
        Err(_) => {
            let object = config.as_object().cloned().unwrap_or_default();
            json!({
                "bot_name": optional_string(&object, "bot_name").unwrap_or_else(|| integration_name.to_string()),
                "parse_mode": normalize_parse_mode(optional_string(&object, "parse_mode").as_deref()).unwrap_or_else(|_| TELEGRAM_PARSE_MODE_HTML.to_string()),
                "message_template_version": optional_string(&object, "message_template_version")
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| TELEGRAM_TEMPLATE_VERSION_V1.to_string()),
                "delivery_enabled": object.get("delivery_enabled").and_then(Value::as_bool).unwrap_or(true),
                "default_chat_id": optional_string(&object, "default_chat_id").unwrap_or_default(),
                "has_secret_ref": optional_string(&object, "secret_ref").map(|value| !value.is_empty()).unwrap_or(false),
                "masked_secret_ref": optional_string(&object, "secret_ref")
                    .filter(|value| !value.is_empty())
                    .map(|value| mask_secret_ref(&value))
                    .unwrap_or_default(),
            })
        }
    }
}

pub fn normalize_binding_scope(scope_type: &str, scope_id: Option<Uuid>) -> AppResult<String> {
    let normalized = scope_type.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" if scope_id.is_some() => Ok("cluster".to_string()),
        "cluster" if scope_id.is_some() => Ok("cluster".to_string()),
        "global" if scope_id.is_none() => Ok("global".to_string()),
        "" => Err(AppError::invalid_argument(
            "scope_type is required; empty values only normalize to `cluster` when scope_id is present",
        )),
        "cluster" => Err(AppError::invalid_argument(
            "cluster scope requires scope_id",
        )),
        "global" => Err(AppError::invalid_argument(
            "global scope must not include scope_id",
        )),
        _ => Err(AppError::invalid_argument("unsupported scope_type")),
    }
}

pub fn normalize_delivery_severity(severity: &str) -> AppResult<String> {
    match severity.trim().to_ascii_lowercase().as_str() {
        "info" => Ok("info".to_string()),
        "low" => Ok("low".to_string()),
        "warning" | "medium" => Ok("medium".to_string()),
        "high" => Ok("high".to_string()),
        "critical" => Ok("critical".to_string()),
        _ => Err(AppError::invalid_argument(format!(
            "unsupported severity {severity}"
        ))),
    }
}

pub fn severity_rank(severity: &str) -> AppResult<u8> {
    match normalize_delivery_severity(severity)?.as_str() {
        "info" => Ok(0),
        "low" => Ok(1),
        "medium" => Ok(2),
        "high" => Ok(3),
        "critical" => Ok(4),
        _ => Err(AppError::invalid_argument("unsupported severity")),
    }
}

pub fn normalize_binding_event_types(kind: &str, value: &Value) -> AppResult<Value> {
    let event_types = normalized_event_types(kind, value)?;
    Ok(json!(event_types))
}

pub fn telegram_binding_matches(
    event_types_json: &Value,
    threshold: &str,
    event_type: &str,
    severity: &str,
) -> bool {
    let Ok(event_types) = normalized_event_types(TELEGRAM_INTEGRATION_KIND, event_types_json)
    else {
        return false;
    };
    let Ok(event_rank) = severity_rank(severity) else {
        return false;
    };
    let Ok(threshold_rank) = severity_rank(threshold) else {
        return false;
    };

    event_types
        .iter()
        .any(|candidate| candidate == event_type.trim())
        && event_rank >= threshold_rank
}

pub fn mask_secret_ref(secret_ref: &str) -> String {
    let trimmed = secret_ref.trim();
    if trimmed.len() <= 12 {
        return "********".to_string();
    }

    let prefix_len = trimmed
        .char_indices()
        .nth(8)
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());
    let suffix_len = trimmed
        .char_indices()
        .rev()
        .nth(3)
        .map(|(index, _)| trimmed.len().saturating_sub(index))
        .unwrap_or(4);
    let suffix_start = trimmed.len().saturating_sub(suffix_len);

    format!("{}...{}", &trimmed[..prefix_len], &trimmed[suffix_start..])
}

fn validate_telegram_keys(object: &Map<String, Value>) -> AppResult<()> {
    for key in object.keys() {
        if !TELEGRAM_ALLOWED_INPUT_KEYS
            .iter()
            .any(|allowed| *allowed == key)
        {
            return Err(AppError::invalid_argument(format!(
                "unsupported telegram_bot config_json field `{key}`"
            )));
        }
    }
    Ok(())
}

fn reject_raw_token(object: &Map<String, Value>) -> AppResult<()> {
    if optional_string(object, "token")
        .map(|value| !value.is_empty())
        .unwrap_or(false)
    {
        return Err(AppError::invalid_argument(
            "telegram_bot config_json must use `secret_ref` instead of raw `token`",
        ));
    }
    Ok(())
}

fn normalize_parse_mode(parse_mode: Option<&str>) -> AppResult<String> {
    match parse_mode
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "html" => Ok(TELEGRAM_PARSE_MODE_HTML.to_string()),
        "plain" => Ok(TELEGRAM_PARSE_MODE_PLAIN.to_string()),
        _ => Err(AppError::invalid_argument(
            "telegram_bot config_json.parse_mode must be `HTML` or `plain`",
        )),
    }
}

fn normalized_event_types(kind: &str, value: &Value) -> AppResult<Vec<String>> {
    let array = value
        .as_array()
        .ok_or_else(|| AppError::invalid_argument("event_types_json must be a JSON array"))?;

    let mut seen = BTreeSet::new();
    let mut event_types = Vec::new();
    for entry in array {
        let entry = entry
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::invalid_argument("event_types_json must contain non-empty strings")
            })?
            .to_ascii_lowercase();

        if kind == TELEGRAM_INTEGRATION_KIND
            && !TELEGRAM_ALLOWED_EVENT_TYPES
                .iter()
                .any(|allowed| *allowed == entry)
        {
            return Err(AppError::invalid_argument(format!(
                "unsupported telegram event_type `{entry}`"
            )));
        }

        if seen.insert(entry.clone()) {
            event_types.push(entry);
        }
    }

    if kind == TELEGRAM_INTEGRATION_KIND {
        event_types.sort();
    }

    Ok(event_types)
}

fn optional_string(object: &Map<String, Value>, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .map(ToString::to_string)
}

fn required_string(object: &Map<String, Value>, field: &str, message: &str) -> AppResult<String> {
    optional_string(object, field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid_argument(message))
}

fn optional_bool(object: &Map<String, Value>, field: &str) -> AppResult<Option<bool>> {
    match object.get(field) {
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| AppError::invalid_argument(format!("{field} must be a boolean"))),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::{
        mask_secret_ref, merge_existing_telegram_config, normalize_binding_event_types,
        normalize_binding_scope, normalize_delivery_severity, normalize_telegram_config,
        sanitize_telegram_config_value, severity_rank, telegram_binding_matches,
        TELEGRAM_INTEGRATION_KIND,
    };

    #[test]
    fn normalizes_valid_telegram_config() {
        let config = normalize_telegram_config(
            TELEGRAM_INTEGRATION_KIND,
            "secops-primary",
            &json!({
                "secret_ref": "vault://kv/data/integrations/tg/secops",
                "delivery_enabled": true
            }),
        )
        .unwrap();

        assert_eq!(
            config,
            json!({
                "bot_name": "secops-primary",
                "parse_mode": "HTML",
                "secret_ref": "vault://kv/data/integrations/tg/secops",
                "message_template_version": "v1",
                "delivery_enabled": true
            })
        );
    }

    #[test]
    fn rejects_raw_token_input() {
        let error = normalize_telegram_config(
            TELEGRAM_INTEGRATION_KIND,
            "secops-primary",
            &json!({
                "token": "123:raw",
                "secret_ref": "vault://kv/data/integrations/tg/secops"
            }),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "telegram_bot config_json must use `secret_ref` instead of raw `token`"
        );
    }

    #[test]
    fn sanitizes_secret_ref_for_response() {
        let sanitized = sanitize_telegram_config_value(
            &json!({
                "bot_name": "secops-primary",
                "parse_mode": "HTML",
                "secret_ref": "vault://kv/data/integrations/tg/secops",
                "message_template_version": "v1",
                "delivery_enabled": true
            }),
            "secops-primary",
        );

        assert_eq!(
            sanitized,
            json!({
                "bot_name": "secops-primary",
                "parse_mode": "HTML",
                "message_template_version": "v1",
                "delivery_enabled": true,
                "has_secret_ref": true,
                "masked_secret_ref": mask_secret_ref("vault://kv/data/integrations/tg/secops")
            })
        );
    }

    #[test]
    fn preserves_existing_secret_ref_during_update_merge() {
        let merged = merge_existing_telegram_config(
            TELEGRAM_INTEGRATION_KIND,
            "secops-primary",
            &json!({
                "bot_name": "secops-primary",
                "parse_mode": "HTML",
                "secret_ref": "vault://kv/data/integrations/tg/secops",
                "message_template_version": "v1",
                "delivery_enabled": true
            }),
            &json!({
                "bot_name": "secops-renamed",
                "parse_mode": "plain",
                "default_chat_id": "-100100",
                "message_template_version": "v1",
                "delivery_enabled": false,
                "has_secret_ref": true,
                "masked_secret_ref": "vault://k...cops"
            }),
        );

        assert_eq!(
            merged.get("secret_ref").and_then(serde_json::Value::as_str),
            Some("vault://kv/data/integrations/tg/secops")
        );
    }

    #[test]
    fn normalizes_warning_to_medium() {
        assert_eq!(normalize_delivery_severity("warning").unwrap(), "medium");
        assert_eq!(
            severity_rank("warning").unwrap(),
            severity_rank("medium").unwrap()
        );
    }

    #[test]
    fn normalizes_cluster_scope_when_scope_id_exists() {
        assert_eq!(
            normalize_binding_scope("", Some(Uuid::new_v4())).unwrap(),
            "cluster"
        );
    }

    #[test]
    fn rejects_empty_global_default() {
        let error = normalize_binding_scope("", None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "scope_type is required; empty values only normalize to `cluster` when scope_id is present"
        );
    }

    #[test]
    fn normalizes_and_validates_telegram_event_types() {
        let event_types = normalize_binding_event_types(
            TELEGRAM_INTEGRATION_KIND,
            &json!(["alerts.firing", "security.finding.opened", "alerts.firing"]),
        )
        .unwrap();

        assert_eq!(
            event_types,
            json!(["alerts.firing", "security.finding.opened"])
        );
    }

    #[test]
    fn matches_binding_by_event_type_and_threshold() {
        assert!(telegram_binding_matches(
            &json!(["alerts.firing"]),
            "warning",
            "alerts.firing",
            "high"
        ));
        assert!(!telegram_binding_matches(
            &json!(["alerts.resolved"]),
            "medium",
            "alerts.firing",
            "high"
        ));
        assert!(!telegram_binding_matches(
            &json!(["alerts.firing"]),
            "critical",
            "alerts.firing",
            "medium"
        ));
    }
}
