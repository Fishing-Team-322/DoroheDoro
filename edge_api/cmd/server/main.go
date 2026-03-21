//go:build legacy

package main

import (
	"context"
	"os/signal"
	"syscall"

	"github.com/example/dorohedoro/internal/app"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()
	application, err := app.New(ctx)
	if err != nil {
		panic(err)
	}
	if err := application.Run(ctx); err != nil {
		panic(err)
	}
}
