use super::client::{Client, pending, print};
use oasis7::world_service::{
    client::WorldServicePort, private_goal_store::client_state::ClientStateKind,
    private_task_context::*, *,
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};
const HELP: &str = "oasis7_game_client --config <device.json> <command> [options]\nCommands:\n  describe\n  task\n  capabilities\n  context --operation <stable-prepare-id>\n  move --context <prepare-id> --to <x,y,z>\n  outcome --context <prepare-id> --reason <executor_aborted|provider_failed|transport_exhausted>\n  lookup --context <prepare-id> --kind <prefix|action|outcome>\nConfiguration: endpoint, world_id, genesis_digest, agent_id, service_public_key, signer_file, state_directory.\nThe signer is an operator-owned mode-0600 raw 32-byte file; state_directory is a private encrypted operation cache.\nHTTP loopback requires all local listening processes to be trusted. Use authenticated TLS for untrusted multi-user hosts.";

pub fn run(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args == ["--help"] {
        println!("{HELP}");
        return Ok(());
    }
    if args.len() < 3 || args[0] != "--config" {
        return Err(HELP.into());
    }
    let command = &args[2];
    let allowed: &[&str] = match command.as_str() {
        "describe" | "task" | "capabilities" => &[],
        "context" => &["--operation"],
        "move" => &["--context", "--to"],
        "outcome" => &["--context", "--reason"],
        "lookup" => &["--context", "--kind"],
        _ => return Err("unknown client command; use --help".into()),
    };
    if !(args.len() - 3).is_multiple_of(2) {
        return Err("options require a value".into());
    }
    let mut options = BTreeMap::new();
    for pair in args[3..].chunks_exact(2) {
        if !allowed.contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unsupported or duplicate option".into());
        }
    }
    let required = |name| {
        options
            .get(name)
            .copied()
            .ok_or_else(|| format!("missing {name}"))
    };
    let client = Client::open(Path::new(&args[1]))?;
    match command.as_str() {
        "describe" => print(
            &client
                .remote
                .describe()
                .map_err(|_| "signed service discovery unavailable")?,
        ),
        "task" => print(&client.task_fence()?),
        "capabilities" => print(&json!({
            "api_revision":1, "task_fence":client.task_fence()?,
            "proposal_forms":[{"action_ref":"native.move_agent","arguments":{"to":"three signed i64 centimeter coordinates x_cm,y_cm,z_cm"},"has_side_effect":true,"admission_required":true}],
            "outcomes":["executor_aborted","provider_failed","transport_exhausted"],
            "scope":"native task-lane client; full Game HTTP action catalog is not delivered"
        })),
        "context" => {
            let operation = required("--operation")?;
            let prefix = client.prefix(operation)?;
            let response = client.submit_and_poll(&prefix)?;
            if !matches!(response.outcome, IntentOutcome::Committed { .. }) {
                return print(&pending(&response));
            }
            let captured = client.captured(operation)?;
            let prepared = &captured.prepared;
            print(&json!({
                "api_revision":1,"context":prepared.identity,
                "goal":{"revision":prepared.snapshot.control.active_goal_revision,"content":prepared.snapshot.active_goal.owner_proof.request.content},
                "actor_observation_schema":prepared.snapshot.actor_observation_schema,
                "actor_observation":prepared.snapshot.actor_observation,
                "capabilities":captured.current_public_context.provider_authority.capability_catalog,
                "capture_commit":captured.capture_commit,
                "current_read_commit":captured.current_public_context.read_commit,
                "prepare_operation_id":operation,
                "action_correlation_identity":prepared.identity.decision_request_id
            }))
        }
        "move" => {
            let operation = required("--context")?;
            let values = required("--to")?
                .split(',')
                .map(str::parse::<i64>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid coordinates")?;
            let [x, y, z] = values.as_slice() else {
                return Err("--to requires x,y,z".into());
            };
            let action = oasis7::runtime::Action::MoveAgent {
                agent_id: client.agent.clone(),
                to: oasis7::GeoPos::new(*x, *y, *z),
            };
            let request = if let Some(request) =
                client.cached_intent(ClientStateKind::Action, operation)?
            {
                let WorldServicePayloadV1::Cognition(signed) = &request.signed_payload else {
                    return Err("invalid cached action".into());
                };
                if signed.request.action != action {
                    return Err(
                        "context already has a different original action; query its result".into(),
                    );
                }
                request
            } else {
                let captured = client.captured(operation)?;
                let signed = authority::sign_read_request(
                    "cognition",
                    cognition_intent(&captured, action)?,
                    &client.key_hex,
                )?;
                client.cache_intent(
                    ClientStateKind::Action,
                    operation,
                    WorldServicePayloadV1::Cognition(signed),
                )?
            };
            print(&client.submit_and_poll(&request)?)
        }
        "outcome" => {
            let operation = required("--context")?;
            let reason = required("--reason")?;
            if !["executor_aborted", "provider_failed", "transport_exhausted"].contains(&reason) {
                return Err("unsupported outcome; expired contexts require a fresh context and original result queries".into());
            }
            let request =
                if let Some(request) = client.cached_intent(ClientStateKind::Outcome, operation)? {
                    let WorldServicePayloadV1::Scheduler(signed) = &request.signed_payload else {
                        return Err("invalid cached outcome".into());
                    };
                    let SchedulerOperationV1::TaskProviderFailure {
                        reason: original, ..
                    } = &signed.request.operation
                    else {
                        return Err("invalid cached outcome".into());
                    };
                    if original != reason {
                        return Err("context already has a different original outcome".into());
                    }
                    request
                } else {
                    let captured = client.captured(operation)?;
                    let signed = authority::sign_read_request(
                        "scheduler",
                        outcome_intent(&captured, reason)?,
                        &client.key_hex,
                    )?;
                    client.cache_intent(
                        ClientStateKind::Outcome,
                        operation,
                        WorldServicePayloadV1::Scheduler(signed),
                    )?
                };
            print(&client.submit_and_poll(&request)?)
        }
        "lookup" => {
            let operation = required("--context")?;
            let kind = match required("--kind")? {
                "prefix" => ClientStateKind::Prefix,
                "action" => ClientStateKind::Action,
                "outcome" => ClientStateKind::Outcome,
                _ => return Err("invalid lookup kind".into()),
            };
            let request = client
                .cached_intent(kind, operation)?
                .ok_or("original operation is not in this device cache")?;
            print(&client.lookup(&request)?)
        }
        _ => unreachable!(),
    }
}
