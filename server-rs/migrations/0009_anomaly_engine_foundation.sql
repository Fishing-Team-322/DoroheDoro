CREATE TABLE IF NOT EXISTS anomaly_baselines (
    id UUID PRIMARY KEY,
    tenant_id UUID,
    host TEXT NOT NULL DEFAULT '',
    service TEXT NOT NULL DEFAULT '',
    signal_kind TEXT NOT NULL,
    window_minutes INTEGER NOT NULL,
    samples INTEGER NOT NULL DEFAULT 0,
    mean DOUBLE PRECISION NOT NULL DEFAULT 0,
    stddev DOUBLE PRECISION NOT NULL DEFAULT 0,
    p95 DOUBLE PRECISION,
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_refreshed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT anomaly_baselines_window_check CHECK (window_minutes > 0),
    CONSTRAINT anomaly_baselines_samples_check CHECK (samples >= 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_anomaly_baselines_unique
    ON anomaly_baselines(host, service, signal_kind, window_minutes);

CREATE INDEX IF NOT EXISTS idx_anomaly_baselines_signal
    ON anomaly_baselines(signal_kind, last_refreshed_at DESC);

CREATE TABLE IF NOT EXISTS anomaly_scores (
    id UUID PRIMARY KEY,
    rule_id UUID REFERENCES alert_rules(id) ON DELETE SET NULL,
    detector TEXT NOT NULL,
    signal_kind TEXT NOT NULL,
    host TEXT NOT NULL DEFAULT '',
    service TEXT NOT NULL DEFAULT '',
    correlation_key TEXT NOT NULL DEFAULT '',
    detection_mode TEXT NOT NULL DEFAULT 'medium',
    signal_id TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL,
    threshold DOUBLE PRECISION NOT NULL,
    evidence_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_anomaly_scores_created
    ON anomaly_scores(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_anomaly_scores_correlation
    ON anomaly_scores(correlation_key, created_at DESC);

ALTER TABLE alert_instances
    ADD COLUMN IF NOT EXISTS detection_mode TEXT NOT NULL DEFAULT 'medium',
    ADD COLUMN IF NOT EXISTS correlation_key TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS source_signals JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS auto_resolved_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_alert_instances_correlation
    ON alert_instances(correlation_key, status, triggered_at DESC);
