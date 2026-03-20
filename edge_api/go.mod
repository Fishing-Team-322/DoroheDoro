module github.com/example/dorohedoro

go 1.23

require (
	github.com/go-chi/chi/v5 v5.2.1
	github.com/google/uuid v1.6.0
	github.com/nats-io/nats.go v0.0.0
	go.uber.org/zap v0.0.0
	google.golang.org/grpc v0.0.0
)

replace github.com/nats-io/nats.go => ./stubs/nats
replace go.uber.org/zap => ./stubs/zap
replace google.golang.org/grpc => ./stubs/grpc
