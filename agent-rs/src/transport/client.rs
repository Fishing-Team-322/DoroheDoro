use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    error::{AppError, AppResult},
    proto::{agent, edge, ingest},
    transport::{
        AgentTransport, EnrollRequest, EnrollResponse, FetchPolicyRequest, PolicySnapshot,
    },
};

const GRPC_OK: i32 = 0;
const SERVICE_PREFIX: &str = "/dorohedoro.edge.v1.AgentIngressService";

#[derive(Debug, Clone)]
pub struct EdgeGrpcTransport {
    client: Client,
    base_url: String,
}

impl EdgeGrpcTransport {
    pub fn new(edge_url: &str, edge_grpc_addr: &str) -> AppResult<Self> {
        let base_url = build_base_url(edge_url, edge_grpc_addr)?;
        let scheme = reqwest::Url::parse(&base_url)
            .map_err(|error| AppError::invalid_config(format!("invalid edge grpc url: {error}")))?
            .scheme()
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/grpc+json"),
        );
        headers.insert("te", HeaderValue::from_static("trailers"));
        headers.insert("grpc-accept-encoding", HeaderValue::from_static("identity"));

        let builder = if scheme == "http" {
            Client::builder()
                .default_headers(headers)
                .http2_prior_knowledge()
        } else {
            Client::builder().default_headers(headers).use_rustls_tls()
        };

        let client = builder.timeout(Duration::from_secs(15)).build()?;

        Ok(Self { client, base_url })
    }

    async fn unary_json<Request, Response>(
        &self,
        method: &str,
        request: &Request,
    ) -> AppResult<Response>
    where
        Request: Serialize + Sync,
        Response: DeserializeOwned,
    {
        let payload = serde_json::to_vec(request)?;
        let mut framed = BytesMut::with_capacity(5 + payload.len());
        framed.put_u8(0);
        framed.put_u32(payload.len() as u32);
        framed.extend_from_slice(&payload);

        let url = format!("{}{}{}", self.base_url, SERVICE_PREFIX, method);
        let mut response = self.client.post(url).body(framed.freeze()).send().await?;
        let http_status = response.status();
        let headers = response.headers().clone();

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            body.extend_from_slice(&chunk);
        }
        if !http_status.is_success() {
            return Err(AppError::HttpStatus {
                status: http_status,
                message: String::from_utf8_lossy(&body).into_owned(),
            });
        }

        let grpc_status = grpc_status(&headers)?;
        if grpc_status.0 != GRPC_OK {
            return Err(AppError::grpc_status(grpc_status.0, grpc_status.1));
        }

        decode_grpc_frame(&body)
    }
}

#[async_trait]
impl AgentTransport for EdgeGrpcTransport {
    async fn enroll(&self, request: EnrollRequest) -> AppResult<EnrollResponse> {
        let mut labels = request.metadata;
        labels.insert("version".to_string(), request.version);
        let response: edge::EnrollResponse = self
            .unary_json(
                "/Enroll",
                &edge::EnrollRequest {
                    enrollment_token: request.bootstrap_token,
                    host: request.hostname,
                    labels,
                },
            )
            .await?;

        Ok(EnrollResponse {
            agent_id: response.agent_id,
            status: response.status,
        })
    }

    async fn fetch_policy(&self, request: FetchPolicyRequest) -> AppResult<PolicySnapshot> {
        let response: edge::FetchPolicyResponse = self
            .unary_json(
                "/FetchPolicy",
                &edge::FetchPolicyRequest {
                    agent_id: request.agent_id.clone(),
                    current_revision: request.current_revision.unwrap_or_default(),
                },
            )
            .await?;

        let policy = response
            .policy
            .ok_or_else(|| AppError::protocol("fetch policy response is missing `policy`"))?;

        Ok(PolicySnapshot {
            agent_id: request.agent_id,
            policy_id: policy.policy_id,
            policy_revision: policy.revision,
            policy_body_json: policy.body_json,
            status: if response.changed {
                "changed".to_string()
            } else {
                "unchanged".to_string()
            },
        })
    }

    async fn send_heartbeat(&self, payload: agent::HeartbeatPayload) -> AppResult<()> {
        let _: edge::Ack = self
            .unary_json(
                "/SendHeartbeat",
                &edge::HeartbeatRequest {
                    agent_id: payload.agent_id,
                    host: payload.hostname,
                    sent_at_unix_ms: payload.sent_at_unix_ms,
                    status: payload.status,
                },
            )
            .await?;
        Ok(())
    }

