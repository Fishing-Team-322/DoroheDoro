package insecure

type insecureCreds struct{}

func NewCredentials() insecureCreds { return insecureCreds{} }
