.PHONY: proto build run test up down tidy fmt

proto:
	@test -f pkg/proto/ingest.pb.go

fmt:
	gofmt -w $(shell rg --files -g '*.go')

build: proto
	go build -o bin/server ./cmd/server
	go build -o bin/fake-agent ./cmd/fake-agent

run: proto
	go run ./cmd/server

test: proto
	go test ./...

up:
	docker compose up --build

down:
	docker compose down -v

tidy:
	go mod tidy
