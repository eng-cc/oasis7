//! Device-side original operation cache. Never substitutes a new nonce or
//! payload after an unknown result. Uses the same encrypted durable boundary
//! as private goal records, with a separate domain and locator namespace.
use super::PrivateGoalStore;
use crate::world_service::{
    private_task_context::{TaskContextPrepareV1, verify_prepare},
    *,
};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientStateKind {
    Prepare,
    Prefix,
    Action,
    Outcome,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ClientStateV1 {
    ContextPrepare(Box<SignedReadRequest<TaskContextPrepareV1>>),
    PublicIntent(Box<SubmitIntentRequest<WorldServicePayloadV1>>),
}
impl ClientStateV1 {
    fn validate(&self, kind: ClientStateKind) -> Result<(), String> {
        match self {
            Self::ContextPrepare(proof) if kind == ClientStateKind::Prepare => {
                verify_prepare(proof)
            }
            Self::PublicIntent(request) if kind != ClientStateKind::Prepare => {
                request.validate().map_err(|_| "invalid cached operation")?;
                let expected = derive_correlation(
                    request.correlation.key.world.clone(),
                    &request.signed_payload,
                )?;
                if expected != request.correlation {
                    return Err("cached correlation mismatch".into());
                }
                match (&request.signed_payload, kind) {
                    (WorldServicePayloadV1::Scheduler(s), ClientStateKind::Prefix)
                        if matches!(
                            s.request.operation,
                            SchedulerOperationV1::TaskProviderPrefix { .. }
                        ) =>
                    {
                        Ok(())
                    }
                    (WorldServicePayloadV1::Scheduler(s), ClientStateKind::Outcome)
                        if matches!(
                            s.request.operation,
                            SchedulerOperationV1::TaskProviderFailure { .. }
                        ) =>
                    {
                        Ok(())
                    }
                    (WorldServicePayloadV1::Cognition(s), ClientStateKind::Action)
                        if s.request.task_fence.is_some() =>
                    {
                        Ok(())
                    }
                    _ => Err("cached operation kind mismatch".into()),
                }
            }
            _ => Err("cached operation kind mismatch".into()),
        }
    }
}
#[cfg(unix)]
const DOMAIN: &str = "oasis7/private-task-client-state/v1";
#[cfg(unix)]
const MAGIC: &[u8; 8] = b"O7CLIENT";
#[cfg(unix)]
const MAXIMUM: usize = oasis7_agent_api::game::MAX_REQUEST_BYTES + 1024;
impl PrivateGoalStore {
    #[cfg(unix)]
    fn client_state_name(&self, kind: ClientStateKind, operation: &str) -> Result<String, String> {
        if operation.trim().is_empty() || operation.len() > 256 {
            return Err("invalid client operation identity".into());
        }
        let key = Zeroizing::new(blake3::derive_key(
            "oasis7/private-task-client-index/v1",
            self.key.as_slice(),
        ));
        let scope = oasis7_wasm_abi::encode_canonical_cbor(&(kind, operation))
            .map_err(|_| "client operation encoding failed")?;
        Ok(format!("client-{}", blake3::keyed_hash(&key, &scope)))
    }
    pub fn read_client_state(
        &self,
        kind: ClientStateKind,
        operation: &str,
    ) -> Result<Option<ClientStateV1>, String> {
        #[cfg(unix)]
        {
            let name = self.client_state_name(kind, operation)?;
            let aad = serde_cbor::to_vec(&(DOMAIN, &name))
                .map_err(|_| "client operation encoding failed")?;
            let Some(bytes) = super::unix::read_bounded_record(
                &self.directory,
                &self.key,
                &name,
                MAGIC,
                &aad,
                MAXIMUM,
            )?
            else {
                return Ok(None);
            };
            let record: ClientStateV1 =
                serde_cbor::from_slice(&bytes).map_err(|_| "client operation recovery required")?;
            record.validate(kind)?;
            Ok(Some(record))
        }
        #[cfg(not(unix))]
        {
            let _ = (kind, operation);
            Err("private client operation storage unsupported".into())
        }
    }
    /// Returns the immutable winner. Callers check exact input compatibility;
    /// a concurrent first nonce is retained rather than silently replaced.
    pub fn put_client_state(
        &self,
        kind: ClientStateKind,
        operation: &str,
        record: &ClientStateV1,
    ) -> Result<ClientStateV1, String> {
        record.validate(kind)?;
        #[cfg(unix)]
        {
            let name = self.client_state_name(kind, operation)?;
            let aad = serde_cbor::to_vec(&(DOMAIN, &name))
                .map_err(|_| "client operation encoding failed")?;
            let bytes = Zeroizing::new(
                serde_cbor::to_vec(record).map_err(|_| "client operation encoding failed")?,
            );
            super::unix::publish_bounded_record(
                &self.directory,
                &self.key,
                &name,
                MAGIC,
                &aad,
                &bytes,
                MAXIMUM,
            )?;
            self.read_client_state(kind, operation)?
                .ok_or_else(|| "client operation recovery required".into())
        }
        #[cfg(not(unix))]
        {
            let _ = (operation, &self.key);
            Err("private client operation storage unsupported".into())
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use oasis7_agent_api::game::{GameTaskControlV1, GameTaskFenceV1};
    use serde_json::json;

    #[test]
    fn client_original_operation_survives_restart_conflict_and_corruption() {
        let directory = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "oasis7-client-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let signer = hex::encode([79; 32]);
        let public = authority::sign_read_request("key", (), &signer)
            .unwrap()
            .subject_public_key;
        let control: GameTaskControlV1 = serde_json::from_value(json!({"schema_revision":1,"task_id":"task","world_id":"world","genesis_digest":"genesis","branch_id":"main","reorg_epoch":"0","agent_id":"agent","owner_binding":"owner","owner_generation":"1","owner_public_key":public,"task_revision":"3","task_status":"active","latest_goal_revision":"1","active_goal_revision":"1","active_goal_reference":{"goal_revision":"1","goal_ref":"cd".repeat(32),"prepare_operation_id":"goal","publish_operation_id":"publish","activation_committed_height":"2"},"executor_epoch":"1","executor_id":"executor","executor_binding":{"executor_id":"executor","executor_public_key":public,"delegation_generation":"1","executor_epoch":"1"},"created_operation_id":"create"})).unwrap();
        let request = TaskContextPrepareV1 {
            schema_revision: 1,
            fence: GameTaskFenceV1::from_control(&control),
            client_operation_id: "stable".into(),
            client_nonce: "aa".repeat(32),
        };
        let proof = authority::sign_read_request(
            crate::world_service::private_task_context::TASK_CONTEXT_PREPARE_PATH,
            request.clone(),
            &signer,
        )
        .unwrap();
        let store = PrivateGoalStore::open(&directory, [61; 32]).unwrap();
        store
            .put_client_state(
                ClientStateKind::Prepare,
                "stable",
                &ClientStateV1::ContextPrepare(Box::new(proof.clone())),
            )
            .unwrap();
        drop(store);
        let store = PrivateGoalStore::open(&directory, [61; 32]).unwrap();
        let ClientStateV1::ContextPrepare(recovered) = store
            .read_client_state(ClientStateKind::Prepare, "stable")
            .unwrap()
            .unwrap()
        else {
            panic!("wrong record kind")
        };
        assert!(*recovered == proof);
        let mut changed = request;
        changed.client_nonce = "bb".repeat(32);
        let changed = authority::sign_read_request(
            crate::world_service::private_task_context::TASK_CONTEXT_PREPARE_PATH,
            changed,
            &signer,
        )
        .unwrap();
        let ClientStateV1::ContextPrepare(winner) = store
            .put_client_state(
                ClientStateKind::Prepare,
                "stable",
                &ClientStateV1::ContextPrepare(Box::new(changed)),
            )
            .unwrap()
        else {
            panic!("wrong record kind")
        };
        assert!(
            *winner == proof,
            "the original nonce must win after restart"
        );
        assert!(
            store
                .put_client_state(
                    ClientStateKind::Action,
                    "stable",
                    &ClientStateV1::ContextPrepare(Box::new(proof.clone()))
                )
                .is_err()
        );
        assert!(
            store
                .read_client_state(ClientStateKind::Prepare, "other")
                .unwrap()
                .is_none()
        );
        let name = store
            .client_state_name(ClientStateKind::Prepare, "stable")
            .unwrap();
        let path = directory.join(name);
        let mut bytes = std::fs::read(&path).unwrap();
        assert!(
            !bytes
                .windows(proof.request.client_nonce.len())
                .any(|part| part == proof.request.client_nonce.as_bytes())
        );
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(path, bytes).unwrap();
        assert!(
            store
                .read_client_state(ClientStateKind::Prepare, "stable")
                .is_err()
        );
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
