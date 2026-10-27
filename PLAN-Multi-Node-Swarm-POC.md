# PLAN: Multi-Node Swarm POC (mTLS + OpenRaft, local)

## Scope
POC demonstrating a Docker-Swarm-like cluster: 3 local nodes, mutually authenticated over mTLS, running real Raft consensus via `openraft` for leader election and task replication. Tested entirely on localhost — no VPS/deploy step in this POC.

---

### Phase 1: Scaffold [STATUS: DONE]
- Goal: Single `node` binary builds, takes a config path as arg.
- Tasks:
  - [x] `cargo init --name swarm-poc`
  - [x] `src/bin/node.rs` (stub `fn main()`, parses a config path from `std::env::args()`)
  - [x] Add deps: `tokio` (full), `tokio-rustls`, `rustls`, `rustls-pemfile`, `openraft`, `serde` + `serde_derive`, `toml`, `anyhow`, `tracing` + `tracing-subscriber`
- Done when: `cargo build` succeeds, producing `swarm-poc-node`.

### Phase 2: Peer config (static) [STATUS: DONE]
- Goal: Each node knows its own identity, cert paths, and the static 3-node membership list — fixed at startup, no dynamic join.
- Tasks:
  - [x] `nodes.toml` format: `self_id` (maps to `openraft::NodeId`), `listen_addr`, `cert_path`, `key_path`, `ca_path`, and a `[[peers]]` list of `{ id, addr }`
  - [x] Three example configs: `config/node1.toml`, `config/node2.toml`, `config/node3.toml`, all pointing at `127.0.0.1` with distinct ports
  - [x] `node.rs` loads and validates config
- Done when: `swarm-poc-node config/node1.toml` parses and prints its own id + peer list, then exits.

### Phase 3: Node certs (mTLS material) [STATUS: DONE]
- Goal: One shared CA, one cert per node, for mutual authentication between the 3 local nodes.
- Tasks:
  - [x] `scripts/gen-node-certs.sh`: one CA (`certs/ca.crt`/`.key`), then a cert+key per node signed by that CA, `CN`/SAN per node id (`CN=node1`, `SAN=DNS:node1,IP:127.0.0.1`)
  - [x] Output: `certs/node{1,2,3}.crt`/`.key`, shared `certs/ca.crt`
  - [x] Idempotent
- Done when: each `certs/nodeN.crt` verifies against `certs/ca.crt` via `openssl verify`.

### Phase 4: mTLS transport [STATUS: DONE]
- Goal: Authenticated, encrypted byte-stream between every pair of nodes — the substrate Raft RPCs ride on.
- Tasks:
  - [x] `ServerConfig` with `.with_client_cert_verifier(...)` against `ca.crt` (reject unverified client certs)
  - [x] `ClientConfig` presenting the node's own cert/key, verifying peers against `ca.crt`
  - [x] Length-prefixed framing over each `TlsStream` for passing serialized Raft RPC messages
  - [x] Dial loop with backoff until mTLS handshake succeeds; log handshake failures (e.g. untrusted cert) distinctly from "peer not yet up"
- Done when: all 3 nodes mutually mTLS-connect, logging each other "up," and a cert not signed by the shared CA is rejected and logged, not silently dropped.

### Phase 5: OpenRaft wiring [STATUS: DONE]
- Goal: Real Raft cluster of 3 static voters, using the mTLS links from Phase 4 as the network layer.
- Tasks:
  - [x] Implement `openraft::RaftNetwork` / `RaftNetworkFactory`: `append_entries`, `vote`, `install_snapshot` serialized and sent over the framed mTLS stream to the target peer
  - [x] Implement `RaftLogStorage` + `RaftStateMachine` as **in-memory** for the POC (no durability across restarts — flagged below)
  - [x] Initialize the 3-node static membership (`openraft::Raft::initialize` with all 3 ids) once, on first boot
- Done when: starting all 3 nodes produces exactly one elected leader (visible in logs with a term number), and `openraft`'s own heartbeat (leader's periodic `AppendEntries`) keeps followers from timing out while the leader is alive.

### Phase 6: Leader election & failover [STATUS: DONE]
- Goal: Prove Raft's actual election behavior, not a hardcoded leader.
- Tasks:
  - [x] Log every role transition (Follower/Candidate/Leader) with term number on each node
  - [x] No app code picks the leader — it's whatever `openraft` elects
- Done when: killing the current leader process causes the remaining 2 nodes to elect a new leader (new term, logged) within the configured election timeout, with no manual intervention.

### Phase 7: Task via Raft log (the "swarm" part) [STATUS: DONE]
- Goal: A demo task is submitted as a Raft log entry to the leader; once committed to a majority, every node's state machine applies it — that's the replicated "broadcast."
- Tasks:
  - [x] Define the log entry payload: `{"cmd": "docker run --rm hello-world"}` (or a no-Docker-required echo command if Docker isn't guaranteed in the test env)
  - [x] Client call (CLI flag on the leader, or a tiny local RPC) submits the entry via `Raft::client_write`
  - [x] `RaftStateMachine::apply` executes the command via `tokio::process::Command` and logs `{node_id, exit_code}` on **every** node once the entry commits
- Done when: submitting one entry on the leader results in all 3 nodes (including followers, via log replication) logging the command's execution and exit code.

### Phase 8: Bootstrap script (local only) [STATUS: DONE]
- Goal: One script proves the whole cluster — mTLS, election, replication — on localhost, no VPS involved.
- Tasks:
  - [x] `scripts/bootstrap-swarm.sh` (separate file from the other POC's `bootstrap.sh`): generates certs via `gen-node-certs.sh` if missing, starts all 3 nodes as background processes on localhost with distinct ports, waits for a leader-elected log line, submits a task entry, asserts all 3 logs show the task applied, kills all processes
  - [x] Trap/cleanup so a failed run still kills all 3 background processes; propagate exit code
  - [x] No `--remote`/VPS mode in this POC
- Done when: `./scripts/bootstrap-swarm.sh` exits 0 on a clean checkout with no manual steps.

---

## Unknowns, risks, assumptions
- **In-memory Raft storage.** `RaftLogStorage`/`RaftStateMachine` are in-memory for this POC — a node that restarts loses its log and state, so "failover" in Phase 6 means the *surviving* 2 nodes carry on, not that a restarted node rejoins with its prior state. Durable storage (e.g. `openraft`'s `RocksDB`/`sled` backed examples) is the natural next step, not in scope here.
- **Static 3-voter membership**, set once at init — no dynamic add/remove-node in this POC (that's `openraft`'s membership-change API, bigger scope).
- **No VPS/deploy script in this plan** — Phase 7/8 from the prior version (deploy to real VPS hosts, `--remote` bootstrap mode) are dropped entirely for now per your call to test local-only. Straightforward to add back once this is proven locally: mostly the deploy script, the config already uses real addrs (just `127.0.0.1` right now).
- mTLS and static config are confirmed (not open questions) — carried over from the earlier version of this plan.
