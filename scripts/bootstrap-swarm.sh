#!/usr/bin/env bash
# End-to-end bootstrap script for the Multi-Node Swarm POC (mTLS + OpenRaft).
# Proves certificate generation, 3-node cluster startup, Raft leader election,
# task submission, replicated state machine execution across all nodes, and cleanup.
set -euo pipefail
export NO_COLOR=1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CERTS_DIR="${CERTS_DIR:-${PROJECT_ROOT}/certs}"

node1_pid=""
node2_pid=""
node3_pid=""

cleanup() {
    echo "Cleaning up background node processes..."
    for pid in "${node1_pid}" "${node2_pid}" "${node3_pid}"; do
        if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
            kill "${pid}" 2>/dev/null || true
            wait "${pid}" 2>/dev/null || true
        fi
    done
}
trap cleanup EXIT INT TERM

wait_for_port() {
    local host="$1"
    local port="$2"
    local pid="$3"
    local tries=0
    local max_tries=50

    while ! (cat < /dev/null > "/dev/tcp/${host}/${port}") 2>/dev/null; do
        if [[ -n "${pid}" ]] && ! kill -0 "${pid}" 2>/dev/null; then
            echo "Error: Node process (PID ${pid}) exited unexpectedly while waiting for port ${port}" >&2
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

echo "=== Step 1: Building swarm-poc-node binary ==="
cargo build --bin swarm-poc-node

echo "=== Step 2: Generating mTLS certificates ==="
"${SCRIPT_DIR}/gen-node-certs.sh"

log_dir="$(mktemp -d)"
log1="${log_dir}/node1.log"
log2="${log_dir}/node2.log"
log3="${log_dir}/node3.log"

echo "=== Step 3: Starting 3 cluster nodes on localhost ==="
"${PROJECT_ROOT}/target/debug/swarm-poc-node" "${PROJECT_ROOT}/config/node1.toml" > "${log1}" 2>&1 &
node1_pid=$!
wait_for_port "127.0.0.1" "8443" "${node1_pid}"

"${PROJECT_ROOT}/target/debug/swarm-poc-node" "${PROJECT_ROOT}/config/node2.toml" > "${log2}" 2>&1 &
node2_pid=$!
wait_for_port "127.0.0.1" "8444" "${node2_pid}"

"${PROJECT_ROOT}/target/debug/swarm-poc-node" "${PROJECT_ROOT}/config/node3.toml" > "${log3}" 2>&1 &
node3_pid=$!
wait_for_port "127.0.0.1" "8445" "${node3_pid}"

echo "All 3 nodes listening (PIDs: ${node1_pid}, ${node2_pid}, ${node3_pid})"

echo "=== Step 4: Waiting for OpenRaft leader election ==="
leader_elected=false
for _ in $(seq 1 40); do
    if grep -q "role=Leader" "${log1}" "${log2}" "${log3}" 2>/dev/null; then
        leader_elected=true
        break
    fi
    sleep 0.2
done

if [[ "${leader_elected}" != "true" ]]; then
    echo "Error: Timed out waiting for cluster leader election" >&2
    echo "=== Node 1 Log ===" >&2
    cat "${log1}" >&2 || true
    echo "=== Node 2 Log ===" >&2
    cat "${log2}" >&2 || true
    echo "=== Node 3 Log ===" >&2
    cat "${log3}" >&2 || true
    exit 1
fi
echo "Leader elected successfully:"
grep "role=Leader" "${log1}" "${log2}" "${log3}"

echo "=== Step 5: Submitting swarm task to leader ==="
task_cmd="echo swarm-task-verified"
task_output=$("${PROJECT_ROOT}/target/debug/swarm-poc-node" --submit-task "${task_cmd}" "${PROJECT_ROOT}/config/node1.toml")
echo "Leader task output: ${task_output}"

# Wait for log replication and state machine commit across all nodes
sleep 1

echo "=== Step 6: Verifying replicated state machine execution on ALL nodes ==="
for i in 1 2 3; do
    log_file="${log_dir}/node${i}.log"
    echo "Checking Node ${i} log..."
    if ! grep -q "Replicated task command completed execution.*cmd=\"${task_cmd}\".*exit_code=0" "${log_file}" && \
       ! grep -q "Replicated task command completed execution.*cmd=${task_cmd}.*exit_code=0" "${log_file}"; then
        echo "Error: Node ${i} did not execute replicated task '${task_cmd}'" >&2
        cat "${log_file}" >&2
        exit 1
    fi
    echo "Node ${i} confirmed executed task '${task_cmd}' with exit code 0"
done

echo "=== Multi-Node Swarm POC verified successfully! ==="
