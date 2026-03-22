CREATE TABLE IF NOT EXISTS telegram_deliveries (
    id UUID PRIMARY KEY,
    integration_id UUID NOT NULL,
    integration_binding_id UUID NOT NULL,
    notification_id TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    event_type TEXT NOT NULL,
    cluster_id UUID NULL,
    cluster_name TEXT NOT NULL DEFAULT '',
    severity TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    entity_kind TEXT NOT NULL DEFAULT '',
    entity_id TEXT NOT NULL DEFAULT '',
    details_url TEXT NOT NULL DEFAULT '',
    telegram_chat_id TEXT NOT NULL DEFAULT '',
    parse_mode TEXT NOT NULL DEFAULT 'HTML',
    message_text TEXT NOT NULL,
    notification_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 4,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_attempt_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    dead_lettered_at TIMESTAMPTZ,
    lease_token TEXT,
    lease_expires_at TIMESTAMPTZ,
    status_code TEXT NOT NULL DEFAULT '',
    status_message TEXT NOT NULL DEFAULT '',
    status_severity TEXT NOT NULL DEFAULT 'info',
    source_component TEXT NOT NULL DEFAULT 'control-plane.telegram',
    suggested_action TEXT NOT NULL DEFAULT '',
    correlation_id TEXT NOT NULL DEFAULT '',
    telegram_message_id TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT telegram_deliveries_status_check CHECK (
        status IN ('queued', 'retry_pending', 'sending', 'delivered', 'dead_letter')
    ),
    CONSTRAINT telegram_deliveries_unique_notification UNIQUE (integration_binding_id, notification_id)
);

CREATE INDEX IF NOT EXISTS idx_telegram_deliveries_due
    ON telegram_deliveries(next_attempt_at ASC, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_telegram_deliveries_status
    ON telegram_deliveries(status, next_attempt_at ASC);

CREATE INDEX IF NOT EXISTS idx_telegram_deliveries_correlation
    ON telegram_deliveries(correlation_id);

CREATE INDEX IF NOT EXISTS idx_telegram_deliveries_binding
    ON telegram_deliveries(integration_binding_id, created_at DESC);

CREATE TABLE IF NOT EXISTS telegram_delivery_attempts (
    id UUID PRIMARY KEY,
    delivery_id UUID NOT NULL,
    batch_id UUID NULL,
    attempt_number INTEGER NOT NULL,
    classification TEXT NOT NULL,
    http_status INTEGER NULL,
    telegram_error_code TEXT NOT NULL DEFAULT '',
    retry_after_seconds INTEGER NULL,
    duration_ms BIGINT NOT NULL DEFAULT 0,
    status_code TEXT NOT NULL,
    status_message TEXT NOT NULL DEFAULT '',
    status_severity TEXT NOT NULL DEFAULT 'info',
    source_component TEXT NOT NULL DEFAULT 'control-plane.telegram',
    suggested_action TEXT NOT NULL DEFAULT '',
    correlation_id TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT telegram_delivery_attempts_classification_check CHECK (
        classification IN ('success', 'retryable', 'permanent', 'invalid_configuration')
    ),
    CONSTRAINT telegram_delivery_attempts_unique UNIQUE (delivery_id, attempt_number)
);

CREATE INDEX IF NOT EXISTS idx_telegram_delivery_attempts_delivery
    ON telegram_delivery_attempts(delivery_id, attempt_number ASC);

CREATE INDEX IF NOT EXISTS idx_telegram_delivery_attempts_batch
    ON telegram_delivery_attempts(batch_id);

CREATE INDEX IF NOT EXISTS idx_telegram_delivery_attempts_classification
    ON telegram_delivery_attempts(classification, created_at DESC);

CREATE TABLE IF NOT EXISTS telegram_delivery_batches (
    id UUID PRIMARY KEY,
    correlation_id TEXT NOT NULL DEFAULT '',
    picked_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    retryable_failure_count INTEGER NOT NULL DEFAULT 0,
    permanent_failure_count INTEGER NOT NULL DEFAULT 0,
    dead_letter_count INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_telegram_delivery_batches_started
    ON telegram_delivery_batches(started_at DESC);

CREATE TABLE IF NOT EXISTS telegram_healthcheck_runs (
    id UUID PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,
    integration_id UUID NOT NULL,
    chat_id_override TEXT NOT NULL DEFAULT '',
    resolved_chat_id TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    classification TEXT NOT NULL DEFAULT '',
    telegram_message_id TEXT NOT NULL DEFAULT '',
    status_code TEXT NOT NULL DEFAULT '',
    status_message TEXT NOT NULL DEFAULT '',
    status_severity TEXT NOT NULL DEFAULT 'info',
    source_component TEXT NOT NULL DEFAULT 'control-plane.telegram',
    suggested_action TEXT NOT NULL DEFAULT '',
    correlation_id TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    CONSTRAINT telegram_healthcheck_runs_status_check CHECK (
        status IN ('running', 'succeeded', 'failed')
    )
);

CREATE INDEX IF NOT EXISTS idx_telegram_healthcheck_runs_integration
    ON telegram_healthcheck_runs(integration_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_telegram_healthcheck_runs_status
    ON telegram_healthcheck_runs(status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_telegram_healthcheck_runs_correlation
    ON telegram_healthcheck_runs(correlation_id);
