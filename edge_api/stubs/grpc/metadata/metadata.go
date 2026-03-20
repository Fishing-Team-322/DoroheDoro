package metadata

import "context"

type MD map[string][]string

type contextKey string

const mdKey contextKey = "grpc-metadata"

func FromIncomingContext(ctx context.Context) (MD, bool) {
	md, ok := ctx.Value(mdKey).(MD)
	return md, ok
}

func NewIncomingContext(ctx context.Context, md MD) context.Context {
	return context.WithValue(ctx, mdKey, md)
}

func (md MD) Get(key string) []string { return md[key] }
