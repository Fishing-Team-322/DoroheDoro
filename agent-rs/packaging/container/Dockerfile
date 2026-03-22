FROM --platform=$TARGETPLATFORM rust:1.90-bookworm AS build

WORKDIR /src

COPY contracts /src/contracts
COPY agent-rs /src/agent-rs

WORKDIR /src/agent-rs

RUN cargo build --release --bin doro-agent

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates tzdata libssl3 && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd --system doro-agent && \
    useradd --system --gid doro-agent --home-dir /var/lib/doro-agent --shell /usr/sbin/nologin doro-agent && \
    mkdir -p /etc/doro-agent/pki /var/lib/doro-agent /var/log/doro-agent /tmp && \
    chown -R doro-agent:doro-agent /var/lib/doro-agent /var/log/doro-agent

COPY --from=build /src/agent-rs/target/release/doro-agent /usr/bin/doro-agent
COPY agent-rs/config/agent.container.example.yaml /etc/doro-agent/config.yaml
COPY agent-rs/config/agent.env.example /etc/doro-agent/agent.env
COPY agent-rs/config/security-rules.example.yaml /etc/doro-agent/security-rules.yaml

RUN chmod 0755 /usr/bin/doro-agent && \
    chmod 0644 /etc/doro-agent/config.yaml /etc/doro-agent/agent.env /etc/doro-agent/security-rules.yaml

VOLUME ["/etc/doro-agent", "/var/lib/doro-agent"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=45s --retries=3 \
  CMD ["/usr/bin/doro-agent", "health", "--config", "/etc/doro-agent/config.yaml"]

ENTRYPOINT ["/usr/bin/doro-agent"]
CMD ["run", "--config", "/etc/doro-agent/config.yaml"]
