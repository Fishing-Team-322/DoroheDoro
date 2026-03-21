use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    path::Path,
    time::Duration,
};

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Certificate, Client, Identity, Url,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    config::TlsConfig,
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

#[derive(Debug, Clone)]
struct PreparedEndpoint {
    url: Url,
    resolve: Option<SocketAddr>,
}

impl EdgeGrpcTransport {
    pub fn new(edge_url: &str, edge_grpc_addr: &str, tls: &TlsConfig) -> AppResult<Self> {
        let base_url = build_base_url(edge_url, edge_grpc_addr)?;
        let prepared = prepare_endpoint(&base_url, tls.server_name.as_deref())?;
        let tls_configured = tls.ca_path.is_some()
            || tls.cert_path.is_some()
            || tls.key_path.is_some()
            || tls
                .server_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some();

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/grpc+json"),
        );
        headers.insert("te", HeaderValue::from_static("trailers"));
        headers.insert("grpc-accept-encoding", HeaderValue::from_static("identity"));

        let mut builder = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(15));

        if prepared.url.scheme() == "http" {
            if tls_configured {
                return Err(AppError::invalid_config(
                    "TLS settings are configured but the gRPC endpoint uses plain HTTP",
                ));
            }
            builder = builder.http2_prior_knowledge();
        } else {
            builder = builder.use_rustls_tls();
            if let Some(path) = tls.ca_path.as_deref() {
                let certificate = load_ca_certificate(path)?;
                builder = builder.add_root_certificate(certificate);
            }
            if let (Some(cert_path), Some(key_path)) =
                (tls.cert_path.as_deref(), tls.key_path.as_deref())
            {
                let identity = load_client_identity(cert_path, key_path)?;
                builder = builder.identity(identity);
            }
            if let Some(resolve) = prepared.resolve {
                if let Some(host) = prepared.url.host_str() {
                    builder = builder.resolve(host, resolve);
                }
            }
        }

        let client = builder.build()?;

        Ok(Self {
            client,
            base_url: prepared.url.to_string().trim_end_matches('/').to_string(),
        })
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

pub fn build_base_url(edge_url: &str, edge_grpc_addr: &str) -> AppResult<String> {
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

pub fn endpoint_uses_tls(edge_url: &str, edge_grpc_addr: &str) -> AppResult<bool> {
    let base_url = build_base_url(edge_url, edge_grpc_addr)?;
    let url = Url::parse(&base_url)
        .map_err(|error| AppError::invalid_config(format!("invalid edge grpc url: {error}")))?;
    Ok(url.scheme() == "https")
}

pub fn derive_server_name(
    edge_url: &str,
    edge_grpc_addr: &str,
    configured_server_name: Option<&str>,
) -> AppResult<Option<String>> {
    if let Some(server_name) = configured_server_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(server_name.to_string()));
    }

    let base_url = build_base_url(edge_url, edge_grpc_addr)?;
    let url = Url::parse(&base_url)
        .map_err(|error| AppError::invalid_config(format!("invalid edge grpc url: {error}")))?;
    if let Some(host) = url.host_str() {
        if host.parse::<IpAddr>().is_err() {
            return Ok(Some(host.to_string()));
        }
    }

    let edge_url = Url::parse(edge_url)
        .map_err(|error| AppError::invalid_config(format!("invalid edge_url: {error}")))?;
    Ok(edge_url
        .host_str()
        .map(ToOwned::to_owned)
        .or_else(|| url.host_str().map(ToOwned::to_owned)))
}

pub(crate) fn load_ca_certificate(path: &Path) -> AppResult<Certificate> {
    let pem = std::fs::read(path)?;
    require_pem_block(&pem, "CERTIFICATE", path, "tls.ca_path")?;
    Certificate::from_pem(&pem).map_err(|error| {
        AppError::invalid_config(format!(
            "failed to parse tls.ca_path `{}`: {error}",
            path.display()
        ))
    })
}

pub(crate) fn load_client_identity(cert_path: &Path, key_path: &Path) -> AppResult<Identity> {
    let cert_pem = std::fs::read(cert_path)?;
    require_pem_block(&cert_pem, "CERTIFICATE", cert_path, "tls.cert_path")?;

    let key_pem = std::fs::read(key_path)?;
    let key_text = std::str::from_utf8(&key_pem).map_err(|error| {
        AppError::invalid_config(format!(
            "failed to parse tls.key_path `{}` as UTF-8 PEM: {error}",
            key_path.display()
        ))
    })?;
    if !key_text.contains("-----BEGIN PRIVATE KEY-----")
        && !key_text.contains("-----BEGIN RSA PRIVATE KEY-----")
        && !key_text.contains("-----BEGIN EC PRIVATE KEY-----")
    {
        return Err(AppError::invalid_config(format!(
            "tls.key_path `{}` does not contain a PEM private key block",
            key_path.display()
        )));
    }

    let mut pem = cert_pem;
    pem.push(b'\n');
    pem.extend_from_slice(&key_pem);
    Identity::from_pem(&pem).map_err(|error| {
        AppError::invalid_config(format!(
            "failed to parse client certificate/key `{}` + `{}`: {error}",
            cert_path.display(),
            key_path.display()
        ))
    })
}

