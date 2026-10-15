use super::*;
use oasis7_agent_api::world_action::{Action, GeoPos, MaterialLedgerId, MaterialTransitPriority};
use oasis7_client_api::world_service::{IntentOutcome, IntentResponse};
use oasis7_game_client::{device_client::Client, operation_store::ClientStateKind};
use serde_json::json;

/// SDK adapter retains the original encrypted device operation namespace.
pub struct SdkNativeTaskHost {
    client: Arc<Client>,
    run_binding: std::sync::Mutex<Option<Value>>,
}
impl SdkNativeTaskHost {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            run_binding: std::sync::Mutex::new(None),
        }
    }
    pub fn client(&self) -> &Client {
        &self.client
    }
}
fn state(response: &IntentResponse<Value>) -> NativeTaskRecovery {
    match response.outcome {
        IntentOutcome::Committed { .. } => NativeTaskRecovery::Settled,
        IntentOutcome::Rejected { .. } => NativeTaskRecovery::Rejected,
        IntentOutcome::Expired => NativeTaskRecovery::Expired,
        _ => NativeTaskRecovery::Pending,
    }
}
impl NativeTaskHost for SdkNativeTaskHost {
    fn initialize_run(&self, config: &NativeTaskRunConfig) -> Result<u32, String> {
        let control = self.current_control()?;
        let binding = json!({"config":config.binding(),"world":self.client.world,
            "agent":self.client.agent,"task_fence":control.fence});
        let spent = self.client.state.initialize_native_run(
            &config.run_id,
            &binding,
            config.max_model_calls,
        )?;
        *self
            .run_binding
            .lock()
            .map_err(|_| "native run binding lock poisoned")? = Some(binding);
        Ok(spent)
    }
    fn reserve_model_call(
        &self,
        config: &NativeTaskRunConfig,
        operation: &str,
    ) -> Result<u32, String> {
        let binding = self
            .run_binding
            .lock()
            .map_err(|_| "native run binding lock poisoned")?
            .clone()
            .ok_or("native run was not initialized")?;
        self.client.state.reserve_native_model_call(
            &config.run_id,
            &binding,
            config.max_model_calls,
            operation,
        )
    }
    fn recover(&self, operation: &str) -> Result<NativeTaskRecovery, String> {
        // Never read a new Context or quote before scanning both possible
        // terminal originals and the existing prefix under this operation ID.
        let action = self
            .client
            .cached_intent(ClientStateKind::Action, operation)?;
        let outcome = self
            .client
            .cached_intent(ClientStateKind::Outcome, operation)?;
        let prefix = self
            .client
            .cached_intent(ClientStateKind::Prefix, operation)?;
        if action.is_some() && outcome.is_some() {
            return Err("context has conflicting original action and outcome; retain both".into());
        }
        if let Some(original) = action {
            return Ok(state(&self.client.lookup(&original)?));
        }
        if let Some(original) = outcome {
            let result = state(&self.client.lookup(&original)?);
            return Ok(if result == NativeTaskRecovery::Settled {
                NativeTaskRecovery::Aborted
            } else {
                result
            });
        }
        if let Some(original) = prefix {
            let result = state(&self.client.lookup(&original)?);
            return Ok(if result == NativeTaskRecovery::Settled {
                NativeTaskRecovery::Ready
            } else {
                result
            });
        }
        Ok(NativeTaskRecovery::Vacant)
    }
    fn prepare(&self, operation: &str) -> Result<NativeTaskContext, String> {
        if self.current_control()?.fence != self.initial_fence()? {
            return Err("task fence differs from immutable native run binding".into());
        }
        let prefix = self.client.prefix(operation)?;
        let result = self.client.submit_and_poll(&prefix)?;
        if state(&result) != NativeTaskRecovery::Settled {
            return Err(
                "context prefix has not committed; retain original operation and lookup".into(),
            );
        }
        let captured = self.client.captured(operation)?;
        let prepared = &captured.prepared;
        Ok(NativeTaskContext {
            operation: operation.into(),
            fence: prepared.identity.fence.clone(),
            valid_until_tick: prepared.valid_until_tick.0,
            model_context: json!({
                "goal":{"revision":prepared.snapshot.control.active_goal_revision,
                    "content":prepared.snapshot.active_goal.owner_proof.request.content},
                "actor_observation_schema":prepared.snapshot.actor_observation_schema,
                "actor_observation":prepared.snapshot.actor_observation,
                "current_actor_observation":captured.current_public_context.actor_observation,
                "capability_catalog":captured.current_public_context.provider_authority.capability_catalog,
                "native_industry_profile":true,"task_fence":prepared.identity.fence,
                "capture_commit":captured.capture_commit,"current_read_commit":captured.current_public_context.read_commit,
                "context_identity":prepared.identity,
            }),
        })
    }
    fn current_control(&self) -> Result<NativeTaskControl, String> {
        let view = self.client.public_view()?;
        let fence = view
            .task_fence()
            .ok_or("native task control unavailable")?
            .clone();
        Ok(NativeTaskControl {
            fence,
            logical_tick: view.logical_tick(),
        })
    }
    fn initial_fence(&self) -> Result<GameTaskFenceV1, String> {
        let binding = self
            .run_binding
            .lock()
            .map_err(|_| "native run binding lock poisoned")?;
        let binding = binding.as_ref().ok_or("native run was not initialized")?;
        serde_json::from_value(binding["task_fence"].clone())
            .map_err(|_| "invalid native run task binding".into())
    }
    fn invoke(
        &self,
        context: &NativeTaskContext,
        tool: &NativeTaskTool,
    ) -> Result<NativeTaskToolResult, String> {
        let operation = &context.operation;
        let mut effect = None;
        let value = match tool {
            NativeTaskTool::Wait(v) => {
                // Observe independently progressing Runtime time. This never
                // steps the World or extends the original context's validity.
                let initial = self.current_control()?;
                let target = initial
                    .logical_tick
                    .checked_add(u64::from(v.ticks))
                    .filter(|tick| *tick <= context.valid_until_tick)
                    .ok_or("wait would exceed original context validity")?;
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
                loop {
                    let current = self.current_control()?;
                    if current.fence != context.fence
                        || current.fence.task_status != GameTaskStatusV1::Active
                        || current.logical_tick > context.valid_until_tick
                    {
                        // Let the runner classify the authenticated control
                        // change on its next mandatory check, before inference
                        // or another tool. No effect was attempted.
                        return Ok(NativeTaskToolResult {
                            value: json!({"wait":"interrupted"}),
                            effect: None,
                        });
                    }
                    if current.logical_tick >= target {
                        break;
                    }
                    if std::time::Instant::now() >= deadline {
                        return Err("world progress wait timed out".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                self.client.industry()?
            }
            NativeTaskTool::Observe(_) => {
                let view = self.client.public_view()?;
                json!({"read_version":view.version(),"logical_tick":view.logical_tick(),"observation":view.raw_projection()})
            }
            NativeTaskTool::Industry(_) => self.client.industry()?,
            NativeTaskTool::Capabilities(_) => self.client.capabilities()?,
            NativeTaskTool::QuoteBuild(v) => self
                .client
                .quote_build(operation, &v.site, &v.factory, &v.kind)?,
            NativeTaskTool::QuoteSchedule(v) => self
                .client
                .quote_schedule(operation, &v.factory, &v.recipe, v.batches)?,
            NativeTaskTool::Build(v) => {
                let result = self.client.build(operation, &v.site, &v.factory, &v.kind)?;
                effect = Some(state(&result));
                serde_json::to_value(result).map_err(|_| "native result encoding failed")?
            }
            NativeTaskTool::Schedule(v) => {
                let result = self
                    .client
                    .schedule(operation, &v.factory, &v.recipe, v.batches)?;
                effect = Some(state(&result));
                serde_json::to_value(result).map_err(|_| "native result encoding failed")?
            }
            NativeTaskTool::Move(v) => {
                let [x, y, z] = v.to;
                let result = self.client.submit_action(
                    operation,
                    Action::MoveAgent {
                        agent_id: self.client.agent.clone(),
                        to: GeoPos::new(x, y, z),
                    },
                )?;
                effect = Some(state(&result));
                serde_json::to_value(result).map_err(|_| "native result encoding failed")?
            }
            NativeTaskTool::Transfer(v) => {
                let priority = match v.priority.as_str() {
                    "standard" => MaterialTransitPriority::Standard,
                    "urgent" => MaterialTransitPriority::Urgent,
                    _ => return Err("invalid native priority".into()),
                };
                let result = self.client.submit_action(
                    operation,
                    Action::TransferMaterial {
                        requester_agent_id: self.client.agent.clone(),
                        from_ledger: MaterialLedgerId::try_from(v.from.clone())
                            .map_err(|_| "invalid source ledger")?,
                        to_ledger: MaterialLedgerId::try_from(v.to.clone())
                            .map_err(|_| "invalid destination ledger")?,
                        kind: v.material.clone(),
                        amount: v.amount,
                        distance_km: v.distance_km,
                        priority: Some(priority),
                        route_id: None,
                        route_ids: v.routes.clone(),
                        auto_reroute: false,
                    },
                )?;
                effect = Some(state(&result));
                serde_json::to_value(result).map_err(|_| "native result encoding failed")?
            }
            NativeTaskTool::Abort(_) => {
                let result = self.client.outcome(operation, "executor_aborted")?;
                effect = Some(state(&result));
                serde_json::to_value(result).map_err(|_| "native result encoding failed")?
            }
        };
        Ok(NativeTaskToolResult { value, effect })
    }
}
