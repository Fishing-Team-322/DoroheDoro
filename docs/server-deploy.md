# Server Deploy Notes for `fishingteam.su`

This document describes the VPS-friendly stack and Nginx layout for the current demo/staging deployment.

Related docs:

- local/server stack overview: [`docs/demo-stack.md`](./demo-stack.md)
- agent distribution contract: [`docs/agent-distribution.md`](./agent-distribution.md)
- dev/test PKI flow: [`docs/dev-pki.md`](./dev-pki.md)

## 1. Start the server stack

Use the server compose file:

```bash
docker compose -f docker-compose.server.yml up -d --build
```

The server stack starts:

- `frontend`
- `edge-api`
- `nats`
- `postgres`
- `enrollment-plane`
- `control-plane`
- `deployment-plane`

Published host ports are intentionally localhost-bound:

- `127.0.0.1:13000 -> frontend:3000`
- `127.0.0.1:18080 -> edge-api:8080`
- `127.0.0.1:19090 -> edge-api:9090`

Internal runtime services stay on the compose network and are not published publicly.

## 2. Environment

The edge boundary uses [`edge_api/.env.server`](../edge_api/.env.server).

Important values there:

- `PUBLIC_BASE_URL=https://fishingteam.su`
- `EDGE_PUBLIC_URL=https://fishingteam.su`
- `AGENT_PUBLIC_GRPC_ADDR=fishingteam.su:443`
- `CORS_ALLOWED_ORIGINS=https://fishingteam.su,https://www.fishingteam.su`
- `COOKIE_SECURE=true`
- `AGENT_MTLS_ENABLED=true`
- `AGENT_TLS_CERT_FILE=/certs/server.crt`
- `AGENT_TLS_KEY_FILE=/certs/server.key`
- `AGENT_TLS_CLIENT_CA_FILE=/certs/ca.crt`

Before public exposure, replace:

- `DEV_TEST_PASSWORD`

The current HTTP auth remains a compat/demo stub. This stack is appropriate for demo/staging, not for a hardened public production rollout.

## 3. Nginx routing

Recommended routing:

- `/` -> `http://127.0.0.1:13000`
- `/api/edge/` -> `http://127.0.0.1:18080` with `/api/edge/` prefix stripped
- gRPC agent ingress -> `127.0.0.1:19090`
- `/docs`
- `/openapi.json`
- `/openapi.yaml`
- `/healthz`
- `/readyz`

Example server block:

```nginx
server {
    listen 80;
    server_name fishingteam.su www.fishingteam.su;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name fishingteam.su www.fishingteam.su;

    ssl_certificate /etc/letsencrypt/live/fishingteam.su/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/fishingteam.su/privkey.pem;

    client_max_body_size 10m;

    location / {
        proxy_pass http://127.0.0.1:13000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 300s;
    }

    location /api/edge/ {
        rewrite ^/api/edge/(.*)$ /$1 break;
        proxy_pass http://127.0.0.1:18080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 3600s;
        add_header X-Accel-Buffering no;
    }

    location = /docs {
        proxy_pass http://127.0.0.1:18080/docs;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /docs/ {
        proxy_pass http://127.0.0.1:18080/docs/;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location = /openapi.json {
        proxy_pass http://127.0.0.1:18080/openapi.json;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location = /openapi.yaml {
        proxy_pass http://127.0.0.1:18080/openapi.yaml;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location = /healthz {
        proxy_pass http://127.0.0.1:18080/healthz;
    }

    location = /readyz {
        proxy_pass http://127.0.0.1:18080/readyz;
    }

    # gRPC agent ingress on the same public domain.
    location /dorohedoro.edge.v1.AgentIngressService/ {
        grpc_pass grpc://127.0.0.1:19090;
        grpc_read_timeout 3600s;
        grpc_send_timeout 3600s;
        grpc_set_header Host $host;
        grpc_set_header X-Real-IP $remote_addr;
        grpc_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        grpc_set_header X-Forwarded-Proto $scheme;
    }
}
```

## 4. Checks after deploy

Local checks on the VPS:

```bash
curl -i http://127.0.0.1:13000/
curl -i http://127.0.0.1:18080/
curl -i http://127.0.0.1:18080/docs
curl -i http://127.0.0.1:18080/openapi.json
curl -i http://127.0.0.1:18080/healthz
curl -i http://127.0.0.1:18080/readyz
```

Checks through the public domain:

```bash
curl -I https://fishingteam.su/
curl -I https://fishingteam.su/docs
curl -I https://fishingteam.su/openapi.json
curl -I https://fishingteam.su/api/edge/api/v1/me
```

## 5. gRPC note

The pre-production single-host stack publishes gRPC only on localhost and expects the reverse proxy to expose it on the same public domain.

That is enough for:

- local compose smoke
- server-side demo/staging of WEB/API
- domain-based external agent enrollment with gRPC+mTLS

The intended path is:

`agent -> fishingteam.su:443 -> reverse proxy grpc_pass -> 127.0.0.1:19090 -> edge-api`

Do not expose `19090` directly on the internet. Keep it loopback-bound and let the reverse proxy forward HTTP/2 gRPC traffic.

## 6. Real agent rollout note

For the current repository state:

- `edge-api` is ready to enforce TLS + mTLS on the public agent ingress
- transport-level mTLS is reproducibly validated with `fake-agent`
- the real `agent-rs` install contract already supports the public domain path through:
  - `edge_url=https://fishingteam.su`
  - `edge_grpc_addr=fishingteam.su:443`

Current honest limitation:

- `agent-rs` does not yet expose client-certificate configuration through the runtime/install contract

That means the first pre-production practical rollout should be split into:

1. real remote-host rollout against the public TLS boundary
2. separate mTLS ingress validation with `fake-agent`

Use the provided examples for the three-host rollout:

- [`deployments/ansible/inventories/three-hosts.example.ini`](../deployments/ansible/inventories/three-hosts.example.ini)
- [`deployments/ansible/group_vars/agents.example.yml`](../deployments/ansible/group_vars/agents.example.yml)
