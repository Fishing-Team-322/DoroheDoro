FROM rust:1.90-bookworm AS build

ARG RUST_BIN=enrollment-plane

WORKDIR /src/server-rs

COPY server-rs/Cargo.toml server-rs/Cargo.lock ./
COPY server-rs/crates ./crates
COPY server-rs/migrations ./migrations
COPY contracts /src/contracts
COPY deployments /src/deployments

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/src/server-rs/target,sharing=locked \
    cargo build --locked -p ${RUST_BIN} --release && \
    cp /src/server-rs/target/release/${RUST_BIN} /tmp/server-runtime

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    openssh-client \
    python3 \
    python3-pip \
    sshpass && \
    pip3 install --break-system-packages --no-cache-dir \
      ansible-core==2.17.7 \
      ansible-runner==2.4.0 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /tmp/server-runtime /usr/local/bin/server-runtime
COPY --from=build /src/deployments /app/deployments

ENTRYPOINT ["/usr/local/bin/server-runtime"]
