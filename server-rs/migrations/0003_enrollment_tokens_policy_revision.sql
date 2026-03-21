ALTER TABLE enrollment_tokens
    ADD COLUMN IF NOT EXISTS policy_revision_id UUID;

UPDATE enrollment_tokens et
SET policy_revision_id = (
    SELECT pr.id
    FROM policy_revisions pr
    WHERE pr.policy_id = et.policy_id
    ORDER BY pr.created_at DESC
    LIMIT 1
)
WHERE et.policy_revision_id IS NULL;

ALTER TABLE enrollment_tokens
    ALTER COLUMN policy_revision_id SET NOT NULL;

ALTER TABLE enrollment_tokens
    ADD CONSTRAINT enrollment_tokens_policy_revision_id_fkey
    FOREIGN KEY (policy_revision_id) REFERENCES policy_revisions(id);

CREATE INDEX IF NOT EXISTS idx_enrollment_tokens_policy_revision_id
    ON enrollment_tokens(policy_revision_id);
