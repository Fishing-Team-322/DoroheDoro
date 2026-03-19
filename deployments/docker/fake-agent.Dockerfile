FROM golang:1.25 AS build
WORKDIR /src
COPY go.mod ./
COPY . ./
RUN --mount=type=cache,target=/go/pkg/mod \
    --mount=type=cache,target=/root/.cache/go-build \
    make proto && go build -o /out/fake-agent ./cmd/fake-agent

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /out/fake-agent /usr/local/bin/fake-agent
COPY scripts /app/scripts
ENTRYPOINT ["/usr/local/bin/fake-agent"]
