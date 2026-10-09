"""Validate canonical production-only evidence from existing live snapshots.

This checks observed runtime settlement, not independent durable commit proofs.
"""

import json

PROFILE = "starter-industrial-smelter-to-assembler-v1"
FACTORY = "factory.smelter.mk1"
RECIPE = "recipe.smelter.iron_ingot"


def state(payload):
    return ((payload.get("latest_snapshot") or {}).get("runtime_snapshot") or {}).get("state") or {}


def response(payload, kind):
    return next((item for item in payload.get("responses", []) if item.get("type") == kind), None)


def positive_int(value):
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def same_json(left, right):
    """Keep JSON numeric types distinct; Python equality merges bool/int/float."""
    return json.dumps(left, sort_keys=True) == json.dumps(right, sort_keys=True)


def validate(build, before, recipe, settled, recovery):
    prior, current, restored = state(before), state(settled), state(recovery)
    factory = (current.get("factories") or {}).get(FACTORY) or {}
    site = (current.get("factory_site_authorities") or {}).get(factory.get("site_id")) or {}
    milestone = (current.get("industry_progress") or {}).get("starter_industrial_milestone") or {}
    restored_milestone = (restored.get("industry_progress") or {}).get("starter_industrial_milestone") or {}
    ledger = milestone.get("output_ledger")
    receipt = (current.get("recipe_completion_receipts") or {}).get(str(milestone.get("settlement_job_id"))) or {}
    feasibility = (settled.get("player_gameplay") or {}).get("starter_industrial_feasibility") or {}
    restored_feasibility = (recovery.get("player_gameplay") or {}).get("starter_industrial_feasibility") or {}
    restored_factory = (restored.get("factories") or {}).get(FACTORY) or {}
    restored_site = (restored.get("factory_site_authorities") or {}).get(restored_factory.get("site_id")) or {}
    restored_receipt = (restored.get("recipe_completion_receipts") or {}).get(str(milestone.get("settlement_job_id"))) or {}
    amounts = [item.get("amount") for item in receipt.get("produce", []) if item.get("kind") == "iron_ingot"]
    produced = sum(amounts) if amounts and all(positive_int(amount) for amount in amounts) else 0
    build_ack = (response(build, "gameplay_action_ack") or {}).get("ack") or {}
    recipe_ack = (response(recipe, "gameplay_action_ack") or {}).get("ack") or {}
    old_ledger = (prior.get("material_ledgers") or {}).get(ledger)
    old_balance = old_ledger.get("iron_ingot", 0) if isinstance(old_ledger, dict) else None
    new_balance = (current.get("material_ledgers") or {}).get(ledger, {}).get("iron_ingot")
    restored_balance = (restored.get("material_ledgers") or {}).get(ledger, {}).get("iron_ingot")
    # Absent ledgers cannot be treated as a proven zero baseline.
    checks = {
        "canonical_build_accepted": bool(response(build, "gameplay_action_ack")) and not response(build, "gameplay_action_error"),
        "canonical_recipe_accepted": bool(response(recipe, "gameplay_action_ack")) and not response(recipe, "gameplay_action_error"),
        "canonical_no_milestone_before_recipe": bool(prior) and not (prior.get("industry_progress") or {}).get("starter_industrial_milestone"),
        "canonical_profile_revision": milestone.get("profile_id") == PROFILE and positive_int(milestone.get("profile_revision")) and milestone.get("profile_revision") == 1 and feasibility.get("profile_id") == PROFILE and positive_int(feasibility.get("profile_revision")) and feasibility.get("profile_revision") == 1 and feasibility.get("evidence_class") == "durable-milestone-backed",
        "canonical_ack_owner_binding": bool(factory.get("builder_agent_id")) and build_ack.get("target_agent_id") == factory.get("builder_agent_id") and recipe_ack.get("target_agent_id") == factory.get("builder_agent_id") and build_ack.get("action_id") == "build_factory_smelter_mk1" and recipe_ack.get("action_id") == "schedule_recipe_smelter_iron_ingot" and bool(build_ack.get("player_id")) and recipe_ack.get("player_id") == build_ack.get("player_id"),
        "canonical_matching_settlement": milestone.get("factory_id") == FACTORY and milestone.get("recipe_id") == RECIPE and all(positive_int(value) for value in [milestone.get("settlement_job_id"), receipt.get("job_id"), recipe_ack.get("runtime_action_id")]) and receipt.get("job_id") == milestone.get("settlement_job_id") and receipt.get("job_id") == recipe_ack.get("runtime_action_id") and receipt.get("factory_id") == FACTORY and receipt.get("recipe_id") == RECIPE and positive_int(receipt.get("accepted_batches")) and produced > 0,
        "canonical_owner_output": bool(factory.get("builder_agent_id")) and site.get("owner_agent_id") == factory.get("builder_agent_id") and ledger == "site:" + factory.get("site_id", "") and receipt.get("requester_agent_id") == factory.get("builder_agent_id") and receipt.get("output_ledger") == ledger and factory.get("output_ledger") == ledger,
        "canonical_positive_iron_credit": isinstance(old_balance, int) and not isinstance(old_balance, bool) and old_balance >= 0 and isinstance(new_balance, int) and not isinstance(new_balance, bool) and new_balance >= 0 and new_balance - old_balance >= produced > 0,
        "canonical_reconnect_same_milestone": bool(milestone) and positive_int(restored_milestone.get("settlement_job_id")) and positive_int(restored_milestone.get("profile_revision")) and same_json(restored_milestone, milestone),
        "canonical_reconnect_profile": restored_feasibility.get("profile_id") == PROFILE and positive_int(restored_feasibility.get("profile_revision")) and restored_feasibility.get("profile_revision") == 1 and restored_feasibility.get("evidence_class") == "durable-milestone-backed",
        "canonical_reconnect_owner_output": bool(factory.get("builder_agent_id")) and restored_factory.get("builder_agent_id") == factory.get("builder_agent_id") and restored_factory.get("site_id") == factory.get("site_id") and restored_site.get("owner_agent_id") == factory.get("builder_agent_id") and restored_factory.get("output_ledger") == ledger and restored_receipt.get("requester_agent_id") == factory.get("builder_agent_id") and restored_receipt.get("output_ledger") == ledger and positive_int(restored_receipt.get("job_id")) and restored_receipt.get("job_id") == milestone.get("settlement_job_id") and same_json(restored_receipt, receipt),
        "canonical_reconnect_output_preserved": bool(ledger) and isinstance(restored_balance, int) and not isinstance(restored_balance, bool) and restored_balance >= 0 and restored_balance == new_balance,
    }
    return {"scope": "live_runtime_production_only_and_reconnect", "checks": checks, "milestone": milestone, "iron_before": old_balance, "iron_after": new_balance, "produced_iron": produced}
