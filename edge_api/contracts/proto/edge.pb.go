package proto

import (
	"context"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type AgentLog struct {
	TimestampUnixMs int64             `json:"timestamp_unix_ms,omitempty"`
	Service         string            `json:"service,omitempty"`
	Severity        string            `json:"severity,omitempty"`
	Message         string            `json:"message,omitempty"`
	Labels          map[string]string `json:"labels,omitempty"`
}

type EnrollRequest struct {
	EnrollmentToken string            `json:"enrollment_token,omitempty"`
	Host            string            `json:"host,omitempty"`
	Labels          map[string]string `json:"labels,omitempty"`
	ExistingAgentId string            `json:"existing_agent_id,omitempty"`
}

type EnrollResponse struct {
	AgentId        string `json:"agent_id,omitempty"`
	Status         string `json:"status,omitempty"`
	IssuedAtUnixMs int64  `json:"issued_at_unix_ms,omitempty"`
	RequestId      string `json:"request_id,omitempty"`
}

type FetchPolicyRequest struct {
	AgentId         string `json:"agent_id,omitempty"`
	CurrentRevision string `json:"current_revision,omitempty"`
}

type PolicyPayload struct {
	PolicyId string `json:"policy_id,omitempty"`
	Revision string `json:"revision,omitempty"`
	BodyJSON string `json:"body_json,omitempty"`
}

type FetchPolicyResponse struct {
	Policy    *PolicyPayload `json:"policy,omitempty"`
	Changed   bool           `json:"changed"`
	RequestId string         `json:"request_id,omitempty"`
}

type HeartbeatRequest struct {
	AgentId      string            `json:"agent_id,omitempty"`
	Host         string            `json:"host,omitempty"`
	SentAtUnixMs int64             `json:"sent_at_unix_ms,omitempty"`
	Status       string            `json:"status,omitempty"`
	Version      string            `json:"version,omitempty"`
	HostMetadata map[string]string `json:"host_metadata,omitempty"`
}

type DiagnosticsRequest struct {
	AgentId      string `json:"agent_id,omitempty"`
	Host         string `json:"host,omitempty"`
	SentAtUnixMs int64  `json:"sent_at_unix_ms,omitempty"`
	PayloadJSON  string `json:"payload_json,omitempty"`
}

type IngestLogsRequest struct {
	AgentId      string      `json:"agent_id,omitempty"`
	Host         string      `json:"host,omitempty"`
	SentAtUnixMs int64       `json:"sent_at_unix_ms,omitempty"`
	Events       []*AgentLog `json:"events,omitempty"`
}

type Ack struct {
	Accepted  bool   `json:"accepted"`
	RequestId string `json:"request_id,omitempty"`
	Message   string `json:"message,omitempty"`
}

type IngestLogsResponse struct {
	Accepted      bool   `json:"accepted"`
	AcceptedCount int32  `json:"accepted_count"`
	RequestId     string `json:"request_id,omitempty"`
}

func (x *EnrollRequest) GetEnrollmentToken() string {
	if x == nil {
		return ""
	}
	return x.EnrollmentToken
}
func (x *EnrollRequest) GetHost() string {
	if x == nil {
		return ""
	}
	return x.Host
}
func (x *EnrollRequest) GetExistingAgentId() string {
	if x == nil {
		return ""
	}
	return x.ExistingAgentId
}
func (x *FetchPolicyRequest) GetAgentId() string {
	if x == nil {
		return ""
	}
	return x.AgentId
}
func (x *FetchPolicyRequest) GetCurrentRevision() string {
	if x == nil {
		return ""
	}
	return x.CurrentRevision
}
func (x *HeartbeatRequest) GetAgentId() string {
	if x == nil {
		return ""
	}
	return x.AgentId
}
func (x *HeartbeatRequest) GetVersion() string {
	if x == nil {
		return ""
	}
	return x.Version
}
func (x *DiagnosticsRequest) GetAgentId() string {
	if x == nil {
		return ""
	}
	return x.AgentId
}
func (x *IngestLogsRequest) GetAgentId() string {
	if x == nil {
		return ""
	}
	return x.AgentId
}
func (x *IngestLogsRequest) GetHost() string {
	if x == nil {
		return ""
	}
	return x.Host
}
func (x *IngestLogsRequest) GetEvents() []*AgentLog {
	if x == nil {
		return nil
	}
	return x.Events
}

type AgentIngressServiceClient interface {
	Enroll(ctx context.Context, in *EnrollRequest, opts ...grpc.CallOption) (*EnrollResponse, error)
	FetchPolicy(ctx context.Context, in *FetchPolicyRequest, opts ...grpc.CallOption) (*FetchPolicyResponse, error)
	SendHeartbeat(ctx context.Context, in *HeartbeatRequest, opts ...grpc.CallOption) (*Ack, error)
	SendDiagnostics(ctx context.Context, in *DiagnosticsRequest, opts ...grpc.CallOption) (*Ack, error)
	IngestLogs(ctx context.Context, in *IngestLogsRequest, opts ...grpc.CallOption) (*IngestLogsResponse, error)
}

type agentIngressServiceClient struct{ cc grpc.ClientConnInterface }

func NewAgentIngressServiceClient(cc grpc.ClientConnInterface) AgentIngressServiceClient {
	return &agentIngressServiceClient{cc: cc}
}

func (c *agentIngressServiceClient) Enroll(ctx context.Context, in *EnrollRequest, opts ...grpc.CallOption) (*EnrollResponse, error) {
	out := new(EnrollResponse)
	if err := c.cc.Invoke(ctx, "/dorohedoro.edge.v1.AgentIngressService/Enroll", in, out, opts...); err != nil {
		return nil, err
	}
	return out, nil
}
func (c *agentIngressServiceClient) FetchPolicy(ctx context.Context, in *FetchPolicyRequest, opts ...grpc.CallOption) (*FetchPolicyResponse, error) {
	out := new(FetchPolicyResponse)
	if err := c.cc.Invoke(ctx, "/dorohedoro.edge.v1.AgentIngressService/FetchPolicy", in, out, opts...); err != nil {
		return nil, err
	}
	return out, nil
}
func (c *agentIngressServiceClient) SendHeartbeat(ctx context.Context, in *HeartbeatRequest, opts ...grpc.CallOption) (*Ack, error) {
	out := new(Ack)
	if err := c.cc.Invoke(ctx, "/dorohedoro.edge.v1.AgentIngressService/SendHeartbeat", in, out, opts...); err != nil {
		return nil, err
	}
	return out, nil
}
func (c *agentIngressServiceClient) SendDiagnostics(ctx context.Context, in *DiagnosticsRequest, opts ...grpc.CallOption) (*Ack, error) {
	out := new(Ack)
	if err := c.cc.Invoke(ctx, "/dorohedoro.edge.v1.AgentIngressService/SendDiagnostics", in, out, opts...); err != nil {
		return nil, err
	}
	return out, nil
}
func (c *agentIngressServiceClient) IngestLogs(ctx context.Context, in *IngestLogsRequest, opts ...grpc.CallOption) (*IngestLogsResponse, error) {
	out := new(IngestLogsResponse)
	if err := c.cc.Invoke(ctx, "/dorohedoro.edge.v1.AgentIngressService/IngestLogs", in, out, opts...); err != nil {
		return nil, err
	}
	return out, nil
}

type AgentIngressServiceServer interface {
	Enroll(context.Context, *EnrollRequest) (*EnrollResponse, error)
	FetchPolicy(context.Context, *FetchPolicyRequest) (*FetchPolicyResponse, error)
	SendHeartbeat(context.Context, *HeartbeatRequest) (*Ack, error)
	SendDiagnostics(context.Context, *DiagnosticsRequest) (*Ack, error)
	IngestLogs(context.Context, *IngestLogsRequest) (*IngestLogsResponse, error)
	mustEmbedUnimplementedAgentIngressServiceServer()
}

type UnimplementedAgentIngressServiceServer struct{}

func (UnimplementedAgentIngressServiceServer) Enroll(context.Context, *EnrollRequest) (*EnrollResponse, error) {
	return nil, status.Errorf(codes.Unimplemented, "method Enroll not implemented")
}
func (UnimplementedAgentIngressServiceServer) FetchPolicy(context.Context, *FetchPolicyRequest) (*FetchPolicyResponse, error) {
	return nil, status.Errorf(codes.Unimplemented, "method FetchPolicy not implemented")
}
func (UnimplementedAgentIngressServiceServer) SendHeartbeat(context.Context, *HeartbeatRequest) (*Ack, error) {
	return nil, status.Errorf(codes.Unimplemented, "method SendHeartbeat not implemented")
}
func (UnimplementedAgentIngressServiceServer) SendDiagnostics(context.Context, *DiagnosticsRequest) (*Ack, error) {
	return nil, status.Errorf(codes.Unimplemented, "method SendDiagnostics not implemented")
}
func (UnimplementedAgentIngressServiceServer) IngestLogs(context.Context, *IngestLogsRequest) (*IngestLogsResponse, error) {
	return nil, status.Errorf(codes.Unimplemented, "method IngestLogs not implemented")
}
func (UnimplementedAgentIngressServiceServer) mustEmbedUnimplementedAgentIngressServiceServer() {}

func RegisterAgentIngressServiceServer(s grpc.ServiceRegistrar, srv AgentIngressServiceServer) {
	s.RegisterService(&AgentIngressService_ServiceDesc, srv)
}

func _AgentIngressService_Enroll_Handler(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error) {
	in := new(EnrollRequest)
	if err := dec(in); err != nil {
		return nil, err
	}
	if interceptor == nil {
		return srv.(AgentIngressServiceServer).Enroll(ctx, in)
	}
	info := &grpc.UnaryServerInfo{Server: srv, FullMethod: "/dorohedoro.edge.v1.AgentIngressService/Enroll"}
	handler := func(ctx context.Context, req interface{}) (interface{}, error) {
		return srv.(AgentIngressServiceServer).Enroll(ctx, req.(*EnrollRequest))
	}
	return interceptor(ctx, in, info, handler)
}
func _AgentIngressService_FetchPolicy_Handler(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error) {
	in := new(FetchPolicyRequest)
	if err := dec(in); err != nil {
		return nil, err
	}
	if interceptor == nil {
		return srv.(AgentIngressServiceServer).FetchPolicy(ctx, in)
	}
	info := &grpc.UnaryServerInfo{Server: srv, FullMethod: "/dorohedoro.edge.v1.AgentIngressService/FetchPolicy"}
	handler := func(ctx context.Context, req interface{}) (interface{}, error) {
		return srv.(AgentIngressServiceServer).FetchPolicy(ctx, req.(*FetchPolicyRequest))
	}
	return interceptor(ctx, in, info, handler)
}
func _AgentIngressService_SendHeartbeat_Handler(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error) {
	in := new(HeartbeatRequest)
	if err := dec(in); err != nil {
		return nil, err
	}
	if interceptor == nil {
		return srv.(AgentIngressServiceServer).SendHeartbeat(ctx, in)
	}
	info := &grpc.UnaryServerInfo{Server: srv, FullMethod: "/dorohedoro.edge.v1.AgentIngressService/SendHeartbeat"}
	handler := func(ctx context.Context, req interface{}) (interface{}, error) {
		return srv.(AgentIngressServiceServer).SendHeartbeat(ctx, req.(*HeartbeatRequest))
	}
	return interceptor(ctx, in, info, handler)
}
func _AgentIngressService_SendDiagnostics_Handler(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error) {
	in := new(DiagnosticsRequest)
	if err := dec(in); err != nil {
		return nil, err
	}
	if interceptor == nil {
		return srv.(AgentIngressServiceServer).SendDiagnostics(ctx, in)
	}
	info := &grpc.UnaryServerInfo{Server: srv, FullMethod: "/dorohedoro.edge.v1.AgentIngressService/SendDiagnostics"}
	handler := func(ctx context.Context, req interface{}) (interface{}, error) {
		return srv.(AgentIngressServiceServer).SendDiagnostics(ctx, req.(*DiagnosticsRequest))
	}
	return interceptor(ctx, in, info, handler)
}
func _AgentIngressService_IngestLogs_Handler(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error) {
	in := new(IngestLogsRequest)
	if err := dec(in); err != nil {
		return nil, err
	}
	if interceptor == nil {
		return srv.(AgentIngressServiceServer).IngestLogs(ctx, in)
	}
	info := &grpc.UnaryServerInfo{Server: srv, FullMethod: "/dorohedoro.edge.v1.AgentIngressService/IngestLogs"}
	handler := func(ctx context.Context, req interface{}) (interface{}, error) {
		return srv.(AgentIngressServiceServer).IngestLogs(ctx, req.(*IngestLogsRequest))
	}
	return interceptor(ctx, in, info, handler)
}

var AgentIngressService_ServiceDesc = grpc.ServiceDesc{
	ServiceName: "dorohedoro.edge.v1.AgentIngressService",
	HandlerType: (*AgentIngressServiceServer)(nil),
	Methods: []grpc.MethodDesc{
		{MethodName: "Enroll", Handler: _AgentIngressService_Enroll_Handler},
		{MethodName: "FetchPolicy", Handler: _AgentIngressService_FetchPolicy_Handler},
		{MethodName: "SendHeartbeat", Handler: _AgentIngressService_SendHeartbeat_Handler},
		{MethodName: "SendDiagnostics", Handler: _AgentIngressService_SendDiagnostics_Handler},
		{MethodName: "IngestLogs", Handler: _AgentIngressService_IngestLogs_Handler},
	},
	Streams:  []grpc.StreamDesc{},
	Metadata: "contracts/proto/edge.proto",
}
