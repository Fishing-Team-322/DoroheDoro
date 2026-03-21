CREATE TABLE IF NOT EXISTS deployment_jobs (
    id UUID PRIMARY KEY,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    policy_id UUID NOT NULL REFERENCES policies(id),
    policy_revision_id UUID NOT NULL REFERENCES policy_revisions(id),
    credential_profile_id UUID NOT NULL REFERENCES credentials_profiles_metadata(id),
    executor_kind TEXT NOT NULL,
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    summary_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT deployment_jobs_job_type_check CHECK (job_type IN ('install', 'reinstall', 'upgrade', 'uninstall')),
    CONSTRAINT deployment_jobs_status_check CHECK (status IN ('queued', 'running', 'partial_success', 'succeeded', 'failed', 'cancelled')),
    CONSTRAINT deployment_jobs_executor_kind_check CHECK (executor_kind IN ('mock', 'ansible'))
);

CREATE TABLE IF NOT EXISTS deployment_attempts (
    id UUID PRIMARY KEY,
    deployment_job_id UUID NOT NULL REFERENCES deployment_jobs(id) ON DELETE CASCADE,
    attempt_no INTEGER NOT NULL,
    status TEXT NOT NULL,
    triggered_by TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT deployment_attempts_status_check CHECK (status IN ('queued', 'running', 'partial_success', 'succeeded', 'failed', 'cancelled')),
    CONSTRAINT deployment_attempts_job_attempt_unique UNIQUE (deployment_job_id, attempt_no)
);

CREATE TABLE IF NOT EXISTS deployment_targets (
    id UUID PRIMARY KEY,
    deployment_job_id UUID NOT NULL REFERENCES deployment_jobs(id) ON DELETE CASCADE,
    deployment_attempt_id UUID NOT NULL REFERENCES deployment_attempts(id) ON DELETE CASCADE,
    host_id UUID NOT NULL REFERENCES hosts(id),
    hostname_snapshot TEXT NOT NULL,
    status TEXT NOT NULL,
    bootstrap_payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    rendered_vars_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT deployment_targets_status_check CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')),
    CONSTRAINT deployment_targets_attempt_host_unique UNIQUE (deployment_attempt_id, host_id)
);

CREATE TABLE IF NOT EXISTS deployment_steps (
    id UUID PRIMARY KEY,
    deployment_job_id UUID NOT NULL REFERENCES deployment_jobs(id) ON DELETE CASCADE,
    deployment_attempt_id UUID NOT NULL REFERENCES deployment_attempts(id) ON DELETE CASCADE,
    deployment_target_id UUID REFERENCES deployment_targets(id) ON DELETE CASCADE,
    step_name TEXT NOT NULL,
    status TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT deployment_steps_status_check CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'skipped'))
);

CREATE INDEX IF NOT EXISTS idx_deployment_jobs_status
    ON deployment_jobs(status);
CREATE INDEX IF NOT EXISTS idx_deployment_jobs_created_at
    ON deployment_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_deployment_attempts_job_attempt_no
    ON deployment_attempts(deployment_job_id, attempt_no DESC);
CREATE INDEX IF NOT EXISTS idx_deployment_targets_job_id
    ON deployment_targets(deployment_job_id);
CREATE INDEX IF NOT EXISTS idx_deployment_targets_attempt_id
    ON deployment_targets(deployment_attempt_id);
CREATE INDEX IF NOT EXISTS idx_deployment_targets_host_id
    ON deployment_targets(host_id);
CREATE INDEX IF NOT EXISTS idx_deployment_steps_job_id
    ON deployment_steps(deployment_job_id);
CREATE INDEX IF NOT EXISTS idx_deployment_steps_attempt_id
    ON deployment_steps(deployment_attempt_id);
