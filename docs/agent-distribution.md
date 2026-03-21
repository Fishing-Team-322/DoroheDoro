# Agent Distribution and Packaging

This repository now defines a delivery contract for `AGENT` artifacts without changing `agent-rs` runtime code.

## Release matrix

Current supported release targets:

- `linux-amd64`
- `linux-arm64`
- Debian/Ubuntu compatible `.deb`
- Astra Linux compatible `.deb`
- generic `tar.gz` fallback for glibc-based Linux hosts

This phase does not promise every Unix variant. The supported baseline is Linux + systemd.

## Artifact formats

Each release can publish:

- `doro-agent_<version>_linux_<arch>.tar.gz`
- `doro-agent_<version>_linux_<arch>.deb`
- `*.sha256`
- `*.artifact.json`
- `agent-release-manifest.json`

The formal manifest schema lives in:

- [`deployments/artifacts/manifest.schema.json`](../deployments/artifacts/manifest.schema.json)

An example manifest lives in:

- [`deployments/artifacts/example.manifest.json`](../deployments/artifacts/example.manifest.json)

## Release bundle contents

Both package modes are aligned to the same install contract:

- `doro-agent` binary
- `doro-agent.service`
- example config
- example env file
- install notes
- build metadata with version, arch and build time

The canonical install contract is documented in:

- [`deployments/packaging/INSTALL.md`](../deployments/packaging/INSTALL.md)

## Build artifacts locally

Generic build:

```bash
bash scripts/release/build-agent-artifacts.sh --version 0.2.0
```

Per target:

```bash
bash scripts/release/build-agent-artifacts.sh \
  --target x86_64-unknown-linux-gnu \
  --arch amd64 \
  --version 0.2.0

bash scripts/release/build-agent-artifacts.sh \
  --target aarch64-unknown-linux-gnu \
  --arch arm64 \
  --version 0.2.0
```

Generate the combined manifest:

```bash
bash scripts/release/generate-manifest.sh --version 0.2.0
```

Artifacts are written into:

- `dist/agent/<version>/`

## CI workflow

The release workflow lives in:

- [`.github/workflows/agent-release.yml`](../.github/workflows/agent-release.yml)

It is designed to:

- build `amd64` and `arm64` artifacts
- emit `tar.gz` and `.deb`
- attach checksums
- generate a release manifest

## Astra Linux note

Astra Linux is handled as a Debian-family packaging target.

Practical rule for this phase:

- prefer `.deb` on Astra
- fall back to `tar.gz` when package installation is constrained

The manifest includes `distro_family` so Ansible or future deployment runtime can prefer the correct artifact without hardcoded guesswork.

## Ansible consumption

The Ansible install layer now lives in:

- [`deployments/ansible/playbooks/install-agent.yml`](../deployments/ansible/playbooks/install-agent.yml)
- [`deployments/ansible/roles/doro-agent`](../deployments/ansible/roles/doro-agent)

It consumes:

- a local manifest path, or
- a manifest URL plus optional release base URL

Artifact selection is based on:

- platform
- arch
- distro family
- package type preference

That same manifest contract is intended to be reused later by `deployment-plane`.
