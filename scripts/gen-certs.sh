#!/usr/bin/env bash
# Generates CA and server certificates for TLS POC.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cert-common.sh"

echo "Generating certificates with CN=${TLS_CN}, SAN=${TLS_SAN}..."
ensure_ca
generate_server_cert "${TLS_CN}" "${TLS_SAN}" "${CERTS_DIR}/server.key" "${CERTS_DIR}/server.crt"

echo "Certificates successfully generated in ${CERTS_DIR}:"
echo "  CA cert:     ${CERTS_DIR}/ca.crt"
echo "  Server cert: ${CERTS_DIR}/server.crt"
echo "  Server key:  ${CERTS_DIR}/server.key"
