.PHONY: build run test tidy fmt compose-config server-up server-down swagger

APP_DIR := defay1x9

build:
	cd $(APP_DIR) && go build ./...

run:
	cd $(APP_DIR) && go run ./cmd/server

test:
	cd $(APP_DIR) && go test ./...

tidy:
	cd $(APP_DIR) && go mod tidy

fmt:
	cd $(APP_DIR) && gofmt -w $$(find . -name '*.go' -type f | sort)

swagger:
	cd $(APP_DIR) && go run github.com/swaggo/swag/cmd/swag init -g cmd/server/main.go -o docs --parseInternal

compose-config:
	docker compose -f docker-compose.server.yml config

server-up:
	docker compose -f docker-compose.server.yml up -d --build

server-down:
	docker compose -f docker-compose.server.yml down
