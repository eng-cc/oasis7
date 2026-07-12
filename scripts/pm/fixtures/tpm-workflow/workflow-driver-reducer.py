#!/usr/bin/env python3
# Test-only reducer/fault-injection implementation; never a production entrypoint.
"""Durable, restartable TPM lifecycle reducer.

The driver owns workflow progress, not professional decisions.  Every remote
mutation is journaled as intent -> action -> readback -> committed and uses a
stable idempotency key.  Capability and human decisions are persisted waits.
"""

from __future__ import annotations

import argparse
import datetime as dt
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Optional
ROOT = Path(__file__).resolve().parents[4]

EX_TEMPFAIL = 75
EX_CRASH = 86
PHASES = ["bootstrap", "route", "dispatch", "execute", "integrate", "freeze", "verify",
          "review", "closeout", "create_pr", "record_pr", "comment", "watch", "fix",
          "reverify", "push", "merge", "merge_receipt", "task_done", "main_sync", "safe_cleanup"]
REQUIRED_PHASES = ("bootstrap", "route", "execute", "integrate", "freeze", "verify",
                   "review", "closeout", "create_pr", "watch", "fix", "reverify", "push",
                   "merge", "task_done", "main_sync", "safe_cleanup")
HELPERS = {
    "bootstrap": "scripts/new-task-worktree.sh", "verify": "scripts/pm/claim-ready.sh",
    "closeout": "scripts/pm/task-closeout.sh", "watch": "scripts/pm/pr-lifecycle-gate.py",
    "task_done": "scripts/pm/task-closeout.sh --status done",
    "safe_cleanup": "scripts/pm/post-merge-cleanup.sh",
    "create_pr": "scripts/prepare-task-pr.sh --create",
}


def action_for(state: dict, phase: str) -> dict:
    command = HELPERS.get(phase)
    argv = command.split() if command else [str((ROOT / "scripts/pm/fixtures/tpm-workflow/workflow-driver.py").resolve()), "--typed-action", phase]
    if argv:
        executable = argv[0]
        candidate = ROOT / executable
        if "/" in executable:
            valid_executable = candidate.is_file() and os.access(candidate, os.X_OK)
        else:
            valid_executable = shutil.which(executable) is not None
        if not valid_executable:
            argv = []
    action_id = f'{state["task_uid"]}:{phase}:r{state.get("revision", 0)}'
    typed_operations = {
        "route": ("route_workflow", "tpm_router", ["task_uid", "task_truth"]),
        "dispatch": ("dispatch_professional_slices", "tpm_subagent_scheduler", ["slice_contracts", "integration_order"]),
        "execute": ("coordinate_execution", "professional_slice_returns", ["execution_plan", "slice_receipts"]),
        "integrate": ("integrate_slice_evidence", "tpm_integrator", ["slice_receipts", "provenance"]),
        "review": ("dispatch_repo_review", "repo_owned_review", ["review_roles", "frozen_head"]),
        "fix": ("route_review_fix", "tpm_fix_router", ["findings", "owner_role"]),
        "reverify": ("run_fresh_verification", "verification_helper", ["verification_profile", "head"]),
        "push": ("push_reviewed_head", "git_remote_helper", ["branch", "reviewed_head"]),
    }
    operation, producer, inputs = typed_operations.get(phase, (f"run_{phase}_stage", "repo_helper", ["task_uid", "phase"]))
    return {"action_id": action_id, "phase": phase,
            "action_type": "helper" if command else "typed_tpm_action",
            "command": argv, "args": argv[1:] if argv else [],
            "dispatch_operation": operation, "operation_schema": f"tpm-{phase}-operation/v1",
            "producer_surface": producer, "required_inputs": inputs,
            "stage_validator": f"validate_{phase}_canonical_operation_v1",
            "receipt_schema": "tpm-stage-action/v1",
            "readback_validator": f"validate_{phase}_identity_and_live_readback_v1",
            "required_readback_fields": ["schema", "task_uid", "phase", "repo", "worktree",
                                         "pr", "head", "epoch", "receipt_digest"]}


def require_action(state: dict, phase: str, now: dt.datetime) -> None:
    action = action_for(state, phase)
    state["status"] = "action_required"; state["next_action"] = phase
    state["blocker"] = {"class": "action_required", "resume_condition": "submit stage-bound live readback receipt",
                        "escalation": "TPM executes the recorded command"}
    state["transitions"][phase] = {"at": iso(now), "action_required": action,
                                    "helper_invocation": action["command"]}


