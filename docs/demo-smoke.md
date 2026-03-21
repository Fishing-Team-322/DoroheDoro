# Demo Smoke Flow

Current honest end-to-end demo for the integrated slice that exists in this repository today.

## 1. Start the stack

```bash
docker compose up --build
```

Services that should come up healthy:

- `frontend`
- `edge-api`
- `nats`
- `postgres`
- `enrollment-plane`

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

## 4. Enroll a demo agent over gRPC + mTLS

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

## 5. Inspect real read-side data through edge-api

Direct HTTP:

```bash
curl http://localhost:8080/api/v1/agents
curl http://localhost:8080/api/v1/policies
```

Same-origin path through WEB proxy:

```bash
curl http://localhost:3000/api/edge/api/v1/agents
curl http://localhost:3000/api/edge/api/v1/policies
```

After the fake agent has enrolled, the agents list and policy list should return real PostgreSQL-backed data via:

`edge-api -> NATS -> enrollment-plane`

## 6. Verify the stream gateway

Open an SSE connection:

```bash
curl -N http://localhost:8080/api/v1/stream/agents
```

From another terminal, publish a demo event into NATS:

```bash
docker run --rm --network dorohedoro_default natsio/nats-box:latest \
  nats pub ui.stream.agents '{"agent_id":"demo-agent","status":"online"}' -s nats://nats:4222
```

The SSE client should receive the event. This verifies the gateway path:

`NATS -> edge-api SSE -> WEB client`

## 7. What is intentionally still unavailable

The current repo does not yet contain live Rust runtime crates for:

- `control-plane`
- `deployment-plane`
- `query-alert-plane`

So these route groups still return controlled `501 not_implemented` with boundary metadata instead of fake Go business logic:

- hosts / host-groups / credentials
- deployments
- query / dashboards / alerts / audit
