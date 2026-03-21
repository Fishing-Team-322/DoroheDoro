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

Run the built-in fake agent inside the running `edge-api` container:

```bash
docker exec dorohedoro-edge-api-1 /bin/sh -lc \
  "FAKE_AGENT_TLS_CA_FILE=/certs/ca.crt \
   FAKE_AGENT_TLS_CERT_FILE=/certs/agent.crt \
   FAKE_AGENT_TLS_KEY_FILE=/certs/agent.key \
   FAKE_AGENT_TLS_SERVER_NAME=edge-api \
   EDGE_API_GRPC_ADDR=127.0.0.1:9090 \
   /usr/local/bin/fake-agent"
```

This smoke path validates the live boundary with a real client certificate. In the current repository state, this is the reproducible mTLS proof because `agent-rs` itself does not yet expose client-certificate configuration in its install/runtime contract.

## Expected security behavior

- bad TLS assets -> `edge-api` should fail fast on startup
- missing client cert with mTLS enabled -> rejected
- invalid client cert -> rejected
- valid client cert signed by the dev CA -> accepted

This is transport-level mTLS for demo/dev. It is not a full production PKI lifecycle system.
