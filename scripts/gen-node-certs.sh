#!/usr/bin/env bash
# Generates mTLS CA and node certificates for the 3-node swarm cluster.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/cert-common.sh"

# Generates a mutual TLS certificate for a node, enabling both server and client authentication.
# Arguments:
#   $1 - Node name (e.g. node1)
#   $2 - Common Name (e.g. node1)
#   $3 - Subject Alternative Names (e.g. DNS:node1,IP:127.0.0.1)
#   $4 - Output private key path
#   $5 - Output certificate path
generate_node_cert() {
    local node_name="$1"
    local cn="$2"
    local san="$3"
    local out_key="$4"
    local out_crt="$5"
    local ca_key="${CERTS_DIR}/ca.key"
    local ca_crt="${CERTS_DIR}/ca.crt"

    ensure_ca

    local ext_file
    ext_file="$(mktemp)"
    printf "subjectAltName=%s\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth,clientAuth\n" "${san}" > "${ext_file}"

    local csr_file
    csr_file="$(mktemp)"

    openssl genrsa -out "${out_key}" 2048 2>/dev/null
    openssl req -new -key "${out_key}" -out "${csr_file}" -subj "/CN=${cn}" 2>/dev/null
    openssl x509 -req -in "${csr_file}" -CA "${ca_crt}" -CAkey "${ca_key}" -CAcreateserial \
        -out "${out_crt}" -days 365 -extfile "${ext_file}" 2>/dev/null

    rm -f "${ext_file}" "${csr_file}"
}

echo "Generating swarm cluster certificates in ${CERTS_DIR}..."
ensure_ca

for i in 1 2 3; do
    node="node${i}"
    key_path="${CERTS_DIR}/${node}.key"
    crt_path="${CERTS_DIR}/${node}.crt"

    # Generate if either key or cert is missing
    if [[ ! -f "${key_path}" || ! -f "${crt_path}" ]]; then
        echo "Generating ${node} cert (CN=${node}, SAN=DNS:${node},IP:127.0.0.1)..."
        generate_node_cert "${node}" "${node}" "DNS:${node},IP:127.0.0.1" "${key_path}" "${crt_path}"
    else
        echo "Using existing certificate for ${node}."
    fi
done

echo "Swarm certificates ready:"
echo "  CA cert:    ${CERTS_DIR}/ca.crt"
for i in 1 2 3; do
    echo "  Node ${i}:   ${CERTS_DIR}/node${i}.crt (${CERTS_DIR}/node${i}.key)"
done
