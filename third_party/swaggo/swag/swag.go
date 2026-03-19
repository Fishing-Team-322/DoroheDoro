package swag

import (
	"fmt"
	"sync"
)

type Swagger interface{ ReadDoc() string }

type Spec struct {
	Version          string
	Host             string
	BasePath         string
	Schemes          []string
	Title            string
	Description      string
	InfoInstanceName string
	SwaggerTemplate  string
}

func (s *Spec) ReadDoc() string { return s.SwaggerTemplate }

var (
	mu       sync.RWMutex
	registry = map[string]Swagger{}
)

func Register(name string, swagger Swagger) {
	mu.Lock()
	defer mu.Unlock()
	registry[name] = swagger
}

func GetSwagger(name string) Swagger {
	mu.RLock()
	defer mu.RUnlock()
	return registry[name]
}

func ReadDoc(name string) (string, error) {
	s := GetSwagger(name)
	if s == nil {
		return "", fmt.Errorf("swagger doc %q not registered", name)
	}
	return s.ReadDoc(), nil
}
