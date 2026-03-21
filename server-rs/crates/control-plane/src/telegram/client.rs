use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::config::TelegramRuntimeConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramSendSuccess {
    pub telegram_message_id: String,
    pub status_code: String,
    pub status_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramSendFailure {
    pub classification: String,
    pub status_code: String,
    pub status_message: String,
    pub status_severity: String,
    pub suggested_action: String,
    pub retry_after_seconds: Option<u32>,
    pub http_status: Option<u16>,
}

impl TelegramSendFailure {
    pub fn is_retryable(&self) -> bool {
        self.classification == "retryable"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramSendOutcome {
    Success(TelegramSendSuccess),
    Failure(TelegramSendFailure),
}

#[derive(Clone)]
pub struct TelegramBotClient {
    http: reqwest::Client,
    api_base_url: String,
}

impl TelegramBotClient {
    pub fn from_config(config: &TelegramRuntimeConfig) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .build()?;
        Ok(Self {
            http,
            api_base_url: config.api_base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn send_message(
        &self,
        bot_token: &str,
        chat_id: &str,
        text: &str,
        parse_mode: &str,
    ) -> TelegramSendOutcome {
        if bot_token.trim().is_empty() {
            return TelegramSendOutcome::Failure(invalid_configuration(
                "telegram_bot.secret_ref",
                "telegram bot token is not available",
                "Check Vault secret contents for a non-empty token value.",
            ));
        }
        if chat_id.trim().is_empty() {
            return TelegramSendOutcome::Failure(invalid_configuration(
                "telegram_bot.default_chat_id",
                "telegram chat_id is required",
                "Configure default_chat_id or provide a chat_id override for the healthcheck.",
            ));
        }
        if text.trim().is_empty() {
            return TelegramSendOutcome::Failure(invalid_configuration(
                "telegram_message_empty",
                "telegram message text is empty",
                "Inspect the notification renderer input and message template configuration.",
            ));
        }

        let parse_mode = normalize_parse_mode(parse_mode);
        let request = self
            .http
            .post(format!(
                "{}/bot{}/sendMessage",
                self.api_base_url,
                bot_token.trim()
            ))
            .json(&TelegramSendMessageRequest {
                chat_id: chat_id.trim(),
                text,
                parse_mode: parse_mode.as_deref(),
                disable_web_page_preview: true,
            });

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                return TelegramSendOutcome::Failure(classify_reqwest_error(error));
            }
        };

        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                return TelegramSendOutcome::Failure(TelegramSendFailure {
                    classification: "retryable".to_string(),
                    status_code: "telegram_response_read_failed".to_string(),
                    status_message: format!("failed to read telegram response body: {error}"),
                    status_severity: "warning".to_string(),
                    suggested_action:
                        "Retry the request; if the issue persists, inspect network stability."
                            .to_string(),
                    retry_after_seconds: None,
                    http_status: Some(status.as_u16()),
                });
            }
        };

        match classify_response(status, &body) {
            TelegramSendOutcome::Success(success) => TelegramSendOutcome::Success(success),
            TelegramSendOutcome::Failure(failure) => TelegramSendOutcome::Failure(failure),
        }
    }
}

#[derive(Debug, Serialize)]
struct TelegramSendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'a str>,
    disable_web_page_preview: bool,
}

