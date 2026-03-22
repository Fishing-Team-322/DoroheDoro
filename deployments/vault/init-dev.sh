#!/bin/sh
set -eu

export VAULT_ADDR="${VAULT_ADDR:-http://vault:8200}"
export VAULT_TOKEN="${VAULT_TOKEN:-root}"

until vault status >/dev/null 2>&1; do
  sleep 1
done

vault secrets enable -path=secret kv-v2 >/dev/null 2>&1 || true
vault auth enable approle >/dev/null 2>&1 || true
vault policy write control-plane /vault/bootstrap/control-plane-policy.hcl
vault policy write deployment-plane /vault/bootstrap/deployment-plane-policy.hcl
vault write auth/approle/role/control-plane token_policies=control-plane token_ttl=1h token_max_ttl=4h >/dev/null
vault write auth/approle/role/control-plane/role-id role_id=control-plane-role-id >/dev/null 2>&1 || true
vault write -f auth/approle/role/control-plane/custom-secret-id secret_id=control-plane-secret-id >/dev/null 2>&1 || true
vault write auth/approle/role/deployment-plane token_policies=deployment-plane token_ttl=1h token_max_ttl=4h >/dev/null
vault write auth/approle/role/deployment-plane/role-id role_id=dev-role-id >/dev/null 2>&1 || true
vault write -f auth/approle/role/deployment-plane/custom-secret-id secret_id=dev-secret-id >/dev/null 2>&1 || true

vault kv put secret/ssh/dev \
  ssh_user=root \
  ssh_private_key="replace-me-with-real-private-key" >/dev/null

if [ -f /certs/ca.crt ]; then
  vault kv put secret/agent/ca ca_pem=@/certs/ca.crt >/dev/null
fi

if [ -f /certs/agent.crt ]; then
  vault kv put secret/agent/cert cert_pem=@/certs/agent.crt >/dev/null
fi

if [ -f /certs/agent.key ]; then
  vault kv put secret/agent/key key_pem=@/certs/agent.key >/dev/null
fi

echo "vault dev bootstrap completed"
