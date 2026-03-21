package main

import (
	"crypto/rand"
	"crypto/rsa"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"fmt"
	"math/big"
	"net"
	"os"
	"path/filepath"
	"time"
)

func main() {
	outDir := getenv("DEV_CERTS_OUT_DIR", filepath.Join("..", ".tmp", "dev-certs"))
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		panic(err)
	}

	caCert, caKey, err := createCertificateAuthority()
	if err != nil {
		panic(err)
	}
	if err := writePEM(filepath.Join(outDir, "ca.crt"), "CERTIFICATE", caCert.Raw); err != nil {
		panic(err)
	}
	if err := writePrivateKey(filepath.Join(outDir, "ca.key"), caKey); err != nil {
		panic(err)
	}

	serverCert, serverKey, err := createSignedCertificate(caCert, caKey, certificateRequest{
		CommonName: "edge-api",
		DNSNames:   []string{"localhost", "edge-api"},
		IPAddrs:    []net.IP{net.ParseIP("127.0.0.1")},
		ServerAuth: true,
	})
	if err != nil {
		panic(err)
	}
	if err := writePEM(filepath.Join(outDir, "server.crt"), "CERTIFICATE", serverCert.Raw); err != nil {
		panic(err)
	}
	if err := writePrivateKey(filepath.Join(outDir, "server.key"), serverKey); err != nil {
		panic(err)
	}

	clientCert, clientKey, err := createSignedCertificate(caCert, caKey, certificateRequest{
		CommonName: "agent-dev",
		DNSNames:   []string{"agent-dev"},
		ClientAuth: true,
	})
	if err != nil {
		panic(err)
	}
	if err := writePEM(filepath.Join(outDir, "agent.crt"), "CERTIFICATE", clientCert.Raw); err != nil {
		panic(err)
	}
	if err := writePrivateKey(filepath.Join(outDir, "agent.key"), clientKey); err != nil {
		panic(err)
	}

	fmt.Printf("generated dev certs in %s\n", outDir)
}

type certificateRequest struct {
	CommonName string
	DNSNames   []string
	IPAddrs    []net.IP
	ServerAuth bool
	ClientAuth bool
}

func createCertificateAuthority() (*x509.Certificate, *rsa.PrivateKey, error) {
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		return nil, nil, fmt.Errorf("generate ca key: %w", err)
	}
	template := &x509.Certificate{
		SerialNumber: randomSerial(),
		Subject: pkix.Name{
			CommonName:   "Dorohedoro Dev CA",
			Organization: []string{"Dorohedoro Dev"},
		},
		NotBefore:             time.Now().Add(-time.Hour),
		NotAfter:              time.Now().Add(7 * 24 * time.Hour),
		KeyUsage:              x509.KeyUsageCertSign | x509.KeyUsageCRLSign | x509.KeyUsageDigitalSignature,
		BasicConstraintsValid: true,
		IsCA:                  true,
		MaxPathLen:            1,
	}
	der, err := x509.CreateCertificate(rand.Reader, template, template, &key.PublicKey, key)
	if err != nil {
		return nil, nil, fmt.Errorf("create ca cert: %w", err)
	}
	cert, err := x509.ParseCertificate(der)
	if err != nil {
		return nil, nil, fmt.Errorf("parse ca cert: %w", err)
	}
	return cert, key, nil
}

func createSignedCertificate(caCert *x509.Certificate, caKey *rsa.PrivateKey, req certificateRequest) (*x509.Certificate, *rsa.PrivateKey, error) {
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		return nil, nil, fmt.Errorf("generate key: %w", err)
	}
	template := &x509.Certificate{
		Subject: pkix.Name{
			CommonName:   req.CommonName,
			Organization: []string{"Dorohedoro Dev"},
		},
		NotBefore:    time.Now().Add(-time.Hour),
		NotAfter:     time.Now().Add(7 * 24 * time.Hour),
		KeyUsage:     x509.KeyUsageDigitalSignature | x509.KeyUsageKeyEncipherment,
		ExtKeyUsage:  extKeyUsage(req),
		DNSNames:     req.DNSNames,
		IPAddresses:  req.IPAddrs,
		IsCA:         false,
		SerialNumber: randomSerial(),
	}
	der, err := x509.CreateCertificate(rand.Reader, template, caCert, &key.PublicKey, caKey)
	if err != nil {
		return nil, nil, fmt.Errorf("create signed cert: %w", err)
	}
	cert, err := x509.ParseCertificate(der)
	if err != nil {
		return nil, nil, fmt.Errorf("parse signed cert: %w", err)
	}
	return cert, key, nil
}

func extKeyUsage(req certificateRequest) []x509.ExtKeyUsage {
	usages := make([]x509.ExtKeyUsage, 0, 2)
	if req.ServerAuth {
		usages = append(usages, x509.ExtKeyUsageServerAuth)
	}
	if req.ClientAuth {
		usages = append(usages, x509.ExtKeyUsageClientAuth)
	}
	if len(usages) == 0 {
		usages = append(usages, x509.ExtKeyUsageClientAuth)
	}
	return usages
}

func writePrivateKey(path string, key *rsa.PrivateKey) error {
	return writePEM(path, "RSA PRIVATE KEY", x509.MarshalPKCS1PrivateKey(key))
}

func writePEM(path, blockType string, data []byte) error {
	file, err := os.OpenFile(path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0o600)
	if err != nil {
		return err
	}
	defer file.Close()
	return pem.Encode(file, &pem.Block{Type: blockType, Bytes: data})
}

func randomSerial() *big.Int {
	limit := new(big.Int).Lsh(big.NewInt(1), 128)
	serial, err := rand.Int(rand.Reader, limit)
	if err != nil {
		panic(err)
	}
	return serial
}

func getenv(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
