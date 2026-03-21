# Demo Smoke Flow

Current honest end-to-end demo for the integrated slice that exists in this repository today.

## 1. Start the local full stack

```bash
docker compose up --build
```

For the stack overview and server-mode variant, see [`docs/demo-stack.md`](./demo-stack.md).

Services that should come up healthy:

- `frontend`
- `edge-api`
- `nats`
- `postgres`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`

## 2. Open WEB

- URL: `http://localhost:3000`
- login: `admin`
- password: `admin123`

The WEB auth flow goes through:

`browser -> frontend -> /api/edge/* -> edge-api`

## 3. Check boundary readiness

```bash
curl http://localhost:8080/readyz
```

Expected:

```json
{"status":"ready"}
```

## 4. Inspect live boundary endpoints

Direct HTTP:

```bash
curl http://localhost:8080/api/v1/policies
curl http://localhost:8080/api/v1/hosts
curl http://localhost:8080/api/v1/credentials
curl http://localhost:8080/api/v1/deployments
```

Same-origin path through the frontend proxy:

```bash
curl http://localhost:3000/api/edge/api/v1/policies
curl http://localhost:3000/api/edge/api/v1/hosts
```

These should return live data through:

`edge-api -> NATS -> control-plane / deployment-plane`

## 5. Create a deployment-friendly policy, host and credentials metadata

Example payloads:

```bash
curl -X POST http://localhost:8080/api/v1/policies \
  -H "Content-Type: application/json" \
  -d '{"name":"deploy-demo-policy","description":"policy for deployment smoke","policy_body_json":{"paths":["/var/log/syslog"],"labels":{"env":"demo","plane":"data"},"revision":"deploy-rev-1","source_type":"file"}}'

curl -X POST http://localhost:8080/api/v1/hosts \
  -H "Content-Type: application/json" \
  -d '{"hostname":"demo-host-1","ip":"10.10.0.11","ssh_port":22,"remote_user":"root","labels":{"env":"demo","role":"web"}}'

curl -X POST http://localhost:8080/api/v1/credentials \
  -H "Content-Type: application/json" \
  -d '{"name":"ssh-default","kind":"ssh_key","description":"Default SSH key","vault_ref":"secret/data/ssh/default"}'
```

## 6. Create a deployment plan and job

```bash
curl -X POST http://localhost:8080/api/v1/deployments/plan \
  -H "Content-Type: application/json" \
  -d '{"job_type":"install","policy_id":"<policy_id>","target_host_ids":["<host_id>"],"credential_profile_id":"<credentials_profile_id>","requested_by":"demo-user"}'

curl -X POST http://localhost:8080/api/v1/deployments \
  -H "Content-Type: application/json" \
  -d '{"job_type":"install","policy_id":"<policy_id>","target_host_ids":["<host_id>"],"credential_profile_id":"<credentials_profile_id>","requested_by":"demo-user"}'

curl http://localhost:8080/api/v1/deployments/<job_id>
curl http://localhost:8080/api/v1/deployments/<job_id>/steps
curl http://localhost:8080/api/v1/deployments/<job_id>/targets
```

The deployment should progress and end in `succeeded` with non-empty steps/targets.

## 7. Verify the deployment SSE gateway

Open the stream:

```bash
curl -N http://localhost:8080/api/v1/stream/deployments
```

Create another deployment job while the stream is open. The SSE client should receive:

- `event: ready`
- `event: status`
- `event: step`

This verifies:

`deployment-plane -> NATS -> edge-api SSE -> WEB client`

## 8. Enroll a demo agent over gRPC + mTLS

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

If you need a standalone cert set outside compose, use the scripts documented in [`docs/dev-pki.md`](./dev-pki.md).

## 9. Inspect enrolled agents through the boundary

```bash
curl http://localhost:8080/api/v1/agents
curl http://localhost:8080/api/v1/agents/<agent_id>
curl http://localhost:8080/api/v1/agents/<agent_id>/diagnostics
curl http://localhost:8080/api/v1/agents/<agent_id>/policy
```

After the fake agent has enrolled, these should return real PostgreSQL-backed data via:

`edge-api -> NATS -> enrollment-plane`

## 10. What is intentionally still unavailable

The current repo still does not contain live Rust runtime crates for:

- query / dashboard runtime
- alerts runtime
- audit runtime

So these route groups still return controlled `501 not_implemented` with boundary metadata instead of fake Go business logic:

- logs search/analytics
- dashboards
- alerts
- audit