def ingest_action(state: dict, phase: str, receipt_path: Path) -> str:
    action = state.get("transitions", {}).get(phase, {}).get("action_required")
    try: receipt = json.loads(receipt_path.read_text())
    except (OSError, json.JSONDecodeError): return "invalid_action_receipt"
    if not isinstance(action, dict): return "invalid_action_receipt"
    required = (receipt.get("schema") == action["receipt_schema"] and
                receipt.get("action_id") == action["action_id"] and
                receipt.get("phase") == phase and receipt.get("command") == action["command"] and
                receipt.get("exit_code") == 0 and receipt.get("validator") == action["readback_validator"])
    readback = receipt.get("readback", {})
    if not required or readback.get("task_uid") != state["task_uid"]: return "invalid_action_receipt"
    if phase != "bootstrap" and readback.get("phase") != phase: return "invalid_action_receipt"
    fixture_validator=os.environ.get("TPM_LIVE_RECEIPT_VALIDATOR")
    if fixture_validator:
        proc=subprocess.run([fixture_validator,str(receipt_path)],input=json.dumps(receipt),text=True,capture_output=True)
        if proc.returncode: return "invalid_live_readback"
    # Caller JSON is a locator. Bootstrap is the only locally verifiable stage
    # currently implemented; every other production stage remains action_required.
    if phase == "bootstrap":
        worktree = Path(str(readback.get("worktree", "")))
        if not readback.get("worktree"): return "invalid_live_readback"
        if not worktree.is_dir() or not (worktree / ".git").exists(): return "invalid_live_readback"
        probe = subprocess.run(["git", "-C", str(worktree), "rev-parse", "--show-toplevel"],
                               text=True, capture_output=True)
        if probe.returncode != 0 or Path(probe.stdout.strip()).resolve() != worktree.resolve(): return "invalid_live_readback"
        mapping = worktree / ".pm/github-project-sync/tasks.json"
        try: mapping_text = mapping.read_text()
        except OSError: return "invalid_live_readback"
        if state["task_uid"] not in mapping_text: return "invalid_live_readback"
        audit = subprocess.run([str(worktree / "scripts/pm/github-project-workflow.sh"), "audit",
                                "--task-uid", state["task_uid"], "--json"],
                               cwd=str(worktree), text=True, capture_output=True)
        if audit.returncode != 0: return "invalid_live_readback"
        ok, _ = hydrate_task_authority(state)
        if not ok: return "task_authority_readback_failed"
    else:
        return "production_live_connector_unavailable"
    journal = {"state": "committed", "intent": action, "readback": readback, "effect_count": 1}
    state["transition_journal"][phase] = journal
    state["transitions"][phase] = {"receipt": receipt, "helper_invocation": action["command"]}
    state["phase"] = phase; state["status"] = "running"; state["blocker"] = None
    return "committed"


