use std::sync::Arc;

use async_nats::Subscriber;
use common::{nats_subjects::LOGS_INGEST_RAW, proto::edge};
use futures::StreamExt;
use tokio::{select, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::service::IngestionService;

pub async fn spawn_handlers(
    client: async_nats::Client,
    service: Arc<IngestionService>,
    shutdown: CancellationToken,
) -> anyhow::Result<Vec<JoinHandle<()>>> {
    let ingest_sub = client.subscribe(LOGS_INGEST_RAW.to_string()).await?;
    Ok(vec![tokio::spawn(run_ingest_handler(
        ingest_sub, service, shutdown,
    ))])
}

async fn run_ingest_handler(
    mut subscription: Subscriber,
    service: Arc<IngestionService>,
    shutdown: CancellationToken,
) {
    loop {
        let message = select! {
            _ = shutdown.cancelled() => break,
            next = subscription.next() => {
                let Some(message) = next else { break; };
                message
            }
        };

        let request = match serde_json::from_slice::<edge::IngestLogsRequest>(message.payload.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                warn!(error = %error, "failed to decode logs.ingest.raw payload");
                continue;
            }
        };

        if let Err(error) = service.ingest_batch(request).await {
            error!(error_code = error.code().as_str(), error = %error, "failed to ingest log batch");
        }
    }
}
