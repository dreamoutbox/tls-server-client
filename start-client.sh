#!/usr/bin/env bash
# Wrapper script to run the TLS client.
set -euo pipefail

export TLS_SERVER_NAME="${TLS_SERVER_NAME:-localhost}"

echo "Starting TLS client (server name: ${TLS_SERVER_NAME})..."
exec cargo run --bin client "$@"
