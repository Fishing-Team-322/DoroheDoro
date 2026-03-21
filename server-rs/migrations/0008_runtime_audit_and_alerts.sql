CREATE TABLE IF NOT EXISTS runtime_audit_events (
    id UUID PRIMARY KEY,
    event_type TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    actor_type TEXT NOT NULL DEFAULT 'system',
    request_id TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_runtime_audit_events_entity
    ON runtime_audit_events(entity_type, entity_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_runtime_audit_events_event_type
    ON runtime_audit_events(event_type, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_runtime_audit_events_request_id
    ON runtime_audit_events(request_id);

INSERT INTO runtime_audit_events (
    id,
    event_type,
    entity_type,
    entity_id,
    actor_id,
    actor_type,
    request_id,
    reason,
    payload_json,
    created_at
)
SELECT
    id,
    action,
    entity_type,
    entity_id,
    actor_id,
    actor_type,
    request_id,
    reason,
    payload_json,
    created_at
FROM control_audit_events
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS alert_rules (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    severity TEXT NOT NULL,
    scope_type TEXT NOT NULL DEFAULT 'global',
    scope_id TEXT,
    condition_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_by TEXT NOT NULL DEFAULT 'system',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT alert_rules_status_check CHECK (status IN ('active', 'paused'))
);

CREATE INDEX IF NOT EXISTS idx_alert_rules_status
    ON alert_rules(status, updated_at DESC);

CREATE TABLE IF NOT EXISTS alert_instances (
    id UUID PRIMARY KEY,
    rule_id UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    severity TEXT NOT NULL,
    host TEXT NOT NULL DEFAULT '',
    service TEXT NOT NULL DEFAULT '',
    fingerprint TEXT NOT NULL DEFAULT '',
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT alert_instances_status_check CHECK (status IN ('active', 'acknowledged', 'resolved'))
);

CREATE INDEX IF NOT EXISTS idx_alert_instances_rule
    ON alert_instances(rule_id, triggered_at DESC);

CREATE INDEX IF NOT EXISTS idx_alert_instances_status
    ON alert_instances(status, triggered_at DESC);

CREATE INDEX IF NOT EXISTS idx_alert_instances_host_service
    ON alert_instances(host, service, triggered_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_alert_instances_active_unique
    ON alert_instances(rule_id, host, service, fingerprint)
    WHERE status IN ('active', 'acknowledged');