#[derive(Debug, Deserialize)]
struct TelegramApiResponse {
    ok: bool,
    #[serde(default)]
    result: Option<TelegramMessageResult>,
    #[serde(default)]
    error_code: Option<i64>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parameters: Option<TelegramResponseParameters>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessageResult {
    message_id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramResponseParameters {
    #[serde(default)]
    retry_after: Option<u32>,
}

fn normalize_parse_mode(parse_mode: &str) -> Option<&str> {
    match parse_mode.trim().to_ascii_lowercase().as_str() {
        "html" => Some("HTML"),
        _ => None,
    }
}

fn classify_reqwest_error(error: reqwest::Error) -> TelegramSendFailure {
    let (status_code, status_message, status_severity, suggested_action) = if error.is_timeout() {
        (
            "network_timeout",
            format!("telegram request timed out: {error}"),
            "warning",
            "Retry the request or increase TELEGRAM_REQUEST_TIMEOUT_MS if the upstream is consistently slow.",
        )
    } else if error.is_connect() {
        (
            "network_connect_failed",
            format!("failed to connect to telegram api: {error}"),
            "warning",
            "Verify egress connectivity to the Telegram Bot API endpoint.",
        )
    } else {
        (
            "telegram_request_failed",
            format!("telegram request failed: {error}"),
            "warning",
            "Retry the request; if the issue persists, inspect proxy or DNS configuration.",
        )
    };

    TelegramSendFailure {
        classification: "retryable".to_string(),
        status_code: status_code.to_string(),
        status_message,
        status_severity: status_severity.to_string(),
        suggested_action: suggested_action.to_string(),
        retry_after_seconds: None,
        http_status: error.status().map(|status| status.as_u16()),
    }
}

fn classify_response(status: StatusCode, body: &str) -> TelegramSendOutcome {
    match serde_json::from_str::<TelegramApiResponse>(body) {
        Ok(payload) if payload.ok => {
            let telegram_message_id = payload
                .result
                .map(|result| result.message_id.to_string())
                .unwrap_or_default();
            TelegramSendOutcome::Success(TelegramSendSuccess {
                telegram_message_id,
                status_code: "telegram_message_sent".to_string(),
                status_message: "telegram message delivered".to_string(),
            })
        }
        Ok(payload) => TelegramSendOutcome::Failure(classify_telegram_error(status, payload)),
        Err(_) if status.is_server_error() => TelegramSendOutcome::Failure(TelegramSendFailure {
            classification: "retryable".to_string(),
            status_code: "telegram_http_5xx".to_string(),
            status_message: format!("telegram api returned {status}: {body}"),
            status_severity: "warning".to_string(),
            suggested_action: "Retry the request; if repeated, inspect Telegram availability."
                .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        }),
        Err(error) => TelegramSendOutcome::Failure(TelegramSendFailure {
            classification: "retryable".to_string(),
            status_code: "telegram_response_decode_failed".to_string(),
            status_message: format!("failed to decode telegram response: {error}; body={body}"),
            status_severity: "warning".to_string(),
            suggested_action:
                "Retry the request; if repeated, inspect the Telegram API response shape."
                    .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        }),
    }
}

fn classify_telegram_error(
    status: StatusCode,
    payload: TelegramApiResponse,
) -> TelegramSendFailure {
    let error_code = payload.error_code.unwrap_or(status.as_u16() as i64);
    let description = payload
        .description
        .unwrap_or_else(|| "telegram request failed".to_string());
    let lowered = description.to_ascii_lowercase();

    if status == StatusCode::TOO_MANY_REQUESTS || error_code == 429 {
        let retry_after = payload
            .parameters
            .and_then(|parameters| parameters.retry_after)
            .unwrap_or(30);
        return TelegramSendFailure {
            classification: "retryable".to_string(),
            status_code: "telegram_flood_wait".to_string(),
            status_message: description,
            status_severity: "warning".to_string(),
            suggested_action: "Respect the retry_after delay before the next delivery attempt."
                .to_string(),
            retry_after_seconds: Some(retry_after),
            http_status: Some(status.as_u16()),
        };
    }

    if status.is_server_error() || error_code >= 500 {
        return TelegramSendFailure {
            classification: "retryable".to_string(),
            status_code: "telegram_upstream_unavailable".to_string(),
            status_message: description,
            status_severity: "warning".to_string(),
            suggested_action: "Retry the request; if repeated, inspect Telegram API availability."
                .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        };
    }

    if status == StatusCode::UNAUTHORIZED || error_code == 401 {
        return TelegramSendFailure {
            classification: "permanent".to_string(),
            status_code: "telegram_invalid_token".to_string(),
            status_message: description,
            status_severity: "error".to_string(),
            suggested_action:
                "Fix the Vault secret referenced by config_json.secret_ref and re-run the healthcheck."
                    .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        };
    }

    if status == StatusCode::FORBIDDEN || error_code == 403 {
        return TelegramSendFailure {
            classification: "permanent".to_string(),
            status_code: "telegram_forbidden".to_string(),
            status_message: description,
            status_severity: "error".to_string(),
            suggested_action:
                "Add the bot to the target chat and confirm it has permission to post messages."
                    .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        };
    }

    if lowered.contains("chat not found")
        || lowered.contains("chat_id")
        || lowered.contains("peer_id_invalid")
        || lowered.contains("peer id invalid")
    {
        return TelegramSendFailure {
            classification: "permanent".to_string(),
            status_code: "telegram_invalid_chat".to_string(),
            status_message: description,
            status_severity: "error".to_string(),
            suggested_action:
                "Fix default_chat_id or the healthcheck chat override and make sure the bot can access that chat."
                    .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        };
    }

    if lowered.contains("message text is empty")
        || lowered.contains("can't parse entities")
        || lowered.contains("message is too long")
    {
        return TelegramSendFailure {
            classification: "invalid_configuration".to_string(),
            status_code: "telegram_message_invalid".to_string(),
            status_message: description,
            status_severity: "error".to_string(),
            suggested_action:
                "Inspect the rendered payload and parse_mode configuration before retrying."
                    .to_string(),
            retry_after_seconds: None,
            http_status: Some(status.as_u16()),
        };
    }

    TelegramSendFailure {
        classification: "permanent".to_string(),
        status_code: "telegram_request_rejected".to_string(),
        status_message: description,
        status_severity: "error".to_string(),
        suggested_action:
            "Inspect the telegram integration configuration and verify the target bot/chat pair."
                .to_string(),
        retry_after_seconds: None,
        http_status: Some(status.as_u16()),
    }
}

fn invalid_configuration(
    status_code: &str,
    status_message: &str,
    suggested_action: &str,
) -> TelegramSendFailure {
    TelegramSendFailure {
        classification: "invalid_configuration".to_string(),
        status_code: status_code.to_string(),
        status_message: status_message.to_string(),
        status_severity: "error".to_string(),
        suggested_action: suggested_action.to_string(),
        retry_after_seconds: None,
        http_status: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_response, TelegramSendOutcome};

    #[test]
    fn classifies_successful_response() {
        let outcome = classify_response(
            reqwest::StatusCode::OK,
            r#"{"ok":true,"result":{"message_id":12345}}"#,
        );

        match outcome {
            TelegramSendOutcome::Success(success) => {
                assert_eq!(success.telegram_message_id, "12345");
                assert_eq!(success.status_code, "telegram_message_sent");
            }
            TelegramSendOutcome::Failure(failure) => {
                panic!("unexpected failure: {:?}", failure);
            }
        }
    }

    #[test]
    fn classifies_invalid_token() {
        let outcome = classify_response(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"ok":false,"error_code":401,"description":"Unauthorized"}"#,
        );

        match outcome {
            TelegramSendOutcome::Failure(failure) => {
                assert_eq!(failure.classification, "permanent");
                assert_eq!(failure.status_code, "telegram_invalid_token");
            }
            TelegramSendOutcome::Success(success) => {
                panic!("unexpected success: {:?}", success);
            }
        }
    }

    #[test]
    fn classifies_flood_wait() {
        let outcome = classify_response(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            r#"{"ok":false,"error_code":429,"description":"Too Many Requests: retry after 37","parameters":{"retry_after":37}}"#,
        );

        match outcome {
            TelegramSendOutcome::Failure(failure) => {
                assert_eq!(failure.classification, "retryable");
                assert_eq!(failure.status_code, "telegram_flood_wait");
                assert_eq!(failure.retry_after_seconds, Some(37));
            }
            TelegramSendOutcome::Success(success) => {
                panic!("unexpected success: {:?}", success);
            }
        }
    }

    #[test]
    fn classifies_invalid_chat() {
        let outcome = classify_response(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#,
        );

        match outcome {
            TelegramSendOutcome::Failure(failure) => {
                assert_eq!(failure.classification, "permanent");
                assert_eq!(failure.status_code, "telegram_invalid_chat");
            }
            TelegramSendOutcome::Success(success) => {
                panic!("unexpected success: {:?}", success);
            }
        }
    }
}
