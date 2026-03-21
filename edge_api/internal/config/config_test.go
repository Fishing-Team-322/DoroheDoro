package config

import (
	"os"
	"testing"
)

func TestLoadSupportsNewAgentGRPCEnvNames(t *testing.T) {
	t.Setenv("HTTP_LISTEN_ADDR", ":8080")
	t.Setenv("AGENT_GRPC_LISTEN_ADDR", ":9443")
	t.Setenv("NATS_URL", "nats://example:4222")
	t.Setenv("AGENT_ALLOW_INSECURE_DEV_MODE", "true")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("load config: %v", err)
	}
	if cfg.GRPC.ListenAddr != ":9443" {
		t.Fatalf("unexpected grpc listen addr: %s", cfg.GRPC.ListenAddr)
	}
}

func TestLoadRejectsImplicitInsecureAgentGRPC(t *testing.T) {
	clearConfigEnv(t)
	t.Setenv("HTTP_LISTEN_ADDR", ":8080")
	t.Setenv("AGENT_GRPC_LISTEN_ADDR", ":9090")
	t.Setenv("NATS_URL", "nats://example:4222")

	if _, err := Load(); err == nil {
		t.Fatal("expected insecure agent grpc config to fail without AGENT_ALLOW_INSECURE_DEV_MODE")
	}
}

func TestLoadRequiresClientCAWhenMTLSEnabled(t *testing.T) {
	clearConfigEnv(t)
	t.Setenv("HTTP_LISTEN_ADDR", ":8080")
	t.Setenv("AGENT_GRPC_LISTEN_ADDR", ":9090")
	t.Setenv("NATS_URL", "nats://example:4222")
	t.Setenv("AGENT_MTLS_ENABLED", "true")
	t.Setenv("AGENT_TLS_CERT_FILE", "server.crt")
	t.Setenv("AGENT_TLS_KEY_FILE", "server.key")

	if _, err := Load(); err == nil {
		t.Fatal("expected missing client CA to fail")
	}
}

func clearConfigEnv(t *testing.T) {
	t.Helper()
	for _, key := range []string{
		"HTTP_LISTEN_ADDR",
		"AGENT_GRPC_LISTEN_ADDR",
		"GRPC_LISTEN_ADDR",
		"NATS_URL",
		"AGENT_ALLOW_INSECURE_DEV_MODE",
		"AGENT_MTLS_ENABLED",
		"GRPC_MTLS_ENABLED",
		"AGENT_TLS_CERT_FILE",
		"AGENT_TLS_KEY_FILE",
		"AGENT_TLS_CLIENT_CA_FILE",
		"GRPC_TLS_CERT_FILE",
		"GRPC_TLS_KEY_FILE",
		"GRPC_CLIENT_CA_FILE",
	} {
		_ = os.Unsetenv(key)
	}
}
