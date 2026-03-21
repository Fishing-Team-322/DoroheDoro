use std::path::Path;

use serde_json::Value;

use crate::{
    config::{SourceConfig, StartAt},
    error::{AppError, AppResult},
};

pub fn parse_file_sources(policy_body_json: &str) -> AppResult<Vec<SourceConfig>> {
    let value: Value = serde_json::from_str(policy_body_json)
        .map_err(|error| AppError::protocol(format!("invalid policy json: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| AppError::protocol("policy body must be a JSON object"))?;

    if let Some(paths) = object.get("paths") {
        return parse_string_sources("paths", paths);
    }

    if let Some(sources) = object.get("sources") {
        let entries = sources
            .as_array()
            .ok_or_else(|| AppError::protocol("policy `sources` must be an array"))?;
        if entries.iter().all(Value::is_string) {
            return parse_string_sources("sources", sources);
        }

        let mut rendered = Vec::with_capacity(entries.len());
        for entry in entries {
            rendered.push(parse_object_source(entry)?);
        }
        if rendered.is_empty() {
            return Err(AppError::protocol(
                "policy `sources` must contain at least one file source",
            ));
        }
        return Ok(rendered);
    }

    Err(AppError::protocol(
        "policy must define supported `paths` or `sources` sections",
    ))
}

fn parse_string_sources(label: &str, value: &Value) -> AppResult<Vec<SourceConfig>> {
    let values = value
        .as_array()
        .ok_or_else(|| AppError::protocol(format!("policy `{label}` must be an array")))?;
    if values.is_empty() {
        return Err(AppError::protocol(format!(
            "policy `{label}` must not be empty"
        )));
    }

    let mut rendered = Vec::with_capacity(values.len());
    for item in values {
        let path = item
            .as_str()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| {
                AppError::protocol(format!("policy `{label}` must contain non-empty strings"))
            })?;
        validate_file_path(path)?;
        rendered.push(normalized_source_from_path(path));
    }
    Ok(rendered)
}

fn parse_object_source(value: &Value) -> AppResult<SourceConfig> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::protocol("policy source entries must be strings or objects"))?;
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("file")
        .trim();
    if kind != "file" {
        return Err(AppError::protocol(format!(
            "unsupported policy source type `{kind}`"
        )));
    }

    let path = object
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| AppError::protocol("policy file source path is required"))?;
    validate_file_path(path)?;

    let source_id = optional_non_empty(object.get("source_id").and_then(Value::as_str))
        .unwrap_or_else(|| default_source_id(path));
    let source = optional_non_empty(object.get("source").and_then(Value::as_str))
        .unwrap_or_else(|| default_source_name(path));
    let service = optional_non_empty(object.get("service").and_then(Value::as_str))
        .unwrap_or_else(|| "host".to_string());
    let severity_hint = optional_non_empty(object.get("severity_hint").and_then(Value::as_str))
        .unwrap_or_else(|| "info".to_string());
    let start_at =
        match optional_non_empty(object.get("start_at").and_then(Value::as_str)).as_deref() {
            Some("beginning") => StartAt::Beginning,
            Some("end") | None => StartAt::End,
            Some(other) => {
                return Err(AppError::protocol(format!(
                    "unsupported start_at `{other}` for policy file source"
                )))
            }
        };

    Ok(SourceConfig {
        kind: "file".to_string(),
        source_id: Some(source_id),
        path: path.into(),
        start_at,
        source,
        service,
        severity_hint,
    })
}

fn validate_file_path(path: &str) -> AppResult<()> {
    if path.eq_ignore_ascii_case("journald") {
        return Err(AppError::protocol(
            "journald sources are not supported by the current agent runtime",
        ));
    }
    if path.contains('*') || path.contains('?') {
        return Err(AppError::protocol(format!(
            "glob paths are not supported by the current agent runtime: {path}"
        )));
    }
    Ok(())
}

fn normalized_source_from_path(path: &str) -> SourceConfig {
    SourceConfig {
        kind: "file".to_string(),
        source_id: Some(default_source_id(path)),
        path: path.into(),
        start_at: StartAt::End,
        source: default_source_name(path),
        service: "host".to_string(),
        severity_hint: "info".to_string(),
    }
}

fn default_source_id(path: &str) -> String {
    format!("file:{path}")
}

fn default_source_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("host-log")
        .to_string()
}

fn optional_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use crate::config::StartAt;

    use super::parse_file_sources;

    #[test]
    fn parses_legacy_paths_shape() {
        let sources = parse_file_sources(r#"{"paths":["/var/log/syslog"]}"#).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id(), "file:/var/log/syslog");
        assert_eq!(sources[0].start_at, StartAt::End);
        assert_eq!(sources[0].source, "syslog");
        assert_eq!(sources[0].service, "host");
    }

    #[test]
    fn parses_object_sources_with_defaults() {
        let sources = parse_file_sources(
            r#"{"sources":[{"type":"file","path":"/var/log/nginx/access.log"}]}"#,
        )
        .unwrap();

        assert_eq!(sources[0].source_id(), "file:/var/log/nginx/access.log");
        assert_eq!(sources[0].source, "access.log");
        assert_eq!(sources[0].severity_hint, "info");
    }

    #[test]
    fn rejects_globs_and_journald() {
        let glob_error = parse_file_sources(r#"{"paths":["/var/log/*.log"]}"#).unwrap_err();
        assert!(glob_error.to_string().contains("glob"));

        let journald_error = parse_file_sources(r#"{"paths":["journald"]}"#).unwrap_err();
        assert!(journald_error.to_string().contains("journald"));
    }

    #[test]
    fn rejects_unsupported_source_types() {
        let error = parse_file_sources(r#"{"sources":[{"type":"journald","path":"journald"}]}"#)
            .unwrap_err();
        assert!(error.to_string().contains("unsupported policy source type"));
    }
}
