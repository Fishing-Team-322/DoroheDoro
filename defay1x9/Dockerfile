FROM golang:1.25 AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN --mount=type=cache,target=/go/pkg/mod go mod download
COPY . .
RUN --mount=type=cache,target=/go/pkg/mod \
    --mount=type=cache,target=/root/.cache/go-build \
    go build -o /out/edge-api ./cmd/edge-api && \
    go build -o /out/fake-agent ./cmd/fake-agent

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /out/edge-api /usr/local/bin/edge-api
COPY --from=build /out/fake-agent /usr/local/bin/fake-agent
ENTRYPOINT ["/usr/local/bin/edge-api"]
