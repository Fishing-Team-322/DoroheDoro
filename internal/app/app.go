package app

import (
	"context"
	"fmt"
	_ "github.com/example/dorohedoro/docs"
	"net"
	"net/http"
	"sync"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
	"google.golang.org/grpc"

	"github.com/example/dorohedoro/internal/bus"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/diagnostics"
	"github.com/example/dorohedoro/internal/enrollment"
	"github.com/example/dorohedoro/internal/grpcapi"
	"github.com/example/dorohedoro/internal/httpapi"
	chindexer "github.com/example/dorohedoro/internal/indexer/clickhouse"
	osindexer "github.com/example/dorohedoro/internal/indexer/opensearch"
	"github.com/example/dorohedoro/internal/ingest"
	"github.com/example/dorohedoro/internal/normalize"
	"github.com/example/dorohedoro/internal/policy"
	"github.com/example/dorohedoro/internal/query"
	"github.com/example/dorohedoro/internal/stream"
	"github.com/example/dorohedoro/internal/telemetry"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type App struct {
	cfg         config.Config
	logger      *zap.Logger
	httpServer  *http.Server
	grpcServer  *grpc.Server
	bus         *bus.JetStream
	osIndexer   *osindexer.Indexer
	chIndexer   *chindexer.Indexer
	closeOnce   sync.Once
	diagnostics *diagnostics.Store
	enrollment  *enrollment.Store
	policy      *policy.Store
}

func New(ctx context.Context) (*App, error) {
	cfg, err := config.Load()
	if err != nil {
		return nil, err
	}
	logger, err := telemetry.NewLogger(cfg.LogLevel)
	if err != nil {
		return nil, err
	}
	jsBus, err := retryValue(ctx, func() (*bus.JetStream, error) { return bus.New(ctx, cfg.NATS, logger) })
	if err != nil {
		return nil, fmt.Errorf("init nats: %w", err)
	}
	indexer, err := retryValue(ctx, func() (*osindexer.Indexer, error) { return osindexer.New(cfg.OpenSearch, logger) })
	if err != nil {
		jsBus.Close()
		return nil, fmt.Errorf("init opensearch: %w", err)
	}
	var ch *chindexer.Indexer
	if cfg.ClickHouse.Enabled {
		ch, err = retryValue(ctx, func() (*chindexer.Indexer, error) { return chindexer.New(ctx, cfg.ClickHouse, logger) })
		if err != nil {
			_ = indexer.Close(context.Background())
			jsBus.Close()
			return nil, fmt.Errorf("init clickhouse: %w", err)
		}
	}

	streamHub := stream.NewHub(cfg.Stream.BufferSize)
	normalizer := normalize.New()
	diagnosticsStore := diagnostics.NewStore()
	defaultPolicy := policy.Policy{
		Revision:   cfg.Policy.DefaultRevision,
		Sources:    []string{"/var/log/*.log", "journald"},
		Labels:     map[string]string{"env": "dev", "plane": "data"},
		BatchSize:  cfg.Policy.DefaultBatchSize,
		BatchWait:  cfg.Policy.DefaultBatchWait.String(),
		SourceType: cfg.Policy.DefaultSourceType,
		UpdatedAt:  time.Now().UTC(),
	}
	policyStore := policy.NewStore(defaultPolicy)
	enrollmentStore := enrollment.NewStore(cfg.Enrollment.DevBootstrapToken)
	ingestService := ingest.NewService(normalizer, jsBus, streamHub, logger, diagnosticsStore, enrollmentStore, policyStore, cfg.Ingest.AllowUnknownAgents)
	grpcSvc := grpcapi.New(ingestService)
	searcher := query.NewSearcher(indexer.BaseURL(), indexer.IndexPrefix(), cfg.OpenSearch.Username, cfg.OpenSearch.Password, cfg.OpenSearch.ContextWindow, cfg.OpenSearch.ContextBefore, cfg.OpenSearch.ContextAfter)
	app := &App{cfg: cfg, logger: logger, bus: jsBus, osIndexer: indexer, chIndexer: ch, diagnostics: diagnosticsStore, enrollment: enrollmentStore, policy: policyStore}

	grpcServer := grpc.NewServer(grpc.ForceServerCodec(logsv1.JSONCodec{}))
	logsv1.RegisterIngestionServiceServer(grpcServer, grpcSvc)
	app.grpcServer = grpcServer
	app.httpServer = &http.Server{
		Addr: cfg.HTTP.ListenAddr,
		Handler: httpapi.NewRouter(httpapi.RouterDeps{
			Searcher:         searcher,
			Analytics:        ch,
			Hub:              streamHub,
			Logger:           logger,
			Ready:            func(ctx context.Context) bool { return searcher.Ping(ctx) },
			Enrollment:       enrollmentStore,
			Policy:           policyStore,
			Diagnostics:      diagnosticsStore,
			GRPCListenAddr:   cfg.GRPC.ListenAddr,
			EnrollmentConfig: cfg.Enrollment,
		}),
	}

	if _, err := jsBus.SubscribeDurable(ctx, cfg.NATS.IndexerConsumer, func(msg *nats.Msg) { indexer.HandleNATS(context.Background(), msg) }); err != nil {
		app.Close()
		return nil, fmt.Errorf("subscribe opensearch consumer: %w", err)
	}
	if ch != nil {
		if _, err := jsBus.SubscribeDurable(ctx, cfg.NATS.AnalyticsConsumer, func(msg *nats.Msg) { ch.HandleNATS(context.Background(), msg) }); err != nil {
			app.Close()
			return nil, fmt.Errorf("subscribe analytics consumer: %w", err)
		}
	}
	return app, nil
}

func (a *App) Run(ctx context.Context) error {
	grpcLn, err := net.Listen("tcp", a.cfg.GRPC.ListenAddr)
	if err != nil {
		return fmt.Errorf("listen grpc: %w", err)
	}
	httpLn, err := net.Listen("tcp", a.cfg.HTTP.ListenAddr)
	if err != nil {
		grpcLn.Close()
		return fmt.Errorf("listen http: %w", err)
	}

	errCh := make(chan error, 2)
	go func() {
		a.logger.Info("starting grpc server", zap.String("addr", a.cfg.GRPC.ListenAddr))
		if err := a.grpcServer.Serve(grpcLn); err != nil && err != grpc.ErrServerStopped {
			errCh <- err
		}
	}()
	go func() {
		a.logger.Info("starting http server", zap.String("addr", a.cfg.HTTP.ListenAddr))
		if err := a.httpServer.Serve(httpLn); err != nil && err != http.ErrServerClosed {
			errCh <- err
		}
	}()

	select {
	case <-ctx.Done():
		return a.Shutdown(context.Background())
	case err := <-errCh:
		_ = a.Shutdown(context.Background())
		return err
	}
}

func (a *App) Shutdown(ctx context.Context) error {
	shutdownCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()
	if a.grpcServer != nil {
		a.grpcServer.GracefulStop()
	}
	if a.httpServer != nil {
		_ = a.httpServer.Shutdown(shutdownCtx)
	}
	a.Close()
	return nil
}

func (a *App) Close() {
	a.closeOnce.Do(func() {
		if a.chIndexer != nil {
			_ = a.chIndexer.Close(context.Background())
		}
		if a.osIndexer != nil {
			_ = a.osIndexer.Close(context.Background())
		}
		if a.bus != nil {
			a.bus.Close()
		}
		if a.logger != nil {
			_ = a.logger.Sync()
		}
	})
}

func retryValue[T any](ctx context.Context, fn func() (T, error)) (T, error) {
	var zero T
	var out T
	var lastErr error
	for attempts := 0; attempts < 15; attempts++ {
		value, err := fn()
		if err == nil {
			return value, nil
		}
		lastErr = err
		select {
		case <-ctx.Done():
			return zero, ctx.Err()
		case <-time.After(time.Duration(attempts+1) * time.Second):
		}
	}
	return out, lastErr
}
