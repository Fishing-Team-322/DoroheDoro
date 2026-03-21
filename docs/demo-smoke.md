# Demo Smoke Flow

Current image-first smoke for the integrated slice that exists in this repository today.

## 1. Start the local full stack

```bash
export AGENT_IMAGE_REPOSITORY=docker.io/<org>/doro-agent
export AGENT_IMAGE_TAG=main
export AGENT_IMAGE_DIGEST=sha256:...
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

## 2. Check the compatibility manifest

```bash
curl http://localhost:18081/manifest.json
```

Expected contract fragments:

- `install_mode=docker_image`
- `package_type=container`
- `artifact_path=docker.io/<org>/doro-agent:main`

## 3. Open WEB

- URL: `http://localhost:3000`
- login: `admin`
- password: `admin123`

## 4. Check boundary readiness

```bash
curl http://localhost:8080/readyz
```

Expected:

```json
{"status":"ready"}
```

## 5. Replace the placeholder Vault SSH secret before real deployment

The compose stack seeds `secret/data/ssh/dev` with a placeholder value. Replace it before creating a real deployment job:

```bash
docker compose exec vault \
  vault kv put secret/ssh/dev \
  ssh_user=root \
  ssh_private_key=@/path/to/real/id_ed25519
```

## 6. Create policy, host, host group and credentials metadata

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

## 7. Create deployment plan and job

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

## 8. Verify image-based agent install on the target host

Expected Ansible behavior:

- detects `docker`, otherwise `podman`
- pulls the image from the manifest
- runs `doctor` before switching the unit
- starts the systemd-backed container
- keeps `/var/lib/doro-agent` across restart and upgrade

Useful host-side checks:

```bash
systemctl status doro-agent
cat /var/lib/doro-agent/last-known-good-image.json
docker ps --format '{{.Names}} {{.Image}}' || podman ps --format '{{.Names}} {{.Image}}'
```

## 9. Verify deployment SSE

```bash
curl -N http://localhost:8080/api/v1/stream/deployments
```

Create another deployment job while the stream is open. You should receive `ready`, `status` and `step` events.

## 10. Enroll an agent over gRPC + mTLS

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

## 11. Inspect agents, logs, alerts and audit

```bash
curl http://localhost:8080/api/v1/agents
curl http://localhost:8080/api/v1/logs/search -H "Content-Type: application/json" -d '{"query":"","limit":20,"offset":0}'
curl http://localhost:8080/api/v1/dashboards/overview
curl http://localhost:8080/api/v1/alerts
curl http://localhost:8080/api/v1/audit
```

## 12. Negative smoke

Run the same deployment against:

- a host with Docker only
- a host with Podman only
- a host with neither engine

Expected failure for the third case:

- clear operator-readable error about missing Docker or Podman
