# Smoke test

1. Start infrastructure:
   - `docker compose up --build nats edge-api`
2. Start Rust responders for request/reply subjects:
   - `agents.enroll.request`
   - `agents.policy.fetch`
   - `agents.list`
   - `agents.get`
   - `agents.diagnostics.get`
   - `policies.list`
   - `policies.get`
   - `deployments.jobs.create`
   - `deployments.jobs.get`
   - `deployments.jobs.list`
   - `query.logs.search`
   - `query.logs.histogram`
   - `query.logs.severity`
   - `query.logs.top_hosts`
   - `query.logs.top_services`
3. Start log stream publisher for `ui.stream.logs`.
4. Optional agent smoke:
   - `go run ./cmd/fake-agent`
5. Verify HTTP:
   - `curl http://localhost:8080/healthz`
   - `curl http://localhost:8080/readyz`
   - `curl http://localhost:8080/api/v1/me`
   - `curl -N 'http://localhost:8080/api/v1/stream/logs?severity=error'`
