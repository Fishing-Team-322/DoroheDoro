package peer

import (
	"context"
	"net"
)

type Peer struct{ Addr net.Addr }

type contextKey string

const peerKey contextKey = "grpc-peer"

func FromContext(ctx context.Context) (*Peer, bool) {
	p, ok := ctx.Value(peerKey).(*Peer)
	return p, ok
}

func NewContext(ctx context.Context, p *Peer) context.Context {
	return context.WithValue(ctx, peerKey, p)
}
