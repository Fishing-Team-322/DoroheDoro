# Agent Distribution and Packaging

`AGENT` is now delivered as a Docker image first. The repository keeps a compatibility manifest so the current `WEB -> deployment-plane -> Ansible` flow can continue to work without `server-rs` changes.

## Delivery model

- published artifact: multi-arch Docker image for `linux/amd64` and `linux/arm64`
- default image contract: `docker.io/<org>/doro-agent`
- tags:
  - floating: `main`
  - immutable rollout tag: `main-<shortsha>`
- rollback pin: digest, not tag
- compatibility bridge: `agent-release-manifest.json` with `package_type=container` and `install_mode=docker_image`

Example contracts:

- [`../deployments/examples/agent-image-compat-manifest.example.json`](../deployments/examples/agent-image-compat-manifest.example.json)
- [`../deployments/examples/agent-image-metadata.example.json`](../deployments/examples/agent-image-metadata.example.json)

## Container runtime contract

Canonical in-container paths:

- binary: `/usr/bin/doro-agent`
- config: `/etc/doro-agent/config.yaml`
- PKI: `/etc/doro-agent/pki`
- state: `/var/lib/doro-agent`

Container commands:

- runtime: `doro-agent run --config /etc/doro-agent/config.yaml`
- preflight/health: `doro-agent doctor --config /etc/doro-agent/config.yaml`

Host mount contract:

- `/etc/doro-agent` -> `/etc/doro-agent` read-only
- `/var/lib/doro-agent` -> `/var/lib/doro-agent` read-write
- `/var/log` -> `/var/log` read-only

Current v1 limitation:

- host log sources outside `/var/log` are not auto-mounted; use `/var/log/...` sources or extend the role intentionally

## Local build and manifest generation

Build the container image locally:

```bash
docker build \
  -f agent-rs/packaging/container/Dockerfile \
  -t docker.io/example/doro-agent:main-local \
  .
```

Generate the compatibility manifest and metadata:

```bash
bash scripts/release/generate-agent-image-manifest.sh \
  --image-repository docker.io/example/doro-agent \
  --image-tag main-local \
  --image-digest sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  --version main-local
```

Output directory:

- `dist/agent-image/<version>/`

## CI publish flow

Workflow:

- [`.github/workflows/agent-release.yml`](../.github/workflows/agent-release.yml)

Trigger:

- push to `main`
- manual `workflow_dispatch`

Required GitHub configuration:

- repository variable: `AGENT_IMAGE_REPOSITORY`
- repository secret: `DOCKERHUB_USERNAME`
- repository secret: `DOCKERHUB_TOKEN`

Workflow result:

- pushes `linux/amd64,linux/arm64` image with `buildx`
- tags `main` and `main-<shortsha>`
- captures the manifest-list digest
- emits `agent-image-metadata.json`
- emits compatibility `agent-release-manifest.json`

## Ansible install contract

Role:

- [`../deployments/ansible/roles/doro-agent`](../deployments/ansible/roles/doro-agent)

Default mode:

- `doro_agent_install_mode: docker_image`

New operator-facing variables:

- `doro_agent_image_repository`
- `doro_agent_image_tag`
- `doro_agent_image_digest`
- `doro_agent_pull_policy`
- `doro_agent_container_name`
- `doro_agent_restart_policy`
- `doro_agent_container_engine`

Container engine selection order:

1. explicit `doro_agent_container_engine`
2. `docker`
3. `podman`

Compatibility behavior:

- direct image vars can be used for manual runs
- `deployment-plane` can still pass `doro_agent_selected_artifact`
- the role translates `package_type=container` / `install_mode=docker_image` into container deployment
- legacy binary install tasks remain available as `legacy_binary`

Direct image input is considered explicit only when you provide a real repository, tag override, or digest. The default placeholder does not bypass the manifest bridge by itself.

## Rollout and rollback

Host-side persisted state:

- running state: `/var/lib/doro-agent`
- last known good image: `/var/lib/doro-agent/last-known-good-image.json`

Deployment flow:

1. pull candidate image
2. run `doctor` preflight in a one-shot container
3. restart systemd unit
4. re-run health validation
5. persist last known good digest reference

Failure behavior:

- failed health does not silently succeed
- if a last known good digest exists, the role re-renders the runner, restarts the previous image and validates rollback health

## Smoke checklist

- image manifest returns `package_type=container`
- `deployment-plane` resolves `source_uri` as `docker.io/...:tag`
- Ansible succeeds on a Docker host
- Ansible succeeds on a Podman host
- Ansible fails clearly on a host with neither engine
- `/var/lib/doro-agent/state.db` and spool survive restart/recreate
- rollback by digest restores the previous healthy image
- `/var/log/syslog` style file sources still work through the default `/var/log` mount
