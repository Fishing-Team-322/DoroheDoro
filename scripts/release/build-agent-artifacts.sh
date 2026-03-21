#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AGENT_DIR="$ROOT_DIR/agent-rs"
DEFAULT_OUTPUT_DIR="$ROOT_DIR/dist/agent"
INSTALL_DOC="$ROOT_DIR/deployments/packaging/INSTALL.md"
SYSTEMD_UNIT="$ROOT_DIR/deployments/systemd/doro-agent.service"
CONFIG_EXAMPLE="$ROOT_DIR/deployments/examples/agent-config.example.yaml"
ENV_EXAMPLE="$ROOT_DIR/deployments/examples/agent.env.example"

TARGET="x86_64-unknown-linux-gnu"
ARCH="amd64"
VERSION="${VERSION:-}"
OUTPUT_DIR="${OUTPUT_DIR:-$DEFAULT_OUTPUT_DIR}"
FORMATS="${FORMATS:-tar.gz,deb}"
BINARY_PATH="${BINARY_PATH:-}"

usage() {
  cat <<'EOF'
Usage: build-agent-artifacts.sh [options]

Options:
  --target <rust-target>        Rust target triple. Default: x86_64-unknown-linux-gnu
  --arch <amd64|arm64>          Logical artifact arch. Default: amd64
  --version <version>           Release version. Default: git describe or timestamp
  --output-dir <dir>            Output root. Default: dist/agent
  --formats <csv>               tar.gz,deb or both. Default: tar.gz,deb
  --binary-path <path>          Use an already built doro-agent binary
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  else
    shasum -a 256 "$file" | awk '{print $1}'
  fi
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --arch)
      ARCH="$2"
      shift 2
      ;;
    --version)
      VERSION="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --formats)
      FORMATS="$2"
      shift 2
      ;;
    --binary-path)
      BINARY_PATH="$2"
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

require_cmd cargo
require_cmd tar
require_cmd install
require_cmd date

if [[ -z "$VERSION" ]]; then
  VERSION="$(git -C "$ROOT_DIR" describe --tags --always --dirty 2>/dev/null || date -u +%Y%m%d%H%M%S)"
fi

case "$ARCH" in
  amd64|arm64) ;;
  *)
    echo "unsupported arch: $ARCH" >&2
    exit 1
    ;;
esac

if [[ -z "$BINARY_PATH" ]]; then
  cargo build --manifest-path "$AGENT_DIR/Cargo.toml" --release --target "$TARGET" --bin doro-agent
  BINARY_PATH="$AGENT_DIR/target/$TARGET/release/doro-agent"
fi

if [[ ! -f "$BINARY_PATH" ]]; then
  echo "binary not found: $BINARY_PATH" >&2
  exit 1
fi

RELEASE_DIR="$OUTPUT_DIR/$VERSION"
STAGE_DIR="$RELEASE_DIR/.stage"
BUNDLE_ROOT="doro-agent_${VERSION}_linux_${ARCH}"
BUILD_TIME="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$RELEASE_DIR" "$STAGE_DIR"

build_bundle() {
  local bundle_dir="$STAGE_DIR/$BUNDLE_ROOT"
  rm -rf "$bundle_dir"
  mkdir -p "$bundle_dir/bin" "$bundle_dir/config" "$bundle_dir/systemd" "$bundle_dir/docs" "$bundle_dir/metadata"
  install -m 0755 "$BINARY_PATH" "$bundle_dir/bin/doro-agent"
  install -m 0644 "$CONFIG_EXAMPLE" "$bundle_dir/config/config.yaml"
  install -m 0644 "$ENV_EXAMPLE" "$bundle_dir/config/agent.env"
  install -m 0644 "$SYSTEMD_UNIT" "$bundle_dir/systemd/doro-agent.service"
  install -m 0644 "$INSTALL_DOC" "$bundle_dir/docs/INSTALL.md"
  cat > "$bundle_dir/metadata/build.json" <<EOF
{
  "name": "doro-agent",
  "version": "$(json_escape "$VERSION")",
  "target": "$(json_escape "$TARGET")",
  "arch": "$(json_escape "$ARCH")",
  "build_time": "$(json_escape "$BUILD_TIME")"
}
EOF
}

write_artifact_metadata() {
  local metadata_path="$1"
  local package_type="$2"
  local distro_family="$3"
  local install_mode="$4"
  local artifact_name="$5"
  local artifact_path="$6"
  local checksum_file="$7"
  local sha256="$8"
  local bundle_root="${9:-}"
  local packaging_preference="${10:-100}"

  cat > "$metadata_path" <<EOF
{
  "platform": "linux",
  "arch": "$(json_escape "$ARCH")",
  "package_type": "$(json_escape "$package_type")",
  "distro_family": "$(json_escape "$distro_family")",
  "install_mode": "$(json_escape "$install_mode")",
  "artifact_name": "$(json_escape "$artifact_name")",
  "artifact_path": "$(json_escape "$artifact_path")",
  "checksum_file": "$(json_escape "$checksum_file")",
  "sha256": "$(json_escape "$sha256")",
  "packaging_preference": $packaging_preference$(if [[ -n "$bundle_root" ]]; then printf ',\n  "bundle_root": "%s"' "$(json_escape "$bundle_root")"; fi)
}
EOF
}

