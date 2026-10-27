# PLAN: Rust TLS Server/Client POC

## Scope
Minimal Rust POC: a TLS server and client in `src/bin/`, communicating over TLS, with cert generation, cert rotation, and a bootstrap script that proves the whole thing works end-to-end.

---

### Phase 1: Scaffold [STATUS: DONE]
- Goal: Cargo project builds with empty server/client binaries.
- Tasks:
  - [x] `cargo init --name tls-poc`
  - [x] `src/bin/server.rs`, `src/bin/client.rs` (stub `fn main()`)
  - [x] Add deps to `Cargo.toml`: `tokio` (full), `tokio-rustls`, `rustls`, `rustls-pemfile`, `anyhow`
- Done when: `cargo build` succeeds, producing `tls-poc-server` and `tls-poc-client` binaries.

### Phase 2: Cert/key generation script [STATUS: DONE]
- Goal: One script produces a CA + server cert/key usable by `rustls`, via `openssl`.
- Tasks:
  - [x] `scripts/gen-certs.sh`: generate CA key+cert, server key+CSR+cert signed by CA, output to `certs/`
  - [x] `CN`/SAN pulled from shell vars at the top of the script (e.g. `CN="${TLS_CN:-localhost}"`, `SAN="${TLS_SAN:-DNS:localhost,IP:127.0.0.1}"`) so Phase 3 can override them
  - [x] Script is idempotent (safe to re-run; cleans or skips existing certs)
- Done when: running the script produces `certs/ca.crt`, `certs/server.crt`, `certs/server.key`, all parseable by `openssl x509 -in ... -noout -text`.

### Phase 3: Cert rotation script [STATUS: DONE]
- Goal: Demonstrate rotating the server cert to a new mock CN/SAN without touching the CA, so a running server can reload.
- Tasks:
  - [x] `scripts/cert-rotation.sh`: sources the same `TLS_CN`/`TLS_SAN` vars as `gen-certs.sh`, lets caller override (e.g. `TLS_CN=rotated.local ./scripts/cert-rotation.sh`), backs up old `server.crt`/`server.key` to `certs/old/`, regenerates new server cert/key signed by the existing CA
  - [x] Print old vs. new cert `Subject`/`SAN` (via `openssl x509 -noout -subject -ext subjectAltName`) so the rotation is visible
- Done when: running the script with a different `TLS_CN` produces a new server cert with that CN, still verifiable against `certs/ca.crt`, and the diff between old/new subject is printed.

### Phase 4: TLS server [STATUS: DONE]
- Goal: Server accepts TLS connections and echoes received data; picks up rotated certs.
- Tasks:
  - [x] Load `certs/server.crt` + `certs/server.key` into `rustls::ServerConfig`
  - [x] `TcpListener` + `tokio_rustls::TlsAcceptor`, bind `127.0.0.1:8443`
  - [x] Per-connection: read bytes, echo back, log peer addr
  - [x] Note (no impl yet): hot-reload after rotation is out of scope for the POC — restarting the server to pick up rotated certs is acceptable
- Done when: `cargo run --bin server` listens and an `openssl s_client -connect 127.0.0.1:8443 -CAfile certs/ca.crt` handshake succeeds.

### Phase 5: TLS client [STATUS: DONE]
- Goal: Client connects to server over TLS, verifying against the CA, sends a message, prints the echo.
- Tasks:
  - [x] Load `certs/ca.crt` into `rustls::RootCertStore` / `ClientConfig`
  - [x] Connect, send a fixed test payload, read and print response
  - [x] Exit non-zero on handshake or I/O failure
- Done when: `cargo run --bin client` against a running server prints the echoed payload and exits 0.

### Phase 6: Bootstrap script [STATUS: DONE]
- Goal: Single script proves the POC end-to-end, including a rotation cycle, usable in CI.
- Tasks:
  - [x] `scripts/bootstrap.sh`: build, run `gen-certs.sh` if certs missing, start server in background, wait for port, run client, assert expected output
  - [x] Run `cert-rotation.sh` with a different `TLS_CN`, restart server, re-run client, assert it still succeeds against the rotated cert
  - [x] Trap/cleanup so a failed run still kills the background server; propagate exit code
- Done when: `./scripts/bootstrap.sh` exits 0 on a clean checkout with no manual steps (pre/post rotation), and exits non-zero with the server killed if either client run fails.

### Phase 7: Polish [STATUS: DONE]
- Goal: POC is readable and safe to hand off.
- Tasks:
  - [x] `README.md`: what it does, how to run `bootstrap.sh`, how to override `TLS_CN`/`TLS_SAN`
  - [x] `.gitignore` for `certs/`, `target/`
  - [x] Basic error handling via `anyhow::Result` in both binaries (no `unwrap()` on I/O paths)
- Done when: fresh clone + `./scripts/bootstrap.sh` is the only documented step required.

---

## Unknowns, risks, assumptions
- `openssl` confirmed available — used for all cert/key generation and rotation (not `ssh-keygen`, which doesn't produce X.509 certs).
- Assumed localhost-only POC (no mTLS); rotation changes CN/SAN but still chains to the same CA.
- Server does not hot-reload certs on rotation (Phase 4) — rotation is proven by regenerate → restart → reconnect, not live reload. Flag if you want live reload instead; that's a bigger scope add (e.g. `ArcSwap<ServerConfig>` + file watcher).
