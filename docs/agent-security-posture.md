# Agent Security Posture

This document describes the agent-side `security_scan` runtime added under `agent-rs/`.

## Goal

The Rust `AGENT` now has an autonomous periodic security posture worker that:

- inspects listening TCP/UDP ports against `allowed_ports` and `blocked_ports`, with best-effort Linux process and service attribution from `/proc`
- inspects watched packages from `package_watchlist` using native `dpkg`/`rpm`/`apk` version comparison when available and falls back to binary banners or heuristic compare when needed
- runs hardening checks for world-writable critical files, firewall state, and root SSH login policy using `nftables`, `iptables`, `firewalld`, `ufw`, `systemctl`, and `sshd -T` where available
- publishes normalized payloads through the existing diagnostics transport without requiring server-side pipeline changes
- persists the latest local scan state and, optionally, the last report JSON under `state_dir/security/last-report.json`

The feature is intentionally isolated inside the agent runtime. It does not change `SERVER` domain logic and it does not introduce new product services.

## Input Contract

Agent config fields:

```yaml
security_scan:
  enabled: true
  interval_sec: 86400
  jitter_sec: 900
  timeout_sec: 120
  max_parallel_checks: 4
  profile: "balanced"
  allowed_ports: [22, 443]
  blocked_ports: [23, 2375]
  package_watchlist: ["openssl", "openssh", "nginx", "docker"]
  version_rules_path: "/etc/doro-agent/security-rules.yaml"
  publish_as_diagnostics: true
  persist_last_report: true
```

Config notes:

- `enabled` defaults to `true`
- `profile` supports `light`, `balanced`, `deep`
- `allowed_ports` and `blocked_ports` must not overlap
- `version_rules_path` is optional at runtime, but if the file exists it must use `schema_version: "v1"`

Rules file example:

```yaml
schema_version: "v1"
packages:
  - name: "openssl"
    min_secure_version: "3.0.0"
    severity: "high"
  - name: "openssh"
    min_secure_version: "9.0p1"
    severity: "medium"
    aliases: ["openssh-server", "openssh-clients", "ssh", "sshd"]
```

## Output Contracts

Events emitted through diagnostics transport:

- `security.posture.report.v1`
- `security.posture.scan.failed.v1`
- `security.posture.scan.skipped.v1`
- `security.posture.rules.loaded.v1`

Reference schemas:

- [security-posture-report-v1.schema.json](/Z:/vs/DoroheDoro/contracts/schemas/security-posture-report-v1.schema.json)
- [security-posture-scan-failed-v1.schema.json](/Z:/vs/DoroheDoro/contracts/schemas/security-posture-scan-failed-v1.schema.json)
- [security-posture-scan-skipped-v1.schema.json](/Z:/vs/DoroheDoro/contracts/schemas/security-posture-scan-skipped-v1.schema.json)
- [security-posture-rules-loaded-v1.schema.json](/Z:/vs/DoroheDoro/contracts/schemas/security-posture-rules-loaded-v1.schema.json)

Reserved subject names:

- `agents.security.posture.report`
- `agents.security.posture.scan.failed`
- `agents.security.posture.scan.skipped`
- `agents.security.posture.rules.loaded`

Persisted local state:

- `last_started_at`
- `last_finished_at`
- `last_status`
- `last_status_reason`
- `last_report_id`
- `last_delivery_status`
- `last_delivery_error`
- `last_rules_loaded_at`
- `last_rules_digest`
- `last_report_path`
- `backoff_until`
- `consecutive_failures`
- finding summary counters

The latest state is also mirrored into the regular diagnostics snapshot under `security_posture`.

Additional payload notes:

- findings now carry stable `finding_fingerprint`, plus optional `remediation` and `evidence`
- `port_states` may include `inode`, `process_id`, `process_name`, `executable_path`, `service_unit`, and `owner_uid`
- `misconfig_checks` may include both `observed_value` and `expected_value`

## Valid Payload Example

```json
{
  "schema_version": "v1",
  "event_name": "security.posture.report.v1",
  "report_id": "1f056a7a-e519-497f-b3fb-70eb7acb58fb",
  "created_at": "2026-03-22T01:23:45.678Z",
  "agent_id": "agent-01",
  "hostname": "linux-a",
  "profile": "balanced",
  "interval_sec": 86400,
  "started_at": "2026-03-22T01:23:10.000Z",
  "finished_at": "2026-03-22T01:23:45.678Z",
  "duration_ms": 35678,
  "status": "findings_detected",
  "port_states": [
    {
      "port_id": "port-a64ddbd6377824de",
      "protocol": "tcp",
      "listen_address": "0.0.0.0",
      "port": 2375,
      "exposure": "all_interfaces",
      "inode": 10428,
      "process_id": 918,
      "process_name": "dockerd",
      "executable_path": "/usr/bin/dockerd",
      "service_unit": "docker.service",
      "owner_uid": 0,
      "detail": "pid=918, process=dockerd, exe=/usr/bin/dockerd, unit=docker.service, uid=0"
    }
  ],
  "asset_versions": [],
  "misconfig_checks": [],
  "runtime_health": [],
  "findings": [
    {
      "finding_id": "finding-b61d915ec94e63f6",
      "finding_fingerprint": "fingerprint-0c85595d7032e078",
      "category": "ports",
      "severity": "critical",
      "title": "blocked port 2375 is listening",
      "detail": "tcp is listening on 0.0.0.0:2375",
      "asset_type": "socket",
      "asset_name": "tcp:2375",
      "observed_value": "0.0.0.0",
      "expected_value": "port must not listen",
      "check_id": "port-blocked-2375",
      "remediation": "stop the listener or remove the port from the blocked list if it is intentionally exposed",
      "evidence": "listener=tcp://0.0.0.0:2375, pid=918, process=dockerd, unit=docker.service"
    }
  ],
  "summary": {
    "total": 1,
    "critical": 1,
    "high": 0,
    "medium": 0,
    "low": 0,
    "info": 0
  }
}
```

