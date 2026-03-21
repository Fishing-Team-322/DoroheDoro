.PHONY: build run test tidy fmt compose-config stack-up stack-down stack-logs edge-config edge-up edge-down swagger agent-release agent-manifest pki-dev-ca pki-edge-cert pki-agent-cert

APP_DIR := edge_api

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
	cd $(APP_DIR) && go run github.com/swaggo/swag/cmd/swag init -g cmd/server/main.go -o docs --parseInternal

compose-config:
	docker compose config

stack-up:
	docker compose up --build

stack-down:
	docker compose down

stack-logs:
	docker compose logs -f

edge-config:
	docker compose -f docker-compose.server.yml config

edge-up:
	docker compose -f docker-compose.server.yml up -d --build

edge-down:
	docker compose -f docker-compose.server.yml down

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
