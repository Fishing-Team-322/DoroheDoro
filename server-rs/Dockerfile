FROM rust:1.90-bookworm AS build
WORKDIR /src/server-rs

COPY server-rs/Cargo.toml server-rs/Cargo.lock ./
COPY server-rs/crates ./crates
COPY server-rs/migrations ./migrations
COPY contracts /src/contracts

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build -p enrollment-plane --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/server-rs/target/release/enrollment-plane /usr/local/bin/enrollment-plane
ENTRYPOINT ["/usr/local/bin/enrollment-plane"]