def iso(value: dt.datetime) -> str:
    return value.astimezone(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def parse_clock() -> dt.datetime:
    return dt.datetime.fromisoformat(os.environ.get("TPM_TEST_CLOCK", iso(dt.datetime.now(dt.timezone.utc))).replace("Z", "+00:00"))


def duration_seconds(value: str) -> int:
    if not value.startswith("PT"):
        raise ValueError("only ISO-8601 PT durations are supported")
    body = value[2:]
    if body.endswith("M"):
        return int(body[:-1]) * 60
    if body.endswith("S"):
        return int(body[:-1])
    if body.endswith("H"):
        return int(body[:-1]) * 3600
    raise ValueError(f"unsupported duration: {value}")


def save(path: Path, state: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    state["revision"] = int(state.get("revision", 0)) + 1
    if path.exists():
        backup = path.with_suffix(path.suffix + ".bak")
        backup.write_bytes(path.read_bytes())
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")
    tmp.replace(path)


def emit(state: dict, code: int = 0) -> int:
    print(json.dumps(state, sort_keys=True))
    return code


def adapter(operation: str, payload: dict) -> tuple[int, dict]:
    return EX_TEMPFAIL, {"ok": False, "status": 503, "reason": "production_github_connector_unavailable"}


def initial(task_uid: str, now: dt.datetime, args: argparse.Namespace) -> dict:
    return {
        "schema": 2, "revision": 0, "task_uid": task_uid, "status": "running", "phase": "initialized",
        "next_action": "bootstrap", "wake_at": iso(now),
        "lease": {"id": str(uuid.uuid4()), "token": str(uuid.uuid4()), "acquired_at": iso(now), "expires_at": iso(now + dt.timedelta(minutes=5))},
        "attempt": 1, "retry": {"attempt": 0, "delay_seconds": 0}, "blocker": None,
        "heartbeat": {"at": iso(now), "phase": "initialized"}, "remote_journal": {}, "transition_journal": {},
        "transitions": {},
        "completed_transitions": [], "slices": [], "pr_head": args.pr_head or "0" * 40,
        "pr_state": "NONE", "task_state": "in_progress", "worktree_state": "active",
    }


def remote_once(state_path: Path, state: dict, operation: str, payload: dict,
                crash_after: str | None, now: dt.datetime) -> tuple[bool, int]:
    journal = state["remote_journal"].get(operation)
    key = f'{state["task_uid"]}:{operation}'
    if journal is None:
        journal = {"state": "intent", "intent": {**payload, "idempotency_key": key}, "created_at": iso(now)}
        state["remote_journal"][operation] = journal
        save(state_path, state)
    if journal["state"] != "committed":
        rc, response = adapter(operation, journal["intent"])
        if rc != 0 or not response.get("ok"):
            status = int(response.get("status", 599))
            delay = int(response.get("retry_after") or min(300, 2 ** min(8, state["retry"]["attempt"] + 1)))
            state["status"] = "external_wait"
            state["retry"] = {"attempt": state["retry"]["attempt"] + 1, "delay_seconds": delay,
                              "status": status, "operation": operation}
            state["wake_at"] = iso(now + dt.timedelta(seconds=delay))
            state["blocker"] = {"class": "github_transient_failure", "operation": operation,
                                "resume_condition": "wake_at reached and GitHub adapter is available",
                                "escalation": "report after retry budget exhaustion"}
            state["heartbeat"] = {"at": iso(now), "phase": state["phase"], "operation": operation}
            save(state_path, state)
            return False, EX_TEMPFAIL
        journal["state"] = "acted"
        journal["readback"] = response["result"]
        save(state_path, state)
        if crash_after == operation:
            return False, EX_CRASH
        journal["state"] = "committed"
        journal["committed_at"] = iso(now)
        journal["effect_count"] = 1
        state["transition_journal"][operation] = journal
        save(state_path, state)
    return True, 0


def schedule_slices(state: dict, args: argparse.Namespace, now: dt.datetime) -> None:
    if not state["slices"] and args.fixture_slices:
        for number in range(args.fixture_slices):
            state["slices"].append({"id": f"slice-{number + 1}", "state": "planned", "attempt": 1,
                                    "deadline": iso(now + dt.timedelta(minutes=30))})
    if args.inject_no_payload:
        original = next((x for x in state["slices"] if x["id"] == args.inject_no_payload), None)
        if original and original["state"] not in {"superseded", "integrated", "terminal"}:
            original["state"] = "superseded"
            state["slices"].append({"id": original["id"] + "-retry-2", "supersedes": original["id"],
                                    "state": "planned", "attempt": 2,
                                    "deadline": iso(now + dt.timedelta(minutes=30))})
    for item in list(state["slices"]):
        deadline = dt.datetime.fromisoformat(item["deadline"].replace("Z", "+00:00"))
        if item["state"] in {"running", "dispatched"} and now > deadline:
            if item.get("attempt", 1) < 2:
                item["state"] = "superseded"
                if not any(x.get("supersedes") == item["id"] for x in state["slices"]):
                    state["slices"].append({"id": item["id"] + "-retry-2", "supersedes": item["id"],
                                            "state": "planned", "attempt": 2,
                                            "deadline": iso(now + dt.timedelta(minutes=30))})
            else: item["state"] = "terminal"
    width = args.slice_batch_width or state.get("slice_batch_width", 2)
    state["slice_batch_width"] = width
    active = sum(x["state"] in {"dispatched", "running"} for x in state["slices"])
    for item in state["slices"]:
        if active >= width:
            break
        if item["state"] == "planned":
            item["state"] = "running"
            active += 1


def set_durable_wait(state: dict, blocker: str, now: dt.datetime) -> None:
    capability_blockers = {"dispatch_attestation_unavailable"}
    state["status"] = ("capability_blocked"
                       if blocker in capability_blockers
                       else "external_wait")
    state["blocker"] = {"class": blocker,
                        "resume_condition": ("runtime-verifiable dispatch attestation becomes available"
                                             if blocker == "dispatch_attestation_unavailable"
                                             else "required human approval is recorded in canonical task truth"),
                        "escalation": "persist heartbeat and notify TPM owner without terminating lifecycle"}
    state["wake_at"] = iso(now + dt.timedelta(minutes=15))
    state["heartbeat"] = {"at": iso(now), "phase": state["phase"], "blocker": blocker}


def migrate_hold(state_path: Path, state: dict, head: str, crash: str | None, now: dt.datetime) -> tuple[bool, int]:
    old = state.get("canonical_hold")
    pending_update = state["remote_journal"].get("update_comment")
    if pending_update and pending_update.get("state") == "acted":
        pending_update["state"] = "committed"
        state["transition_journal"]["update_comment"] = pending_update
        save(state_path, state)
    if old and old["head_oid"] != head and old["disposition"] != "superseded":
        payload = {**old, "disposition": "superseded", "superseded_by_head": head,
                   "idempotency_key": old["idempotency_key"]}
        update_journal = state["remote_journal"].get("update_comment")
        if not update_journal or update_journal.get("intent") != payload:
            update_journal = {"state": "intent", "intent": payload, "effect_count": 0}
            state["remote_journal"]["update_comment"] = update_journal
            save(state_path, state)
        if update_journal["state"] == "intent":
            rc, response = adapter("update_comment", payload)
            if rc != 0 or not response.get("ok"): return False, EX_TEMPFAIL
            update_journal["state"] = "acted"; update_journal["readback"] = response["result"]
            update_journal["effect_count"] = 1
        old["disposition"] = "superseded"
        state["transition_journal"]["update_comment"] = update_journal
        save(state_path, state)
        if crash == "update_comment": return False, EX_CRASH
        update_journal["state"] = "committed"; save(state_path, state)
    if not old or old["head_oid"] != head:
        key = f'{state["task_uid"]}:normal-hold:{head}'
        intent = {"idempotency_key": key, "kind": "normal_pr_ci_watch",
                  "head_oid": head, "disposition": "inactive",
                  "source": "github_task_issue_comment"}
        journal = {"state": "intent", "intent": intent, "created_at": iso(now)}
        state["remote_journal"]["comment"] = journal
        save(state_path, state)
        rc, response = adapter("comment", intent)
        if rc != 0 or not response.get("ok"): return False, EX_TEMPFAIL
        journal["state"] = "acted"; journal["readback"] = response["result"]
        state["canonical_hold"] = response["result"]
        save(state_path, state)
        if crash in {"comment", "update_comment"}:
            state["transition_journal"]["update_comment"] = journal
            save(state_path, state)
            return False, EX_CRASH
        journal["state"] = "committed"; journal["committed_at"] = iso(now)
        journal["effect_count"] = 1
        state["transition_journal"]["update_comment"] = journal
        save(state_path, state)
    return True, 0


def transition(state: dict, phase: str, now: dt.datetime, *, action_required: str | None = None) -> None:
    record = {"at": iso(now)}
    if action_required:
        record["action_required"] = action_required
    else:
        record["receipt"] = {"kind": f"{phase}_receipt", "task_uid": state["task_uid"], "at": iso(now)}
    if phase in HELPERS:
        record["helper_invocation"] = HELPERS[phase]
    elif phase in {"dispatch", "execute", "integrate", "freeze", "review", "fix", "reverify", "push",
                  "merge", "merge_receipt", "main_sync"}:
        record["action_required"] = action_for(state, phase)
    state["transitions"][phase] = record


def local_once(state_path: Path, state: dict, operation: str, now: dt.datetime,
               crash_after: str | None) -> bool:
    journal = state["transition_journal"].setdefault(operation, {
        "state": "intent", "intent": {"operation": operation}, "effect_count": 0})
    if journal["state"] != "committed":
        journal["state"] = "acted"; journal["effect_count"] = 1
        save(state_path, state)
        if crash_after == operation:
            return False
        journal["state"] = "committed"; journal["readback"] = {"verified": True, "at": iso(now)}
        save(state_path, state)
    return True


def gate_state() -> dict:
    remote = os.environ.get("TPM_GITHUB_STATE")
    if remote and Path(remote).exists():
        return json.loads(Path(remote).read_text()).get("gate", {})
    return {}


def live_comment(node_id: str) -> Optional[dict]:
    rc, response = adapter("read_comment", {"node_id": node_id})
    if rc != 0 or not response.get("ok") or not isinstance(response.get("result"), dict): return None
    return response["result"]


def hydrate_task_authority(state: dict) -> tuple[bool, dict]:
    _, response = adapter("read_task_authority", {"task_uid": state["task_uid"]})
    authority = response.get("result") if response.get("ok") else None
    valid = (isinstance(authority, dict) and authority.get("task_uid") == state["task_uid"] and
             authority.get("readback_verified") is True and isinstance(authority.get("resume_authorities"), list) and
             authority.get("required_resume_permission") in {"triage", "write", "maintain", "admin"} and
             authority.get("evidence_node_id"))
    if not valid: return False, response
    state["task_truth"] = {"resume_authorities": authority["resume_authorities"],
        "required_resume_permission": authority["required_resume_permission"], "repo": authority.get("repo"),
        "issue": authority.get("issue"), "issue_author": authority.get("issue_author"),
        "issue_author_permission": authority.get("issue_author_permission"),
        "authority_evidence_node_id": authority["evidence_node_id"], "readback_verified": True}
    return True, response


def typed_action_main(argv: list[str]) -> int:
    phase = argv[argv.index("--typed-action") + 1]
    if phase not in PHASES:
        print(json.dumps({"status": "invalid_typed_action", "phase": phase}))
        return 2
    try: payload = json.loads(sys.stdin.read() or "{}")
    except json.JSONDecodeError:
        print(json.dumps({"status": "invalid_inputs", "phase": phase}))
        return 2
    action = action_for({"task_uid": str(payload.get("task_uid", "typed-action")), "revision": 0}, phase)
    missing = [key for key in action["required_inputs"] if key not in payload]
    result = {"status": "produced" if not missing else "inputs_incomplete",
              "operation_schema": action["operation_schema"], "producer_surface": action["producer_surface"],
              "inputs": payload, "missing_inputs": missing}
    print(json.dumps({"action": {"phase": phase, "operation": action["dispatch_operation"],
                                  "operation_schema": action["operation_schema"]}, "result": result}, sort_keys=True))
    return 0


def main() -> int:
    if "--typed-action" in sys.argv:
        return typed_action_main(sys.argv[1:])
    p = argparse.ArgumentParser()
    p.add_argument("--state", type=Path, required=True); p.add_argument("--json", action="store_true")
    mode = p.add_mutually_exclusive_group(required=True); mode.add_argument("--initialize", action="store_true"); mode.add_argument("--resume", action="store_true"); mode.add_argument("--serve", action="store_true")
    p.add_argument("--task-uid"); p.add_argument("--stop-after"); p.add_argument("--run-to-completion", action="store_true")
    p.add_argument("--crash-after-remote", choices=("create_task", "create_pr", "comment", "update_comment", "merge"))
    p.add_argument("--crash-after-local", choices=("task_done", "main_sync", "safe_cleanup"))
    p.add_argument("--fixture-slices", type=int, default=0)
    p.add_argument("--slice-batch-width", type=int); p.add_argument("--advance-clock"); p.add_argument("--inject-no-payload")
    p.add_argument("--pr-head"); p.add_argument("--inject-blocker"); p.add_argument("--resolve-blocker")
    p.add_argument("--complete-action"); p.add_argument("--complete-slice"); p.add_argument("--receipt-file", type=Path)
    p.add_argument("--expected-revision", type=int); p.add_argument("--lease-token")
    p.add_argument("--describe-actions", action="store_true"); p.add_argument("--once", action="store_true")
    p.add_argument("--delivery-ack-file", type=Path)
    args = p.parse_args()
    now = parse_clock()
    if args.advance_clock: now += dt.timedelta(seconds=duration_seconds(args.advance_clock))
    if args.initialize:
        if not args.task_uid: p.error("--task-uid is required with --initialize")
        state = initial(args.task_uid, now, args); save(args.state, state)
    else:
        try:
            state = json.loads(args.state.read_text())
        except (json.JSONDecodeError, OSError):
            backup = args.state.with_suffix(args.state.suffix + ".bak")
            try:
                state = json.loads(backup.read_text()); state["recovered_from"] = str(backup)
            except (json.JSONDecodeError, OSError):
                blocked = {"schema": 2, "revision": 0, "status": "external_wait",
                           "lease": {"token": "unavailable"},
                           "blocker": {"class": "checkpoint_corrupt",
                                       "resume_condition": "restore a valid checkpoint generation",
                                       "escalation": "repository_health_engineer recovery"}}
                return emit(blocked, EX_TEMPFAIL)
        if state.get("schema") != 2:
            state["status"] = "external_wait"
            state["blocker"] = {"class": "checkpoint_schema_incompatible",
                                "resume_condition": "migrate checkpoint to supported schema 2",
                                "escalation": "repository_health_engineer migration"}
            return emit(state, EX_TEMPFAIL)
        if args.expected_revision is not None and args.expected_revision != state.get("revision"):
            state["blocker"] = {"class": "revision_conflict", "resume_condition": "reload checkpoint revision",
                                "escalation": "retry with current revision"}
            return emit(state, EX_TEMPFAIL)
        if args.lease_token is not None and args.lease_token != state.get("lease", {}).get("token"):
            state["blocker"] = {"class": "lease_token_mismatch", "resume_condition": "use current lease token",
                                "escalation": "reload checkpoint ownership"}
            return emit(state, EX_TEMPFAIL)
        state["attempt"] += 1
    if args.describe_actions:
        actions = {}
        for phase in PHASES:
            action = action_for(state, phase)
            if action["command"]:
                actions[phase] = {**action, "status": "action_required"}
            else:
                actions[phase] = {"status": "blocked", "blocker": {"class": "helper_unavailable"}}
        return emit({"actions": actions})
    if args.serve:
        if args.expected_revision != state.get("revision") or args.lease_token != state.get("lease", {}).get("token"):
            state["blocker"] = {"class": "cas_required"}; return emit(state, EX_TEMPFAIL)
        schedule = state.get("transitions", {}).get("dispatch", {}).get("schedule")
        if not schedule or not args.delivery_ack_file:
            state["blocker"] = {"class": "scheduler_delivery_unavailable"}; return emit(state, EX_TEMPFAIL)
        wake = dt.datetime.fromisoformat(schedule["wake_at"].replace("Z", "+00:00"))
        if now < wake:
            state["status"] = "external_wait"; state["blocker"] = {"class": "scheduler_not_due"}
            return emit(state, EX_TEMPFAIL)
        state["status"]="capability_blocked"; state["blocker"]={"class":"scheduler_delivery_connector_unavailable"}; save(args.state,state); return emit(state,EX_TEMPFAIL)
        ack["at"] = iso(now)
        args.delivery_ack_file.write_text(json.dumps(ack, sort_keys=True))
        schedule["delivery_ack"] = ack
        # Consume the ack inside the current lock instead of recursively
        # invoking the driver. The consumer is bound to the caller's CAS epoch.
        current_index = PHASES.index(state["phase"]) if state["phase"] in PHASES else -1
        result_phase = PHASES[min(current_index + 1, len(PHASES) - 1)]
        transition(state, result_phase, now)
        state["phase"] = result_phase; state["next_action"] = PHASES[min(current_index + 2, len(PHASES) - 1)]
        state["status"] = "action_required" if state["transitions"][result_phase].get("action_required") else "running"
        schedule["consumer_run"] = {"delivery_id": schedule["delivery_id"],
                                    "expected_revision": args.expected_revision,
                                    "lease_token": args.lease_token, "status": state["status"],
                                    "result_phase": result_phase}
        save(args.state, state); return emit(state)
    production_actions = True
    if (args.resume and not args.resolve_blocker and
            (args.expected_revision is None or args.lease_token is None)):
        state["status"] = "external_wait"
        state["blocker"] = {"class": "cas_required",
                            "resume_condition": "retry with expected revision and current lease token",
                            "escalation": "reload checkpoint before production resume"}
        return emit(state, EX_TEMPFAIL)
    if args.resolve_blocker:
        state["status"] = "external_wait"
        state["blocker"] = {"class": "canonical_evidence_required",
                            "resume_condition": "ingest canonical evidence receipt",
                            "escalation": "read back GitHub task issue evidence"}
        save(args.state, state); return emit(state, EX_TEMPFAIL)
    if args.complete_action == "resolve_wait":
        try: evidence = json.loads(args.receipt_file.read_text())
        except Exception: evidence = {}
        live = live_comment(str(evidence.get("node_id", "")))
        body = (live or {}).get("body", "")
        body_digest = hashlib.sha256(body.encode()).hexdigest() if isinstance(body, str) else ""
        if live is None:
            state["blocker"] = {"class": "canonical_evidence_readback_failed"}; return emit(state, EX_TEMPFAIL)
        if evidence.get("authority") != live.get("author"):
            state["blocker"] = {"class": "unauthorized_evidence_author"}; return emit(state, EX_TEMPFAIL)
        if not evidence.get("digest"):
            state["blocker"] = {"class": "evidence_digest_required"}; return emit(state, EX_TEMPFAIL)
        truth = state.get("task_truth", {})
        authorities = truth.get("resume_authorities", [])
        if live.get("author") not in authorities:
            state["blocker"] = {"class": "resume_authority_not_allowed"}; return emit(state, EX_TEMPFAIL)
        levels = {"read": 0, "triage": 1, "write": 2, "maintain": 3, "admin": 4}
        if levels.get(live.get("author_permission"), -1) < levels.get(truth.get("required_resume_permission"), 99):
            state["blocker"] = {"class": "resume_permission_insufficient"}; return emit(state, EX_TEMPFAIL)
        valid = (evidence.get("schema") == "tpm-canonical-evidence/v1" and
                 evidence.get("task_uid") == state["task_uid"] and evidence.get("readback_verified") is True and
                 evidence.get("source") == "github_task_issue_comment" and live is not None and
                 live.get("task_uid") == state["task_uid"] and
                 live.get("blocker") == state.get("blocker", {}).get("class") and
                 evidence.get("digest") == body_digest and
                 evidence.get("repo") == live.get("repo") and evidence.get("issue") == live.get("issue") and
                 evidence.get("created_at") == live.get("created_at"))
        if not valid:
            state["blocker"] = {"class": "canonical_evidence_readback_failed"}; return emit(state, EX_TEMPFAIL)
        state["blocker"] = None; state["status"] = "running"; save(args.state, state)
        args.complete_action = None
    if args.complete_slice:
        try: receipt = json.loads(args.receipt_file.read_text())
        except Exception: receipt = {}
        item = next((x for x in state["slices"] if x["id"] == args.complete_slice), None)
        live = live_comment(str(receipt.get("node_id", "")))
        if (not item or receipt.get("schema") != "tpm-slice-return/v1" or
                receipt.get("slice_id") != item["id"] or receipt.get("attempt") != item["attempt"] or
                live is None or live.get("slice_id") != item["id"] or
                live.get("attempt") != item["attempt"] or not live.get("dispatch_attestation")):
            state["blocker"] = {"class": "slice_live_readback_required"}; save(args.state, state); return emit(state, EX_TEMPFAIL)
        item["state"] = "integrated"; item["return_receipt"] = receipt; state["blocker"] = None; state["status"] = "running"
        # Superseded predecessors do not participate in the integration barrier.
        save(args.state, state)
    if production_actions:
        if args.complete_action:
            phase = args.complete_action
            result = ingest_action(state, phase, args.receipt_file) if args.receipt_file else "invalid_action_receipt"
            if result != "committed":
                state["status"] = "external_wait"; state["blocker"] = {"class": result,
                    "resume_condition": "submit exact stage-bound helper readback", "escalation": "rerun recorded action"}
                if result == "task_authority_readback_failed": save(args.state, state)
                return emit(state, EX_TEMPFAIL)
            save(args.state, state)
        elif state.get("status") == "action_required":
            save(args.state, state); return emit(state, EX_TEMPFAIL)
        start_index = PHASES.index(state["phase"]) + 1 if state["phase"] in PHASES else 0
        if start_index >= len(PHASES):
            state["status"] = "completed"; state["next_action"] = None; save(args.state, state)
            return emit(state, EX_TEMPFAIL)
        next_phase = PHASES[start_index]
        require_action(state, next_phase, now); save(args.state, state); return emit(state, EX_TEMPFAIL)
    if state.get("status") in {"external_wait", "capability_blocked"} and state.get("blocker") and not args.resolve_blocker:
        wake = dt.datetime.fromisoformat(state["wake_at"].replace("Z", "+00:00"))
        # Human/capability waits never self-clear; transient waits honor wake_at.
        if state["blocker"]["class"] != "github_transient_failure" or now < wake:
            save(args.state, state); return emit(state, EX_TEMPFAIL)
        state["status"] = "running"; state["blocker"] = None
    if args.pr_head: state["pr_head"] = args.pr_head
    if args.resolve_blocker and state.get("blocker", {}).get("class") == args.resolve_blocker:
        state["blocker"] = None; state["status"] = "running"; state["wake_at"] = iso(now)
    if args.inject_blocker:
        set_durable_wait(state, args.inject_blocker, now); save(args.state, state); return emit(state, EX_TEMPFAIL)

    start = 0
    if state["phase"] in PHASES:
        start = PHASES.index(state["phase"]) + (0 if state["next_action"] == state["phase"] else 1)
    target = args.stop_after
    if target == "next": target = PHASES[min(start, len(PHASES) - 1)]
    if not target and args.crash_after_remote:
        target = {"create_task": "bootstrap", "update_comment": "record_pr"}.get(args.crash_after_remote, args.crash_after_remote)
    if not target and args.crash_after_local: target = args.crash_after_local
    if not target and not args.run_to_completion: target = PHASES[min(start, len(PHASES) - 1)]

    for phase in PHASES[start:]:
        state["next_action"] = phase; state["heartbeat"] = {"at": iso(now), "phase": phase}; save(args.state, state)
        if phase == "bootstrap":
            ok, code = remote_once(args.state, state, "create_task", {"task_uid": state["task_uid"]}, args.crash_after_remote, now)
            if not ok: return emit(state, code)
            authority_ok, authority_response = hydrate_task_authority(state)
            if not authority_ok:
                state["status"] = "external_wait"; state["blocker"] = {"class": "task_authority_readback_failed",
                    "resume_condition": "GitHub task authority readback succeeds", "escalation": "retry canonical readback"}
                delay = int(authority_response.get("retry_after") or 5); state["wake_at"] = iso(now + dt.timedelta(seconds=delay))
                save(args.state, state); return emit(state, EX_TEMPFAIL)
        elif phase == "dispatch": schedule_slices(state, args, now)
        elif phase == "integrate" and state["slices"] and any(
                x["state"] not in {"integrated", "superseded"} for x in state["slices"]):
            state["status"] = "external_wait"
            state["blocker"] = {"class": "slice_integration_pending",
                                "resume_condition": "all required slices return and are integrated",
                                "escalation": "retry or replace terminal/no-payload slice"}
            state["wake_at"] = iso(now + dt.timedelta(minutes=5)); save(args.state, state)
            return emit(state, EX_TEMPFAIL)
        elif phase in {"create_pr", "comment", "merge"}:
            payload = {"task_uid": state["task_uid"]}
            if phase == "create_pr": payload["head_oid"] = state["pr_head"]
            # record_pr's canonical hold is the lifecycle comment mutation.
            if phase == "comment" and state.get("remote_journal", {}).get("comment"):
                journal = state["remote_journal"]["comment"]
                if journal["state"] == "acted":
                    journal["state"] = "committed"; journal["committed_at"] = iso(now); save(args.state, state)
                journal["effect_count"] = 1
                state["transition_journal"]["update_comment"] = journal
                ok, code = True, 0
            else:
                ok, code = remote_once(args.state, state, phase, payload, args.crash_after_remote, now)
            if not ok: return emit(state, code)
            if phase == "create_pr": state["pr_state"] = "OPEN"
            if phase == "merge": state["pr_state"] = "MERGED"
        elif phase == "record_pr":
            ok, code = migrate_hold(args.state, state, state["pr_head"], args.crash_after_remote, now)
            if not ok: return emit(state, code)
        elif phase == "merge_receipt": state["completed_transitions"].append("merge_receipt")
        elif phase == "task_done":
            if not local_once(args.state, state, phase, now, args.crash_after_local): return emit(state, EX_CRASH)
            state["task_state"] = "done"; state["completed_transitions"].append("task_done")
        elif phase == "main_sync":
            if not local_once(args.state, state, phase, now, args.crash_after_local): return emit(state, EX_CRASH)
            state["completed_transitions"].append("main_sync")
        elif phase == "safe_cleanup":
            if not local_once(args.state, state, phase, now, args.crash_after_local): return emit(state, EX_CRASH)
            state["worktree_state"] = "cleaned"; state["cleanup_receipt"] = {"safe": True, "at": iso(now)}
            state["completed_transitions"].append("safe_cleanup"); state["status"] = "completed"; state["next_action"] = None
        if phase == "watch":
            gate = gate_state()
            if gate.get("active_hold"):
                state["status"] = "external_wait"; state["blocker"] = {"class": "active_merge_hold",
                    "resume_condition": "canonical hold is cleared", "escalation": "notify recorded hold authority"}
                save(args.state, state); return emit(state, EX_TEMPFAIL)
            if gate.get("checks_ready") is False:
                state["status"] = "external_wait"; state["blocker"] = {"class": "checks_unready",
                    "resume_condition": "required checks are green", "escalation": "watch checks and route failures"}
                save(args.state, state); return emit(state, EX_TEMPFAIL)
        transition(state, phase, now)
        if phase == "dispatch" and state["slices"]:
            item = next((x for x in state["slices"] if x["state"] in {"running", "dispatched", "planned"}), state["slices"][-1])
            state["transitions"]["dispatch"]["schedule"] = {
                "delivery_id": f'{state["task_uid"]}:{item["id"]}:attempt-{item["attempt"]}',
                "wake_at": item["deadline"], "receipt_schema": "tpm-slice-delivery-ack/v1",
                "slice_id": item["id"], "delivery_ack_required": True,
                "action_type": "scheduler_delivery"}
        if phase == "watch":
            state["transitions"]["watch"].update({"gate_source": state.get("canonical_hold", {}).get("source", "github_task_issue_comment"),
                                                    "head_oid": state["pr_head"],
                                                    "hold_node_id": state.get("canonical_hold", {}).get("node_id"),
                                                    "live_gate_readback": True})
        state["phase"] = phase; save(args.state, state)
        if phase == target: break
    return emit(state)


def locked_main() -> int:
    # The lock guards load/CAS/save. Revision and lease token make ownership
    # observable; a second process gets structured busy rather than corruption.
    state_arg = next((sys.argv[i + 1] for i, x in enumerate(sys.argv[:-1]) if x == "--state"), None)
    if not state_arg:
        return main()
    lock_path = Path(state_arg + ".lock")
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+") as lock:
        try:
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            try: state = json.loads(Path(state_arg).read_text())
            except Exception: state = {"revision": 0, "lease": {"token": "unknown"}}
            state["status"] = "external_wait"
            state["blocker"] = {"class": "lease_busy", "resume_condition": "owner lease releases",
                                "escalation": "retry after lease expiry"}
            return emit(state, EX_TEMPFAIL)
        if "--resume" in sys.argv:
            time.sleep(0.12)
        return main()


if __name__ == "__main__":
    raise SystemExit(locked_main())