    async fn send_batch(&self, batch: ingest::LogBatch) -> AppResult<()> {
        let events = batch
            .events
            .into_iter()
            .map(|event| {
                let mut labels = BTreeMap::new();
                labels.extend(event.labels);
                labels.entry("source".to_string()).or_insert(event.source);
                labels
                    .entry("source_type".to_string())
                    .or_insert(event.source_type);
                edge::AgentLog {
                    timestamp_unix_ms: event.timestamp_unix_ms,
                    service: event.service,
                    severity: event.severity,
                    message: event.message,
                    labels,
                }
            })
            .collect::<Vec<_>>();

        let response: edge::IngestLogsResponse = self
            .unary_json(
                "/IngestLogs",
                &edge::IngestLogsRequest {
                    agent_id: batch.agent_id,
                    host: batch.host,
                    sent_at_unix_ms: batch.sent_at_unix_ms,
                    events,
                },
            )
            .await?;

        if !response.accepted {
            return Err(AppError::protocol(
                "edge ingest response rejected the batch",
            ));
        }

        Ok(())
    }

    async fn send_diagnostics(&self, payload: agent::DiagnosticsPayload) -> AppResult<()> {
        let diagnostics: crate::runtime::DiagnosticsSnapshot =
            serde_json::from_str(&payload.payload_json)?;
        let _: edge::Ack = self
            .unary_json(
                "/SendDiagnostics",
                &edge::DiagnosticsRequest {
                    agent_id: payload.agent_id,
                    host: diagnostics.hostname,
                    sent_at_unix_ms: payload.sent_at_unix_ms,
                    payload_json: payload.payload_json,
                },
            )
            .await?;
        Ok(())
    }
}

fn build_base_url(edge_url: &str, edge_grpc_addr: &str) -> AppResult<String> {
    if edge_grpc_addr.contains("://") {
        return Ok(edge_grpc_addr.trim_end_matches('/').to_string());
    }

    let scheme = if edge_url.trim_start().starts_with("https://") {
        "https"
    } else {
        "http"
    };
    Ok(format!(
        "{scheme}://{}",
        edge_grpc_addr.trim_end_matches('/')
    ))
}

fn grpc_status(headers: &HeaderMap) -> AppResult<(i32, String)> {
    let status = headers
        .get("grpc-status")
        .map(|value| value.to_str().unwrap_or("0"))
        .unwrap_or("0");
    let message = headers
        .get("grpc-message")
        .map(|value| value.to_str().unwrap_or_default().to_string())
        .unwrap_or_default();

    let code = status
        .parse::<i32>()
        .map_err(|error| AppError::protocol(format!("invalid grpc status `{status}`: {error}")))?;

    Ok((code, message))
}

fn decode_grpc_frame<Response>(body: &[u8]) -> AppResult<Response>
where
    Response: DeserializeOwned,
{
    if body.len() < 5 {
        return Err(AppError::protocol(
            "grpc body is smaller than a unary frame",
        ));
    }
    if body[0] != 0 {
        return Err(AppError::protocol(
            "compressed grpc responses are not supported",
        ));
    }

    let expected_len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    let payload = &body[5..];
    if payload.len() != expected_len {
        return Err(AppError::protocol(format!(
            "grpc frame length mismatch: expected {expected_len}, got {}",
            payload.len()
        )));
    }

    Ok(serde_json::from_slice(payload)?)
}

#[cfg(test)]
mod tests {
    use crate::proto::edge::IngestLogsResponse;

    use super::{build_base_url, decode_grpc_frame};

    #[test]
    fn builds_url_from_edge_address() {
        assert_eq!(
            build_base_url("https://edge.example.local", "edge.example.local:7443").unwrap(),
            "https://edge.example.local:7443"
        );
        assert_eq!(
            build_base_url("http://localhost:8080", "localhost:9090").unwrap(),
            "http://localhost:9090"
        );
    }

    #[test]
    fn decodes_grpc_json_frame() {
        let payload = br#"{"accepted":true,"accepted_count":2,"request_id":"req-1"}"#;
        let mut frame = Vec::new();
        frame.push(0);
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload);

        let decoded: IngestLogsResponse = decode_grpc_frame(&frame).unwrap();
        assert!(decoded.accepted);
        assert_eq!(decoded.accepted_count, 2);
    }
}
