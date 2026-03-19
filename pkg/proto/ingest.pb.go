package proto

import (
	"context"

	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

type LogEvent struct {
	TimestampUnixMs int64             `json:"timestamp_unix_ms,omitempty"`
	Message         string            `json:"message,omitempty"`
	Source          string            `json:"source,omitempty"`
	SourceType      string            `json:"source_type,omitempty"`
	Service         string            `json:"service,omitempty"`
	Severity        string            `json:"severity,omitempty"`
	Labels          map[string]string `json:"labels,omitempty"`
	Raw             string            `json:"raw,omitempty"`
}

func (x *LogEvent) GetTimestampUnixMs() int64 {
	if x == nil {
		return 0
	}
	return x.TimestampUnixMs
}
func (x *LogEvent) GetMessage() string {
	if x == nil {
		return ""
	}
	return x.Message
}
func (x *LogEvent) GetSource() string {
	if x == nil {
		return ""
	}
	return x.Source
}
func (x *LogEvent) GetSourceType() string {
	if x == nil {
		return ""
	}
	return x.SourceType
}
func (x *LogEvent) GetService() string {
	if x == nil {
		return ""
	}
	return x.Service
}
func (x *LogEvent) GetSeverity() string {
	if x == nil {
		return ""
	}
	return x.Severity
}
func (x *LogEvent) GetLabels() map[string]string {
	if x == nil {
		return nil
	}
	return x.Labels
}
func (x *LogEvent) GetRaw() string {
	if x == nil {
		return ""
	}
	return x.Raw
}

type LogBatch struct {
	AgentId      string      `json:"agent_id,omitempty"`
	Host         string      `json:"host,omitempty"`
	Events       []*LogEvent `json:"events,omitempty"`
	SentAtUnixMs int64       `json:"sent_at_unix_ms,omitempty"`
}

func (x *LogBatch) GetAgentId() string {
	if x == nil {
		return ""
	}
	return x.AgentId
}
func (x *LogBatch) GetHost() string {
	if x == nil {
		return ""
	}
	return x.Host
}
func (x *LogBatch) GetEvents() []*LogEvent {
	if x == nil {
		return nil
	}
	return x.Events
}
func (x *LogBatch) GetSentAtUnixMs() int64 {
	if x == nil {
		return 0
	}
	return x.SentAtUnixMs
}

type IngestResponse struct {
	AcceptedCount int32    `json:"accepted_count,omitempty"`
	RejectedCount int32    `json:"rejected_count,omitempty"`
	Errors        []string `json:"errors,omitempty"`
	RequestId     string   `json:"request_id,omitempty"`
}

func (x *IngestResponse) GetAcceptedCount() int32 {
	if x == nil {
		return 0
	}
	return x.AcceptedCount
}
func (x *IngestResponse) GetRejectedCount() int32 {
	if x == nil {
		return 0
	}
	return x.RejectedCount
}
func (x *IngestResponse) GetErrors() []string {
	if x == nil {
		return nil
	}
	return x.Errors
}
func (x *IngestResponse) GetRequestId() string {
	if x == nil {
		return ""
	}
	return x.RequestId
}

type IngestionServiceClient interface {
	IngestBatch(ctx context.Context, in *LogBatch, opts ...grpc.CallOption) (*IngestResponse, error)
}

type ingestionServiceClient struct{ cc grpc.ClientConnInterface }

func NewIngestionServiceClient(cc grpc.ClientConnInterface) IngestionServiceClient {
	return &ingestionServiceClient{cc}
}

func (c *ingestionServiceClient) IngestBatch(ctx context.Context, in *LogBatch, opts ...grpc.CallOption) (*IngestResponse, error) {
	out := new(IngestResponse)
	err := c.cc.Invoke(ctx, "/dorohedoro.v1.IngestionService/IngestBatch", in, out, opts...)
	if err != nil {
		return nil, err
	}
	return out, nil
}

type IngestionServiceServer interface {
	IngestBatch(context.Context, *LogBatch) (*IngestResponse, error)
	mustEmbedUnimplementedIngestionServiceServer()
}

type UnimplementedIngestionServiceServer struct{}

func (UnimplementedIngestionServiceServer) IngestBatch(context.Context, *LogBatch) (*IngestResponse, error) {
	return nil, status.Errorf(codes.Unimplemented, "method IngestBatch not implemented")
}
func (UnimplementedIngestionServiceServer) mustEmbedUnimplementedIngestionServiceServer() {}

func RegisterIngestionServiceServer(s grpc.ServiceRegistrar, srv IngestionServiceServer) {
	s.RegisterService(&IngestionService_ServiceDesc, srv)
}

func _IngestionService_IngestBatch_Handler(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error) {
	in := new(LogBatch)
	if err := dec(in); err != nil {
		return nil, err
	}
	if interceptor == nil {
		return srv.(IngestionServiceServer).IngestBatch(ctx, in)
	}
	info := &grpc.UnaryServerInfo{Server: srv, FullMethod: "/dorohedoro.v1.IngestionService/IngestBatch"}
	handler := func(ctx context.Context, req interface{}) (interface{}, error) {
		return srv.(IngestionServiceServer).IngestBatch(ctx, req.(*LogBatch))
	}
	return interceptor(ctx, in, info, handler)
}

var IngestionService_ServiceDesc = grpc.ServiceDesc{
	ServiceName: "dorohedoro.v1.IngestionService",
	HandlerType: (*IngestionServiceServer)(nil),
	Methods:     []grpc.MethodDesc{{MethodName: "IngestBatch", Handler: _IngestionService_IngestBatch_Handler}},
	Streams:     []grpc.StreamDesc{},
	Metadata:    "proto/ingest.proto",
}
