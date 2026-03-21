use common::{AppError, AppResult};
use serde_json::Value;

pub fn policy_to_source_paths(policy_body_json: &Value) -> AppResult<Vec<String>> {
    if let Some(paths) = policy_body_json.get("paths").and_then(Value::as_array) {
        return collect_string_paths("paths", paths);
    }

    if let Some(sources) = policy_body_json.get("sources").and_then(Value::as_array) {
        if sources.iter().all(Value::is_string) {
            return collect_string_paths("sources", sources);
        }

        let mut rendered = Vec::new();
        for source in sources {
            let Some(object) = source.as_object() else {
                return Err(AppError::invalid_argument(
                    "policy sources entries must be strings or objects",
                ));
            };
            let kind = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("file")
                .trim();
            if kind != "file" {
                return Err(AppError::invalid_argument(format!(
                    "unsupported source type `{kind}` in deployment bootstrap"
                )));
            }
            let path = object
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::invalid_argument("file source path is required"))?;
            if path.contains('*') || path.contains('?') {
                return Err(AppError::invalid_argument(format!(
                    "glob paths are not supported in deployment bootstrap: {path}"
                )));
            }
            rendered.push(path.to_string());
        }
        return Ok(rendered);
    }

    Err(AppError::invalid_argument(
        "policy must define `paths` or `sources` for deployment bootstrap",
    ))
}

pub fn policy_to_source_paths_preview(
    policy_body_json: &Value,
) -> AppResult<(Vec<String>, Vec<String>)> {
    if let Some(paths) = policy_body_json.get("paths").and_then(Value::as_array) {
        return collect_preview_paths("paths", paths);
    }

    if let Some(sources) = policy_body_json.get("sources").and_then(Value::as_array) {
        if sources.iter().all(Value::is_string) {
            return collect_preview_paths("sources", sources);
        }

        let mut rendered = Vec::new();
        let mut warnings = Vec::new();
        for source in sources {
            let Some(object) = source.as_object() else {
                warnings
                    .push("policy sources entry is not an object and will be skipped".to_string());
                continue;
            };
            let kind = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("file")
                .trim();
            if kind != "file" {
                warnings.push(format!("unsupported source type `{kind}` will be skipped"));
                continue;
            }
            let Some(path) = object
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                warnings.push("file source without path will be skipped".to_string());
                continue;
            };
            if path.eq_ignore_ascii_case("journald") {
                warnings.push(
                    "journald source is not supported by current agent bootstrap".to_string(),
                );
                continue;
            }
            if path.contains('*') || path.contains('?') {
                warnings.push(format!(
                    "glob path `{path}` is not supported by current agent bootstrap"
                ));
                continue;
            }
            rendered.push(path.to_string());
        }
        return Ok((rendered, warnings));
    }

    Ok((
        Vec::new(),
        vec!["policy does not define `paths` or `sources` for deployment bootstrap".to_string()],
    ))
}

fn collect_string_paths(label: &str, values: &[Value]) -> AppResult<Vec<String>> {
    let mut rendered = Vec::new();
    for value in values {
        let path = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::invalid_argument(format!("{label} must contain strings")))?;
        if path.eq_ignore_ascii_case("journald") {
            return Err(AppError::invalid_argument(
                "journald sources are not supported in deployment bootstrap",
            ));
        }
        if path.contains('*') || path.contains('?') {
            return Err(AppError::invalid_argument(format!(
                "glob paths are not supported in deployment bootstrap: {path}"
            )));
        }
        rendered.push(path.to_string());
    }
    Ok(rendered)
}

fn collect_preview_paths(label: &str, values: &[Value]) -> AppResult<(Vec<String>, Vec<String>)> {
    let mut rendered = Vec::new();
    let mut warnings = Vec::new();
    for value in values {
        let Some(path) = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            warnings.push(format!(
                "{label} entry is not a non-empty string and will be skipped"
            ));
            continue;
        };
        if path.eq_ignore_ascii_case("journald") {
            warnings
                .push("journald source is not supported by current agent bootstrap".to_string());
            continue;
        }
        if path.contains('*') || path.contains('?') {
            warnings.push(format!(
                "glob path `{path}` is not supported by current agent bootstrap"
            ));
            continue;
        }
        rendered.push(path.to_string());
    }
    Ok((rendered, warnings))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::policy_to_source_paths;

    #[test]
    fn accepts_legacy_paths_shape() {
        let sources = policy_to_source_paths(&json!({ "paths": ["/var/log/syslog"] })).unwrap();
        assert_eq!(sources, vec!["/var/log/syslog"]);
    }

    #[test]
    fn rejects_globs() {
        let error = policy_to_source_paths(&json!({ "paths": ["/var/log/*.log"] })).unwrap_err();
        assert!(error.to_string().contains("glob"));
    }
}
