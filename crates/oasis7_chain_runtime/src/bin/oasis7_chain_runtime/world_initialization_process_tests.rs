//! Shipping world-init CLI, externally signed approval and real protected P/R.
//! Custody oracle is a controlled fixture; no deployment/physical-fencing claim.
use super::*;
use ed25519_dalek::Signer;
use oasis7_distfs::controlled_authority::formal::{
    coordinator::FormalDurabilityEndpoint,
    endpoint::{FormalQueryOutcomeV1, FormalRequestStateV1},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    os::unix::process::ExitStatusExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(serde::Deserialize)]
struct PublicPreparedRequest {
    world: FormalWorldIdentityV1,
    activation_body: InitialActivationBodyV1,
    request_digest: String,
    trusted_issuer_public_key: String,
}
struct CliChild(Child);
impl Drop for CliChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start_cli(binary: &Path, args: &[&str]) -> Result<CliChild, String> {
    Ok(CliChild(
        Command::new(binary)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?,
    ))
}
fn finish_cli(mut child: CliChild, success: bool) -> Result<Vec<u8>, String> {
    let deadline = Instant::now() + Duration::from_secs(40);
    while child.0.try_wait().map_err(|e| e.to_string())?.is_none() {
        if Instant::now() >= deadline {
            return Err("world-init shipping CLI deadline".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    let mut stdout = Vec::new();
    child
        .0
        .stdout
        .take()
        .ok_or("world-init stdout missing")?
        .read_to_end(&mut stdout)
        .map_err(|e| e.to_string())?;
    let mut stderr = Vec::new();
    child
        .0
        .stderr
        .take()
        .ok_or("world-init stderr missing")?
        .read_to_end(&mut stderr)
        .map_err(|e| e.to_string())?;
    let status = child.0.wait().map_err(|e| e.to_string())?;
    if status.success() != success {
        return Err(format!(
            "world-init expected success={success}: {status}: {}",
            String::from_utf8_lossy(&stderr)
        ));
    }
    if !success {
        // The shipping tracing subscriber may write diagnostics to stdout.
        // Keep both streams when asserting the explicit refusal cause.
        stdout.extend_from_slice(&stderr);
    }
    Ok(stdout)
}
fn p(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "world-init non UTF8 fixture path".into())
}
fn execute(binary: &Path, args: &[&str], success: bool) -> Result<Vec<u8>, String> {
    finish_cli(start_cli(binary, args)?, success)
}
fn approve(
    request: &Path,
    approval: &Path,
) -> Result<
    (
        PublicPreparedRequest,
        SignedInitialActivationV1,
        InitialActivationTrustV1,
    ),
    String,
> {
    let bytes = std::fs::read(request).map_err(|e| e.to_string())?;
    let prepared: PublicPreparedRequest =
        serde_cbor::from_slice(&bytes).map_err(|e| e.to_string())?;
    if prepared.trusted_issuer_public_key != fixtures::public(9)
        || prepared.world != prepared.activation_body.authority.world
        || prepared.activation_body.authority.profile_version != 2
    {
        return Err("world-init prepared request differs from independently provisioned issuer/world contract".into());
    }
    let certificate = SignedInitialActivationV1 {
        signature_hex: hex::encode(
            fixtures::key(9)
                .sign(
                    &initial_activation_signing_bytes(&prepared.activation_body)
                        .map_err(|e| e.to_string())?,
                )
                .to_bytes(),
        ),
        body: prepared.activation_body.clone(),
    };
    let trust = InitialActivationTrustV1 {
        issuer_public_key: fixtures::public(9),
        world: prepared.world.clone(),
        initial_state_root: certificate.body.initial_state_root.clone(),
        governing_manifest_digest: certificate.body.governing_manifest_digest.clone(),
        recovery_policy_digest: certificate
            .body
            .recovery_policy
            .digest()
            .map_err(|e| e.to_string())?,
    };
    verify_initial_activation(&certificate, &trust).map_err(|e| e.to_string())?;
    oracle_support::write_private(
        approval,
        &serde_json::to_vec(&serde_json::json!({"certificate":certificate}))
            .map_err(|e| e.to_string())?,
    )?;
    Ok((prepared, certificate, trust))
}
fn activation_args<'a>(
    config: &'a Path,
    request: &'a Path,
    approval: &'a Path,
    key: &'a Path,
    primary: &'a str,
    replica: &'a str,
    output: &'a Path,
) -> Result<Vec<&'a str>, String> {
    Ok(vec![
        "world",
        "init",
        "activate",
        "--config",
        p(config)?,
        "--request",
        p(request)?,
        "--certificate",
        p(approval)?,
        "--writer-key",
        p(key)?,
        "--primary",
        primary,
        "--replica",
        replica,
        "--timeout-ms",
        "30000",
        "--output",
        p(output)?,
    ])
}
fn run_initialization() -> Result<(), String> {
    let binary = PathBuf::from(
        std::env::var_os("OASIS7_FORMAL_NODE_TEST_BIN")
            .ok_or("set shipping OASIS7_FORMAL_NODE_TEST_BIN")?,
    );
    let endpoint_binary = std::env::var_os("OASIS7_FORMAL_ENDPOINT_TEST_BIN")
        .ok_or("set OASIS7_FORMAL_ENDPOINT_TEST_BIN")?;
    let root_guard = oracle_support::PrivateRoot::new()?;
    let root = root_guard.path();
    let pdir = root.join("p");
    let rdir = root.join("r");
    oracle_support::private_directory(&pdir)?;
    oracle_support::private_directory(&rdir)?;
    let (mut world, _trust, seed, _proof) = fixtures::actual_world_initial_fixture();
    world.initialize_empty_formal_history()?;
    let identity = seed.authority().definition().world.clone();
    let mut definition = seed.authority().definition().clone();
    definition.profile_version = 2;
    definition.primary.instance.storage_fingerprint =
        directory_fingerprint(&pdir).map_err(|e| e.to_string())?;
    definition.replica.instance.storage_fingerprint =
        directory_fingerprint(&rdir).map_err(|e| e.to_string())?;
    let snapshot = serde_cbor::to_vec(&world.snapshot()).map_err(|e| e.to_string())?;
    let journal = serde_cbor::to_vec(world.journal()).map_err(|e| e.to_string())?;
    let snapshot_path = root.join("source-snapshot.cbor");
    let journal_path = root.join("source-journal.cbor");
    oracle_support::write_private(&snapshot_path, &snapshot)?;
    oracle_support::write_private(&journal_path, &journal)?;
    let config = root.join("world-init.json");
    let mut config_value = serde_json::json!({"schema_version":2,"mode":"genesis","world":identity,
        "snapshot_file":snapshot_path,"journal_file":journal_path,"trusted_issuer_public_key":fixtures::public(9),
        "node_bootstrap":{"height":0,"consensus_block_hash":fixtures::h(81),"execution_block_hash":fixtures::h(82),"execution_state_root":blake3_hex(&snapshot)},
        "authority":definition,"recovery_policy":seed.authority().policy()});
    oracle_support::write_private(
        &config,
        &serde_json::to_vec(&config_value).map_err(|e| e.to_string())?,
    )?;
    let request = root.join("original-request.cbor");
    let approval = root.join("original-approval.json");
    let output = root.join("original-qualified.cbor");
    let writer_key = root.join("writer-key");
    oracle_support::write_private(
        &writer_key,
        hex::encode(fixtures::key(4).to_bytes()).as_bytes(),
    )?;
    let prepare_args = [
        "world",
        "init",
        "prepare",
        "--config",
        p(&config)?,
        "--request",
        p(&request)?,
    ];
    if !cfg!(target_os = "linux") {
        let error = execute(&binary, &prepare_args, false)?;
        if !String::from_utf8_lossy(&error).contains("Unavailable") || request.exists() {
            return Err(format!(
                "unsupported native platform did not reject preparation without creating a request (request_exists={}): {}",
                request.exists(),
                String::from_utf8_lossy(&error),
            ));
        }
        return Ok(());
    }
    execute(&binary, &prepare_args, true)?;
    let request_bytes = std::fs::read(&request).map_err(|e| e.to_string())?;
    execute(&binary, &prepare_args, true)?;
    if std::fs::read(&request).map_err(|e| e.to_string())? != request_bytes {
        return Err("world-init repeated prepare changed original request".into());
    }
    let (prepared, certificate, trust) = approve(&request, &approval)?;
    if prepared.world != seed.authority().definition().world {
        return Err("world-init prepared new identity".into());
    }
    let quote = execute(
        &binary,
        &[
            "world",
            "init",
            "quote",
            "--config",
            p(&config)?,
            "--request",
            p(&request)?,
            "--certificate",
            p(&approval)?,
        ],
        true,
    )?;
    let quote: Value = serde_json::from_slice(&quote).map_err(|e| e.to_string())?;
    if quote["request_digest"] != prepared.request_digest
        || quote["world"] != serde_json::to_value(&prepared.world).map_err(|e| e.to_string())?
    {
        return Err("world-init quote did not bind original externally approved request".into());
    }
    let authority = verify_initial_activation(&certificate, &trust).map_err(|e| e.to_string())?;
    let oracle_fixture = oracle_support::Fixture::new(
        root.to_path_buf(),
        pdir.clone(),
        rdir.clone(),
        authority.clone(),
    );
    let p_oracle = oracle_support::start_oracle(&oracle_fixture, EndpointRole::Primary);
    let r_oracle = oracle_support::start_oracle(&oracle_fixture, EndpointRole::Replica);
    let oracle_auth = root.join("oracle-auth");
    oracle_support::write_private(&oracle_auth, hex::encode([17; 32]).as_bytes())?;
    let pcfg = root.join("p-endpoint.json");
    let rcfg = root.join("r-endpoint.json");
    write_endpoint_config(
        &pcfg,
        &pdir,
        &oracle_auth,
        p_oracle.address(),
        EndpointRole::Primary,
        &trust,
        &certificate,
    )?;
    write_endpoint_config(
        &rcfg,
        &rdir,
        &oracle_auth,
        r_oracle.address(),
        EndpointRole::Replica,
        &trust,
        &certificate,
    )?;
    let (_pchild, paddr) = oracle_support::start_endpoint(&endpoint_binary, &pcfg)?;
    let (_rchild, raddr) = oracle_support::start_endpoint(&endpoint_binary, &rcfg)?;
    let gate = ReplicaPrepareGate::start(raddr)?;
    let paddr_text = paddr.to_string();
    let gate_text = gate.address.to_string();
    let raddr_text = raddr.to_string();
    let mut interrupted = start_cli(
        &binary,
        &activation_args(
            &config,
            &request,
            &approval,
            &writer_key,
            &paddr_text,
            &gate_text,
            &output,
        )?,
    )?;
    // Native activation first transfers a bounded original image closure. The
    // fault is at Prepare, so bound inactivity during that progressing transfer
    // rather than treating its total bytes as an idle Prepare wait. The suite's
    // process deadline still bounds the whole test.
    while !gate.blocked.load(Ordering::Acquire) {
        if let Some(status) = interrupted.0.try_wait().map_err(|e| e.to_string())? {
            let diagnostics = finish_cli(interrupted, status.success())?;
            return Err(format!(
                "world-init exited before durable boundary: {status}: {}",
                String::from_utf8_lossy(&diagnostics),
            ));
        }
        if let Some(error) = gate.error.lock().map_err(|_| "gate error lock")?.clone() {
            return Err(error);
        }
        if gate
            .last_progress
            .lock()
            .map_err(|_| "gate progress lock")?
            .elapsed()
            >= Duration::from_secs(30)
        {
            return Err(
                "world-init made no RPC progress toward replica Prepare for 30 seconds".into(),
            );
        }
        thread::sleep(Duration::from_millis(5));
    }
    interrupted.0.kill().map_err(|e| e.to_string())?;
    let killed = interrupted.0.wait().map_err(|e| e.to_string())?;
    if killed.signal() != Some(9) || output.exists() {
        return Err("world-init interruption did not preserve unknown/unpublished result".into());
    }
    drop(gate);
    let genesis = authority
        .definition()
        .world
        .genesis_head()
        .map_err(|e| e.to_string())?;
    let mut pendpoint = RemoteFormalEndpoint::new(
        paddr,
        Duration::from_secs(30),
        authority.clone(),
        EndpointRole::Primary,
        fixtures::key(4),
        genesis.clone(),
    )
    .map_err(|e| e.to_string())?;
    let mut rendpoint = RemoteFormalEndpoint::new(
        raddr,
        Duration::from_secs(30),
        authority.clone(),
        EndpointRole::Replica,
        fixtures::key(4),
        genesis.clone(),
    )
    .map_err(|e| e.to_string())?;
    if !matches!(
        pendpoint
            .query(&prepared.request_digest)
            .map_err(|e| e.to_string())?,
        FormalQueryOutcomeV1::Recorded(state) if matches!(*state, FormalRequestStateV1::Prepared { .. })
    ) || !matches!(
        rendpoint
            .query(&prepared.request_digest)
            .map_err(|e| e.to_string())?,
        FormalQueryOutcomeV1::NotRecorded
    ) {
        return Err("world-init did not retain exactly primary durable Prepare".into());
    }
    let args = activation_args(
        &config,
        &request,
        &approval,
        &writer_key,
        &paddr_text,
        &raddr_text,
        &output,
    )?;
    execute(&binary, &args, true)?;
    let result = std::fs::read(&output).map_err(|e| e.to_string())?;
    let publication: crate::formal_live_config::ApprovedInitialPublicationV1 =
        serde_cbor::from_slice(&result).map_err(|e| e.to_string())?;
    verify_formal_qualified(&publication.proof, &authority, None, None)
        .map_err(|e| e.to_string())?;
    let record = &publication.proof.decision.record;
    if record.position != 1
        || record.world != prepared.world
        || record.parent_record_id != genesis.record_id
        || record.request_digest != prepared.request_digest
    {
        return Err("world-init resumed another initial identity/request".into());
    }
    execute(&binary, &args, true)?;
    if std::fs::read(&output).map_err(|e| e.to_string())? != result {
        return Err("world-init duplicate activate changed original qualified result".into());
    }
    let head = pendpoint.head().map_err(|e| e.to_string())?;
    if head.position != 1 || head != rendpoint.head().map_err(|e| e.to_string())? {
        return Err("world-init repeated activation created another head".into());
    }
    // The issuer may approve an alternate bootstrap; endpoint reconciliation
    // must still refuse a conflicting Initial at the same World/position.
    let conflict_config = root.join("conflict-config.json");
    config_value["node_bootstrap"]["consensus_block_hash"] = Value::String(fixtures::h(99));
    oracle_support::write_private(
        &conflict_config,
        &serde_json::to_vec(&config_value).map_err(|e| e.to_string())?,
    )?;
    let conflict_request = root.join("conflict-request.cbor");
    let conflict_approval = root.join("conflict-approval.json");
    let conflict_output = root.join("conflict-result.cbor");
    execute(
        &binary,
        &[
            "world",
            "init",
            "prepare",
            "--config",
            p(&conflict_config)?,
            "--request",
            p(&conflict_request)?,
        ],
        true,
    )?;
    let (conflict, _, _) = approve(&conflict_request, &conflict_approval)?;
    if conflict.request_digest == prepared.request_digest {
        return Err("world-init conflict fixture did not alter original obligation".into());
    }
    execute(
        &binary,
        &activation_args(
            &conflict_config,
            &conflict_request,
            &conflict_approval,
            &writer_key,
            &paddr_text,
            &raddr_text,
            &conflict_output,
        )?,
        false,
    )?;
    if conflict_output.exists()
        || pendpoint.head().map_err(|e| e.to_string())? != head
        || rendpoint.head().map_err(|e| e.to_string())? != head
        || std::fs::read(&snapshot_path).map_err(|e| e.to_string())? != snapshot
        || std::fs::read(&journal_path).map_err(|e| e.to_string())? != journal
    {
        return Err("world-init conflict changed original qualified head or source World".into());
    }
    Ok(())
}

