use oasis7::world_service::{
    client::{RemoteWorldServiceClient, WorldServiceClientConfig, WorldServicePort},
    private_goal_store::{
        PrivateGoalStore,
        client_state::{ClientStateKind, ClientStateV1},
    },
    private_key_file::read_private_key_file,
    private_task_context::*,
    *,
};
use oasis7_agent_api::game::GameTaskFenceV1;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceConfig {
    endpoint: String,
    world_id: String,
    genesis_digest: String,
    agent_id: String,
    service_public_key: String,
    signer_file: PathBuf,
    state_directory: PathBuf,
}
pub struct Client {
    pub remote: RemoteWorldServiceClient,
    pub world: WorldIdentity,
    pub agent: String,
    pub public_key: String,
    pub key_hex: Zeroizing<String>,
    pub state: PrivateGoalStore,
}
impl Client {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|_| "client configuration unavailable")?;
        let mut bytes = Vec::new();
        file.take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| "client configuration unavailable")?;
        if bytes.len() > 65536 {
            return Err("client configuration exceeds limit".into());
        }
        let value = oasis7_agent_api::game::parse_request(&bytes)
            .map_err(|_| "invalid client configuration")?;
        let config: DeviceConfig =
            serde_json::from_value(value).map_err(|_| "invalid client configuration")?;
        let endpoint =
            reqwest::Url::parse(&config.endpoint).map_err(|_| "invalid client endpoint")?;
        if !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err("client endpoint must not contain credentials, query or fragment".into());
        }
        let key = read_private_key_file(&config.signer_file)?;
        let key_hex = Zeroizing::new(hex::encode(key.as_slice()));
        let public_key =
            authority::sign_read_request("client-key", (), &key_hex)?.subject_public_key;
        let cache_key = Zeroizing::new(blake3::derive_key(
            "oasis7/private-task-client-key/v1",
            key.as_slice(),
        ));
        let state = PrivateGoalStore::open(&config.state_directory, *cache_key)?;
        let world = WorldIdentity {
            world_id: config.world_id,
            genesis_digest: config.genesis_digest,
        };
        let remote = RemoteWorldServiceClient::new(WorldServiceClientConfig {
            endpoint: config.endpoint,
            trusted_service_public_key: config.service_public_key,
            expected_world: world.clone(),
            scope_id: format!("agent:{}", config.agent_id),
            read_private_key_hex: key_hex.to_string(),
            timeout: Duration::from_secs(5),
            max_response_bytes: MAX_PRIVATE_CONTEXT_BYTES,
        })
        .map_err(|_| "client configuration rejected")?;
        Ok(Self {
            remote,
            world,
            agent: config.agent_id,
            public_key,
            key_hex,
            state,
        })
    }
    pub fn task_fence(&self) -> Result<GameTaskFenceV1, String> {
        let view = self
            .remote
            .read_view(ReadWorldViewRequest {
                contract_version: 1,
                world: self.world.clone(),
                scope_id: format!("agent:{}", self.agent),
                min_commit: None,
                fixed_commit: None,
                deadline_unix_ms: None,
            })
            .map_err(|_| "signed task read unavailable")?;
        let control = view
            .projection()
            .game_task_control
            .as_ref()
            .ok_or("task control unavailable for this device")?;
        let fence = GameTaskFenceV1::from_control(control);
        fence.validate()?;
        if fence.world_id != self.world.world_id
            || fence.genesis_digest != self.world.genesis_digest
            || fence.agent_id != self.agent
        {
            return Err("task read scope mismatch".into());
        }
        Ok(fence)
    }
    pub fn original_prepare(
        &self,
        operation: &str,
    ) -> Result<SignedReadRequest<TaskContextPrepareV1>, String> {
        let record = if let Some(record) = self
            .state
            .read_client_state(ClientStateKind::Prepare, operation)?
        {
            record
        } else {
            let mut nonce = Zeroizing::new([0; 32]);
            getrandom::fill(&mut *nonce).map_err(|_| "device randomness unavailable")?;
            let proof = authority::sign_read_request(
                TASK_CONTEXT_PREPARE_PATH,
                TaskContextPrepareV1 {
                    schema_revision: 1,
                    fence: self.task_fence()?,
                    client_operation_id: operation.to_owned(),
                    client_nonce: hex::encode(nonce.as_slice()),
                },
                &self.key_hex,
            )?;
            self.state.put_client_state(
                ClientStateKind::Prepare,
                operation,
                &ClientStateV1::ContextPrepare(Box::new(proof)),
            )?
        };
        let ClientStateV1::ContextPrepare(proof) = record else {
            return Err("client preparation recovery required".into());
        };
        if proof.subject_public_key != self.public_key
            || proof.request.client_operation_id != operation
            || proof.request.fence.world_id != self.world.world_id
            || proof.request.fence.genesis_digest != self.world.genesis_digest
            || proof.request.fence.agent_id != self.agent
        {
            return Err("cached preparation scope mismatch".into());
        }
        Ok(*proof)
    }
    pub fn cached_intent(
        &self,
        kind: ClientStateKind,
        operation: &str,
    ) -> Result<Option<SubmitIntentRequest<WorldServicePayloadV1>>, String> {
        let Some(record) = self.state.read_client_state(kind, operation)? else {
            return Ok(None);
        };
        let ClientStateV1::PublicIntent(request) = record else {
            return Err("client intent recovery required".into());
        };
        if request.correlation.key.world != self.world
            || request.correlation.key.verified_subject != self.public_key
        {
            return Err("cached intent scope mismatch".into());
        }
        let agent = match &request.signed_payload {
            WorldServicePayloadV1::Scheduler(s) => &s.request.agent_id,
            WorldServicePayloadV1::Cognition(s) => &s.request.request.agent_id,
            _ => return Err("unsupported cached intent".into()),
        };
        if agent != &self.agent {
            return Err("cached Agent scope mismatch".into());
        }
        Ok(Some(*request))
    }
    pub fn cache_intent(
        &self,
        kind: ClientStateKind,
        operation: &str,
        payload: WorldServicePayloadV1,
    ) -> Result<SubmitIntentRequest<WorldServicePayloadV1>, String> {
        let request = SubmitIntentRequest {
            contract_version: 1,
            correlation: derive_correlation(self.world.clone(), &payload)?,
            deadline_unix_ms: None,
            signed_payload: payload,
        };
        let record = self.state.put_client_state(
            kind,
            operation,
            &ClientStateV1::PublicIntent(Box::new(request.clone())),
        )?;
        let ClientStateV1::PublicIntent(winner) = record else {
            return Err("client intent recovery required".into());
        };
        if *winner != request {
            return Err("client operation conflict; original intent retained".into());
        }
        Ok(*winner)
    }
    pub fn lookup(
        &self,
        request: &SubmitIntentRequest<WorldServicePayloadV1>,
    ) -> Result<IntentResponse<Value>, String> {
        self.remote
            .lookup(
                LookupIntentRequest {
                    contract_version: 1,
                    key: request.correlation.key.clone(),
                },
                request.signed_payload.clone(),
            )
            .map_err(|_| "original result query unavailable; retain original operation".to_owned())
    }
    pub fn submit_and_poll(
        &self,
        request: &SubmitIntentRequest<WorldServicePayloadV1>,
    ) -> Result<IntentResponse<Value>, String> {
        // The original signed payload is already durable before the first send.
        // A transport failure can only cause lookup/replay of this exact intent.
        let submission = self.remote.submit(request.clone());
        if let Ok(SubmitObservation::Response(response)) = submission
            && matches!(
                response.outcome,
                IntentOutcome::Committed { .. }
                    | IntentOutcome::Rejected { .. }
                    | IntentOutcome::Expired
            )
        {
            return Ok(*response);
        }
        for _ in 0..40 {
            let response = self.lookup(request)?;
            if !matches!(
                response.outcome,
                IntentOutcome::Received { .. } | IntentOutcome::Pending | IntentOutcome::Unknown
            ) {
                return Ok(response);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        self.lookup(request)
    }
    pub fn prefix(
        &self,
        operation: &str,
    ) -> Result<SubmitIntentRequest<WorldServicePayloadV1>, String> {
        if let Some(request) = self.cached_intent(ClientStateKind::Prefix, operation)? {
            return Ok(request);
        }
        let original = self.original_prepare(operation)?;
        let prepared = self
            .remote
            .prepare_task_context(&original)
            .map_err(|_| "private preparation rejected or unavailable; retry original operation")?;
        let signed =
            authority::sign_read_request("scheduler", public_prefix(&prepared)?, &self.key_hex)?;
        self.cache_intent(
            ClientStateKind::Prefix,
            operation,
            WorldServicePayloadV1::Scheduler(signed),
        )
    }
    pub fn captured(&self, operation: &str) -> Result<CapturedTaskContextV1, String> {
        let prefix = self
            .cached_intent(ClientStateKind::Prefix, operation)?
            .ok_or("context has not been prepared")?;
        let WorldServicePayloadV1::Scheduler(signed) = prefix.signed_payload else {
            return Err("invalid cached context prefix".into());
        };
        let SchedulerOperationV1::TaskProviderPrefix { context } = signed.request.operation else {
            return Err("invalid cached context prefix".into());
        };
        let signed = authority::sign_read_request(
            TASK_CONTEXT_READ_PATH,
            TaskContextGetV1 {
                schema_revision: 1,
                context,
                prepare_operation_id: operation.to_owned(),
            },
            &self.key_hex,
        )?;
        self.remote.read_task_context(&signed).map_err(|_| {
            "context stale, finished or unavailable; query original results and refresh".to_owned()
        })
    }
}
pub fn print(value: &impl Serialize) -> Result<(), String> {
    let text = serde_json::to_string(value).map_err(|_| "response encoding failed")?;
    println!("{text}");
    Ok(())
}
pub fn pending(response: &IntentResponse<Value>) -> Value {
    json!({"operation":response,"next":"lookup_original","usable_context":false})
}
