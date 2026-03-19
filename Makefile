.PHONY: proto build run test up down tidy fmt compose-config swagger

proto:
	@test -f pkg/proto/ingest.pb.go

swagger:
	go run github.com/swaggo/swag/cmd/swag init -g cmd/server/main.go -o docs --parseInternal

fmt:
	gofmt -w $(shell rg --files -g '*.go')

build: proto
	go build ./...

run: proto
	go run ./cmd/server

test: proto
	go test ./...

compose-config:
	docker compose config

up:
	docker compose up --build

down:
	docker compose down -v

tidy:
	go mod tidy
