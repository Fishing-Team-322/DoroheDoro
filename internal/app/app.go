package app

import (
	"context"
	"fmt"
	"net"
	"net/http"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
	"google.golang.org/grpc"

	"github.com/example/dorohedoro/internal/bus"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/grpcapi"
	"github.com/example/dorohedoro/internal/httpapi"
	osindexer "github.com/example/dorohedoro/internal/indexer/opensearch"
	"github.com/example/dorohedoro/internal/ingest"
	"github.com/example/dorohedoro/internal/normalize"
	"github.com/example/dorohedoro/internal/query"
	"github.com/example/dorohedoro/internal/stream"
	"github.com/example/dorohedoro/internal/telemetry"
	logsv1 "github.com/example/dorohedoro/pkg/proto"
)

type App struct {
	cfg        config.Config
	logger     *zap.Logger
	httpServer *http.Server
	grpcServer *grpc.Server
	bus        *bus.JetStream
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
	streamHub := stream.NewHub(cfg.Stream.BufferSize)
	normalizer := normalize.New()
	ingestService := ingest.NewService(normalizer, jsBus, streamHub, logger)
	grpcSvc := grpcapi.New(ingestService)
	searcher := query.NewSearcher(indexer.BaseURL(), indexer.IndexPrefix(), cfg.OpenSearch.Username, cfg.OpenSearch.Password)
	app := &App{cfg: cfg, logger: logger, bus: jsBus}

	grpcServer := grpc.NewServer(grpc.ForceServerCodec(logsv1.JSONCodec{}))
	logsv1.RegisterIngestionServiceServer(grpcServer, grpcSvc)
	app.grpcServer = grpcServer
	app.httpServer = &http.Server{
		Addr: cfg.HTTP.ListenAddr,
		Handler: httpapi.NewRouter(httpapi.RouterDeps{
			Searcher: searcher,
			Hub:      streamHub,
			Logger:   logger,
			Ready: func(ctx context.Context) bool {
				return searcher.Ping(ctx)
			},
		}),
	}

	if _, err := jsBus.SubscribeDurable(ctx, func(msg *nats.Msg) { indexer.HandleNATS(context.Background(), msg) }); err != nil {
		app.Close()
		return nil, fmt.Errorf("subscribe jetstream consumer: %w", err)
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
	if a.bus != nil {
		a.bus.Close()
	}
	if a.logger != nil {
		_ = a.logger.Sync()
	}
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
