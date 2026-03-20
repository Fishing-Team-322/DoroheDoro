# Smoke test

1. `docker compose up --build nats edge-api`
2. Поднять responders на Rust subjects:
   - `agents.enroll.request`
   - `agents.policy.fetch`
   - `deployments.jobs.create`
   - `deployments.jobs.status`
   - `query.logs.search`
   - `query.logs.histogram`
   - `query.logs.severity`
   - `query.logs.top_hosts`
   - `query.logs.top_services`
   - `alerts.list`
   - `alerts.get`
3. Запустить `go run ./cmd/fake-agent`
4. Проверить:
   - `curl http://localhost:8080/healthz`
   - `curl http://localhost:8080/readyz`
   - `curl http://localhost:8080/api/v1/me`
   - `curl -N http://localhost:8080/api/v1/stream/logs`
