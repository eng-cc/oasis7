use oasis7::simulator::WorldSnapshot;
use serde_json::Value;

pub(super) fn extend_agent_details_with_claim_lines(
    agent_id: &str,
    snapshot: &WorldSnapshot,
    lines: &mut Vec<String>,
) {
    lines.push("".to_string());
    lines.push("Agent Claim:".to_string());

    if extend_claim_lines_from_runtime_snapshot(agent_id, snapshot, lines) {
        return;
    }

    lines.push("- Status: unclaimed".to_string());
    extend_claim_quote_lines(agent_id, snapshot, lines);
}

fn extend_claim_lines_from_runtime_snapshot(
    agent_id: &str,
    snapshot: &WorldSnapshot,
    lines: &mut Vec<String>,
) -> bool {
    let Some(runtime_snapshot) = snapshot.runtime_snapshot.as_ref() else {
        return false;
    };
    let runtime_snapshot = runtime_snapshot_to_value(runtime_snapshot);
    let state = runtime_snapshot.get("state").and_then(Value::as_object);
    let Some(state) = state else {
        return false;
    };

    let current_epoch = agent_claim_epoch(
        value_u64(state.get("time")),
        value_u64(
            runtime_snapshot
                .get("governance_execution_policy")
                .and_then(|policy| policy.get("epoch_length_ticks")),
        ),
    );
    let Some(claim) = state
        .get("agent_claims")
        .and_then(|claims| claims.get(agent_id))
        .and_then(Value::as_object)
    else {
        return false;
    };

    let owner = claim
        .get("claim_owner_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    lines.push(format!("- Owner: {owner}"));
    lines.push(format!("- Status: {}", claim_status(claim, current_epoch)));
    lines.push(format!(
        "- Bond Locked: {} | Upkeep/Epoch: {}",
        value_u64(claim.get("locked_bond_amount")),
        value_u64(claim.get("upkeep_per_epoch"))
    ));
    lines.push(format!(
        "- Funding Mix: upfront restricted={} liquid={} | bond restricted={} liquid={}",
        value_u64(claim.get("upfront_restricted_spent_amount")),
        value_u64(claim.get("upfront_liquid_spent_amount")),
        value_u64(claim.get("claim_bond_locked_restricted_amount")),
        value_u64(claim.get("claim_bond_locked_liquid_amount"))
    ));

    if let Some(remaining) = claim
        .get("release_ready_at_epoch")
        .and_then(Value::as_u64)
        .map(|epoch| epoch.saturating_sub(current_epoch))
    {
        lines.push(format!("- Release Ready In Epochs: {remaining}"));
    }
    if let Some(remaining) = claim
        .get("grace_deadline_epoch")
        .and_then(Value::as_u64)
        .map(|epoch| epoch.saturating_sub(current_epoch))
    {
        lines.push(format!("- Grace Remaining Epochs: {remaining}"));
    }

    let last_control_epoch = state
        .get("agents")
        .and_then(|agents| agents.get(agent_id))
        .and_then(|cell| cell.get("last_active"))
        .and_then(Value::as_u64)
        .map(|last_active| {
            agent_claim_epoch(
                last_active,
                value_u64(
                    runtime_snapshot
                        .get("governance_execution_policy")
                        .and_then(|policy| policy.get("epoch_length_ticks")),
                ),
            )
        })
        .unwrap_or(current_epoch);
    let forced_reclaim_in_epochs = last_control_epoch
        .saturating_add(value_u64(claim.get("forced_idle_reclaim_epochs")))
        .saturating_sub(current_epoch);
    lines.push(format!(
        "- Forced Reclaim In Epochs: {forced_reclaim_in_epochs}"
    ));
    true
}

fn extend_claim_quote_lines(agent_id: &str, snapshot: &WorldSnapshot, lines: &mut Vec<String>) {
    let Some(primary_claim) = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
    else {
        let primary_agent = snapshot.model.agents.keys().next().map(String::as_str);
        let label = primary_agent.unwrap_or("unknown");
        lines.push(format!(
            "- Quote For {label}: unavailable (missing player gameplay snapshot)"
        ));
        return;
    };

    if let Some(owned_claim) = primary_claim
        .owned_claims
        .iter()
        .find(|claim| claim.target_agent_id == agent_id)
    {
        lines.push(format!("- Owner: {}", primary_claim.claimer_agent_id));
        lines.push(format!("- Status: {}", owned_claim.status));
        if let Some(remaining) = owned_claim.release_ready_in_epochs {
            lines.push(format!("- Release Ready In Epochs: {remaining}"));
        }
        if let Some(remaining) = owned_claim.grace_remaining_epochs {
            lines.push(format!("- Grace Remaining Epochs: {remaining}"));
        }
        if let Some(remaining) = owned_claim.forced_reclaim_in_epochs {
            lines.push(format!("- Forced Reclaim In Epochs: {remaining}"));
        }
        return;
    }

    let Some(quote) = primary_claim.next_claim_quote.as_ref() else {
        lines.push(format!(
            "- Quote For {}: unavailable",
            primary_claim.claimer_agent_id
        ));
        return;
    };

    lines.push(format!(
        "- Quote For {}: slot={} tier={} cap={} owned={} upfront={} upkeep={} liquid={} restricted={} eligible={}",
        primary_claim.claimer_agent_id,
        quote.slot_index,
        quote.reputation_tier,
        quote.claim_cap,
        quote.owned_claim_count,
        quote.total_upfront_amount,
        quote.upkeep_per_epoch,
        quote.transferable_liquid_balance,
        quote.restricted_starter_claim_balance,
        quote.eligible_claim_balance
    ));
    if let Some(reason) = quote.blocked_reason.as_deref() {
        lines.push(format!("- Claim Blocker: {reason}"));
    }
}

fn claim_status(claim: &serde_json::Map<String, Value>, current_epoch: u64) -> &'static str {
    if claim
        .get("grace_deadline_epoch")
        .is_some_and(|value| !value.is_null())
    {
        "upkeep_grace"
    } else if let Some(ready_at_epoch) = claim.get("release_ready_at_epoch").and_then(Value::as_u64)
    {
        if current_epoch >= ready_at_epoch {
            "release_ready"
        } else {
            "release_cooldown"
        }
    } else if claim
        .get("idle_warning_emitted_at_epoch")
        .is_some_and(|value| !value.is_null())
    {
        "idle_reclaim_candidate"
    } else {
        "claimed_active"
    }
}

fn agent_claim_epoch(time: u64, epoch_length_ticks: u64) -> u64 {
    time / epoch_length_ticks.max(1)
}

fn value_u64(value: Option<&Value>) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn runtime_snapshot_to_value(runtime_snapshot: &Value) -> Value {
    runtime_snapshot.clone()
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_snapshot_to_value<T>(runtime_snapshot: &T) -> Value
where
    T: serde::Serialize,
{
    serde_json::to_value(runtime_snapshot).unwrap_or(Value::Null)
}
