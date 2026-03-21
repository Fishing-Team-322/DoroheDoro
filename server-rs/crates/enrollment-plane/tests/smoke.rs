use std::{path::PathBuf, sync::Arc, time::Duration};

use async_nats::Client;
use axum::{body::Body, http::Request};
use common::{
    nats_subjects::{
        AGENTS_DIAGNOSTICS, AGENTS_ENROLL_REQUEST, AGENTS_HEARTBEAT, AGENTS_POLICY_FETCH,
    },
    proto::{
        agent::{
            DiagnosticsPayload, EnrollRequest, EnrollResponse, FetchPolicyRequest,
            FetchPolicyResponse, HeartbeatPayload,
        },
        decode_message, encode_message, runtime::RuntimeReplyEnvelope,
    },
    EnrollmentPlaneConfig,
};
use enrollment_plane::{
    http::{self, HttpState},
    repository::EnrollmentRepository,
    service::EnrollmentService,
    transport,
};
use reqwest::StatusCode;
use serial_test::serial;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{net::TcpListener, task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

struct TestHarness {
    http_addr: String,
    http_client: reqwest::Client,
    nats: Client,
    pool: PgPool,
    shutdown: CancellationToken,
    server_task: JoinHandle<()>,
    subscriber_tasks: Vec<JoinHandle<()>>,
}

impl TestHarness {
    async fn start() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let config = EnrollmentPlaneConfig::from_env()?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.shared.postgres_dsn)
            .await?;
        run_migrations(&pool).await?;
        truncate_tables(&pool).await?;

        let repo = EnrollmentRepository::new(pool.clone());
        let service = Arc::new(EnrollmentService::new(repo));
        service
            .bootstrap_defaults(&config.dev_bootstrap_token)
            .await?;

        let nats = async_nats::connect(&config.shared.nats_url).await?;
        let shutdown = CancellationToken::new();
        let subscriber_tasks =
            transport::spawn_handlers(nats.clone(), service, shutdown.clone()).await?;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let http_addr = format!("http://{}", addr);
        let http_shutdown = shutdown.clone();
        let pool_for_server = pool.clone();
        let nats_for_server = nats.clone();
        let server_task = tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                http::router(HttpState::new(pool_for_server, nats_for_server)),
            )
            .with_graceful_shutdown(async move {
                http_shutdown.cancelled().await;
            })
            .await;
        });

        let harness = Self {
            http_addr,
            http_client: reqwest::Client::new(),
            nats,
            pool,
            shutdown,
            server_task,
            subscriber_tasks,
        };

        harness.wait_until_ready().await?;
        Ok(harness)
    }

    async fn wait_until_ready(&self) -> anyhow::Result<()> {
        for _ in 0..40 {
            let response = self
                .http_client
                .get(format!("{}/readyz", self.http_addr))
                .send()
                .await;

            if let Ok(response) = response {
                if response.status() == StatusCode::OK {
                    return Ok(());
                }
            }

            sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!("service did not become ready in time");
    }

    async fn shutdown(self) {
        self.shutdown.cancel();
        self.server_task.abort();
        let _ = self.server_task.await;
        for task in self.subscriber_tasks {
            task.abort();
            let _ = task.await;
        }
    }
}

