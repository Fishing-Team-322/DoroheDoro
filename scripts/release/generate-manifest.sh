#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT_DIR/dist/agent}"
VERSION="${VERSION:-}"

usage() {
  cat <<'EOF'
Usage: generate-manifest.sh --version <version> [--output-dir <dir>]
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
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

if [[ -z "$VERSION" ]]; then
  echo "--version is required" >&2
  exit 1
fi

require_cmd jq

ARTIFACT_DIR="$OUTPUT_DIR/$VERSION"
if [[ ! -d "$ARTIFACT_DIR" ]]; then
  echo "artifact directory not found: $ARTIFACT_DIR" >&2
  exit 1
fi

mapfile -t ARTIFACT_JSONS < <(find "$ARTIFACT_DIR" -maxdepth 1 -name '*.artifact.json' | sort)
if [[ ${#ARTIFACT_JSONS[@]} -eq 0 ]]; then
  echo "no artifact descriptor files found in $ARTIFACT_DIR" >&2
  exit 1
fi

GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MANIFEST_PATH="$ARTIFACT_DIR/agent-release-manifest.json"

jq -s \
  --arg version "$VERSION" \
  --arg generated_at "$GENERATED_AT" \
  '{
    schema_version: "1.0",
    version: $version,
    generated_at: $generated_at,
    artifacts: .
  }' \
  "${ARTIFACT_JSONS[@]}" > "$MANIFEST_PATH"

echo "Release manifest written to $MANIFEST_PATH"
