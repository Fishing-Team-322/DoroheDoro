package apidocs

import "embed"

// Files embeds the OpenAPI specification and local browser UI assets.
//
//go:embed openapi.json openapi.yaml ui/*
var Files embed.FS
