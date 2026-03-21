#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-.tmp/pki/dev}"
mkdir -p "$OUT_DIR"

openssl genrsa -out "$OUT_DIR/ca.key" 4096
openssl req -x509 -new -nodes -key "$OUT_DIR/ca.key" -sha256 -days 3650 \
  -subj "/CN=DoroheDoro Dev CA/O=DoroheDoro/C=RU" \
  -out "$OUT_DIR/ca.crt"

echo "Dev CA written to $OUT_DIR"
