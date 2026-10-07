use std::{net::TcpStream, path::Path, sync::{Arc, Mutex}};
use oasis7::world_service::*;
use oasis7_node::NodeRuntime;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use super::{execution_bridge::world_service_read::{self, PinnedWorld}, feedback_submit_api::FeedbackSubmitSigner};

#[path = "world_service_operations.rs"]
mod operations;

pub(super) struct Service<'a> {
    records: &'a Path, storage: &'a Path,
    identity: WorldIdentity, signer: &'a FeedbackSubmitSigner,
    runtime: &'a Arc<Mutex<NodeRuntime>>,
}
impl Service<'_> {
    fn pin(&self, fixed: Option<&CommitRef>) -> Result<PinnedWorld, String> {
        world_service_read::pin(self.records, self.storage, &self.identity, fixed)
    }
    fn signed<Q: Serialize, T: Serialize>(&self, path: &str, request: &Q, payload: T) -> Result<Value, String> {
        serde_json::to_value(sign_service_response(path, request_digest(path, request)?, payload, &self.signer.private_key_hex)?)
            .map_err(|e| e.to_string())
    }
    fn scope(&self, pinned: &PinnedWorld, scope: &str, key: &str) -> Result<Option<String>, String> {
        if scope == "public" { return Ok(None); }
        let agent = scope.strip_prefix("agent:").ok_or("unsupported visibility scope")?;
        let generation = if agent_authority::owner_public_key(&pinned.world, agent)? == key { 0 }
            else { pinned.world.capability_revocation_state().agent_signer_delegations.get(agent)
                .ok_or("scope delegation unavailable")?.generation };
        agent_authority::validate_agent_signer(&pinned.world, agent, key, generation)?;
        Ok(Some(agent.into()))
    }
    fn check_payload(&self, pinned: &PinnedWorld, payload: &WorldServicePayloadV1) -> Result<(), String> {
        match payload {
            WorldServicePayloadV1::GameplayJson(bytes) => { gameplay::authenticated_action(&pinned.world, bytes)?; }
            WorldServicePayloadV1::Cognition(signed) => agent_authority::validate_cognition(&pinned.world, signed)?,
            WorldServicePayloadV1::Delegation(signed) => agent_authority::validate_delegation(&pinned.world, signed)?,
            WorldServicePayloadV1::Scheduler(signed) => {
                verify_read_request("scheduler", signed)?;
                agent_authority::validate_agent_signer(&pinned.world, &signed.request.agent_id,
                    &signed.subject_public_key, signed.request.delegation_generation)?;
            }
        }
        Ok(())
    }
}

fn parse<T: DeserializeOwned>(body: &[u8]) -> Result<T, String> {
    if body.len() > 256 * 1024 { return Err("service request exceeds byte bound".into()); }
    serde_json::from_slice(body).map_err(|e| e.to_string())
}

#[expect(clippy::too_many_arguments, reason = "Existing status dispatcher carries explicit authority and storage roots.")]
pub(super) fn maybe_handle(stream: &mut TcpStream, bytes: &[u8], runtime: &Arc<Mutex<NodeRuntime>>,
    method: &str, path: &str, world_id: &str, world_dir: &Path, records: &Path, storage: &Path,
    signer: &FeedbackSubmitSigner) -> Result<bool, String>
{
    if ![DESCRIBE_PATH, SUBMIT_PATH, LOOKUP_PATH, VIEW_PATH, CHANGES_PATH].contains(&path) { return Ok(false); }
    let result = (|| {
        if method != "POST" { return Err("world service requires POST".into()); }
        let body = super::feedback_submit_api::extract_http_json_body(bytes)?;
        let identity = world_service_read::identity(world_dir, world_id)?;
        let service = Service { records, storage, identity, signer, runtime };
        match path {
            DESCRIBE_PATH => operations::describe(&service, parse(body)?),
            SUBMIT_PATH => operations::submit(&service, parse(body)?),
            LOOKUP_PATH => operations::lookup(&service, parse(body)?),
            VIEW_PATH => operations::view(&service, parse(body)?),
            CHANGES_PATH => operations::changes(&service, parse(body)?),
            _ => unreachable!(),
        }
    })();
    let (status, value) = match result {
        Ok(value) => (200, value),
        Err(reason) => (503, json!({"error": "world_service_unavailable", "reason": reason})),
    };
    let encoded = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    super::write_json_response(stream, status, &encoded, false).map_err(|e| e.to_string())?;
    Ok(true)
}