if [[ ",$FORMATS," == *",tar.gz,"* ]]; then
  build_bundle
  TARBALL_NAME="${BUNDLE_ROOT}.tar.gz"
  TARBALL_PATH="$RELEASE_DIR/$TARBALL_NAME"
  tar -C "$STAGE_DIR" -czf "$TARBALL_PATH" "$BUNDLE_ROOT"
  TARBALL_SHA="$(sha256_file "$TARBALL_PATH")"
  printf '%s  %s\n' "$TARBALL_SHA" "$TARBALL_NAME" > "$TARBALL_PATH.sha256"
  write_artifact_metadata \
    "$RELEASE_DIR/${TARBALL_NAME}.artifact.json" \
    "tar.gz" \
    "generic-glibc" \
    "tarball" \
    "$TARBALL_NAME" \
    "$VERSION/$TARBALL_NAME" \
    "$VERSION/${TARBALL_NAME}.sha256" \
    "$TARBALL_SHA" \
    "$BUNDLE_ROOT" \
    "100"
fi

if [[ ",$FORMATS," == *",deb,"* ]]; then
  require_cmd dpkg-deb
  DEB_STAGE="$STAGE_DIR/deb"
  rm -rf "$DEB_STAGE"
  mkdir -p "$DEB_STAGE/DEBIAN" "$DEB_STAGE/usr/bin" "$DEB_STAGE/etc/doro-agent" "$DEB_STAGE/lib/systemd/system" "$DEB_STAGE/usr/share/doc/doro-agent"
  install -m 0755 "$BINARY_PATH" "$DEB_STAGE/usr/bin/doro-agent"
  install -m 0644 "$CONFIG_EXAMPLE" "$DEB_STAGE/etc/doro-agent/config.yaml"
  install -m 0644 "$ENV_EXAMPLE" "$DEB_STAGE/etc/doro-agent/agent.env"
  install -m 0644 "$SYSTEMD_UNIT" "$DEB_STAGE/lib/systemd/system/doro-agent.service"
  install -m 0644 "$INSTALL_DOC" "$DEB_STAGE/usr/share/doc/doro-agent/INSTALL.md"

  cat > "$DEB_STAGE/DEBIAN/control" <<EOF
Package: doro-agent
Version: $VERSION
Section: admin
Priority: optional
Architecture: $(if [[ "$ARCH" == "amd64" ]]; then echo "amd64"; else echo "arm64"; fi)
Maintainer: DoroheDoro Team <ops@dorohedoro.local>
Description: DoroheDoro Linux agent
EOF

  cat > "$DEB_STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
id -u doro-agent >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin doro-agent
mkdir -p /var/lib/doro-agent /var/log/doro-agent
chown -R doro-agent:doro-agent /var/lib/doro-agent /var/log/doro-agent
systemctl daemon-reload >/dev/null 2>&1 || true
exit 0
EOF
  chmod 0755 "$DEB_STAGE/DEBIAN/postinst"

  cat > "$DEB_STAGE/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e
if command -v systemctl >/dev/null 2>&1; then
  systemctl stop doro-agent.service >/dev/null 2>&1 || true
  systemctl disable doro-agent.service >/dev/null 2>&1 || true
fi
exit 0
EOF
  chmod 0755 "$DEB_STAGE/DEBIAN/prerm"

  cat > "$DEB_STAGE/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v systemctl >/dev/null 2>&1; then
  systemctl daemon-reload >/dev/null 2>&1 || true
fi
exit 0
EOF
  chmod 0755 "$DEB_STAGE/DEBIAN/postrm"

  DEB_NAME="doro-agent_${VERSION}_linux_${ARCH}.deb"
  DEB_PATH="$RELEASE_DIR/$DEB_NAME"
  dpkg-deb --build "$DEB_STAGE" "$DEB_PATH"
  DEB_SHA="$(sha256_file "$DEB_PATH")"
  printf '%s  %s\n' "$DEB_SHA" "$DEB_NAME" > "$DEB_PATH.sha256"

  write_artifact_metadata \
    "$RELEASE_DIR/${DEB_NAME}.debian.artifact.json" \
    "deb" \
    "debian" \
    "package" \
    "$DEB_NAME" \
    "$VERSION/$DEB_NAME" \
    "$VERSION/${DEB_NAME}.sha256" \
    "$DEB_SHA" \
    "" \
    "20"

  write_artifact_metadata \
    "$RELEASE_DIR/${DEB_NAME}.astra-linux.artifact.json" \
    "deb" \
    "astra-linux" \
    "package" \
    "$DEB_NAME" \
    "$VERSION/$DEB_NAME" \
    "$VERSION/${DEB_NAME}.sha256" \
    "$DEB_SHA" \
    "" \
    "30"
fi

echo "Agent artifacts written to $RELEASE_DIR"
