package credentials

import "crypto/tls"

type TransportCredentials interface{ Config() *tls.Config }

type tlsCreds struct{ cfg *tls.Config }

func (t tlsCreds) Config() *tls.Config            { return t.cfg }
func NewTLS(cfg *tls.Config) TransportCredentials { return tlsCreds{cfg: cfg} }
