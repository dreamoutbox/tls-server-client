#!/usr/bin/env bash
# Common configuration and helpers for certificate generation and rotation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CERTS_DIR="${CERTS_DIR:-${PROJECT_ROOT}/certs}"

TLS_CN="${TLS_CN:-localhost}"
TLS_SAN="${TLS_SAN:-DNS:localhost,IP:127.0.0.1}"

# Ensures CA certificate and private key exist in CERTS_DIR.
ensure_ca() {
    mkdir -p "${CERTS_DIR}"
    local ca_key="${CERTS_DIR}/ca.key"
    local ca_crt="${CERTS_DIR}/ca.crt"

    if [[ ! -f "${ca_key}" || ! -f "${ca_crt}" ]]; then
        echo "Generating CA key and certificate..."
        openssl req -x509 -newkey rsa:2048 -nodes \
            -keyout "${ca_key}" \
            -out "${ca_crt}" \
            -days 365 \
            -subj "/CN=Test CA"
    fi
}

# Generates a server private key and certificate signed by the CA.
# Arguments:
#   $1 - Subject Common Name (CN)
#   $2 - Subject Alternative Names (SAN)
#   $3 - Destination key path
#   $4 - Destination certificate path
generate_server_cert() {
    local cn="$1"
    local san="$2"
    local out_key="$3"
    local out_crt="$4"
    local ca_key="${CERTS_DIR}/ca.key"
    local ca_crt="${CERTS_DIR}/ca.crt"

    ensure_ca

    local ext_file
    ext_file="$(mktemp)"
    printf "subjectAltName=%s\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n" "${san}" > "${ext_file}"

    local csr_file
    csr_file="$(mktemp)"

    openssl genrsa -out "${out_key}" 2048 2>/dev/null
    openssl req -new -key "${out_key}" -out "${csr_file}" -subj "/CN=${cn}" 2>/dev/null
    openssl x509 -req -in "${csr_file}" -CA "${ca_crt}" -CAkey "${ca_key}" -CAcreateserial \
        -out "${out_crt}" -days 365 -extfile "${ext_file}" 2>/dev/null

    rm -f "${ext_file}" "${csr_file}"
}
