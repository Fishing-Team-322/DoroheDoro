# Agent Status Read Model ADR

## Goal

WEB and operators need one stable read-side view of the agent lifecycle without stitching together inventory, deployment, enrollment, diagnostics and traffic calls in the browser.

This document records the `edge_api` read-model introduced for:

- `GET /api/v1/hosts/{host_id}/agent-status`
- `GET /api/v1/hosts/{host_id}/agent-diagnostics`
- `GET /api/v1/clusters/{cluster_id}/agents/overview`
- `GET /api/v1/deployments/jobs/{job_id}/timeline`
- `GET /api/v1/stream/agent-events`

## Decision

`edge_api` now owns an in-memory read model in `internal/app/agentstatus`.

The package exposes:

- `GetHostAgentStatus`
- `GetHostDiagnostics`
- `GetClusterAgentsOverview`
- `GetDeploymentTimeline`
- JSON mappers for normalized lifecycle SSE events

The read model is intentionally additive and read-only. It does not replace any upstream contracts and does not require protobuf changes on the Rust agent side.

## Data sources

The aggregator merges data from the existing boundary-visible sources:

- `control.hosts.*` for host identity
- `control.clusters.*` for cluster membership
- `agents.list` and `agents.get` for enrollment/runtime identity
- `agents.diagnostics.get` for doctor/runtime snapshot
- `deployments.jobs.list` and `deployments.jobs.get` for deployment status and steps
- `query.logs.search` and `query.logs.severity` for recent ingest activity
- `ui.stream.agents` and `deployments.jobs.step` for live lifecycle events

Host-to-agent matching is currently resolved by:

1. `host.labels.agent_id` if present
2. hostname match against the agent registry

This keeps the edge layer compatible with the current upstream contracts while we wait for a stricter host/agent binding source of truth.

## Public status model

The public API uses stable snake_case enums and summaries instead of upstream passthrough payloads.

Key normalized fields:

- `deployment_status`
- `enrollment_status`
- `heartbeat_status`
- `doctor_status`
- `primary_failure_domain`
- `spool_status`
- `transport_status`
- `tls_status`
- `source_status`

The diagnostics detail route also returns:

- normalized `checks[]`
- `top_issues[]`
- `runtime_errors`
- `transport_errors`
- `spool_warnings`
- `degraded_mode`

## Failure-domain classification

The edge read model classifies raw issues into stable domains:

- `deployment`
- `enrollment`
- `transport`
- `tls`
- `permissions`
- `source`
- `spool`
- `ingestion`
- `runtime`
- `unknown`

For the operator-facing host summary, these are collapsed into the broader `primary_failure_domain` values used by WEB:

- `deployment`
- `network`
- `tls`
- `enrollment`
- `runtime`
- `ingestion`
- `unknown`

## Graceful degradation

The aggregator does not fail the whole endpoint when one upstream read path is missing or stale.

Instead it returns:

- `missing_sections[]`
- per-section `data_freshness.sections`
- best-effort derived summaries

Malformed diagnostics payloads are treated as partial data. The raw snapshot is still returned where possible, while normalized sections are marked degraded.

## Caching

The read model uses a short-lived in-memory cache with bounded staleness.

Current behavior:

- TTL-based memoization for expensive read models
- invalidation on normalized agent/deployment SSE events
- separate cached keys for host status, diagnostics, deployment timeline and supporting lookups

This keeps host pages and cluster overview pages from issuing multiple expensive upstream roundtrips on every refresh.

## Timeline normalization

Deployment steps are normalized into stable phase names such as:

- `plan_created`
- `inventory_rendered`
- `manifest_resolved`
- `ansible_started`
- `host_connected`
- `image_selected`
- `image_pulled`
- `unit_rendered`
- `service_restarted`
- `health_check_started`
- `health_check_passed`
- `health_check_failed`
- `rollback_started`
- `rollback_succeeded`
- `rollback_failed`

If upstream step names do not yet expose every phase directly, the edge layer maps the closest stable public phase and preserves the raw upstream step name in `raw_step_name`.

## SSE normalization

`GET /api/v1/stream/agent-events` is a normalized lifecycle stream for WEB.

It combines:

- `ui.stream.agents`
- `deployments.jobs.step`

Into event types:

- `agent.deployment.updated`
- `agent.enrollment.updated`
- `agent.heartbeat.updated`
- `agent.diagnostics.updated`
- `agent.connectivity.problem`
- `agent.recovered`

Every event includes:

- `event_type`
- `host_id`
- `cluster_id`
- `severity`
- `occurred_at`
- `payload`

## Trade-offs

- Deployment lookup is still derived from recent job scans because upstream does not yet expose a direct host deployment index.
- Traffic summaries are best-effort and query-plane dependent.
- Some deployment phases are inferred from current upstream step names until deployment-plane publishes finer-grained steps.

These trade-offs are accepted because they improve operator visibility immediately without blocking on changes in `server-rs` or `agent-rs`.
