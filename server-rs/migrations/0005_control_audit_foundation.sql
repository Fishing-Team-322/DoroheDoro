ALTER TABLE policies
    ADD COLUMN IF NOT EXISTS created_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS updated_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS request_id TEXT NOT NULL DEFAULT 'system-bootstrap',
    ADD COLUMN IF NOT EXISTS update_reason TEXT NOT NULL DEFAULT '';

ALTER TABLE policy_revisions
    ADD COLUMN IF NOT EXISTS created_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS request_id TEXT NOT NULL DEFAULT 'system-bootstrap',
    ADD COLUMN IF NOT EXISTS reason TEXT NOT NULL DEFAULT '';

ALTER TABLE hosts
    ADD COLUMN IF NOT EXISTS created_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS updated_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS request_id TEXT NOT NULL DEFAULT 'system-bootstrap',
    ADD COLUMN IF NOT EXISTS update_reason TEXT NOT NULL DEFAULT '';

ALTER TABLE host_groups
    ADD COLUMN IF NOT EXISTS created_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS updated_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS request_id TEXT NOT NULL DEFAULT 'system-bootstrap',
    ADD COLUMN IF NOT EXISTS update_reason TEXT NOT NULL DEFAULT '';

ALTER TABLE credentials_profiles_metadata
    ADD COLUMN IF NOT EXISTS created_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS updated_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS request_id TEXT NOT NULL DEFAULT 'system-bootstrap',
    ADD COLUMN IF NOT EXISTS update_reason TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS control_audit_events (
    id UUID PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    actor_type TEXT NOT NULL DEFAULT 'system',
    request_id TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_control_audit_events_entity
    ON control_audit_events(entity_type, entity_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_control_audit_events_request_id
    ON control_audit_events(request_id);

CREATE INDEX IF NOT EXISTS idx_agent_policy_bindings_agent_assigned_at
    ON agent_policy_bindings(agent_id, assigned_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_diagnostics_agent_created_at
    ON agent_diagnostics(agent_id, created_at DESC);
