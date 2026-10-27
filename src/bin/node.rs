//! Entrypoint for the swarm-poc node binary.

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use openraft::storage::Adaptor;
use openraft::{BasicNode, Config as RaftConfig, Raft};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use tls_server_client::cert::CertLoader;
use tls_server_client::config::NodeConfig;
use tls_server_client::consensus::{
    InMemoryStore, NetworkFactory, RaftRpcRequest, RaftRpcResponse, TaskCommand, TypeConfig,
};
use tls_server_client::network::{FramedStream, MtlsConnector, MtlsListener};

/// CLI options parsed from command-line arguments.
struct CliOptions {
    config_path: PathBuf,
    validate_only: bool,
    force_init: bool,
    submit_task: Option<String>,
}

/// Parses CLI arguments to extract the node configuration file path, subcommands, and flags.
///
/// # Arguments
/// * `args` - Iterator over command-line arguments (including binary name as first element).
///
/// # Example
/// ```rust
/// let args = vec!["swarm-poc-node".to_string(), "config/node1.toml".to_string()];
/// let opts = parse_args(args.into_iter()).unwrap();
/// assert_eq!(opts.config_path, std::path::PathBuf::from("config/node1.toml"));
/// ```
fn parse_args<I>(mut args: I) -> Result<CliOptions>
where
    I: Iterator<Item = String>,
{
    let _bin_name = args.next();
    let mut config_path = None;
    let mut validate_only = false;
    let mut force_init = false;
    let mut submit_task = None;

    while let Some(arg) = args.next() {
        if arg == "--validate" || arg == "--check" {
            validate_only = true;
        } else if arg == "--init" {
            force_init = true;
        } else if arg == "--submit-task" {
            let cmd = args
                .next()
                .context("Expected command after --submit-task")?;
            submit_task = Some(cmd);
        } else if config_path.is_none() {
            config_path = Some(PathBuf::from(arg));
        } else {
            bail!("Unexpected argument: {}", arg);
        }
    }

    let config_path = match config_path {
        Some(p) => p,
        None => {
            bail!("Usage: swarm-poc-node [--validate] [--init] [--submit-task <cmd>] <config_path>")
        }
    };

    Ok(CliOptions {
        config_path,
        validate_only,
        force_init,
        submit_task,
    })
}

/// Submits a task command to a cluster node over mTLS.
///
/// # Arguments
/// * `config` - Local node configuration for certificates and target address.
/// * `cmd` - Shell command string to replicate and execute across the swarm.
async fn submit_task_client(config: &NodeConfig, cmd: &str) -> Result<()> {
    let client_tls =
        CertLoader::mtls_client_config(&config.cert_path, &config.key_path, &config.ca_path)
            .context("Failed to load client TLS credentials")?;

    let connector = MtlsConnector::new(client_tls);
    let mut stream = connector
        .connect_with_backoff(
            config.self_id,
            config.listen_addr,
            5,
            Duration::from_millis(100),
            Duration::from_secs(1),
        )
        .await
        .context("Failed to connect to target node")?;

    let req = RaftRpcRequest::Task(TaskCommand::new(cmd));
    let req_bytes = serde_json::to_vec(&req)?;
    stream.write_frame(&req_bytes).await?;

    let resp_bytes = stream
        .read_frame()
        .await?
        .context("Node closed connection unexpectedly")?;

    let resp: RaftRpcResponse = serde_json::from_slice(&resp_bytes)?;
    match resp {
        RaftRpcResponse::Task(task_resp) => {
            info!(
                node_id = task_resp.node_id,
                exit_code = task_resp.exit_code,
                output = %task_resp.output,
                "Task executed successfully on cluster"
            );
            println!("{}", task_resp.output);
            if task_resp.exit_code != 0 {
                bail!("Task failed with exit code {}", task_resp.exit_code);
            }
        }
        RaftRpcResponse::Error(err) => {
            bail!("Cluster returned error for task: {}", err);
        }
        other => {
            bail!("Unexpected RPC response variant: {:?}", other);
        }
    }

    Ok(())
}

