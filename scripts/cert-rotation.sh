#!/usr/bin/env bash
# Rotates server certificates using the existing CA.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cert-common.sh"

# Handle TLS_CN configuration and user guidance
if [[ -z "${TLS_CN:-}" || "${TLS_CN}" == "localhost" ]]; then
    TLS_CN="rotated.local"
    echo "Notice: TLS_CN was not specified. Defaulting to '${TLS_CN}'."
    echo "Tip: You can specify a custom domain name via: TLS_CN=yourdomain.local $0"
else
    echo "Using specified TLS_CN='${TLS_CN}'."
fi

# Align SAN with CN if TLS_SAN was not explicitly overridden
if [[ -z "${TLS_SAN:-}" || "${TLS_SAN}" == "DNS:localhost,IP:127.0.0.1" ]]; then
    TLS_SAN="DNS:${TLS_CN},IP:127.0.0.1"
fi

if [[ ! -f "${CERTS_DIR}/server.crt" || ! -f "${CERTS_DIR}/server.key" ]]; then
    echo "Existing server cert/key not found; generating initial certs first..."
    ensure_ca
    generate_server_cert "localhost" "DNS:localhost,IP:127.0.0.1" "${CERTS_DIR}/server.key" "${CERTS_DIR}/server.crt"
fi

# Backup existing certificates
mkdir -p "${CERTS_DIR}/old"
cp "${CERTS_DIR}/server.crt" "${CERTS_DIR}/old/server.crt"
cp "${CERTS_DIR}/server.key" "${CERTS_DIR}/old/server.key"

echo "Rotating server certificate to CN=${TLS_CN}, SAN=${TLS_SAN}..."
generate_server_cert "${TLS_CN}" "${TLS_SAN}" "${CERTS_DIR}/server.key" "${CERTS_DIR}/server.crt"

echo "=== Old Certificate Details ==="
openssl x509 -in "${CERTS_DIR}/old/server.crt" -noout -subject -ext subjectAltName

echo "=== Rotated Certificate Details ==="
openssl x509 -in "${CERTS_DIR}/server.crt" -noout -subject -ext subjectAltName

echo "=== CA Verification ==="
openssl verify -CAfile "${CERTS_DIR}/ca.crt" "${CERTS_DIR}/server.crt"

echo ""
echo "Certificate rotated successfully."
echo "Next steps:"
echo "  1. Restart the server: ./start-server.sh"
echo "  2. Connect the client:"
echo "       ./start-client-after-rotate.sh"
echo "     or with custom server name:"
echo "       TLS_SERVER_NAME=\"${TLS_CN}\" ./start-client.sh"
