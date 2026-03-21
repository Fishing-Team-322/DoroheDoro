CREATE TABLE IF NOT EXISTS clusters (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_by TEXT NOT NULL DEFAULT 'system'
);

CREATE TABLE IF NOT EXISTS cluster_hosts (
    id UUID PRIMARY KEY,
    cluster_id UUID NOT NULL REFERENCES clusters(id) ON DELETE CASCADE,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cluster_hosts_unique UNIQUE (cluster_id, host_id)
);

CREATE TABLE IF NOT EXISTS cluster_agents (
    id UUID PRIMARY KEY,
    cluster_id UUID NOT NULL REFERENCES clusters(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT cluster_agents_unique UNIQUE (cluster_id, agent_id)
);

CREATE TABLE IF NOT EXISTS cluster_metadata (
    cluster_id UUID PRIMARY KEY REFERENCES clusters(id) ON DELETE CASCADE,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cluster_hosts_cluster ON cluster_hosts(cluster_id);
CREATE INDEX IF NOT EXISTS idx_cluster_hosts_host ON cluster_hosts(host_id);
CREATE INDEX IF NOT EXISTS idx_cluster_agents_cluster ON cluster_agents(cluster_id);
CREATE INDEX IF NOT EXISTS idx_cluster_agents_agent ON cluster_agents(agent_id);

CREATE TABLE IF NOT EXISTS roles (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_by TEXT NOT NULL DEFAULT 'system'
);

CREATE TABLE IF NOT EXISTS permissions (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS role_permissions (
    id UUID PRIMARY KEY,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT role_permissions_unique UNIQUE (role_id, permission_id)
);

CREATE TABLE IF NOT EXISTS user_role_bindings (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    scope_type TEXT NOT NULL DEFAULT 'global',
    scope_id UUID NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT user_role_bindings_scope_check CHECK (
        (scope_type = 'global' AND scope_id IS NULL)
        OR (scope_type = 'cluster' AND scope_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_user_role_bindings_user ON user_role_bindings(user_id);
CREATE INDEX IF NOT EXISTS idx_user_role_bindings_scope ON user_role_bindings(scope_type, scope_id);

CREATE TABLE IF NOT EXISTS integrations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_by TEXT NOT NULL DEFAULT 'system'
);

CREATE TABLE IF NOT EXISTS integration_bindings (
    id UUID PRIMARY KEY,
    integration_id UUID NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    scope_type TEXT NOT NULL DEFAULT 'cluster',
    scope_id UUID NULL,
    event_types_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    severity_threshold TEXT NOT NULL DEFAULT 'info',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT integration_bindings_scope_check CHECK (
        (scope_type = 'global' AND scope_id IS NULL)
        OR (scope_type = 'cluster' AND scope_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_integration_bindings_scope ON integration_bindings(scope_type, scope_id);
CREATE INDEX IF NOT EXISTS idx_integration_bindings_integration ON integration_bindings(integration_id);

CREATE TABLE IF NOT EXISTS tickets (
    id UUID PRIMARY KEY,
    ticket_key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    cluster_id UUID NOT NULL REFERENCES clusters(id) ON DELETE RESTRICT,
    source_type TEXT NOT NULL,
    source_id TEXT,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    assignee_user_id TEXT,
    created_by TEXT NOT NULL,
    resolution TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_tickets_cluster_status ON tickets(cluster_id, status);
CREATE INDEX IF NOT EXISTS idx_tickets_source ON tickets(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_tickets_assignee ON tickets(assignee_user_id);

CREATE TABLE IF NOT EXISTS ticket_comments (
    id UUID PRIMARY KEY,
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    author_user_id TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ticket_events (
    id UUID PRIMARY KEY,
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ticket_events_ticket ON ticket_events(ticket_id, created_at);
CREATE INDEX IF NOT EXISTS idx_ticket_comments_ticket ON ticket_comments(ticket_id, created_at);

CREATE TABLE IF NOT EXISTS anomaly_rules (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    scope_type TEXT NOT NULL DEFAULT 'cluster',
    scope_id UUID NULL,
    config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_by TEXT NOT NULL DEFAULT 'system',
    CONSTRAINT anomaly_rules_scope_check CHECK (
        (scope_type = 'global' AND scope_id IS NULL)
        OR (scope_type = 'cluster' AND scope_id IS NOT NULL)
    )
);

CREATE TABLE IF NOT EXISTS anomaly_instances (
    id UUID PRIMARY KEY,
    rule_id UUID NOT NULL REFERENCES anomaly_rules(id) ON DELETE CASCADE,
    cluster_id UUID REFERENCES clusters(id) ON DELETE SET NULL,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_anomaly_instances_rule ON anomaly_instances(rule_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_anomaly_instances_cluster ON anomaly_instances(cluster_id, status);
