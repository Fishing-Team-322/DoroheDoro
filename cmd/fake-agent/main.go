package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/example/dorohedoro/internal/config"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

func main() {
	addr := getenv("INGEST_GRPC_ADDR", "localhost:9090")
	host := getenv("FAKE_AGENT_HOST", "demo-host")
	agentID := getenv("FAKE_AGENT_ID", "fake-agent-1")
	interval := config.ParseDuration(getenv("FAKE_AGENT_INTERVAL", "0s"), 0)
	generate := strings.EqualFold(getenv("FAKE_AGENT_GENERATE", "true"), "true")

	conn, err := grpc.Dial(addr, grpc.WithTransportCredentials(insecure.NewCredentials()), grpc.WithDefaultCallOptions(grpc.ForceCodec(logsv1.JSONCodec{})))
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
			{TimestampUnixMs: now.Add(-3 * time.Second).UnixMilli(), Message: "sshd accepted password for demo", Source: "/var/log/auth.log", SourceType: "file", Service: "sshd", Severity: "info", Labels: map[string]string{"env": "demo", "team": "platform"}},
			{TimestampUnixMs: now.Add(-2 * time.Second).UnixMilli(), Message: "nginx upstream timed out while reading response header", Source: "nginx", SourceType: "journald", Service: "nginx", Severity: "warn", Labels: map[string]string{"env": "demo", "team": "edge"}},
			{TimestampUnixMs: now.Add(-1 * time.Second).UnixMilli(), Message: "kernel: memory pressure relieved", Source: "kernel", SourceType: "journald", Service: "kernel", Severity: "info", Labels: map[string]string{"env": "demo", "team": "infra"}},
		},
	}, nil
}

func getenv(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
