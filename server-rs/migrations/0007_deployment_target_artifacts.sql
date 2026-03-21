ALTER TABLE deployment_targets
    ADD COLUMN IF NOT EXISTS artifact_payload_json JSONB NOT NULL DEFAULT '{}'::jsonb;
