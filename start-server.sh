#!/usr/bin/env bash
# Wrapper script to start the TLS server.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="${SCRIPT_DIR}/certs"

if [[ ! -f "${CERTS_DIR}/server.crt" || ! -f "${CERTS_DIR}/server.key" ]]; then
    echo "Server certificates not found in ${CERTS_DIR}. Generating initial certificates..."
    "${SCRIPT_DIR}/scripts/gen-certs.sh"
fi

echo "Starting TLS server..."
exec cargo run --bin server "$@"