struct ReplicaPrepareGate {
    address: SocketAddr,
    blocked: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    error: Arc<std::sync::Mutex<Option<String>>>,
    last_progress: Arc<std::sync::Mutex<Instant>>,
    worker: Option<thread::JoinHandle<()>>,
}
impl ReplicaPrepareGate {
    fn start(target: SocketAddr) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let blocked = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let error = Arc::new(std::sync::Mutex::new(None));
        let last_progress = Arc::new(std::sync::Mutex::new(Instant::now()));
        let progress = last_progress.clone();
        let (b, s, e) = (blocked.clone(), stop.clone(), error.clone());
        let worker = thread::spawn(move || {
            while !s.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut incoming, _)) => {
                        let outcome = (|| -> Result<(), String> {
                            incoming.set_nonblocking(false).map_err(|e| e.to_string())?;
                            incoming
                                .set_read_timeout(Some(Duration::from_secs(5)))
                                .map_err(|e| e.to_string())?;
                            let frame = read_frame(&mut incoming)?;
                            let probe: CommandProbe =
                                serde_cbor::from_slice(&frame).map_err(|e| e.to_string())?;
                            if probe.body.command.0 {
                                b.store(true, Ordering::Release);
                                while !s.load(Ordering::Acquire) {
                                    thread::sleep(Duration::from_millis(5));
                                }
                                return Ok(());
                            }
                            let mut outgoing =
                                TcpStream::connect_timeout(&target, Duration::from_secs(5))
                                    .map_err(|e| e.to_string())?;
                            outgoing
                                .set_read_timeout(Some(Duration::from_secs(5)))
                                .map_err(|e| e.to_string())?;
                            write_frame(&mut outgoing, &frame)?;
                            let response = read_frame(&mut outgoing)?;
                            write_frame(&mut incoming, &response)?;
                            *progress.lock().map_err(|_| "gate progress lock")? = Instant::now();
                            Ok(())
                        })();
                        if let Err(reason) = outcome {
                            *e.lock().unwrap() = Some(reason);
                            break;
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(err) => {
                        *e.lock().unwrap() = Some(err.to_string());
                        break;
                    }
                }
            }
        });
        Ok(Self {
            address,
            blocked,
            stop,
            error,
            last_progress,
            worker: Some(worker),
        })
    }
}
impl Drop for ReplicaPrepareGate {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[derive(serde::Deserialize)]
struct CommandProbe {
    body: CommandBodyProbe,
}
#[derive(serde::Deserialize)]
struct CommandBodyProbe {
    command: IsPrepare,
}
struct IsPrepare(bool);
impl<'de> serde::Deserialize<'de> for IsPrepare {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = IsPrepare;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("signed RPC command")
            }
            fn visit_enum<A: serde::de::EnumAccess<'de>>(
                self,
                a: A,
            ) -> Result<IsPrepare, A::Error> {
                use serde::de::VariantAccess;
                let (name, variant) = a.variant::<String>()?;
                if name == "Head" || name == "ArchiveManifest" {
                    variant.unit_variant()?;
                } else {
                    variant.newtype_variant::<serde::de::IgnoredAny>()?;
                }
                Ok(IsPrepare(name == "Prepare"))
            }
        }
        d.deserialize_enum("Command", &[], Visitor)
    }
}
fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut size = [0; 4];
    stream.read_exact(&mut size).map_err(|e| e.to_string())?;
    let size = u32::from_be_bytes(size) as usize;
    if size == 0 || size > 64 * 1024 * 1024 {
        return Err("world-init gate RPC frame bound".into());
    }
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}
fn write_frame(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), String> {
    let size = u32::try_from(bytes.len()).map_err(|e| e.to_string())?;
    stream
        .write_all(&size.to_be_bytes())
        .and_then(|_| stream.write_all(bytes))
        .map_err(|e| e.to_string())
}
#[test]
#[ignore = "requires shipping runtime + protected endpoint binaries; real CLI SIGKILL activation"]
fn formal_world_initialization_cli_native_platform_and_original_activation() {
    thread::Builder::new()
        .name("formal-world-init-cli".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(run_initialization)
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
}
