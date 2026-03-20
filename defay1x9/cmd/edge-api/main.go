package main

import (
	"log"

	"github.com/example/dorohedoro/internal/app"
)

func main() {
	if err := app.Main(); err != nil {
		log.Fatal(err)
	}
}
