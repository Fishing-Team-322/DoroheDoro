# Doro Agent Install Contract

This document fixes the delivery contract that release tooling, Ansible and future deployment runtime should rely on.

## Supported release targets

- `linux-amd64`
- `linux-arm64`
- Debian/Ubuntu compatible install path via `.deb`
- Astra Linux compatible install path via `.deb`
- generic `tar.gz` fallback for manual or constrained installs

## Artifact formats

Every release can publish:

- `tar.gz` bundle
- `.deb` package
- `sha256` checksum for each artifact
- manifest entry in `deployments/artifacts/*.json`

## Install paths

Package install path:

- binary: `/usr/bin/doro-agent`
- config: `/etc/doro-agent/config.yaml`
- env file: `/etc/doro-agent/agent.env`
- state dir: `/var/lib/doro-agent`
- log dir: `/var/log/doro-agent`
- systemd unit: `/lib/systemd/system/doro-agent.service`

Tarball install path:

- binary: `/usr/local/bin/doro-agent`
- config: `/etc/doro-agent/config.yaml`
- env file: `/etc/doro-agent/agent.env`
- state dir: `/var/lib/doro-agent`
- log dir: `/var/log/doro-agent`
- systemd unit: `/etc/systemd/system/doro-agent.service`

## Required bundle contents

Each release bundle must include:

- `doro-agent` binary
- `doro-agent.service`
- example config
- example env file
- install notes
- build metadata with version, arch and build time

## Install behavior

- fresh install: create service user/group, directories, install binary/config/service, enable service
- upgrade: replace artifact, keep config and state by default, restart service
- reinstall: same version or same artifact can be reapplied without manual cleanup
- uninstall: remove service and binary, keep config/state unless cleanup is explicitly requested

## Packaging preference

Artifact consumers should select packages using this order:

1. exact distro family match and preferred package type
2. same arch with another package type
3. generic glibc tarball for the same arch

Current recommendation:

- Debian/Ubuntu: prefer `.deb`
- Astra Linux: prefer `.deb`
- fallback or constrained host: prefer `tar.gz`
