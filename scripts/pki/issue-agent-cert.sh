#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-.tmp/pki/dev}"
COMMON_NAME="${2:-fake-agent}"
mkdir -p "$OUT_DIR"

if [[ ! -f "$OUT_DIR/ca.crt" || ! -f "$OUT_DIR/ca.key" ]]; then
  echo "missing CA in $OUT_DIR; run scripts/pki/dev-ca.sh first" >&2
  exit 1
fi

openssl genrsa -out "$OUT_DIR/agent.key" 2048
openssl req -new -key "$OUT_DIR/agent.key" \
  -subj "/CN=${COMMON_NAME}/OU=agents/O=DoroheDoro/C=RU" \
  -out "$OUT_DIR/agent.csr"

cat > "$OUT_DIR/agent.ext" <<EOF
extendedKeyUsage=clientAuth
subjectAltName=DNS:${COMMON_NAME}
EOF

openssl x509 -req -in "$OUT_DIR/agent.csr" -CA "$OUT_DIR/ca.crt" -CAkey "$OUT_DIR/ca.key" \
  -CAcreateserial -out "$OUT_DIR/agent.crt" -days 825 -sha256 -extfile "$OUT_DIR/agent.ext"

rm -f "$OUT_DIR/agent.csr" "$OUT_DIR/agent.ext"
echo "Agent client certificate written to $OUT_DIR"
