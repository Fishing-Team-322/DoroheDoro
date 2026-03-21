CREATE TABLE IF NOT EXISTS hosts (
    id UUID PRIMARY KEY,
    hostname TEXT NOT NULL,
    ip TEXT NOT NULL,
    ssh_port INTEGER NOT NULL DEFAULT 22,
    remote_user TEXT NOT NULL,
    labels_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT hosts_hostname_unique UNIQUE (hostname),
    CONSTRAINT hosts_ip_unique UNIQUE (ip)
);

CREATE TABLE IF NOT EXISTS host_groups (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS host_group_members (
    id UUID PRIMARY KEY,
    host_group_id UUID NOT NULL REFERENCES host_groups(id) ON DELETE CASCADE,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT host_group_members_unique UNIQUE (host_group_id, host_id)
);

CREATE TABLE IF NOT EXISTS credentials_profiles_metadata (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    vault_ref TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_hosts_hostname ON hosts(hostname);
CREATE INDEX IF NOT EXISTS idx_hosts_ip ON hosts(ip);
CREATE INDEX IF NOT EXISTS idx_host_groups_name ON host_groups(name);
CREATE INDEX IF NOT EXISTS idx_credentials_profiles_kind ON credentials_profiles_metadata(kind);
