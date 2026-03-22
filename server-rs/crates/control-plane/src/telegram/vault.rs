use serde_json::Value;

use common::{AppError, AppResult};

use crate::config::VaultRuntimeConfig;

#[derive(Debug, Clone)]
pub struct VaultSecretMap {
    data: serde_json::Map<String, Value>,
}

impl VaultSecretMap {
    pub fn get_first_string(&self, keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|key| {
            self.data
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramSecretMaterial {
    pub bot_token: String,
}

pub async fn read_telegram_secret(
    config: &VaultRuntimeConfig,
    vault_ref: &str,
) -> AppResult<TelegramSecretMaterial> {
    let secret = read_secret(config, vault_ref).await?;
    let bot_token = secret
        .get_first_string(&["bot_token", "token", "telegram_token"])
        .ok_or_else(|| {
            AppError::internal(format!(
                "vault secret `{vault_ref}` is missing telegram bot token material"
            ))
        })?;

    Ok(TelegramSecretMaterial { bot_token })
}

async fn read_secret(config: &VaultRuntimeConfig, vault_ref: &str) -> AppResult<VaultSecretMap> {
    let token = login(config).await?;
    let normalized_ref = normalize_vault_ref(vault_ref);
    let url = format!(
        "{}/v1/{}",
        config.addr.trim_end_matches('/'),
        normalized_ref.trim_start_matches('/')
    );
    let payload = reqwest::Client::new()
        .get(url)
        .header("X-Vault-Token", token)
        .send()
        .await
        .map_err(|error| {
            AppError::internal(format!("request vault secret `{vault_ref}`: {error}"))
        })?
        .error_for_status()
        .map_err(|error| {
            AppError::internal(format!("request vault secret `{vault_ref}`: {error}"))
        })?
        .json::<Value>()
        .await
        .map_err(|error| {
            AppError::internal(format!("decode vault secret `{vault_ref}`: {error}"))
        })?;

    extract_secret_map(payload).ok_or_else(|| {
        AppError::internal(format!(
            "vault secret `{vault_ref}` did not return a kv-like data object"
        ))
    })
}

fn normalize_vault_ref(vault_ref: &str) -> String {
    vault_ref
        .trim()
        .strip_prefix("vault://")
        .unwrap_or_else(|| vault_ref.trim())
        .trim_start_matches('/')
        .to_string()
}

async fn login(config: &VaultRuntimeConfig) -> AppResult<String> {
    let url = format!(
        "{}/v1/auth/approle/login",
        config.addr.trim_end_matches('/')
    );
    let payload = reqwest::Client::new()
        .post(url)
        .json(&serde_json::json!({
            "role_id": config.role_id,
            "secret_id": config.secret_id,
        }))
        .send()
        .await
        .map_err(|error| AppError::internal(format!("vault approle login: {error}")))?
        .error_for_status()
        .map_err(|error| AppError::internal(format!("vault approle login: {error}")))?
        .json::<Value>()
        .await
        .map_err(|error| {
            AppError::internal(format!("decode vault approle login response: {error}"))
        })?;

    payload
        .get("auth")
        .and_then(|value| value.get("client_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::internal("vault approle login response is missing auth.client_token")
        })
}

fn extract_secret_map(payload: Value) -> Option<VaultSecretMap> {
    let top = payload.get("data")?;
    let nested = top
        .get("data")
        .and_then(Value::as_object)
        .cloned()
        .or_else(|| top.as_object().cloned())?;
    Some(VaultSecretMap { data: nested })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{extract_secret_map, normalize_vault_ref, TelegramSecretMaterial, VaultSecretMap};

    #[test]
    fn extracts_kv_v2_secret_payload() {
        let secret = extract_secret_map(json!({
            "data": {
                "data": {
                    "bot_token": "12345:abc"
                }
            }
        }))
        .unwrap();

        assert_eq!(
            secret.get_first_string(&["bot_token"]).as_deref(),
            Some("12345:abc")
        );
    }

    #[test]
    fn resolves_first_matching_token_key() {
        let secret = VaultSecretMap {
            data: serde_json::from_value(json!({
                "telegram_token": "999:xyz"
            }))
            .unwrap(),
        };

        let material = TelegramSecretMaterial {
            bot_token: secret
                .get_first_string(&["bot_token", "token", "telegram_token"])
                .unwrap(),
        };

        assert_eq!(material.bot_token, "999:xyz");
    }

    #[test]
    fn normalizes_vault_scheme_prefix() {
        assert_eq!(
            normalize_vault_ref("vault://secret/data/integrations/tg/secops"),
            "secret/data/integrations/tg/secops"
        );
        assert_eq!(
            normalize_vault_ref("secret/data/integrations/tg/secops"),
            "secret/data/integrations/tg/secops"
        );
    }
}
