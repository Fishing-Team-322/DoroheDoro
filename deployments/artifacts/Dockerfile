FROM rust:1.90-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends jq && rm -rf /var/lib/apt/lists/*

WORKDIR /src

COPY . .

RUN bash scripts/release/build-agent-artifacts.sh --version 0.2.0 --output-dir /out && \
    bash scripts/release/generate-manifest.sh --version 0.2.0 --output-dir /out && \
    mkdir -p /srv/0.2.0 && \
    cp -r /out/0.2.0/* /srv/0.2.0/ && \
    cp /out/0.2.0/agent-release-manifest.json /srv/manifest.json

FROM nginx:1.27-alpine

COPY --from=build /srv /usr/share/nginx/html

EXPOSE 8080

RUN sed -i 's/listen       80;/listen       8080;/' /etc/nginx/conf.d/default.conf