#[tokio::test]
#[ignore]
#[serial]
async fn health_and_readiness_work() -> anyhow::Result<()> {
    let harness = TestHarness::start().await?;

    let health = harness
        .http_client
        .get(format!("{}/healthz", harness.http_addr))
        .send()
        .await?;
    assert_eq!(health.status(), StatusCode::OK);

    let ready = harness
        .http_client
        .get(format!("{}/readyz", harness.http_addr))
        .send()
        .await?;
    assert_eq!(ready.status(), StatusCode::OK);

    let bad_pool =
        PgPoolOptions::new().connect_lazy("postgres://postgres:postgres@127.0.0.1:1/doro")?;
    let response = http::router(HttpState::new(bad_pool, harness.nats.clone()))
        .oneshot(Request::builder().uri("/readyz").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
#[ignore]
#[serial]
async fn enrollment_policy_heartbeat_and_diagnostics_flow() -> anyhow::Result<()> {
    let harness = TestHarness::start().await?;

    let enroll_request = EnrollRequest {
        correlation_id: "corr-enroll-1".to_string(),
        bootstrap_token: "dev-bootstrap-token".to_string(),
        hostname: "smoke-host".to_string(),
        version: "0.1.0".to_string(),
        metadata: [("source".to_string(), "smoke-test".to_string())]
            .into_iter()
            .collect(),
        existing_agent_id: String::new(),
    };

    let enroll_message = harness
        .nats
        .request(
            AGENTS_ENROLL_REQUEST.to_string(),
            encode_message(&enroll_request).into(),
        )
        .await?;
    let enroll_envelope: RuntimeReplyEnvelope = decode_message(enroll_message.payload.as_ref())?;
    assert_eq!(enroll_envelope.status, "ok");
    let enroll_response: EnrollResponse = decode_message(&enroll_envelope.payload)?;
    assert!(enroll_response.agent_id.starts_with("agent-"));
    assert_eq!(enroll_response.policy_revision, "rev-1");

    let agent_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents WHERE agent_id = $1")
            .bind(&enroll_response.agent_id)
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(agent_count, 1);

    let binding_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM agent_policy_bindings WHERE agent_id = $1",
    )
    .bind(&enroll_response.agent_id)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(binding_count, 1);

    let fetch_request = FetchPolicyRequest {
        correlation_id: "corr-policy-1".to_string(),
        agent_id: enroll_response.agent_id.clone(),
    };
    let fetch_message = harness
        .nats
        .request(
            AGENTS_POLICY_FETCH.to_string(),
            encode_message(&fetch_request).into(),
        )
        .await?;
    let fetch_envelope: RuntimeReplyEnvelope = decode_message(fetch_message.payload.as_ref())?;
    assert_eq!(fetch_envelope.status, "ok");
    let fetch_response: FetchPolicyResponse = decode_message(&fetch_envelope.payload)?;
    assert_eq!(fetch_response.agent_id, enroll_response.agent_id);
    assert_eq!(fetch_response.policy_revision, "rev-1");
    assert!(fetch_response.policy_body_json.contains("/var/log/*.log"));

    let heartbeat = HeartbeatPayload {
        agent_id: fetch_response.agent_id.clone(),
        hostname: "smoke-host".to_string(),
        version: "0.2.0".to_string(),
        status: "active".to_string(),
        host_metadata: [("kernel".to_string(), "6.8".to_string())]
            .into_iter()
            .collect(),
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    };
    harness
        .nats
        .publish(
            AGENTS_HEARTBEAT.to_string(),
            encode_message(&heartbeat).into(),
        )
        .await?;
    harness.nats.flush().await?;

    eventually(Duration::from_secs(5), || {
        let pool = harness.pool.clone();
        let agent_id = fetch_response.agent_id.clone();
        async move {
            let row = sqlx::query_as::<_, (String, String)>(
                "SELECT status, COALESCE(version, '') FROM agents WHERE agent_id = $1",
            )
            .bind(agent_id)
            .fetch_one(&pool)
            .await?;
            anyhow::ensure!(row.0 == "active");
            anyhow::ensure!(row.1 == "0.2.0");
            Ok(())
        }
    })
    .await?;

    let diagnostics = DiagnosticsPayload {
        agent_id: fetch_response.agent_id.clone(),
        payload_json: r#"{"last_error":"","sources":[{"path":"/var/log/test.log","status":"ok"}]}"#
            .to_string(),
        sent_at_unix_ms: chrono::Utc::now().timestamp_millis(),
    };
    harness
        .nats
        .publish(
            AGENTS_DIAGNOSTICS.to_string(),
            encode_message(&diagnostics).into(),
        )
        .await?;
    harness.nats.flush().await?;

    eventually(Duration::from_secs(5), || {
        let pool = harness.pool.clone();
        let agent_id = fetch_response.agent_id.clone();
        async move {
            let count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM agent_diagnostics WHERE agent_id = $1",
            )
            .bind(agent_id)
            .fetch_one(&pool)
            .await?;
            anyhow::ensure!(count == 1);
            Ok(())
        }
    })
    .await?;

    harness.shutdown().await;
    Ok(())
}

async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let migrations_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("migrations");
    let migrator = sqlx::migrate::Migrator::new(migrations_path.as_path()).await?;
    migrator.run(pool).await?;
    Ok(())
}

async fn truncate_tables(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "TRUNCATE TABLE agent_diagnostics, agent_policy_bindings, enrollment_tokens, policy_revisions, policies, agents RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn eventually<F, Fut>(timeout: Duration, mut check: F) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    let started = std::time::Instant::now();
    loop {
        match check().await {
            Ok(()) => return Ok(()),
            Err(error) if started.elapsed() < timeout => {
                let _ = error;
                sleep(Duration::from_millis(100)).await;
            }
            Err(error) => return Err(error),
        }
    }
}
