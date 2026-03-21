.PHONY: build run test tidy fmt compose-config stack-up stack-down stack-logs edge-config edge-up edge-down swagger

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
