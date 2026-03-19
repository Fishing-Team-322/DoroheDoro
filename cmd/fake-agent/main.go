package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/example/dorohedoro/internal/config"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type enrollResponse struct {
	AgentID string `json:"agent_id"`
	Error   string `json:"error"`
	Policy  struct {
		Revision  string   `json:"revision"`
		Sources   []string `json:"sources"`
		BatchSize int      `json:"batch_size"`
		BatchWait string   `json:"batch_wait"`
	} `json:"policy"`
}

func main() {
	grpcAddr := getenv("INGEST_GRPC_ADDR", "localhost:9090")
	httpAddr := getenv("SERVER_HTTP_ADDR", "http://localhost:8080")
	host := getenv("FAKE_AGENT_HOST", "demo-host")
	interval := config.ParseDuration(getenv("FAKE_AGENT_INTERVAL", "0s"), 0)
	generate := strings.EqualFold(getenv("FAKE_AGENT_GENERATE", "true"), "true")
	bootstrapToken := getenv("FAKE_AGENT_ENROLLMENT_TOKEN", "dev-bootstrap-token")

	agentID, policyRevision, err := enroll(httpAddr, host, bootstrapToken)
	if err != nil {
		panic(err)
	}
	fmt.Printf("enrolled agent_id=%s policy_revision=%s\n", agentID, policyRevision)

	conn, err := grpc.Dial(grpcAddr, grpc.WithTransportCredentials(insecure.NewCredentials()), grpc.WithDefaultCallOptions(grpc.ForceCodec(logsv1.JSONCodec{})))
	if err != nil {
		panic(err)
	}
	defer conn.Close()
	client := logsv1.NewIngestionServiceClient(conn)

	send := func() error {
		batch, err := loadBatch(agentID, host, generate)
		if err != nil {
			return err
		}
		resp, err := client.IngestBatch(context.Background(), batch)
		if err != nil {
			return err
		}
		fmt.Printf("request_id=%s accepted=%d rejected=%d errors=%v\n", resp.GetRequestId(), resp.GetAcceptedCount(), resp.GetRejectedCount(), resp.GetErrors())
		return nil
	}

	if interval <= 0 {
		if err := send(); err != nil {
			panic(err)
		}
		return
	}
	ticker := time.NewTicker(interval)
	defer ticker.Stop()
	for {
		if err := send(); err != nil {
			fmt.Println("send error:", err)
		}
		<-ticker.C
	}
}

func enroll(httpAddr, host, token string) (string, string, error) {
	payload, _ := json.Marshal(map[string]any{
		"bootstrap_token": token,
		"host":            host,
		"metadata": map[string]string{
			"agent": "fake-agent",
			"mode":  "demo",
		},
	})
	resp, err := http.Post(strings.TrimRight(httpAddr, "/")+"/api/v1/enroll", "application/json", bytes.NewReader(payload))
	if err != nil {
		return "", "", err
	}
	defer resp.Body.Close()
	var out enrollResponse
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return "", "", err
	}
	if resp.StatusCode >= 400 {
		return "", "", fmt.Errorf("enroll failed: %s", out.Error)
	}
	return out.AgentID, out.Policy.Revision, nil
}

func loadBatch(agentID, host string, generate bool) (*logsv1.LogBatch, error) {
	if path := os.Getenv("FAKE_AGENT_FILE"); path != "" {
		data, err := os.ReadFile(path)
		if err != nil {
			return nil, err
		}
		var batch logsv1.LogBatch
		if err := json.Unmarshal(data, &batch); err != nil {
			return nil, err
		}
		if batch.AgentId == "" {
			batch.AgentId = agentID
		}
		if batch.Host == "" {
			batch.Host = host
		}
		return &batch, nil
	}
	if !generate {
		return nil, fmt.Errorf("no FAKE_AGENT_FILE provided and generation disabled")
	}
	now := time.Now().UTC()
	return &logsv1.LogBatch{
		AgentId:      agentID,
		Host:         host,
		SentAtUnixMs: now.UnixMilli(),
		Events: []*logsv1.LogEvent{
			{TimestampUnixMs: now.Add(-3 * time.Second).UnixMilli(), Message: `{"timestamp":"` + now.Add(-3*time.Second).Format(time.RFC3339) + `","message":"sshd accepted password for demo","service":"sshd","severity":"info","user":"demo","pid":1234}`, Source: "/var/log/auth.log", SourceType: "file", Service: "sshd", Severity: "info", Labels: map[string]string{"env": "demo", "team": "platform"}},
			{TimestampUnixMs: now.Add(-2 * time.Second).UnixMilli(), Message: "nginx upstream timed out while reading response header from 10.0.0.12 request_id=123e4567-e89b-12d3-a456-426614174000", Source: "nginx", SourceType: "journald", Service: "nginx", Severity: "warn", Labels: map[string]string{"env": "demo", "team": "edge"}},
			{TimestampUnixMs: now.Add(-1 * time.Second).UnixMilli(), Message: "kernel: memory pressure relieved on node 192.168.1.10", Source: "kernel", SourceType: "journald", Service: "kernel", Severity: "", Labels: map[string]string{"env": "demo", "team": "infra"}},
		},
	}, nil
}

func getenv(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