fn prepare_endpoint(base_url: &str, server_name: Option<&str>) -> AppResult<PreparedEndpoint> {
    let mut url = Url::parse(base_url)
        .map_err(|error| AppError::invalid_config(format!("invalid edge grpc url: {error}")))?;
    let original_host = url
        .host_str()
        .ok_or_else(|| AppError::invalid_config("edge gRPC url must include a host"))?
        .to_string();

    let Some(server_name) = server_name.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(PreparedEndpoint { url, resolve: None });
    };

    if server_name == original_host {
        return Ok(PreparedEndpoint { url, resolve: None });
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| AppError::invalid_config("edge gRPC url must include a port"))?;
    let ip = original_host.parse::<IpAddr>().map_err(|_| {
        AppError::invalid_config(
            "tls.server_name override requires edge_grpc_addr to use an IP literal host",
        )
    })?;
    url.set_host(Some(server_name))
        .map_err(|_| AppError::invalid_config("invalid tls.server_name"))?;

    Ok(PreparedEndpoint {
        url,
        resolve: Some(SocketAddr::new(ip, port)),
    })
}

fn require_pem_block(pem: &[u8], block_name: &str, path: &Path, field_name: &str) -> AppResult<()> {
    let text = std::str::from_utf8(pem).map_err(|error| {
        AppError::invalid_config(format!(
            "failed to parse {field_name} `{}` as UTF-8 PEM: {error}",
            path.display()
        ))
    })?;
    if text.contains(&format!("-----BEGIN {block_name}-----")) {
        Ok(())
    } else {
        Err(AppError::invalid_config(format!(
            "{field_name} `{}` does not contain a PEM {block_name} block",
            path.display()
        )))
    }
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
    use std::net::SocketAddr;

    use crate::{config::TlsConfig, proto::edge::IngestLogsResponse, transport::EdgeGrpcTransport};

    use super::{build_base_url, decode_grpc_frame, derive_server_name, prepare_endpoint};

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
    fn derives_server_name_from_endpoint_when_not_overridden() {
        assert_eq!(
            derive_server_name(
                "https://edge.example.local",
                "edge.example.local:7443",
                None
            )
            .unwrap()
            .as_deref(),
            Some("edge.example.local")
        );
    }

    #[test]
    fn falls_back_to_edge_url_host_when_grpc_addr_uses_ip_literal() {
        assert_eq!(
            derive_server_name("https://edge.example.local", "10.0.0.5:7443", None)
                .unwrap()
                .as_deref(),
            Some("edge.example.local")
        );
    }

    #[test]
    fn prepares_endpoint_for_ip_literal_server_name_override() {
        let prepared =
            prepare_endpoint("https://10.0.0.5:7443", Some("edge.example.local")).unwrap();
        assert_eq!(prepared.url.host_str(), Some("edge.example.local"));
        assert_eq!(
            prepared.resolve,
            Some("10.0.0.5:7443".parse::<SocketAddr>().unwrap())
        );
    }

    #[test]
    fn rejects_server_name_override_for_non_ip_host() {
        let error =
            prepare_endpoint("https://edge.internal:7443", Some("edge.example.local")).unwrap_err();
        assert!(error.to_string().contains(
            "tls.server_name override requires edge_grpc_addr to use an IP literal host"
        ));
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

    #[test]
    fn keeps_tls_config_shape_serializable() {
        let tls = TlsConfig {
            ca_path: Some("/etc/doro-agent/ca.pem".into()),
            cert_path: Some("/etc/doro-agent/agent.pem".into()),
            key_path: Some("/etc/doro-agent/agent.key".into()),
            server_name: Some("edge.example.local".to_string()),
        };
        assert_eq!(tls.server_name.as_deref(), Some("edge.example.local"));
    }

    #[test]
    fn rejects_tls_config_on_plain_http_endpoint() {
        let error = EdgeGrpcTransport::new(
            "http://edge.example.local:8080",
            "edge.example.local:9090",
            &TlsConfig {
                ca_path: Some("/etc/doro-agent/ca.pem".into()),
                cert_path: None,
                key_path: None,
                server_name: None,
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("TLS settings are configured but the gRPC endpoint uses plain HTTP"));
    }
}
