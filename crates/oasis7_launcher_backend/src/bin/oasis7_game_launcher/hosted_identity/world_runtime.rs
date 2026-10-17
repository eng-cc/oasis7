//! Bounded loopback-only fencing queries and issuer-authorized release RPC.
use super::*;
use oasis7::viewer::{
    AuthoritativeReconnectSyncRequest, AuthoritativeRecoveryCommand, HostedSessionReleaseRequest,
    VIEWER_PROTOCOL_VERSION, ViewerRequest,
};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct RuntimeFence {
    pub key: Option<String>,
    pub epoch: u64,
}

pub(super) fn query(
    live_bind: &str,
    world_id: &str,
    player: &str,
) -> Result<RuntimeFence, StoreError> {
    let ack = rpc(
        live_bind,
        world_id,
        AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: player.into(),
                session_pubkey: None,
                last_known_log_cursor: None,
                expected_reorg_epoch: None,
            },
        },
    )?;
    if ack["player_id"].as_str() != Some(player) {
        return Err(reject("runtime_fence_mismatch"));
    }
    Ok(RuntimeFence {
        key: ack["session_pubkey"].as_str().map(str::to_owned),
        epoch: ack["session_epoch"]
            .as_u64()
            .ok_or_else(|| reject("runtime_fence_unavailable"))?,
    })
}

pub(super) fn release(
    live_bind: &str,
    world_id: &str,
    grant: String,
    player: &str,
    expected_epoch: u64,
) -> Result<(), StoreError> {
    let ack = rpc(
        live_bind,
        world_id,
        AuthoritativeRecoveryCommand::ReleaseHostedSession {
            request: HostedSessionReleaseRequest {
                authorization_grant: grant,
            },
        },
    )?;
    if ack["player_id"].as_str() != Some(player)
        || ack["status"].as_str() != Some("session_revoked")
        || ack["session_epoch"].as_u64() != expected_epoch.checked_add(1)
    {
        return Err(reject("runtime_fence_mismatch"));
    }
    Ok(())
}

fn rpc(
    live_bind: &str,
    world_id: &str,
    command: AuthoritativeRecoveryCommand,
) -> Result<Value, StoreError> {
    let unavailable = || StoreError::Unavailable("runtime control unavailable".into());
    let addr: SocketAddr = live_bind.parse().map_err(|_| unavailable())?;
    if !addr.ip().is_loopback() {
        return Err(reject("runtime_control_not_loopback"));
    }
    let timeout = Duration::from_secs(2);
    let deadline = Instant::now() + timeout;
    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(|_| unavailable())?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|_| unavailable())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|_| unavailable())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|_| unavailable())?);
    fn write(stream: &mut TcpStream, request: &ViewerRequest) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(request).map_err(|_| StoreError::Corrupt)?;
        stream
            .write_all(&bytes)
            .and_then(|_| stream.write_all(b"\n"))
            .map_err(|_| StoreError::Unavailable("runtime request unavailable".into()))
    }
    write(
        &mut stream,
        &ViewerRequest::Hello {
            client: "hosted_identity_v2_world_adapter".into(),
            version: VIEWER_PROTOCOL_VERSION,
        },
    )?;
    let mut greeted = false;
    for _ in 0..32 {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(&unavailable)?;
        reader
            .get_ref()
            .set_read_timeout(Some(remaining))
            .map_err(|_| unavailable())?;
        let mut line = String::new();
        (&mut reader)
            .take(2_097_153)
            .read_line(&mut line)
            .map_err(|_| unavailable())?;
        if line.is_empty() || line.len() > 2_097_152 || !line.ends_with('\n') {
            return Err(unavailable());
        }
        let response: Value = serde_json::from_str(&line).map_err(|_| unavailable())?;
        match response["type"].as_str() {
            Some("hello_ack") if !greeted => {
                if response["world_id"].as_str() != Some(world_id) {
                    return Err(reject("runtime_world_mismatch"));
                }
                greeted = true;
                write(
                    &mut stream,
                    &ViewerRequest::AuthoritativeRecovery {
                        command: command.clone(),
                    },
                )?;
            }
            Some("authoritative_recovery_ack") if greeted => return Ok(response["ack"].clone()),
            Some("authoritative_recovery_error") if greeted => {
                return Err(reject("runtime_state_changed"));
            }
            _ => {}
        }
    }
    Err(unavailable())
}
