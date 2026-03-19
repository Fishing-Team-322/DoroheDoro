module github.com/example/dorohedoro

go 1.23

require (
	github.com/go-chi/chi/v5 v5.2.1
	github.com/google/uuid v1.6.0
	github.com/gorilla/websocket v1.5.3
	github.com/nats-io/nats.go v1.39.1
	github.com/swaggo/http-swagger/v2 v2.0.2
	github.com/swaggo/swag v1.16.4
	go.uber.org/zap v1.27.0
	google.golang.org/grpc v1.71.1
)

replace github.com/swaggo/http-swagger/v2 => ./third_party/swaggo/http-swagger/v2
replace github.com/swaggo/swag => ./third_party/swaggo/swag
