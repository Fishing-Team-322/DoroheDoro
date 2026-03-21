.PHONY: build run test tidy fmt compose-config stack-up stack-down stack-logs edge-config edge-up edge-down swagger swagger-check agent-release agent-manifest pki-dev-ca pki-edge-cert pki-agent-cert server-smoke

APP_DIR := edge_api
SERVER_ENV_FILE ?= .env.server

build:
	cd $(APP_DIR) && go build ./cmd/edge-api ./cmd/fake-agent ./cmd/dev-certs

run:
	cd $(APP_DIR) && go run ./cmd/edge-api

test:
	cd $(APP_DIR) && go test ./...

tidy:
	cd $(APP_DIR) && go mod tidy

fmt:
	cd $(APP_DIR) && gofmt -w $$(find . -name '*.go' -type f | sort)

swagger:
	cd $(APP_DIR) && node scripts/render-openapi.cjs

swagger-check:
	cd $(APP_DIR) && node scripts/render-openapi.cjs --check

compose-config:
	docker compose config

stack-up:
	docker compose up --build

stack-down:
	docker compose down

stack-logs:
	docker compose logs -f

edge-config:
	docker compose --env-file $(SERVER_ENV_FILE) -f docker-compose.server.yml config

edge-up:
	docker compose --env-file $(SERVER_ENV_FILE) -f docker-compose.server.yml up -d --build

edge-down:
	docker compose --env-file $(SERVER_ENV_FILE) -f docker-compose.server.yml down

server-smoke:
	cd server-rs && cargo test --manifest-path Cargo.toml -p enrollment-plane --test smoke -- --ignored --nocapture
	cd server-rs && cargo test --manifest-path Cargo.toml -p control-plane --test smoke -- --ignored --nocapture
	cd server-rs && cargo test --manifest-path Cargo.toml -p deployment-plane --test smoke -- --ignored --nocapture

agent-release:
	bash scripts/release/build-agent-artifacts.sh

agent-manifest:
	bash scripts/release/generate-manifest.sh --version "$(VERSION)"

pki-dev-ca:
	bash scripts/pki/dev-ca.sh

pki-edge-cert:
	bash scripts/pki/issue-edge-cert.sh

pki-agent-cert:
	bash scripts/pki/issue-agent-cert.sh
