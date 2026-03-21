#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-.tmp/pki/dev}"
COMMON_NAME="${2:-edge-api}"
mkdir -p "$OUT_DIR"

if [[ ! -f "$OUT_DIR/ca.crt" || ! -f "$OUT_DIR/ca.key" ]]; then
  echo "missing CA in $OUT_DIR; run scripts/pki/dev-ca.sh first" >&2
  exit 1
fi

openssl genrsa -out "$OUT_DIR/server.key" 2048
openssl req -new -key "$OUT_DIR/server.key" \
  -subj "/CN=${COMMON_NAME}/O=DoroheDoro/C=RU" \
  -out "$OUT_DIR/server.csr"

cat > "$OUT_DIR/server.ext" <<EOF
subjectAltName=DNS:${COMMON_NAME},DNS:localhost,IP:127.0.0.1
extendedKeyUsage=serverAuth
EOF

openssl x509 -req -in "$OUT_DIR/server.csr" -CA "$OUT_DIR/ca.crt" -CAkey "$OUT_DIR/ca.key" \
  -CAcreateserial -out "$OUT_DIR/server.crt" -days 825 -sha256 -extfile "$OUT_DIR/server.ext"

rm -f "$OUT_DIR/server.csr" "$OUT_DIR/server.ext"
echo "Edge server certificate written to $OUT_DIR"