/// Handles inbound RPC requests on an established peer mTLS stream.
async fn handle_rpc_stream<S>(
    mut stream: FramedStream<S>,
    raft: Arc<Raft<TypeConfig>>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(frame) = stream.read_frame().await? {
        let req: RaftRpcRequest = match serde_json::from_slice(&frame) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = RaftRpcResponse::Error(format!("Invalid JSON: {}", e));
                let resp_bytes = serde_json::to_vec(&err_resp)?;
                stream.write_frame(&resp_bytes).await?;
                continue;
            }
        };

        let resp = match req {
            RaftRpcRequest::Vote(rpc) => {
                let res = raft.vote(rpc).await;
                match res {
                    Ok(resp) => RaftRpcResponse::Vote(resp),
                    Err(e) => RaftRpcResponse::Error(e.to_string()),
                }
            }
            RaftRpcRequest::AppendEntries(rpc) => {
                let res = raft.append_entries(rpc).await;
                match res {
                    Ok(resp) => RaftRpcResponse::AppendEntries(resp),
                    Err(e) => RaftRpcResponse::Error(e.to_string()),
                }
            }
            RaftRpcRequest::InstallSnapshot(rpc) => {
                let res = raft.install_snapshot(rpc).await;
                match res {
                    Ok(resp) => RaftRpcResponse::InstallSnapshot(resp),
                    Err(e) => RaftRpcResponse::Error(e.to_string()),
                }
            }
            RaftRpcRequest::Task(cmd) => {
                let res = raft.client_write(cmd).await;
                match res {
                    Ok(client_write_resp) => RaftRpcResponse::Task(client_write_resp.data),
                    Err(e) => RaftRpcResponse::Error(e.to_string()),
                }
            }
        };

        let resp_bytes = serde_json::to_vec(&resp)?;
        stream.write_frame(&resp_bytes).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let opts = parse_args(env::args()).context("Failed to parse command-line arguments")?;
    let config = NodeConfig::from_file(&opts.config_path)
        .with_context(|| format!("Failed to load config from '{:?}'", opts.config_path))?;

    info!(
        self_id = config.self_id,
        listen_addr = %config.listen_addr,
        peers = ?config.peers,
        "Node configuration parsed successfully"
    );

    if opts.validate_only {
        return Ok(());
    }

    if let Some(cmd) = opts.submit_task {
        return submit_task_client(&config, &cmd).await;
    }

    // 1. Initialize mutual TLS configs
    let server_tls =
        CertLoader::mtls_server_config(&config.cert_path, &config.key_path, &config.ca_path)
            .context("Failed to initialize mTLS server configuration")?;

    let client_tls =
        CertLoader::mtls_client_config(&config.cert_path, &config.key_path, &config.ca_path)
            .context("Failed to initialize mTLS client configuration")?;

    let connector = MtlsConnector::new(client_tls);
    let network_factory = NetworkFactory::new(connector);

    // 2. Setup in-memory Raft storage via Adaptor
    let store = InMemoryStore::new(config.self_id);
    let (log_store, state_machine) = Adaptor::new(store);

    // 3. Configure and instantiate OpenRaft node
    let mut raft_cfg = RaftConfig::default();
    raft_cfg.heartbeat_interval = 200;
    raft_cfg.election_timeout_min = 500;
    raft_cfg.election_timeout_max = 800;
    let raft_cfg = Arc::new(
        raft_cfg
            .validate()
            .context("Invalid OpenRaft runtime configuration")?,
    );

    let raft = Arc::new(
        Raft::new(
            config.self_id,
            raft_cfg,
            network_factory,
            log_store,
            state_machine,
        )
        .await
        .context("Failed to construct OpenRaft instance")?,
    );

    // 4. Start role transition metrics monitoring task
    let mut metrics_rx = raft.metrics();
    let self_id = config.self_id;
    tokio::spawn(async move {
        let mut prev_role = None;
        let mut prev_term = 0;

        while metrics_rx.changed().await.is_ok() {
            let metrics = metrics_rx.borrow_and_update();
            let current_role = metrics.state;
            let current_term = metrics.current_term;

            if prev_role != Some(current_role) || prev_term != current_term {
                info!(
                    node_id = self_id,
                    role = ?current_role,
                    term = current_term,
                    leader = ?metrics.current_leader,
                    "Raft role state transition"
                );
                prev_role = Some(current_role);
                prev_term = current_term;
            }
        }
    });

    // 5. Automatic cluster initialization if self_id == 1 or --init passed
    if config.self_id == 1 || opts.force_init {
        let raft_init = raft.clone();
        let init_self_id = config.self_id;
        let mut members = BTreeMap::new();
        members.insert(
            config.self_id,
            BasicNode {
                addr: config.listen_addr.to_string(),
            },
        );
        for peer in &config.peers {
            members.insert(
                peer.id,
                BasicNode {
                    addr: peer.addr.to_string(),
                },
            );
        }

        tokio::spawn(async move {
            // Wait for peers to start their listeners
            sleep(Duration::from_millis(500)).await;
            info!(
                node_id = init_self_id,
                "Initializing 3-node static cluster membership"
            );
            match raft_init.initialize(members).await {
                Ok(_) => {
                    info!(
                        node_id = init_self_id,
                        "Cluster membership initialized successfully"
                    );
                }
                Err(err) => {
                    debug!(
                        node_id = init_self_id,
                        ?err,
                        "Cluster initialization completed or already initialized"
                    );
                }
            }
        });
    }

    // 6. Bind mTLS listener and serve incoming peer RPCs
    let listener = MtlsListener::bind(config.listen_addr, server_tls)
        .await
        .context("Failed to bind mTLS listener")?;

    let raft_serve = raft.clone();
    let listener_task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!(%peer_addr, "Inbound peer connection accepted; handling RPC stream");
                    let raft_conn = raft_serve.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_rpc_stream(stream, raft_conn).await {
                            debug!(%peer_addr, ?e, "Inbound RPC stream ended");
                        }
                    });
                }
                Err(err) => {
                    warn!(?err, "Error accepting inbound mTLS peer connection");
                }
            }
        }
    });

    // Wait for shutdown signal or listener failure
    tokio::select! {
        Err(e) = listener_task => {
            error!(?e, "Listener task terminated unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, exiting node");
        }
    }

    Ok(())
}
