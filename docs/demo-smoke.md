# Demo Smoke Flow

Current honest smoke for the integrated slice that exists in this repository today.

## 1. Start the local full stack

```bash
docker compose up --build
```

Healthy services should include:

- `frontend`
- `edge-api`
- `agent-artifacts`
- `nats`
- `postgres`
- `vault`
- `vault-init`
- `opensearch`
- `clickhouse`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`
- `ingestion-plane`
- `query-alert-plane`

## 2. Open WEB

- URL: `http://localhost:3000`
- login: `admin`
- password: `admin123`

## 3. Check boundary readiness

```bash
curl http://localhost:8080/readyz
```

Expected:

```json
{"status":"ready"}
```

## 4. Replace the placeholder Vault SSH secret before real deployment

The compose stack seeds `secret/data/ssh/dev` with a placeholder value. Replace it before creating a real deployment job:

```bash
docker compose exec vault \
  vault kv put secret/ssh/dev \
  ssh_user=root \
  ssh_private_key=@/path/to/real/id_ed25519
```

## 5. Create policy, host, host group and credentials metadata

Example payloads:

```bash
curl -X POST http://localhost:8080/api/v1/policies \
  -H "Content-Type: application/json" \
  -d '{"name":"deploy-demo-policy","description":"policy for deployment smoke","policy_body_json":{"sources":[{"type":"file","path":"/var/log/syslog","service":"host","severity_hint":"info"}],"labels":{"env":"demo","plane":"data"}}}'

curl -X POST http://localhost:8080/api/v1/hosts \
  -H "Content-Type: application/json" \
  -d '{"hostname":"demo-host-1","ip":"10.10.0.11","ssh_port":22,"remote_user":"root","labels":{"env":"demo","arch":"amd64","distro_family":"debian"}}'

curl -X POST http://localhost:8080/api/v1/host-groups \
  -H "Content-Type: application/json" \
  -d '{"name":"demo-linux","description":"practical run hosts"}'

curl -X POST http://localhost:8080/api/v1/credentials \
  -H "Content-Type: application/json" \
  -d '{"name":"ssh-dev","kind":"ssh_key","description":"Vault-backed SSH key","vault_ref":"secret/data/ssh/dev"}'
```

Add the host to the group:

```bash
curl -X POST http://localhost:8080/api/v1/host-groups/<group_id>/members \
  -H "Content-Type: application/json" \
  -d '{"host_id":"<host_id>"}'
```

## 6. Create deployment plan and job

```bash
curl -X POST http://localhost:8080/api/v1/deployments/plan \
  -H "Content-Type: application/json" \
  -d '{"job_type":"install","policy_id":"<policy_id>","target_host_group_ids":["<group_id>"],"credential_profile_id":"<credentials_profile_id>","requested_by":"demo-user"}'

curl -X POST http://localhost:8080/api/v1/deployments \
  -H "Content-Type: application/json" \
  -d '{"job_type":"install","policy_id":"<policy_id>","target_host_group_ids":["<group_id>"],"credential_profile_id":"<credentials_profile_id>","requested_by":"demo-user"}'
```

Inspect:

```bash
curl http://localhost:8080/api/v1/deployments/<job_id>
curl http://localhost:8080/api/v1/deployments/<job_id>/steps
curl http://localhost:8080/api/v1/deployments/<job_id>/targets
```

## 7. Verify deployment SSE

```bash
curl -N http://localhost:8080/api/v1/stream/deployments
```

Create another deployment job while the stream is open. You should receive `ready`, `status` and `step` events.

## 8. Enroll an agent over gRPC + mTLS

Run the smoke client inside the live `edge-api` container:

```bash
docker exec dorohedoro-edge-api-1 /bin/sh -lc \
  "FAKE_AGENT_TLS_CA_FILE=/certs/ca.crt \
   FAKE_AGENT_TLS_CERT_FILE=/certs/agent.crt \
   FAKE_AGENT_TLS_KEY_FILE=/certs/agent.key \
   FAKE_AGENT_TLS_SERVER_NAME=edge-api \
   EDGE_API_GRPC_ADDR=127.0.0.1:9090 \
   /usr/local/bin/fake-agent"
```

Expected:

- `Enroll` succeeds
- `FetchPolicy` succeeds
- `SendHeartbeat` succeeds
- `SendDiagnostics` succeeds
- `IngestLogs` succeeds

## 9. Inspect agents, logs, alerts and audit

```bash
curl http://localhost:8080/api/v1/agents
curl http://localhost:8080/api/v1/logs/search -H "Content-Type: application/json" -d '{"query":"","limit":20,"offset":0}'
curl http://localhost:8080/api/v1/dashboards/overview
curl http://localhost:8080/api/v1/alerts
curl http://localhost:8080/api/v1/audit
```

This verifies:

- `edge-api -> NATS -> enrollment-plane`
- `edge-api -> NATS -> ingestion-plane`
- `edge-api -> NATS -> query-alert-plane`
- `edge-api -> NATS -> control-plane` audit read-side

## 10. Prepare the 3-host practical run

Use:

- [`../deployments/ansible/inventories/three-hosts.example.ini`](../deployments/ansible/inventories/three-hosts.example.ini)
- [`../deployments/ansible/group_vars/agents.example.yml`](../deployments/ansible/group_vars/agents.example.yml)

Practical target shape:

- one public boundary host under a domain
- three Linux hosts reachable by Ansible
- Vault-backed SSH credentials
- deployment launched from WEB or the same HTTP API
- agent enrollment, heartbeat, diagnostics, logs, alerts and audit all visible afterwards

## 11. Run the Rust runtime smoke gates

After the local stack is healthy, run:

```bash
make server-smoke
```

These ignored-by-default tests validate the `enrollment-plane`, `control-plane`, and `deployment-plane` against live Postgres and NATS dependencies instead of only unit-test mocks.
