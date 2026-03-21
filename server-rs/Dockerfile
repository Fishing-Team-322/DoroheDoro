FROM rust:1.90-bookworm AS build

ARG RUST_BIN=enrollment-plane

WORKDIR /src/server-rs

COPY server-rs/Cargo.toml server-rs/Cargo.lock ./
COPY server-rs/crates ./crates
COPY server-rs/migrations ./migrations
COPY contracts /src/contracts

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/server-rs/target \
    cargo build --locked -p ${RUST_BIN} --release && \
    cp /src/server-rs/target/release/${RUST_BIN} /tmp/server-runtime

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /tmp/server-runtime /usr/local/bin/server-runtime

ENTRYPOINT ["/usr/local/bin/server-runtime"]
