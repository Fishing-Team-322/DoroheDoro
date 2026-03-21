FROM alpine:3.21 AS build

RUN apk add --no-cache bash

WORKDIR /src

COPY scripts/release /src/scripts/release

ARG AGENT_IMAGE_REPOSITORY=docker.io/<org>/doro-agent
ARG AGENT_IMAGE_TAG=main
ARG AGENT_IMAGE_DIGEST=sha256:0000000000000000000000000000000000000000000000000000000000000000
ARG AGENT_IMAGE_VERSION=main
ARG AGENT_RELEASE_CHANNEL=main

RUN bash /src/scripts/release/generate-agent-image-manifest.sh \
    --image-repository "${AGENT_IMAGE_REPOSITORY}" \
    --image-tag "${AGENT_IMAGE_TAG}" \
    --image-digest "${AGENT_IMAGE_DIGEST}" \
    --version "${AGENT_IMAGE_VERSION}" \
    --release-channel "${AGENT_RELEASE_CHANNEL}" \
    --output-dir /out && \
    cp /out/agent-release-manifest.json /out/manifest.json

FROM nginx:1.27-alpine

COPY --from=build /out /usr/share/nginx/html

EXPOSE 8080

RUN sed -i 's/listen       80;/listen       8080;/' /etc/nginx/conf.d/default.conf
