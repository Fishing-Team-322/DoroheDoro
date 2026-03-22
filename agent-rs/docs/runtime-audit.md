# Runtime Audit

This document captures the current startup and runtime failure model for `doro-agent` in the container-oriented Linux path.

## Startup Sequence

Expected startup order:

1. `config_load`
2. `runtime_metadata_detect`
3. `state_db_open`
4. `transport_init`
5. `enrollment_connect`
6. `source_validation`
7. `background_loops_start`

The runtime should either:

- finish startup and move to `online` or `degraded`
- exit with a non-zero status and a concrete error

Silent exits are treated as bugs.

## Failure Classification

## Fatal At Startup

- configuration file missing or invalid
- runtime metadata detection fails
- `state_dir` or `state.db` is not writable or not openable
- transport configuration is structurally invalid
- TLS material is syntactically invalid
- persisted local state is unreadable or SQLite open fails

Expected behavior:

- `preflight` fails
- `run` exits non-zero
- stderr contains the terminal error
- structured startup logs show the last completed phase

## Non-Fatal At Startup

- edge endpoint not reachable
- policy fetch fails but a persisted policy exists
- source files are missing
- source files are temporarily unreadable after startup
- security rules file is absent

Expected behavior:

- `preflight` warns, but does not block startup for temporary edge reachability issues
- runtime enters `degraded` when needed
- local diagnostics snapshot still exists
- runtime keeps retrying enrollment, heartbeat, diagnostics, and batch delivery according to backoff logic

## Runtime Failure Points

## Source Readers

Failure modes:

- source file missing
- source file unreadable
- source worker exits unexpectedly

Current behavior:

- missing file moves source to `waiting`
- unreadable file moves source to `error`
- supervisor restarts unexpectedly exited source workers

Desired operator signal:

- source status visible in diagnostics snapshot
- local logs show restart transitions

## Transport

Failure modes:

- DNS failure
- TCP connect failure
- TLS validation failure
- server rejection
- prolonged unavailability

Current behavior:

- heartbeat, diagnostics, and sender update connectivity errors
- successful transport activity clears stale connect/TLS errors
- prolonged delivery trouble can move runtime into `degraded`
- permanent transport rejection can block delivery

Desired operator signal:

- `last_transport_error`
- `last_handshake_success_at`
- `blocked_delivery`
- spool backlog counters

## Scheduler Loops

Failure modes:

- heartbeat loop exits
- diagnostics loop exits
- degraded controller exits
- sender exits
- state writer exits

Current behavior:

- the main runtime polls worker handles
- unexpected worker exit moves runtime to `error`
- shutdown follows with non-zero process exit

Desired operator signal:

- no false-positive healthy state after loop death
- error phase persisted to local runtime state

## Local Troubleshooting Artifacts

Required local artifacts:

- `state_dir/state.db`
- `state_dir/runtime/diagnostics-snapshot.json`
- last transport/connect/TLS errors in diagnostics snapshot
- heartbeat and diagnostics loop timestamps
- spool backlog counters

These artifacts must exist even when remote diagnostics delivery is failing.

## Warn vs Fail Guidance

Warn:

- temporary edge unreachability during preflight
- missing source file
- missing security rules file
- ambiguous install mode
- non-canonical package hints in dev/container layouts

Fail:

- invalid config
- invalid TLS material
- unreadable configured source path
- unwritable state or spool directories
- inaccessible `state.db`
- dead runtime health loop
- stale or missing local runtime snapshot in `health`

## Operational Contract

- `preflight` / `doctor` must be non-mutating and machine-readable
- `health` must reflect live runtime viability, not only config validity
- successful health requires a fresh local diagnostics snapshot
- degraded runtime may still be alive, but must not pretend to be fully healthy when transport is dead
