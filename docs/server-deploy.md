# Server Deploy Notes for `fishingteam.su`

This document describes the VPS-friendly stack and Nginx layout for the current demo/staging deployment.

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

Internal runtime services stay on the compose network and are not published publicly.

## 2. Environment

The edge boundary uses [`edge_api/.env.server`](../edge_api/.env.server).

Important values there:

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

The server compose keeps gRPC internal by default. That is enough for:

- local compose smoke
- server-side demo/staging of WEB/API

If you want real external agents to enroll over the public internet, add a separate external gRPC ingress step later. Do not expose gRPC casually without deciding how TLS and mTLS will be terminated and forwarded.
