package grpc

import (
	"context"
	"errors"
	"net"
	"sync"
)

type CallOption interface{}
type DialOption interface{}
type ServerOption interface{}
type ClientConnInterface interface {
	Invoke(context.Context, string, any, any, ...CallOption) error
}
type UnaryHandler func(context.Context, interface{}) (interface{}, error)
type UnaryServerInterceptor func(context.Context, any, *UnaryServerInfo, UnaryHandler) (any, error)
type UnaryServerInfo struct {
	Server     any
	FullMethod string
}
type MethodDesc struct {
	MethodName string
	Handler    any
}
type StreamDesc struct{}
type ServiceDesc struct {
	ServiceName string
	HandlerType any
	Methods     []MethodDesc
	Streams     []StreamDesc
	Metadata    any
}
type ServiceRegistrar interface {
	RegisterService(*ServiceDesc, interface{})
}

type codec interface{}

type Server struct {
	mu       sync.Mutex
	services map[string]interface{}
	stopCh   chan struct{}
}

func NewServer(opts ...ServerOption) *Server {
	return &Server{services: map[string]interface{}{}, stopCh: make(chan struct{})}
}
func (s *Server) RegisterService(desc *ServiceDesc, impl interface{}) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.services[desc.ServiceName] = impl
}
func (s *Server) Serve(lis net.Listener) error {
	<-s.stopCh
	return nil
}
func (s *Server) GracefulStop() {
	select {
	case <-s.stopCh:
	default:
		close(s.stopCh)
	}
}

type ClientConn struct{}

func Dial(target string, opts ...DialOption) (*ClientConn, error) { return &ClientConn{}, nil }
func (c *ClientConn) Close() error                                { return nil }
func (c *ClientConn) Invoke(ctx context.Context, method string, in any, out any, opts ...CallOption) error {
	return errors.New("grpc stub runtime does not implement network invoke")
}

func ChainUnaryInterceptor(interceptors ...UnaryServerInterceptor) ServerOption { return nil }
func ForceServerCodec(codec any) ServerOption                                   { return nil }
func MaxRecvMsgSize(n int) ServerOption                                         { return nil }
func MaxSendMsgSize(n int) ServerOption                                         { return nil }
func Creds(creds any) ServerOption                                              { return nil }
func KeepaliveParams(any) ServerOption                                          { return nil }
func WithTransportCredentials(creds any) DialOption                             { return nil }
func WithDefaultCallOptions(opts ...CallOption) DialOption                      { return nil }
func ForceCodec(codec any) CallOption                                           { return nil }
