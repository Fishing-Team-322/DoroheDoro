CREATE TABLE IF NOT EXISTS agents (
    id UUID PRIMARY KEY,
    agent_id TEXT NOT NULL UNIQUE,
    hostname TEXT NOT NULL,
    status TEXT NOT NULL,
    version TEXT,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    first_seen_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS policies (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS enrollment_tokens (
    id UUID PRIMARY KEY,
    token_hash TEXT NOT NULL UNIQUE,
    policy_id UUID NOT NULL REFERENCES policies(id),
    expires_at TIMESTAMPTZ,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS policy_revisions (
    id UUID PRIMARY KEY,
    policy_id UUID NOT NULL REFERENCES policies(id) ON DELETE CASCADE,
    revision TEXT NOT NULL,
    body_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT policy_revisions_unique UNIQUE (policy_id, revision)
);

CREATE TABLE IF NOT EXISTS agent_policy_bindings (
    id UUID PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    policy_id UUID NOT NULL REFERENCES policies(id),
    policy_revision_id UUID NOT NULL REFERENCES policy_revisions(id),
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS agent_diagnostics (
    id UUID PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    payload_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_agents_agent_id ON agents(agent_id);
CREATE INDEX IF NOT EXISTS idx_agents_last_seen_at ON agents(last_seen_at);
CREATE INDEX IF NOT EXISTS idx_enrollment_tokens_token_hash ON enrollment_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_policy_revisions_policy_id ON policy_revisions(policy_id);
