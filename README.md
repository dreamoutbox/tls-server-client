# Rust TLS Server & Client POC

Minimal Rust proof-of-concept demonstrating TLS communication between a server and client using `tokio`, `rustls` (ring crypto provider), and `rustls-pemfile`, along with automated X.509 certificate generation and rotation.

## Quickstart

Run the end-to-end bootstrap script:

```bash
./scripts/bootstrap.sh
```

This single command handles:
1. Compiling the project binaries (`server` and `client`).
2. Generating a local test Certificate Authority (CA) and server certificates (if missing).
3. Starting the TLS echo server in the background.
4. Running the client, verifying TLS handshake against the CA, and testing echo data transmission.
5. Rotating the server certificate to a new Common Name (`rotated.local`), restarting the server, and re-verifying communication.
6. Performing negative verification (testing that mismatched SNI is rejected).

## Scripts

All scripts are located in `scripts/`:

- `scripts/gen-certs.sh`: Generates `certs/ca.key`, `certs/ca.crt`, `certs/server.key`, and `certs/server.crt`.
- `scripts/cert-rotation.sh`: Backs up existing server cert/key to `certs/old/` and generates new server certificates signed by the existing CA.
- `scripts/bootstrap.sh`: Automates end-to-end build, test, and rotation validation.

### Overriding Certificate Properties

You can customize Common Name (CN) and Subject Alternative Name (SAN) via environment variables:

```bash
TLS_CN="api.example.local" TLS_SAN="DNS:api.example.local,IP:127.0.0.1" ./scripts/gen-certs.sh
```

Or when rotating certificates:

```bash
TLS_CN="newhost.local" ./scripts/cert-rotation.sh
```

## Running Manually

You can use the root wrapper scripts:

1. Start the server (generates default certs automatically if absent):
   ```bash
   ./start-server.sh
   ```

2. Run the client against baseline `localhost`:
   ```bash
   ./start-client.sh
   ```

3. Rotate the certificate (specifying `TLS_CN` or using default `rotated.local`):
   ```bash
   TLS_CN="newhost.local" ./scripts/cert-rotation.sh
   ```

4. Restart the server (`Ctrl+C` then `./start-server.sh`), then run:
   ```bash
   ./start-client-after-rotate.sh
   ```
   Or connect with custom server name:
   ```bash
   TLS_SERVER_NAME="newhost.local" ./start-client.sh
   ```

4. Environment variable configuration:
   - Server:
     - `TLS_BIND_ADDR` (default: `0.0.0.0:8443`)
     - `TLS_CERT_PATH` (default: `certs/server.crt`)
     - `TLS_KEY_PATH` (default: `certs/server.key`)
   - Client:
     - `TLS_SERVER_ADDR` (default: `127.0.0.1:8443`)
     - `TLS_SERVER_NAME` (default: `localhost`)
     - `TLS_CA_PATH` (default: `certs/ca.crt`)
     - `TLS_PAYLOAD` (default: `Hello, Secure Rust TLS World!`)
