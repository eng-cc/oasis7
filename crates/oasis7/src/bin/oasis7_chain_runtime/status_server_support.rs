use super::*;
use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use tracing::Level;

#[derive(Debug)]
pub(super) struct ChainStatusServer {
    stop_tx: Sender<()>,
    error_rx: Receiver<String>,
    join_handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Default, Serialize)]
pub(super) struct ChainReplicationDebugStatus {
    pub(super) local_peer_id: String,
    pub(super) connected_peers: Vec<String>,
    pub(super) peer_healths: Vec<ChainPeerHealthStatus>,
    pub(super) registered_protocols: Vec<String>,
    pub(super) protocol_retry_cooldown_peers: BTreeMap<String, Vec<String>>,
    pub(super) transport_retry_cooldown_peers: Vec<String>,
    pub(super) recent_errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainPeerHealthStatus {
    pub(super) peer_id: String,
    pub(super) status: String,
    pub(super) issues: Vec<String>,
    pub(super) discovery_sources: Vec<String>,
    pub(super) active_path_kind: Option<String>,
    pub(super) source_operator: Option<String>,
    pub(super) source_asn: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainBalancesResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) node_id: String,
    pub(super) world_id: String,
    pub(super) execution_world_dir: String,
    pub(super) load_error: Option<String>,
    pub(super) node_asset_balance: Option<NodeAssetBalance>,
    pub(super) node_power_credit_balance: u64,
    pub(super) node_main_token_account: Option<String>,
    pub(super) node_main_token_liquid_balance: u64,
    pub(super) reward_mint_record_count: usize,
    pub(super) recent_reward_mint_records: Vec<NodeRewardMintRecord>,
}