## Invalid Payload Example

This payload is invalid because it omits `schema_version`, uses vague field names, and leaks raw secret material:

```json
{
  "event": "security",
  "meta": {
    "id": "1"
  },
  "data": {
    "token": "real-secret",
    "ports": [22]
  }
}
```

## Error Codes

Current agent-side `error_code` values in `security.posture.scan.failed.v1`:

- `rules_load_failed`
- `scan_timeout`
- `report_serialize_failed`

Skipped reasons in `security.posture.scan.skipped.v1`:

- `disabled_by_config`
- `unsupported_platform`

## New Logs

Current security posture logs emitted by the agent:

- `security posture report generated`
- `security posture scan failed`
- `security posture scan skipped`
- `security posture report persistence failed`

The worker also reuses the existing runtime error path when diagnostics publishing fails.

## Metrics

This slice does not introduce a dedicated metrics exporter.

Operationally, use:

- `security_posture` inside diagnostics snapshots
- persisted SQLite state in `agent_security_scan_state`
- agent logs listed above

## Backward Compatibility

- Existing log ingestion, heartbeat, diagnostics, spool, and policy refresh flows remain unchanged.
- Security posture events are transported as diagnostics payload JSON, so no server API or protobuf changes were required.
- Existing diagnostics consumers can ignore the new optional `security_posture` block.
- The worker degrades cleanly on non-Linux hosts by publishing a skipped event and exiting.

## Known Limitations

- Package comparison now uses native `dpkg`/`rpm`/`apk` comparison when the corresponding tool exists, but binary-only fallback paths still use a heuristic compare.
- Firewall inspection covers `nft list ruleset`, `iptables-save`, `firewalld`, `ufw`, and `systemctl`. Cloud firewalls, external appliances, and orchestrator-managed network policy remain invisible to the agent.
- SSH policy prefers `sshd -T -f /etc/ssh/sshd_config`; if that cannot be evaluated, the agent falls back to a conservative parse of the main config file and does not fully expand complex include graphs.
- Port owner attribution depends on `/proc/<pid>/fd` visibility. Non-root agents, hardened `procfs` settings, or container namespaces can produce only partial attribution.
- Open-port discovery remains Linux-specific because it reads `/proc/net/*`.
- No dedicated metrics endpoint was added in this slice.

## Linux Runtime Prerequisites

Best results come from Linux hosts where these commands are present and readable by the agent process:

- `sshd` for effective SSH config via `sshd -T`
- `nft` and/or `iptables-save` for firewall rule inspection
- `dpkg-query`, `dpkg`, `rpm`, or `apk` for native package version resolution
- `/proc/net/*` and `/proc/<pid>/fd` for listener discovery and process attribution

If any of these are missing, the scan still runs, but the report may fall back to `unknown`, `not_installed`, or partial runtime health warnings instead of a fully attributed result.

## Smoke Validation

Local smoke:

1. Use [agent.example.yaml](/Z:/vs/DoroheDoro/agent-rs/config/agent.example.yaml).
2. Create `./.tmp/doro-test.log` once before the first run.
3. Run `cargo run -- run --config ./config/agent.example.yaml`.
4. Confirm SQLite contains `agent_security_scan_state`.
5. On Linux, confirm `./.tmp/doro-agent/security/last-report.json` appears after the first scan.
6. On non-Linux development hosts, expect `security.posture.scan.skipped.v1` with `unsupported_platform` instead of a report file.

Linux host smoke:

1. Start the agent with `transport.mode=edge` or `transport.mode=mock`.
2. Confirm `cargo run -- check-config --config <path>` reports `security-scan` and `security-rules`.
3. Open a blocked port or lower a watched package version in the rules file.
4. Wait for the next scan or reduce `interval_sec` temporarily.
5. Confirm the report contains findings and that the diagnostics snapshot exposes `security_posture`.

## Rollout

- Roll out with `profile=light` first on a small Linux canary group.
- Keep `publish_as_diagnostics=true` and `persist_last_report=true` enabled during canary.
- Use shorter `interval_sec` only for smoke or canary, then return to daily cadence.
- Promote to `balanced` after validating that report volume and false positives are acceptable.

## Rollback

Fast rollback:

1. Set `security_scan.enabled=false`.
2. Restart the agent.
3. Confirm a single `security.posture.scan.skipped.v1` event with `disabled_by_config`.

Hard rollback:

1. Revert the agent binary to the previous build.
2. Keep `state_dir` intact.
3. Remove or ignore `state_dir/security/last-report.json` if it is no longer needed.

Rollback does not require clearing the rest of the agent runtime state.
