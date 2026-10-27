#!/usr/bin/env bash
# End-to-end bootstrap script for TLS server-client POC.
# Proves certificate generation, server startup, client communication,
# certificate rotation, server restart, and client re-verification.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CERTS_DIR="${CERTS_DIR:-${PROJECT_ROOT}/certs}"

server_pid=""

cleanup() {
    if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
        echo "Stopping server process ${server_pid}..."
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
        server_pid=""
    fi
}
trap cleanup EXIT INT TERM

wait_for_port() {
    local host="$1"
    local port="$2"
    local tries=0
    local max_tries=50

    while ! (cat < /dev/null > "/dev/tcp/${host}/${port}") 2>/dev/null; do
        if [[ -n "${server_pid}" ]] && ! kill -0 "${server_pid}" 2>/dev/null; then
            echo "Error: Server process exited unexpectedly" >&2
            exit 1
        fi
        tries=$((tries + 1))
        if [[ ${tries} -ge ${max_tries} ]]; then
            echo "Error: Timed out waiting for ${host}:${port}" >&2
            exit 1
        fi
        sleep 0.1
    done
}

start_server() {
    echo "Starting TLS server..."
    "${PROJECT_ROOT}/target/debug/server" &
    server_pid=$!
    wait_for_port "127.0.0.1" "8443"
    echo "Server successfully listening on 127.0.0.1:8443 (PID: ${server_pid})"
}

stop_server() {
    if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
        echo "Stopping server (PID: ${server_pid})..."
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
        server_pid=""
    fi
}

echo "=== Step 1: Building project binaries ==="
cargo build --bins

echo "=== Step 2: Preparing baseline certificates (CN=localhost) ==="
"${SCRIPT_DIR}/gen-certs.sh"

echo "=== Step 3: Starting server (initial cert: localhost) ==="
start_server

echo "=== Step 4: Running client (verifying initial echo) ==="
initial_payload="Hello, Initial TLS World!"
client_out=$(TLS_PAYLOAD="${initial_payload}" TLS_SERVER_NAME="localhost" "${PROJECT_ROOT}/target/debug/client")
if [[ "${client_out}" != "${initial_payload}" ]]; then
    echo "Error: Unexpected client output: '${client_out}' (expected '${initial_payload}')" >&2
    exit 1
fi
echo "Initial client echo verified: '${client_out}'"

echo "=== Step 5: Stopping server for certificate rotation ==="
stop_server

echo "=== Step 6: Rotating certificate to rotated.local ==="
TLS_CN="rotated.local" "${SCRIPT_DIR}/cert-rotation.sh"

echo "=== Step 7: Restarting server with rotated certificate ==="
start_server

echo "=== Step 8: Running client against rotated certificate ==="
rotated_payload="Hello, Rotated TLS World!"
rotated_client_out=$(TLS_PAYLOAD="${rotated_payload}" TLS_SERVER_NAME="rotated.local" "${PROJECT_ROOT}/target/debug/client")
if [[ "${rotated_client_out}" != "${rotated_payload}" ]]; then
    echo "Error: Unexpected client output after rotation: '${rotated_client_out}' (expected '${rotated_payload}')" >&2
    exit 1
fi
echo "Rotated client echo verified: '${rotated_client_out}'"

echo "=== Step 9: Testing negative case (connecting with mismatched SNI should fail) ==="
if TLS_PAYLOAD="Fail payload" TLS_SERVER_NAME="wrong.name.local" "${PROJECT_ROOT}/target/debug/client" 2>/dev/null; then
    echo "Error: Client connection succeeded with invalid server name, expected failure" >&2
    exit 1
fi
echo "Client properly rejected connection to mismatched server name."

stop_server

echo "=== Bootstrap POC completed successfully! ==="
