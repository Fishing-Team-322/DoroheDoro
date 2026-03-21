#!/usr/bin/env bash
set -euo pipefail

IMAGE_REPOSITORY=""
IMAGE_TAG=""
IMAGE_DIGEST=""
VERSION=""
RELEASE_CHANNEL="main"
OUTPUT_DIR=""
GENERATED_AT="${GENERATED_AT:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"

usage() {
  cat <<'EOF'
Usage: generate-agent-image-manifest.sh \
  --image-repository <repo> \
  --image-tag <tag> \
  --image-digest <sha256:...> \
  --version <version> \
  [--release-channel <channel>] \
  [--output-dir <dir>]
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --image-repository)
      IMAGE_REPOSITORY="$2"
      shift 2
      ;;
    --image-tag)
      IMAGE_TAG="$2"
      shift 2
      ;;
    --image-digest)
      IMAGE_DIGEST="$2"
      shift 2
      ;;
    --version)
      VERSION="$2"
      shift 2
      ;;
    --release-channel)
      RELEASE_CHANNEL="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$IMAGE_REPOSITORY" || -z "$IMAGE_TAG" || -z "$IMAGE_DIGEST" || -z "$VERSION" ]]; then
  usage >&2
  exit 1
fi

if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="$(pwd)/dist/agent-image/${VERSION}"
fi

case "$IMAGE_DIGEST" in
  sha256:*)
    DIGEST_HEX="${IMAGE_DIGEST#sha256:}"
    ;;
  *)
    echo "image digest must start with sha256:" >&2
    exit 1
    ;;
esac

if [[ ! "$DIGEST_HEX" =~ ^[a-f0-9]{64}$ ]]; then
  echo "image digest must contain 64 lowercase hex characters" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

TAGGED_REFERENCE="${IMAGE_REPOSITORY}:${IMAGE_TAG}"
DIGEST_REFERENCE="${IMAGE_REPOSITORY}@${IMAGE_DIGEST}"

cat > "${OUTPUT_DIR}/agent-image-metadata.json" <<EOF
{
  "schema_version": "1.0",
  "version": "${VERSION}",
  "release_channel": "${RELEASE_CHANNEL}",
  "created_at": "${GENERATED_AT}",
  "image_repository": "${IMAGE_REPOSITORY}",
  "image_tag": "${IMAGE_TAG}",
  "image_digest": "${IMAGE_DIGEST}",
  "image_reference": "${TAGGED_REFERENCE}",
  "image_digest_reference": "${DIGEST_REFERENCE}",
  "platforms": ["linux/amd64", "linux/arm64"]
}
EOF

cat > "${OUTPUT_DIR}/agent-release-manifest.json" <<EOF
{
  "schema_version": "1.0",
  "version": "${VERSION}",
  "generated_at": "${GENERATED_AT}",
  "release_channel": "${RELEASE_CHANNEL}",
  "image": {
    "schema_version": "1.0",
    "version": "${VERSION}",
    "created_at": "${GENERATED_AT}",
    "image_repository": "${IMAGE_REPOSITORY}",
    "image_tag": "${IMAGE_TAG}",
    "image_digest": "${IMAGE_DIGEST}",
    "image_reference": "${TAGGED_REFERENCE}",
    "image_digest_reference": "${DIGEST_REFERENCE}",
    "platforms": ["linux/amd64", "linux/arm64"]
  },
  "artifacts": [
    {
      "platform": "linux",
      "arch": "amd64",
      "package_type": "container",
      "distro_family": "generic-glibc",
      "install_mode": "docker_image",
      "artifact_name": "doro-agent_${VERSION}_linux_amd64.image-ref",
      "artifact_path": "${TAGGED_REFERENCE}",
      "source_uri": "${TAGGED_REFERENCE}",
      "checksum_file": "${IMAGE_DIGEST}",
      "sha256": "${DIGEST_HEX}",
      "image_repository": "${IMAGE_REPOSITORY}",
      "image_tag": "${IMAGE_TAG}",
      "image_digest": "${IMAGE_DIGEST}",
      "image_reference": "${TAGGED_REFERENCE}",
      "image_digest_reference": "${DIGEST_REFERENCE}",
      "packaging_preference": 100
    },
    {
      "platform": "linux",
      "arch": "arm64",
      "package_type": "container",
      "distro_family": "generic-glibc",
      "install_mode": "docker_image",
      "artifact_name": "doro-agent_${VERSION}_linux_arm64.image-ref",
      "artifact_path": "${TAGGED_REFERENCE}",
      "source_uri": "${TAGGED_REFERENCE}",
      "checksum_file": "${IMAGE_DIGEST}",
      "sha256": "${DIGEST_HEX}",
      "image_repository": "${IMAGE_REPOSITORY}",
      "image_tag": "${IMAGE_TAG}",
      "image_digest": "${IMAGE_DIGEST}",
      "image_reference": "${TAGGED_REFERENCE}",
      "image_digest_reference": "${DIGEST_REFERENCE}",
      "packaging_preference": 100
    }
  ]
}
EOF

echo "Image compatibility manifest written to ${OUTPUT_DIR}/agent-release-manifest.json"
