"""Validate canonical production-only evidence from existing live snapshots.

This checks observed runtime settlement, not independent durable commit proofs.
"""

import json

PROFILE = "starter-industrial-smelter-to-assembler-v1"
FACTORY = "factory.smelter.mk1"
RECIPE = "recipe.smelter.iron_ingot"


def bootstrap_failure(snapshot, action_id):
    """Describe an unavailable entry action without inventing runtime progress."""
    return {
        "status": "failed",
        "evidence_scope": "bootstrap_diagnostic_only",
        "checks": {"canonical_bootstrap_action_available": False},
        "blocker_kind": snapshot.get("blocker_kind") or "canonical_bootstrap_action_missing",
        "blocker_detail": snapshot.get("blocker_detail") or "canonical build action has no target agent",
        "required_action_id": action_id,
        "available_action_ids": [a.get("action_id") for a in snapshot.get("available_actions", [])],
        "canonical_production_verified": False,
    }


def state(payload):
    return ((payload.get("latest_snapshot") or {}).get("runtime_snapshot") or {}).get("state") or {}


def response(payload, kind):
    return next((item for item in payload.get("responses", []) if item.get("type") == kind), None)


def verify_recovery_ack(payload, player_id, public_key):
    ack = (response(payload, "authoritative_recovery_ack") or {}).get("ack") or {}
    if (response(payload, "authoritative_recovery_error") or
        ack.get("status") != "session_registered" or ack.get("player_id") != player_id or
        ack.get("session_pubkey") != public_key or not isinstance(ack.get("agent_id"), str) or
        not ack["agent_id"].strip()):
        raise ValueError("session registration did not acknowledge the authenticated bound Agent")
    return ack["agent_id"]


def nonempty_str(value):
    return isinstance(value, str) and bool(value.strip())


def positive_int(value):
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def same_json(left, right):
    """Keep JSON numeric types distinct; Python equality merges bool/int/float."""
    return json.dumps(left, sort_keys=True) == json.dumps(right, sort_keys=True)


def validate(build, before, recipe, settled, recovery, registration=None):
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
    origin = receipt.get("committed_recipe_origin") or {}
    submission = origin.get("submission") or {}
    registered = (response(registration or {}, "authoritative_recovery_ack") or {}).get("ack") or {}
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
        "canonical_ack_owner_binding": nonempty_str(factory.get("builder_agent_id")) and build_ack.get("target_agent_id") == factory.get("builder_agent_id") and recipe_ack.get("target_agent_id") == factory.get("builder_agent_id") and build_ack.get("action_id") == "build_factory_smelter_mk1" and recipe_ack.get("action_id") == "schedule_recipe_smelter_iron_ingot" and nonempty_str(build_ack.get("player_id")) and recipe_ack.get("player_id") == build_ack.get("player_id"),
        "canonical_matching_settlement": milestone.get("factory_id") == FACTORY and milestone.get("recipe_id") == RECIPE and all(positive_int(value) for value in [milestone.get("settlement_job_id"), receipt.get("job_id"), recipe_ack.get("runtime_action_id")]) and receipt.get("job_id") == milestone.get("settlement_job_id") and origin.get("consensus_action_id") == recipe_ack.get("runtime_action_id") and receipt.get("factory_id") == FACTORY and receipt.get("recipe_id") == RECIPE and positive_int(receipt.get("accepted_batches")) and produced > 0,
        "canonical_exact_submission_origin": all(nonempty_str(value) for value in [origin.get("consensus_submitter_player_id"), submission.get("verified_player_id"), submission.get("requester_agent_id"), recipe_ack.get("player_id"), recipe_ack.get("target_agent_id"), registered.get("player_id"), registered.get("agent_id")]) and all(positive_int(value) for value in [origin.get("consensus_action_id"), origin.get("committed_height"), submission.get("auth_nonce")]) and isinstance(origin.get("action_payload_hash"), str) and len(origin["action_payload_hash"]) == 64 and all(c in "0123456789abcdef" for c in origin["action_payload_hash"]) and origin.get("action_payload_hash") == recipe_ack.get("consensus_action_payload_hash") and isinstance(origin.get("action_root"), str) and len(origin["action_root"]) == 64 and all(c in "0123456789abcdef" for c in origin["action_root"]) and nonempty_str(origin.get("consensus_submitter_player_id")) and submission.get("verified_player_id") == recipe_ack.get("player_id") and registered.get("status") == "session_registered" and not response(registration or {}, "authoritative_recovery_error") and registered.get("player_id") == recipe_ack.get("player_id") and registered.get("agent_id") == recipe_ack.get("target_agent_id") and isinstance(submission.get("public_key"), str) and len(submission["public_key"]) == 64 and all(c in "0123456789abcdef" for c in submission["public_key"]) and submission.get("public_key") == registered.get("session_pubkey") and (submission.get("hosted_registration_nonce") is None or nonempty_str(submission.get("hosted_registration_nonce"))) and submission.get("requester_agent_id") == factory.get("builder_agent_id") and submission.get("factory_id") == FACTORY and submission.get("recipe_id") == RECIPE and same_json(milestone.get("committed_recipe_origin"), origin),
        "canonical_owner_output": nonempty_str(factory.get("builder_agent_id")) and site.get("owner_agent_id") == factory.get("builder_agent_id") and ledger == "site:" + factory.get("site_id", "") and receipt.get("requester_agent_id") == factory.get("builder_agent_id") and receipt.get("output_ledger") == ledger and factory.get("output_ledger") == ledger,
        "canonical_positive_iron_credit": isinstance(old_balance, int) and not isinstance(old_balance, bool) and old_balance >= 0 and isinstance(new_balance, int) and not isinstance(new_balance, bool) and new_balance >= 0 and new_balance - old_balance >= produced > 0,
        "canonical_reconnect_same_origin": bool(origin) and same_json(restored_milestone.get("committed_recipe_origin"), origin) and same_json(restored_receipt.get("committed_recipe_origin"), origin),
        "canonical_reconnect_same_milestone": bool(milestone) and positive_int(restored_milestone.get("settlement_job_id")) and positive_int(restored_milestone.get("profile_revision")) and same_json(restored_milestone, milestone),
        "canonical_reconnect_profile": restored_feasibility.get("profile_id") == PROFILE and positive_int(restored_feasibility.get("profile_revision")) and restored_feasibility.get("profile_revision") == 1 and restored_feasibility.get("evidence_class") == "durable-milestone-backed",
        "canonical_reconnect_owner_output": nonempty_str(factory.get("builder_agent_id")) and restored_factory.get("builder_agent_id") == factory.get("builder_agent_id") and restored_factory.get("site_id") == factory.get("site_id") and restored_site.get("owner_agent_id") == factory.get("builder_agent_id") and restored_factory.get("output_ledger") == ledger and restored_receipt.get("requester_agent_id") == factory.get("builder_agent_id") and restored_receipt.get("output_ledger") == ledger and positive_int(restored_receipt.get("job_id")) and restored_receipt.get("job_id") == milestone.get("settlement_job_id") and same_json(restored_receipt, receipt),
        "canonical_reconnect_output_preserved": bool(ledger) and isinstance(restored_balance, int) and not isinstance(restored_balance, bool) and restored_balance >= 0 and restored_balance == new_balance,
    }
    return {"scope": "live_runtime_production_only_and_reconnect", "checks": checks, "milestone": milestone, "iron_before": old_balance, "iron_after": new_balance, "produced_iron": produced}
