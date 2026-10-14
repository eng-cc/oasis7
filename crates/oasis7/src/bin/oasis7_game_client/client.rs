#[path = "client_build.rs"]
mod client_build;

use oasis7::runtime as crate_action;
use oasis7::world_service::{
    client::{RemoteWorldServiceClient, WorldServiceClientConfig, WorldServicePort},
    private_goal_store::{
        PrivateGoalStore,
        client_state::{ClientStateKind, ClientStateV1},
    },
    private_key_file::read_private_key_file,
    private_task_context::*,
    task_recipe_quote::{TASK_RECIPE_QUOTE_PATH, TaskRecipeQuoteRequestV1, TaskRecipeQuoteV1},
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
            formal_read_trust: None,
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
    pub fn public_view(
        &self,
    ) -> Result<oasis7::world_service::verified_view::VerifiedWorldView, String> {
        self.remote
            .read_view(ReadWorldViewRequest {
                observation_nonce: None,
                formal_anchor: None,
                contract_version: 1,
                world: self.world.clone(),
                scope_id: format!("agent:{}", self.agent),
                min_commit: None,
                fixed_commit: None,
                deadline_unix_ms: None,
            })
            .map_err(|_| "signed Agent read unavailable".to_owned())
    }
    pub fn capabilities(&self) -> Result<Value, String> {
        let view = self.public_view()?;
        let projection = view.projection();
        let grants: Vec<_> = projection
            .native_delegation_authorizations
            .iter()
            .take(128)
            .map(|authorization| {
                let grant = &authorization.grant;
                let status = if grant.revoked {
                    "revoked"
                } else if view.logical_tick() < grant.valid_from_tick {
                    "not_yet_valid"
                } else if view.logical_tick() > grant.valid_until_tick {
                    "expired"
                } else if authorization.remaining_units == 0 {
                    "exhausted"
                } else {
                    "current"
                };
                json!({"authorization":authorization,"status":status,"admission_required":true})
            })
            .collect();
        Ok(
            json!({"api_revision":1,"read_version":view.version(),"logical_tick":view.logical_tick(),
                "task_control":projection.game_task_control,
                "proposal_forms":[
                    {"action_ref":"native.build_factory","arguments":{"site":"authorized ready site ID","factory":"new factory ID","kind":"factory.smelter.mk1|factory.assembler.mk1"},"signed_quote_required":true,"admission_required":true},
                    {"action_ref":"native.move_agent","arguments":{"to":"three signed i64 centimeter coordinates x,y,z"},"admission_required":true},
                    {"action_ref":"native.schedule_recipe","arguments":{"factory":"factory ID","recipe":"exact built-in recipe ID","batches":"positive u32"},"signed_quote_required":true,"admission_required":true},
                    {"action_ref":"native.transfer_material","arguments":{"from":"ledger ID","to":"ledger ID","material":"kind","amount":"positive i64","distance_km":"nonnegative i64","priority":"standard|urgent","routes":"ordered route IDs or direct"},"admission_required":true}
                ],
                "module_capability_catalog":projection.agent_context.as_ref().map(|context| &context.capability_catalog),
                "native_delegation_authorizations":grants,
                "native_authorization_count":projection.native_delegation_authorizations.len(),
                "native_authorizations_truncated":projection.native_delegation_authorizations.len()>128,
                "outcomes":["executor_aborted","provider_failed","transport_exhausted"],
                "scope":"local native proposal forms are distinct from module commands and native action/object electricity grants; current grants do not guarantee admission"
            }),
        )
    }
    pub fn industry(&self) -> Result<Value, String> {
        let view = self.public_view()?;
        let state = serde_json::to_value(&view.projection().state)
            .map_err(|_| "industry encoding failed")?;
        let mut fields = serde_json::Map::new();
        for name in [
            "materials",
            "material_ledgers",
            "material_profiles",
            "logistics_routes",
            "factories",
            "recipe_profiles",
            "factory_profiles",
            "product_profiles",
            "pending_material_transits",
            "pending_recipe_jobs",
            "pending_factory_builds",
            "agents",
            "agent_location_authorities",
            "location_anchors",
            "factory_site_authorities",
            "factory_construction_power_profiles",
            "industry_progress",
        ] {
            fields.insert(
                name.into(),
                bounded_collection(state.get(name).cloned().unwrap_or(Value::Null)),
            );
        }
        Ok(
            json!({"api_revision":1,"read_version":view.version(),"logical_tick":view.logical_tick(),
            "industry":fields,"collection_limit":128,"scope":"authenticated public industry observation; no signed transfer quote is provided"}),
        )
    }
    pub fn quote_schedule(
        &self,
        operation: &str,
        factory: &str,
        recipe: &str,
        batches: u32,
    ) -> Result<Value, String> {
        if let Some(original) = self.cached_intent(ClientStateKind::Action, operation)? {
            validate_original_schedule(&original, &self.agent, factory, recipe, batches)?;
            let response = self.lookup(&original)?;
            return Ok(
                json!({"original_operation":response,"quote":null,"next":"retain_original_action; capture a new context only after a settled result"}),
            );
        }
        let captured = self.captured(operation)?;
        serde_json::to_value(self.recipe_quote(operation, &captured, factory, recipe, batches)?)
            .map_err(|_| "recipe quote encoding failed".into())
    }
    fn recipe_quote(
        &self,
        operation: &str,
        captured: &CapturedTaskContextV1,
        factory: &str,
        recipe: &str,
        batches: u32,
    ) -> Result<TaskRecipeQuoteV1, String> {
        let proof = self.context_read_proof(operation)?;
        let request = authority::sign_read_request(
            TASK_RECIPE_QUOTE_PATH,
            TaskRecipeQuoteRequestV1 {
                schema_revision: 1,
                context_proof: proof,
                expected_base_binding: captured
                    .current_public_context
                    .captured_base_binding
                    .clone(),
                factory_id: factory.to_owned(),
                recipe_id: recipe.to_owned(),
                batches,
            },
            &self.key_hex,
        )?;
        self.remote.quote_task_recipe(&request).map_err(|error| {
            format!(
                "recipe quote {}; no replacement action created",
                failure_tag(&error)
            )
        })
    }
    pub fn schedule(
        &self,
        operation: &str,
        factory: &str,
        recipe: &str,
        batches: u32,
    ) -> Result<IntentResponse<Value>, String> {
        // Recovery precedes context reads and quotes, including Unknown outcomes.
        // Never replace an immutable original with a newly quoted plan.
        if let Some(original) = self.cached_intent(ClientStateKind::Action, operation)? {
            validate_original_schedule(&original, &self.agent, factory, recipe, batches)?;
            return self.lookup(&original);
        }
        let mut captured = self.captured(operation)?;
        let quoted = self.recipe_quote(operation, &captured, factory, recipe, batches)?;
        if quoted.quote.rejection.is_some() {
            return Err(
                "recipe quote rejected; no action submitted; inspect quote-schedule".into(),
            );
        }
        captured.current_public_context = quoted.current_public_context;
        let signed = authority::sign_read_request(
            "cognition",
            cognition_intent(&captured, quoted.quote.action)?,
            &self.key_hex,
        )?;
        let request = self.cache_intent(
            ClientStateKind::Action,
            operation,
            WorldServicePayloadV1::Cognition(signed),
        )?;
        self.submit_and_poll(&request)
    }
    pub fn submit_action(
        &self,
        operation: &str,
        action: crate_action::Action,
    ) -> Result<IntentResponse<Value>, String> {
        let request =
            if let Some(request) = self.cached_intent(ClientStateKind::Action, operation)? {
                validate_original_action(&request, &action)?;
                request
            } else {
                let captured = self.captured(operation)?;
                let signed = authority::sign_read_request(
                    "cognition",
                    cognition_intent(&captured, action)?,
                    &self.key_hex,
                )?;
                self.cache_intent(
                    ClientStateKind::Action,
                    operation,
                    WorldServicePayloadV1::Cognition(signed),
                )?
            };
        self.submit_and_poll(&request)
    }
    pub fn task_fence(&self) -> Result<GameTaskFenceV1, String> {
        let view = self
            .remote
            .read_view(ReadWorldViewRequest {
                observation_nonce: None,
                formal_anchor: None,
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
            .map_err(|error| {
                format!(
                    "private preparation {}; retry original operation",
                    failure_tag(&error)
                )
            })?;
        let signed =
            authority::sign_read_request("scheduler", public_prefix(&prepared)?, &self.key_hex)?;
        self.cache_intent(
            ClientStateKind::Prefix,
            operation,
            WorldServicePayloadV1::Scheduler(signed),
        )
    }
    fn context_read_proof(
        &self,
        operation: &str,
    ) -> Result<SignedReadRequest<TaskContextGetV1>, String> {
        let prefix = self
            .cached_intent(ClientStateKind::Prefix, operation)?
            .ok_or("context has not been prepared")?;
        let WorldServicePayloadV1::Scheduler(signed) = prefix.signed_payload else {
            return Err("invalid cached context prefix".into());
        };
        let SchedulerOperationV1::TaskProviderPrefix { context } = signed.request.operation else {
            return Err("invalid cached context prefix".into());
        };
        authority::sign_read_request(
            TASK_CONTEXT_READ_PATH,
            TaskContextGetV1 {
                schema_revision: 1,
                context,
                prepare_operation_id: operation.to_owned(),
            },
            &self.key_hex,
        )
    }
    pub fn captured(&self, operation: &str) -> Result<CapturedTaskContextV1, String> {
        let signed = self.context_read_proof(operation)?;
        self.remote.read_task_context(&signed).map_err(|error| {
            format!(
                "private context {}; query original results and refresh",
                failure_tag(&error)
            )
        })
    }
}
pub fn print(value: &impl Serialize) -> Result<(), String> {
    let text = serde_json::to_string(value).map_err(|_| "response encoding failed")?;
    println!("{text}");
    Ok(())
}
pub fn pending(response: &IntentResponse<Value>) -> Value {
    json!({"operation":response,"next":context_next_step(&response.outcome),"usable_context":false})
}
fn context_next_step<T>(outcome: &IntentOutcome<T>) -> &'static str {
    match outcome {
        IntentOutcome::Rejected { .. } | IntentOutcome::Expired => "refresh_after_settled_result",
        _ => "lookup_original",
    }
}

/// Emit only a bounded public category/status, never response text or proofs.
fn failure_tag(error: &oasis7::world_service::client::WorldServiceClientError) -> String {
    use oasis7::world_service::client::WorldServiceClientError;
    match error {
        WorldServiceClientError::Configuration(_) => "configuration rejected".into(),
        WorldServiceClientError::Transport(_) => "transport unavailable".into(),
        WorldServiceClientError::Assurance(_) => "response not verified".into(),
        WorldServiceClientError::Rejected { status, .. } => format!("rejected HTTP {status}"),
        WorldServiceClientError::Cooldown { status, .. } => format!("cooldown HTTP {status}"),
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn unresolved_contexts_never_advise_a_replacement_operation() {
        for outcome in [
            IntentOutcome::<Value>::Unknown,
            IntentOutcome::Pending,
            IntentOutcome::HistoryUnavailable,
            IntentOutcome::Received {
                durability: AdmissionDurability::Volatile,
            },
            IntentOutcome::Received {
                durability: AdmissionDurability::Durable,
            },
        ] {
            assert_eq!(context_next_step(&outcome), "lookup_original");
        }
        for outcome in [
            IntentOutcome::<Value>::Rejected {
                reason: WorldServiceErrorKind::Conflict,
            },
            IntentOutcome::Expired,
        ] {
            assert_eq!(context_next_step(&outcome), "refresh_after_settled_result");
        }
    }
}

fn validate_original_schedule(
    request: &SubmitIntentRequest<WorldServicePayloadV1>,
    agent: &str,
    factory: &str,
    recipe: &str,
    batches: u32,
) -> Result<(), String> {
    let WorldServicePayloadV1::Cognition(signed) = &request.signed_payload else {
        return Err("invalid cached action".into());
    };
    validate_cached_schedule_action(&signed.request.action, agent, factory, recipe, batches)
}
fn validate_cached_schedule_action(
    action: &crate_action::Action,
    agent: &str,
    factory: &str,
    recipe: &str,
    batches: u32,
) -> Result<(), String> {
    match action {
        crate_action::Action::ScheduleRecipe {
            requester_agent_id,
            factory_id,
            recipe_id,
            plan,
            logistics_route_ids,
            logistics_path_ids,
        } if requester_agent_id == agent
            && factory_id == factory
            && recipe_id == recipe
            && plan.accepted_batches == batches
            && logistics_route_ids.is_empty()
            && logistics_path_ids.is_empty() =>
        {
            Ok(())
        }
        _ => Err("context already has a different original action; query its result".into()),
    }
}

fn validate_original_action(
    request: &SubmitIntentRequest<WorldServicePayloadV1>,
    action: &crate_action::Action,
) -> Result<(), String> {
    let WorldServicePayloadV1::Cognition(signed) = &request.signed_payload else {
        return Err("invalid cached action".into());
    };
    validate_action_identity(&signed.request.action, action)
}
fn validate_action_identity(
    original: &crate_action::Action,
    proposed: &crate_action::Action,
) -> Result<(), String> {
    if original != proposed {
        return Err("context already has a different original action; query its result".into());
    }
    Ok(())
}
fn bounded_collection(value: Value) -> Value {
    match value {
        Value::Object(values) => {
            let count = values.len();
            let items: serde_json::Map<_, _> = values.into_iter().take(128).collect();
            json!({"items":items,"count":count,"truncated":count>128})
        }
        Value::Array(values) => {
            let count = values.len();
            let items: Vec<_> = values.into_iter().take(128).collect();
            json!({"items":items,"count":count,"truncated":count>128})
        }
        other => json!({"value":other,"truncated":false}),
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;
    #[test]
    fn cached_schedule_recovery_compares_selection_without_requoting_original() {
        let recipe = "recipe.smelter.iron_ingot";
        let original = crate_action::Action::ScheduleRecipe {
            requester_agent_id: "configured".into(),
            factory_id: "smelter".into(),
            recipe_id: recipe.into(),
            plan: crate_action::builtin_recipe_plan(recipe, 3).unwrap(),
            logistics_route_ids: vec![],
            logistics_path_ids: vec![],
        };
        assert!(
            validate_cached_schedule_action(&original, "configured", "smelter", recipe, 3).is_ok()
        );
        for (agent, factory, recipe, batches) in [
            ("other", "smelter", recipe, 3),
            ("configured", "other", recipe, 3),
            ("configured", "smelter", "recipe.assembler.gear", 3),
            ("configured", "smelter", recipe, 4),
        ] {
            assert!(
                validate_cached_schedule_action(&original, agent, factory, recipe, batches)
                    .is_err()
            );
        }
        let mut changed = original.clone();
        let crate_action::Action::ScheduleRecipe {
            logistics_route_ids,
            ..
        } = &mut changed
        else {
            unreachable!()
        };
        logistics_route_ids.push("injected".into());
        assert!(
            validate_cached_schedule_action(&changed, "configured", "smelter", recipe, 3).is_err()
        );
        assert!(
            validate_cached_schedule_action(&transfer(), "configured", "smelter", recipe, 3)
                .is_err()
        );
    }
    fn transfer() -> crate_action::Action {
        crate_action::Action::TransferMaterial {
            requester_agent_id: "configured".into(),
            from_ledger: oasis7::industry_types::MaterialLedgerId::agent("configured"),
            to_ledger: oasis7::industry_types::MaterialLedgerId::factory("smelter"),
            kind: "iron_ore".into(),
            amount: 2,
            distance_km: 1,
            priority: Some(crate_action::MaterialTransitPriority::Standard),
            route_id: None,
            route_ids: vec!["a".into(), "b".into()],
            auto_reroute: false,
        }
    }
    #[test]
    fn original_transfer_identity_protects_every_business_field_and_action_kind() {
        let original = transfer();
        assert!(validate_action_identity(&original, &original).is_ok());
        for field in 0..10 {
            let mut changed = transfer();
            let crate_action::Action::TransferMaterial {
                requester_agent_id,
                from_ledger,
                to_ledger,
                kind,
                amount,
                distance_km,
                priority,
                route_id,
                route_ids,
                auto_reroute,
            } = &mut changed
            else {
                unreachable!()
            };
            match field {
                0 => *requester_agent_id = "foreign".into(),
                1 => *from_ledger = oasis7::industry_types::MaterialLedgerId::world(),
                2 => *to_ledger = oasis7::industry_types::MaterialLedgerId::site("other"),
                3 => *kind = "copper".into(),
                4 => *amount += 1,
                5 => *distance_km += 1,
                6 => *priority = Some(crate_action::MaterialTransitPriority::Urgent),
                7 => *route_id = Some("a".into()),
                8 => route_ids.reverse(),
                9 => *auto_reroute = true,
                _ => unreachable!(),
            }
            assert!(validate_action_identity(&original, &changed).is_err());
        }
        let movement = crate_action::Action::MoveAgent {
            agent_id: "configured".into(),
            to: oasis7::GeoPos::new(0, 0, 0),
        };
        assert!(validate_action_identity(&original, &movement).is_err());
        assert!(validate_action_identity(&movement, &original).is_err());
    }
    #[test]
    fn industry_truncation_reports_omitted_entries() {
        let value = bounded_collection(json!((0..129).collect::<Vec<_>>()));
        assert_eq!(value["items"].as_array().unwrap().len(), 128);
        assert_eq!(value["count"], 129);
        assert_eq!(value["truncated"], true);
    }
}
