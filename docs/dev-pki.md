# Dev PKI and mTLS Flow

`edge_api` already supports TLS + mTLS for AGENT ingress. This document covers the reproducible operational flow for local and demo certificates.

## Scripts

The repository now ships repeatable PKI helpers:

- [`scripts/pki/dev-ca.sh`](../scripts/pki/dev-ca.sh)
- [`scripts/pki/issue-edge-cert.sh`](../scripts/pki/issue-edge-cert.sh)
- [`scripts/pki/issue-agent-cert.sh`](../scripts/pki/issue-agent-cert.sh)

## Generate a local dev PKI set

```bash
bash scripts/pki/dev-ca.sh
bash scripts/pki/issue-edge-cert.sh
bash scripts/pki/issue-agent-cert.sh
```

Default output:

- `.tmp/pki/dev/ca.crt`
- `.tmp/pki/dev/ca.key`
- `.tmp/pki/dev/server.crt`
- `.tmp/pki/dev/server.key`
- `.tmp/pki/dev/agent.crt`
- `.tmp/pki/dev/agent.key`

## Runtime layout

`edge_api` expects:

- server cert: `AGENT_TLS_CERT_FILE`
- server key: `AGENT_TLS_KEY_FILE`
- client CA: `AGENT_TLS_CLIENT_CA_FILE`

The default local compose stack already mounts a generated cert volume at:

- `/certs/server.crt`
- `/certs/server.key`
- `/certs/ca.crt`
- `/certs/agent.crt`
- `/certs/agent.key`

## Local compose behavior

The root [`docker-compose.yml`](../docker-compose.yml) already includes:

- `edge-api-certs` service that generates dev certificates
- `edge-api` with `AGENT_MTLS_ENABLED=true`
- shared `edge-api-certs` volume mounted read-only into `edge-api`

That means the local demo stack is mTLS-ready by default and does not require manual certificate steps.

## Manual smoke against live compose

Run the real `agent-rs` smoke flow:

```bash
make agent-smoke
```

The smoke script:

- starts the local compose stack
- copies the live dev CA and client cert from `edge-api`
- issues a bootstrap token through `POST /api/v1/agents/bootstrap-tokens`
- runs the real `agent-rs`
- verifies enroll, policy apply, heartbeat, diagnostics, ingest, and restart without duplicate identity

Important identity rule:

- when mTLS is enabled, the client certificate CN or first SAN becomes the logical `agent_id`
- the default compose dev client certificate uses `agent-dev`
- for a custom identity, issue a custom client certificate with:

```bash
bash scripts/pki/issue-agent-cert.sh .tmp/pki/dev <agent_id>
```

## Expected security behavior

- bad TLS assets -> `edge-api` should fail fast on startup
- missing client cert with mTLS enabled -> rejected
- invalid client cert -> rejected
- valid client cert signed by the dev CA -> accepted
- post-enrollment requests with mismatched `agent_id` -> rejected

This is transport-level mTLS for demo/dev. It is not a full production PKI lifecycle system.
