package middleware

import (
	"context"
	"crypto/sha256"
	"crypto/tls"
	"encoding/hex"
	"runtime/debug"
	"strings"
	"time"

	"github.com/google/uuid"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/peer"
	"google.golang.org/grpc/status"

	"github.com/example/dorohedoro/internal/auth"
	"github.com/example/dorohedoro/internal/natsbridge"
)

func UnaryServerInterceptors(logger *zap.Logger, timeout time.Duration, extra ...grpc.UnaryServerInterceptor) grpc.ServerOption {
	chain := []grpc.UnaryServerInterceptor{
		grpcRequestID(),
		grpcRecovery(logger),
		grpcTimeout(timeout),
		grpcPeerIdentity(),
		grpcAccessLog(logger),
	}
	chain = append(chain, extra...)
	return grpc.ChainUnaryInterceptor(chain...)
}

func grpcRequestID() grpc.UnaryServerInterceptor {
	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		requestID := requestIDFromMetadata(ctx)
		ctx = context.WithValue(ctx, requestIDKey, requestID)
		ctx = natsbridge.WithRequestID(ctx, requestID)
		return handler(ctx, req)
	}
}

func grpcRecovery(logger *zap.Logger) grpc.UnaryServerInterceptor {
	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (resp any, err error) {
		defer func() {
			if rec := recover(); rec != nil {
				logger.Error("grpc panic recovered", zap.Any("panic", rec), zap.String("rpc_method", info.FullMethod), zap.ByteString("stack", debug.Stack()))
				err = status.Error(codes.Internal, "internal server error")
			}
		}()
		return handler(ctx, req)
	}
}

func grpcTimeout(timeout time.Duration) grpc.UnaryServerInterceptor {
	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		ctx, cancel := context.WithTimeout(ctx, timeout)
		defer cancel()
		return handler(ctx, req)
	}
}

func grpcPeerIdentity() grpc.UnaryServerInterceptor {
	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		if identity, ok := tlsIdentityFromContext(ctx); ok {
			ctx = auth.WithTLSIdentity(ctx, identity)
		}
		return handler(ctx, req)
	}
}

func grpcAccessLog(logger *zap.Logger) grpc.UnaryServerInterceptor {
	return func(ctx context.Context, req any, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (any, error) {
		started := time.Now()
		resp, err := handler(ctx, req)
		peerInfo, _ := peer.FromContext(ctx)
		result := "ok"
		if err != nil {
			result = status.Code(err).String()
		}
		identity, _ := auth.TLSIdentity(ctx)
		logger.Info("grpc access",
			zap.String("request_id", GetRequestID(ctx)),
			zap.String("rpc_method", info.FullMethod),
			zap.String("peer_addr", peerString(peerInfo)),
			zap.String("tls_subject", identity.Subject),
			zap.Strings("tls_san", identity.SANs),
			zap.String("tls_fingerprint", identity.Fingerprint),
			zap.String("agent_id", maskedValue(auth.Context(ctx).AgentID)),
			zap.Duration("duration", time.Since(started)),
			zap.String("result", result),
		)
		return resp, err
	}
}

func requestIDFromMetadata(ctx context.Context) string {
	if md, ok := metadata.FromIncomingContext(ctx); ok {
		for _, key := range []string{"x-request-id", "request-id"} {
			values := md.Get(key)
			if len(values) > 0 && strings.TrimSpace(values[0]) != "" {
				return strings.TrimSpace(values[0])
			}
		}
	}
	return uuid.NewString()
}

func peerString(p *peer.Peer) string {
	if p == nil || p.Addr == nil {
		return ""
	}
	return p.Addr.String()
}

func tlsIdentityFromContext(ctx context.Context) (auth.AgentTLSIdentity, bool) {
	peerInfo, ok := peer.FromContext(ctx)
	if !ok || peerInfo == nil {
		return auth.AgentTLSIdentity{}, false
	}
	tlsInfo, ok := peerInfo.AuthInfo.(credentials.TLSInfo)
	if !ok {
		return auth.AgentTLSIdentity{}, false
	}
	return tlsIdentityFromState(tlsInfo.State), true
}

func tlsIdentityFromState(state tls.ConnectionState) auth.AgentTLSIdentity {
	if len(state.PeerCertificates) == 0 {
		return auth.AgentTLSIdentity{}
	}
	leaf := state.PeerCertificates[0]
	sans := make([]string, 0, len(leaf.DNSNames)+len(leaf.EmailAddresses)+len(leaf.URIs))
	sans = append(sans, leaf.DNSNames...)
	sans = append(sans, leaf.EmailAddresses...)
	for _, uri := range leaf.URIs {
		sans = append(sans, uri.String())
	}
	fingerprint := sha256.Sum256(leaf.Raw)
	return auth.AgentTLSIdentity{
		Subject:     leaf.Subject.String(),
		CommonName:  leaf.Subject.CommonName,
		SANs:        sans,
		Fingerprint: strings.ToUpper(hex.EncodeToString(fingerprint[:])),
	}
}
