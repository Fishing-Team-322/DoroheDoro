package main

import (
	"context"
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"os"
	"time"

	edgev1 "github.com/example/dorohedoro/contracts/proto"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/credentials/insecure"
)

func main() {
	grpcAddr := getenv("EDGE_API_GRPC_ADDR", "localhost:9090")
	transportCreds, err := dialTransportCredentials()
	if err != nil {
		panic(err)
	}
	conn, err := grpc.Dial(
		grpcAddr,
		grpc.WithTransportCredentials(transportCreds),
		grpc.WithDefaultCallOptions(grpc.ForceCodec(edgev1.JSONCodec{})),
	)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	client := edgev1.NewAgentIngressServiceClient(conn)
	ctx := context.Background()

	enrollResp, err := client.Enroll(ctx, &edgev1.EnrollRequest{
		EnrollmentToken: getenv("FAKE_AGENT_ENROLLMENT_TOKEN", "dev-bootstrap-token"),
		Host:            getenv("FAKE_AGENT_HOST", "demo-host"),
		Labels: map[string]string{
			"agent":   "fake-agent",
			"mode":    "smoke",
			"version": getenv("FAKE_AGENT_VERSION", "0.1.0"),
		},
	})
	if err != nil {
		panic(err)
	}
	agentID := enrollResp.AgentId
	fmt.Printf("enroll agent_id=%s request_id=%s\n", agentID, enrollResp.RequestId)

	policyResp, err := client.FetchPolicy(ctx, &edgev1.FetchPolicyRequest{AgentId: agentID, CurrentRevision: ""})
	if err != nil {
		panic(err)
	}
	fmt.Printf("policy changed=%v revision=%s\n", policyResp.Changed, policyResp.Policy.Revision)

	if _, err := client.SendHeartbeat(ctx, &edgev1.HeartbeatRequest{
		AgentId:      agentID,
		Host:         getenv("FAKE_AGENT_HOST", "demo-host"),
		SentAtUnixMs: time.Now().UnixMilli(),
		Status:       "online",
	}); err != nil {
		panic(err)
	}

	if _, err := client.SendDiagnostics(ctx, &edgev1.DiagnosticsRequest{
		AgentId:      agentID,
		Host:         getenv("FAKE_AGENT_HOST", "demo-host"),
		SentAtUnixMs: time.Now().UnixMilli(),
		PayloadJSON:  `{"cpu":12,"mem":34}`,
	}); err != nil {
		panic(err)
	}

	logsResp, err := client.IngestLogs(ctx, &edgev1.IngestLogsRequest{
		AgentId:      agentID,
		Host:         getenv("FAKE_AGENT_HOST", "demo-host"),
		SentAtUnixMs: time.Now().UnixMilli(),
		Events: []*edgev1.AgentLog{
			{TimestampUnixMs: time.Now().Add(-2 * time.Second).UnixMilli(), Service: "nginx", Severity: "warn", Message: "upstream timeout", Labels: map[string]string{"env": "demo"}},
			{TimestampUnixMs: time.Now().UnixMilli(), Service: "sshd", Severity: "info", Message: "accepted password", Labels: map[string]string{"env": "demo"}},
		},
	})
	if err != nil {
		panic(err)
	}
	fmt.Printf("ingest accepted=%v count=%d request_id=%s\n", logsResp.Accepted, logsResp.AcceptedCount, logsResp.RequestId)
}

func dialTransportCredentials() (credentials.TransportCredentials, error) {
	if getenv("FAKE_AGENT_ALLOW_INSECURE", "false") == "true" {
		return insecure.NewCredentials(), nil
	}

	serverName := getenv("FAKE_AGENT_TLS_SERVER_NAME", "")
	caFile := getenv("FAKE_AGENT_TLS_CA_FILE", "")
	certFile := getenv("FAKE_AGENT_TLS_CERT_FILE", "")
	keyFile := getenv("FAKE_AGENT_TLS_KEY_FILE", "")

	tlsConfig := &tls.Config{
		MinVersion:         tls.VersionTLS12,
		ServerName:         serverName,
		InsecureSkipVerify: getenv("FAKE_AGENT_TLS_INSECURE_SKIP_VERIFY", "false") == "true",
	}

	if caFile != "" {
		caPEM, err := os.ReadFile(caFile)
		if err != nil {
			return nil, fmt.Errorf("read CA file: %w", err)
		}
		rootCAs := x509.NewCertPool()
		if ok := rootCAs.AppendCertsFromPEM(caPEM); !ok {
			return nil, fmt.Errorf("parse CA bundle")
		}
		tlsConfig.RootCAs = rootCAs
	}

	if certFile != "" || keyFile != "" {
		if certFile == "" || keyFile == "" {
			return nil, fmt.Errorf("both FAKE_AGENT_TLS_CERT_FILE and FAKE_AGENT_TLS_KEY_FILE are required for mTLS")
		}
		certificate, err := tls.LoadX509KeyPair(certFile, keyFile)
		if err != nil {
			return nil, fmt.Errorf("load client certificate: %w", err)
		}
		tlsConfig.Certificates = []tls.Certificate{certificate}
	}

	return credentials.NewTLS(tlsConfig), nil
}

func getenv(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