pub(super) fn start_chain_status_server(
    host: &str,
    port: u16,
    runtime: Arc<Mutex<NodeRuntime>>,
    replication_network: Arc<Libp2pReplicationNetwork>,
    options: CliOptions,
    node_id: String,
    world_id: String,
    execution_world_dir: PathBuf,
    loaded_network_tier_manifest: Option<LoadedNetworkTierManifest>,
    release_security_policy: ReleaseSecurityPolicy,
    effective_p2p_policy: NodeNetworkPolicy,
    reward_runtime_metrics: SharedRewardRuntimeMetrics,
    storage_metrics: storage_metrics::SharedStorageMetrics,
    feedback_submit_signer: FeedbackSubmitSigner,
) -> Result<ChainStatusServer, String> {
    let listener = TcpListener::bind((host, port))
        .map_err(|err| format!("failed to bind status server at {host}:{port}: {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("failed to set status server listener nonblocking: {err}"))?;

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (error_tx, error_rx) = mpsc::channel::<String>();

    let join_handle = thread::spawn(move || {
        if let Err(err) = run_chain_status_server_loop(
            listener,
            stop_rx,
            runtime,
            replication_network,
            options,
            node_id,
            world_id,
            execution_world_dir,
            loaded_network_tier_manifest,
            release_security_policy,
            effective_p2p_policy,
            reward_runtime_metrics,
            storage_metrics,
            feedback_submit_signer,
        ) {
            let _ = error_tx.send(err);
        }
    });

    Ok(ChainStatusServer {
        stop_tx,
        error_rx,
        join_handle: Some(join_handle),
    })
}

fn run_chain_status_server_loop(
    listener: TcpListener,
    stop_rx: Receiver<()>,
    runtime: Arc<Mutex<NodeRuntime>>,
    replication_network: Arc<Libp2pReplicationNetwork>,
    options: CliOptions,
    node_id: String,
    world_id: String,
    execution_world_dir: PathBuf,
    loaded_network_tier_manifest: Option<LoadedNetworkTierManifest>,
    release_security_policy: ReleaseSecurityPolicy,
    effective_p2p_policy: NodeNetworkPolicy,
    reward_runtime_metrics: SharedRewardRuntimeMetrics,
    storage_metrics: storage_metrics::SharedStorageMetrics,
    feedback_submit_signer: FeedbackSubmitSigner,
) -> Result<(), String> {
    loop {
        match stop_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        match listener.accept() {
            Ok((stream, _addr)) => {
                let runtime = Arc::clone(&runtime);
                let replication_network = Arc::clone(&replication_network);
                let options = options.clone();
                let node_id = node_id.clone();
                let world_id = world_id.clone();
                let execution_world_dir = execution_world_dir.clone();
                let loaded_network_tier_manifest = loaded_network_tier_manifest.clone();
                let release_security_policy = release_security_policy.clone();
                let effective_p2p_policy = effective_p2p_policy.clone();
                let reward_runtime_metrics = Arc::clone(&reward_runtime_metrics);
                let storage_metrics = Arc::clone(&storage_metrics);
                let feedback_submit_signer = feedback_submit_signer.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_chain_status_connection(
                        stream,
                        runtime,
                        replication_network,
                        &options,
                        node_id.as_str(),
                        world_id.as_str(),
                        execution_world_dir.as_path(),
                        loaded_network_tier_manifest.as_ref(),
                        &release_security_policy,
                        effective_p2p_policy,
                        reward_runtime_metrics,
                        storage_metrics,
                        &feedback_submit_signer,
                    ) {
                        let stderr_message =
                            format!("warning: chain status connection failed: {err}");
                        oasis7::observability::emit_stderr_or_event(
                            Level::WARN,
                            stderr_message.as_str(),
                            "chain status connection failed",
                        );
                    }
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(err) => {
                return Err(format!("chain status server accept failed: {err}"));
            }
        }
    }
}

fn handle_chain_status_connection(
    mut stream: TcpStream,
    runtime: Arc<Mutex<NodeRuntime>>,
    replication_network: Arc<Libp2pReplicationNetwork>,
    options: &CliOptions,
    node_id: &str,
    world_id: &str,
    execution_world_dir: &Path,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
    release_security_policy: &ReleaseSecurityPolicy,
    effective_p2p_policy: NodeNetworkPolicy,
    reward_runtime_metrics: SharedRewardRuntimeMetrics,
    storage_metrics: storage_metrics::SharedStorageMetrics,
    feedback_submit_signer: &FeedbackSubmitSigner,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| format!("failed to set read timeout: {err}"))?;

    let mut buffer = [0_u8; 65_536];
    let bytes = stream
        .read(&mut buffer)
        .map_err(|err| format!("failed to read request: {err}"))?;
    if bytes == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let Some(line) = request.lines().next() else {
        write_json_response(&mut stream, 400, b"{\"error\":\"bad request\"}", false)
            .map_err(|err| format!("failed to write 400 response: {err}"))?;
        return Ok(());
    };

    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    let path = target.split('?').next().unwrap_or(target);
    let head_only = method.eq_ignore_ascii_case("HEAD");

    if transfer_submit_api::maybe_handle_transfer_submit_request(
        &mut stream,
        &buffer[..bytes],
        &runtime,
        method,
        path,
        node_id,
        world_id,
        execution_world_dir,
    )? {
        return Ok(());
    }

    if agent_claim_api::maybe_handle_agent_claim_request(
        &mut stream,
        &buffer[..bytes],
        &runtime,
        method,
        target,
        path,
        node_id,
        world_id,
        execution_world_dir,
    )? {
        return Ok(());
    }

    if gameplay_submit_api::maybe_handle_gameplay_submit_request(
        &mut stream,
        &buffer[..bytes],
        &runtime,
        method,
        path,
    )? {
        return Ok(());
    }

    if module_release_attestation_submit_api::maybe_handle_module_release_attestation_submit_request(
        &mut stream,
        &buffer[..bytes],
        &runtime,
        method,
        path,
    )? {
        return Ok(());
    }

    if method.eq_ignore_ascii_case("POST") && path == "/v1/chain/feedback/submit" {
        let body = match extract_http_json_body(&buffer[..bytes]) {
            Ok(body) => body,
            Err(err) => {
                write_feedback_submit_error(&mut stream, 400, err.as_str())?;
                return Ok(());
            }
        };
        let submit_request = match parse_feedback_submit_request(body) {
            Ok(request) => request,
            Err(err) => {
                write_feedback_submit_error(&mut stream, 400, err.as_str())?;
                return Ok(());
            }
        };
        let submit_ip = stream
            .peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let create_request = match build_feedback_create_request(
            submit_request,
            feedback_submit_signer,
            node_id,
            submit_ip.as_str(),
            now_unix_ms(),
        ) {
            Ok(request) => request,
            Err(err) => {
                write_feedback_submit_error(&mut stream, 400, err.as_str())?;
                return Ok(());
            }
        };
        let receipt = match runtime
            .lock()
            .map_err(|_| "failed to lock node runtime for feedback submit".to_string())?
            .submit_feedback(create_request)
        {
            Ok(receipt) => receipt,
            Err(err) => {
                write_feedback_submit_error(
                    &mut stream,
                    502,
                    format!("feedback submit failed: {err}").as_str(),
                )?;
                return Ok(());
            }
        };
        let response = ChainFeedbackSubmitResponse::success(&receipt);
        let body = serde_json::to_vec_pretty(&response)
            .map_err(|err| format!("failed to encode feedback submit response: {err}"))?;
        write_json_response(&mut stream, 200, body.as_slice(), false)
            .map_err(|err| format!("failed to write /v1/chain/feedback/submit response: {err}"))?;
        return Ok(());
    }

    if !method.eq_ignore_ascii_case("GET") && !head_only {
        write_json_response(
            &mut stream,
            405,
            b"{\"error\":\"method not allowed\"}",
            head_only,
        )
        .map_err(|err| format!("failed to write 405 response: {err}"))?;
        return Ok(());
    }

    match path {
        "/healthz" => {
            write_json_response(&mut stream, 200, b"{\"ok\":true}", head_only)
                .map_err(|err| format!("failed to write /healthz response: {err}"))?;
        }
        "/v1/chain/status" => {
            let (snapshot, udp_gossip_traffic) = runtime
                .lock()
                .map_err(|_| "failed to read node runtime snapshot: lock poisoned".to_string())
                .map(|locked| (locked.snapshot(), locked.gossip_traffic_snapshot()))?;
            let live_snapshot = replication_network.reachability_snapshot();
            let (p2p_recommendation, p2p_detection) =
                build_live_node_network_policy_recommendation(options, Some(&live_snapshot))?;
            let applied_effective_user_mode =
                applied_runtime_user_mode_label(options).map(str::to_string);
            let replication_debug_status =
                build_chain_replication_debug_status(replication_network.as_ref());
            let transactions = transfer_submit_api::build_chain_transfer_metrics_status(&runtime)?;
            let payload = build_chain_status_payload(
                snapshot,
                execution_world_dir,
                loaded_network_tier_manifest,
                &p2p_recommendation,
                applied_effective_user_mode,
                effective_p2p_policy,
                &live_snapshot,
                p2p_detection,
                release_security_policy.clone(),
                snapshot_metrics(&reward_runtime_metrics),
                storage_metrics::snapshot_storage_metrics(&storage_metrics),
                build_chain_wasm_status(),
                build_chain_traffic_status(replication_network.as_ref(), udp_gossip_traffic),
                transactions,
                replication_debug_status,
            );
            let body = serde_json::to_vec_pretty(&payload)
                .map_err(|err| format!("failed to encode status payload: {err}"))?;
            write_json_response(&mut stream, 200, body.as_slice(), head_only)
                .map_err(|err| format!("failed to write /v1/chain/status response: {err}"))?;
        }
        "/v1/chain/balances" => {
            let payload = build_chain_balances_payload(node_id, world_id, execution_world_dir);
            let body = serde_json::to_vec_pretty(&payload)
                .map_err(|err| format!("failed to encode balances payload: {err}"))?;
            write_json_response(&mut stream, 200, body.as_slice(), head_only)
                .map_err(|err| format!("failed to write /v1/chain/balances response: {err}"))?;
        }
        _ => {
            write_json_response(&mut stream, 404, b"{\"error\":\"not found\"}", head_only)
                .map_err(|err| format!("failed to write 404 response: {err}"))?;
        }
    }

    Ok(())
}

pub(super) fn build_chain_replication_debug_status(
    replication_network: &Libp2pReplicationNetwork,
) -> ChainReplicationDebugStatus {
    let debug_snapshot = replication_network.debug_snapshot();
    let peer_healths: Vec<ChainPeerHealthStatus> = debug_snapshot
        .peer_healths
        .into_iter()
        .map(|health| ChainPeerHealthStatus {
            peer_id: health.peer_id,
            status: health.status,
            issues: health.issues,
            discovery_sources: health.discovery_sources,
            active_path_kind: health.active_path_kind,
            source_operator: health.source_operator,
            source_asn: health.source_asn,
        })
        .collect();

    let mut protocol_retry_cooldown_peers: BTreeMap<String, Vec<String>> = debug_snapshot
        .protocol_retry_cooldown_peers
        .into_iter()
        .collect();
    for peer_ids in protocol_retry_cooldown_peers.values_mut() {
        peer_ids.sort();
        peer_ids.dedup();
    }
    let mut transport_retry_cooldown_peers = debug_snapshot.transport_retry_cooldown_peers;
    transport_retry_cooldown_peers.sort();
    transport_retry_cooldown_peers.dedup();

    ChainReplicationDebugStatus {
        local_peer_id: debug_snapshot.local_peer_id,
        connected_peers: debug_snapshot.connected_peers,
        peer_healths,
        registered_protocols: debug_snapshot.registered_protocols,
        protocol_retry_cooldown_peers,
        transport_retry_cooldown_peers,
        recent_errors: debug_snapshot.recent_errors,
    }
}

pub(super) fn poll_chain_status_server_error(
    server: &mut ChainStatusServer,
) -> Result<Option<String>, String> {
    match server.error_rx.try_recv() {
        Ok(err) => Ok(Some(format!("status server failed: {err}"))),
        Err(TryRecvError::Disconnected) => Ok(Some(
            "status server channel disconnected unexpectedly".to_string(),
        )),
        Err(TryRecvError::Empty) => {
            if let Some(handle) = server.join_handle.as_ref() {
                if handle.is_finished() {
                    return Ok(Some("status server exited unexpectedly".to_string()));
                }
            }
            Ok(None)
        }
    }
}

pub(super) fn stop_chain_status_server(server: &mut ChainStatusServer) {
    let _ = server.stop_tx.send(());
    if let Some(handle) = server.join_handle.take() {
        let _ = handle.join();
    }
}

pub(super) fn write_json_response(
    stream: &mut TcpStream,
    status_code: u16,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        502 => "Bad Gateway",
        _ => "Internal Server Error",
    };
    let headers = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}
