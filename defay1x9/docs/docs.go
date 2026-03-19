package docs

import "github.com/swaggo/swag"

const docTemplate = `{
  "swagger": "2.0",
  "info": {
    "title": "DoroheDoro HTTP API",
    "description": "HTTP API for health checks, log search, diagnostics, policy sync, and optional ClickHouse analytics.",
    "version": "1.0"
  },
  "basePath": "/",
  "paths": {
    "/": {"get": {"summary": "API root", "produces": ["application/json"], "responses": {"200": {"description": "OK"}}}},
    "/health": {"get": {"summary": "Liveness probe", "produces": ["application/json"], "responses": {"200": {"description": "OK"}}}},
    "/healthz": {"get": {"summary": "Liveness probe (legacy)", "produces": ["application/json"], "responses": {"200": {"description": "OK"}}}},
    "/ready": {"get": {"summary": "Readiness probe", "produces": ["application/json"], "responses": {"200": {"description": "Ready"}, "503": {"description": "Not ready"}}}},
    "/readyz": {"get": {"summary": "Readiness probe (legacy)", "produces": ["application/json"], "responses": {"200": {"description": "Ready"}, "503": {"description": "Not ready"}}}},
    "/openapi.json": {"get": {"summary": "OpenAPI document", "produces": ["application/json"], "responses": {"200": {"description": "OpenAPI spec"}}}},
    "/api/v1/logs/search": {"get": {"summary": "Search logs", "produces": ["application/json"], "parameters": [{"name": "q", "in": "query", "type": "string"}, {"name": "from", "in": "query", "type": "string"}, {"name": "to", "in": "query", "type": "string"}, {"name": "host", "in": "query", "type": "string"}, {"name": "service", "in": "query", "type": "string"}, {"name": "severity", "in": "query", "type": "string"}, {"name": "limit", "in": "query", "type": "integer", "default": 100}, {"name": "offset", "in": "query", "type": "integer", "default": 0}], "responses": {"200": {"description": "Search result"}, "502": {"description": "Backend error"}}}},
    "/api/v1/logs/{id}/context": {"get": {"summary": "Get log context", "produces": ["application/json"], "parameters": [{"name": "id", "in": "path", "required": true, "type": "string"}], "responses": {"200": {"description": "Context result"}, "502": {"description": "Backend error"}}}},
    "/api/v1/agents": {"get": {"summary": "List agents", "produces": ["application/json"], "responses": {"200": {"description": "Agents list"}}}},
    "/api/v1/agents/{id}": {"get": {"summary": "Get agent", "produces": ["application/json"], "parameters": [{"name": "id", "in": "path", "required": true, "type": "string"}], "responses": {"200": {"description": "Agent status"}, "404": {"description": "Not found"}}}},
    "/api/v1/agents/{id}/diagnostics": {"get": {"summary": "Get agent diagnostics", "produces": ["application/json"], "parameters": [{"name": "id", "in": "path", "required": true, "type": "string"}], "responses": {"200": {"description": "Diagnostics payload"}, "404": {"description": "Not found"}}}},
    "/api/v1/policy": {"get": {"summary": "Get effective policy", "produces": ["application/json"], "parameters": [{"name": "agent_id", "in": "query", "required": true, "type": "string"}, {"name": "current_revision", "in": "query", "type": "string"}], "responses": {"200": {"description": "Policy payload"}, "400": {"description": "Bad request"}}}},
    "/api/v1/analytics/histogram": {"get": {"summary": "Get event histogram", "produces": ["application/json"], "parameters": [{"name": "from", "in": "query", "type": "string"}, {"name": "to", "in": "query", "type": "string"}], "responses": {"200": {"description": "Histogram"}, "502": {"description": "Backend error"}, "503": {"description": "Analytics disabled"}}}},
    "/api/v1/analytics/severity": {"get": {"summary": "Get severity aggregation", "produces": ["application/json"], "parameters": [{"name": "from", "in": "query", "type": "string"}, {"name": "to", "in": "query", "type": "string"}, {"name": "limit", "in": "query", "type": "integer"}], "responses": {"200": {"description": "Severity counts"}, "502": {"description": "Backend error"}, "503": {"description": "Analytics disabled"}}}},
    "/api/v1/analytics/top-hosts": {"get": {"summary": "Get top hosts", "produces": ["application/json"], "parameters": [{"name": "from", "in": "query", "type": "string"}, {"name": "to", "in": "query", "type": "string"}, {"name": "limit", "in": "query", "type": "integer", "default": 10}], "responses": {"200": {"description": "Host counts"}, "502": {"description": "Backend error"}, "503": {"description": "Analytics disabled"}}}},
    "/api/v1/analytics/top-services": {"get": {"summary": "Get top services", "produces": ["application/json"], "parameters": [{"name": "from", "in": "query", "type": "string"}, {"name": "to", "in": "query", "type": "string"}, {"name": "limit", "in": "query", "type": "integer", "default": 10}], "responses": {"200": {"description": "Service counts"}, "502": {"description": "Backend error"}, "503": {"description": "Analytics disabled"}}}}
  }
}`

var SwaggerInfo = &swag.Spec{
	Version:          "1.0",
	BasePath:         "/",
	Title:            "DoroheDoro HTTP API",
	Description:      "HTTP API for health checks, log search, diagnostics, policy sync, and optional ClickHouse analytics.",
	InfoInstanceName: "swagger",
	SwaggerTemplate:  docTemplate,
}

func init() {
	swag.Register(SwaggerInfo.InfoInstanceName, SwaggerInfo)
}
