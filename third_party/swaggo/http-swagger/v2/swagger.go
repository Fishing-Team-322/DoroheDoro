package httpswagger

import (
	"fmt"
	"html/template"
	"net/http"
	"path"
	"strings"

	"github.com/swaggo/swag"
)

type Config struct{ URL string }

type Option func(*Config)

func URL(url string) Option { return func(c *Config) { c.URL = url } }

func Handler(opts ...Option) http.HandlerFunc {
	cfg := Config{URL: "/swagger/doc.json"}
	for _, opt := range opts {
		if opt != nil {
			opt(&cfg)
		}
	}
	page := template.Must(template.New("swagger-ui").Parse(`<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({ url: {{ printf "%q" .URL }}, dom_id: '#swagger-ui' });
  </script>
</body>
</html>`))
	return func(w http.ResponseWriter, r *http.Request) {
		switch base := path.Base(r.URL.Path); {
		case base == "doc.json":
			doc, err := swag.ReadDoc("swagger")
			if err != nil {
				http.Error(w, err.Error(), http.StatusInternalServerError)
				return
			}
			w.Header().Set("Content-Type", "application/json")
			_, _ = w.Write([]byte(doc))
		case r.URL.Path == "/swagger" || r.URL.Path == "/swagger/" || base == "index.html" || !strings.Contains(base, "."):
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			if err := page.Execute(w, cfg); err != nil {
				http.Error(w, fmt.Sprintf("render swagger ui: %v", err), http.StatusInternalServerError)
			}
		default:
			http.NotFound(w, r)
		}
	}
}
