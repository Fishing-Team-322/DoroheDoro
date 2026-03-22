package app

import (
	"context"
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/keepalive"

	edgev1 "github.com/example/dorohedoro/contracts/proto"
	"github.com/example/dorohedoro/internal/app/agentstatus"
	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/config"
	"github.com/example/dorohedoro/internal/grpcapi"
	"github.com/example/dorohedoro/internal/httpapi"
	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
	"github.com/example/dorohedoro/internal/observability"
	"github.com/example/dorohedoro/internal/stream"
)

type App struct {
	cfg        config.Config
	logger     *zap.Logger
	bridge     *natsbridge.Bridge
	httpServer *http.Server
	grpcServer *grpc.Server
}

func New(ctx context.Context) (*App, error) {
	_ = ctx
	cfg, err := config.Load()
	if err != nil {
		return nil, err
	}
	logger, err := observability.NewLogger(cfg.LogLevel)
	if err != nil {
		return nil, err
	}
	bridge, err := natsbridge.New(cfg.NATS, logger)
	if err != nil {
		return nil, err
	}

	authHooks := auth.Hooks{}
	streamGateway := stream.NewGateway(bridge, cfg.Stream)
	agentStatusService := agentstatus.New(agentstatus.Dependencies{
		Bridge:   bridge,
		Logger:   logger,
		Subjects: cfg.NATS.Subjects,
		Settings: agentstatus.Settings{
			CacheTTL:       10 * time.Second,
			RequestTimeout: cfg.NATS.RequestTimeout,
		},
	})
	httpHandler := httpapi.NewRouter(httpapi.RouterDeps{
		Config:      cfg,
		Bridge:      bridge,
		Stream:      streamGateway,
		Logger:      logger,
		Auth:        authHooks,
		ReadyFn:     bridge.Ready,
		AgentStatus: agentStatusService,
	})
	httpServer := &http.Server{
		Addr:              cfg.HTTP.ListenAddr,
		Handler:           httpHandler,
		ReadHeaderTimeout: 5 * time.Second,
	}

	grpcOptions := []grpc.ServerOption{
		grpc.ForceServerCodec(edgev1.JSONCodec{}),
		grpc.MaxRecvMsgSize(cfg.GRPC.MaxRecvBytes),
		grpc.MaxSendMsgSize(cfg.GRPC.MaxSendBytes),
		grpc.KeepaliveParams(keepalive.ServerParameters{Time: cfg.GRPC.Keepalive}),
		middleware.UnaryServerInterceptors(logger, cfg.Timeouts.GRPC, authHooks.GRPCUnaryInterceptor),
	}

	transportCreds, err := grpcTransportCredentials(cfg)
	if err != nil {
		bridge.Close()
		return nil, err
	}
	if transportCreds != nil {
		grpcOptions = append(grpcOptions, grpc.Creds(transportCreds))
	} else if cfg.GRPC.AllowInsecureDevMode {
		logger.Warn("starting agent gRPC ingress without TLS because AGENT_ALLOW_INSECURE_DEV_MODE=true")
	}

	grpcServer := grpc.NewServer(grpcOptions...)
	edgev1.RegisterAgentIngressServiceServer(grpcServer, grpcapi.New(cfg, bridge, logger))

	return &App{cfg: cfg, logger: logger, bridge: bridge, httpServer: httpServer, grpcServer: grpcServer}, nil
}

func (a *App) Run(ctx context.Context) error {
	grpcLn, err := net.Listen("tcp", a.cfg.GRPC.ListenAddr)
	if err != nil {
		return fmt.Errorf("listen grpc: %w", err)
	}
	httpLn, err := net.Listen("tcp", a.cfg.HTTP.ListenAddr)
	if err != nil {
		_ = grpcLn.Close()
		return fmt.Errorf("listen http: %w", err)
	}

	errCh := make(chan error, 2)
	go func() {
		a.logger.Info("starting grpc server", zap.String("addr", a.cfg.GRPC.ListenAddr), zap.Bool("mtls_enabled", a.cfg.GRPC.MTLSEnabled))
		if err := a.grpcServer.Serve(grpcLn); err != nil {
			errCh <- err
		}
	}()
	go func() {
		a.logger.Info("starting http server", zap.String("addr", a.cfg.HTTP.ListenAddr))
		if err := serveHTTP(a.httpServer, httpLn, a.cfg); err != nil && err != http.ErrServerClosed {
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
	ctx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()
	shutdownErr := a.httpServer.Shutdown(ctx)
	a.grpcServer.GracefulStop()
	a.bridge.Close()
	if a.logger != nil {
		_ = a.logger.Sync()
	}
	return shutdownErr
}

func Main() error {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM, syscall.SIGINT)
	defer stop()
	app, err := New(ctx)
	if err != nil {
		return err
	}
	return app.Run(ctx)
}

func grpcTransportCredentials(cfg config.Config) (credentials.TransportCredentials, error) {
	if cfg.GRPC.TLSCert == "" || cfg.GRPC.TLSKey == "" {
		return nil, nil
	}

	certificate, err := tls.LoadX509KeyPair(cfg.GRPC.TLSCert, cfg.GRPC.TLSKey)
	if err != nil {
		return nil, fmt.Errorf("load grpc tls cert: %w", err)
	}

	tlsConfig := &tls.Config{
		Certificates: []tls.Certificate{certificate},
		MinVersion:   tls.VersionTLS12,
	}

	if cfg.GRPC.MTLSEnabled {
		caPEM, err := os.ReadFile(cfg.GRPC.ClientCA)
		if err != nil {
			return nil, fmt.Errorf("read grpc client ca: %w", err)
		}
		clientCAs := x509.NewCertPool()
		if ok := clientCAs.AppendCertsFromPEM(caPEM); !ok {
			return nil, fmt.Errorf("parse grpc client ca bundle")
		}
		tlsConfig.ClientAuth = tls.RequireAndVerifyClientCert
		tlsConfig.ClientCAs = clientCAs
	}

	return credentials.NewTLS(tlsConfig), nil
}

func serveHTTP(server *http.Server, ln net.Listener, cfg config.Config) error {
	if cfg.HTTP.TLSCert != "" && cfg.HTTP.TLSKey != "" {
		return server.ServeTLS(ln, cfg.HTTP.TLSCert, cfg.HTTP.TLSKey)
	}
	return server.Serve(ln)
}
