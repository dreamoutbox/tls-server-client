#!/usr/bin/env bash
# Wrapper script to run the TLS client against the rotated server certificate.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERT_PATH="${SCRIPT_DIR}/certs/server.crt"

detected_cn=""
if [[ -f "${CERT_PATH}" ]]; then
    detected_cn="$(openssl x509 -in "${CERT_PATH}" -noout -subject 2>/dev/null | sed -n 's/.*CN = \([^,]*\).*/\1/p' || true)"
fi

if [[ -z "${TLS_SERVER_NAME:-}" ]]; then
    if [[ -n "${detected_cn}" && "${detected_cn}" != "localhost" ]]; then
        export TLS_SERVER_NAME="${detected_cn}"
    else
        export TLS_SERVER_NAME="rotated.local"
    fi
fi

echo "Starting TLS client against rotated server (server name: ${TLS_SERVER_NAME})..."
exec cargo run --bin client "$@"
