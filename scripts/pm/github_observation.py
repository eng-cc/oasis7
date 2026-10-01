#!/usr/bin/env python3
"""Derived-only, cross-process PR observation cache and bounded watcher."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import time
from datetime import datetime, timezone
from typing import Any, Callable


SCHEMA = "oasis7.github-pr-observation/v1"
DEFAULT_MIN_INTERVAL = 60
DEFAULT_MAX_INTERVAL = 600
DEFAULT_MAX_POLLS = 6
DEFAULT_MAX_UNCHANGED_POLLS = 1


def _api_module():
    path = pathlib.Path(__file__).with_name("github_api.py")
    spec = importlib.util.spec_from_file_location("oasis7_github_api_for_observation", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load shared GitHub client: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _digest(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _body_digest(value: Any) -> str:
    return hashlib.sha256(str(value or "").encode("utf-8")).hexdigest()


def _business_projection(data: dict[str, Any], context: dict[str, Any] | None) -> dict[str, Any]:
    """Build a digest input from stable, decision-relevant fields only."""
    pr_fields = (
        "repository", "number", "url", "state", "isDraft", "headRefName", "headRefOid",
        "baseRefName", "baseRefOid", "mergeable", "mergeStateStatus", "reviewDecision",
    )
    projection: dict[str, Any] = {key: data.get(key) for key in pr_fields if key in data}
    projection["body_digest"] = _body_digest(data.get("body"))
    for key in ("comments", "reviews"):
        rows = []
        for item in data.get(key) or []:
            if not isinstance(item, dict):
                rows.append({"malformed": True})
                continue
            author = item.get("author")
            rows.append({
                "id": item.get("id"), "body_digest": _body_digest(item.get("body")),
                "state": item.get("state"), "submittedAt": item.get("submittedAt"),
                "createdAt": item.get("createdAt"),
                "author": author.get("login") if isinstance(author, dict) else author,
                "authorAssociation": item.get("authorAssociation"),
            })
        projection[key] = rows
    projection["threads"] = [
        {"id": item.get("id"), "isResolved": item.get("isResolved", item.get("is_resolved"))}
        if isinstance(item, dict) else {"malformed": True}
        for item in data.get("threads") or []
    ]
    checks = []
    for item in data.get("statusCheckRollup") or data.get("checks") or []:
        if not isinstance(item, dict):
            checks.append({"malformed": True})
            continue
        suite = item.get("checkSuite") if isinstance(item.get("checkSuite"), dict) else {}
        app = suite.get("app") if isinstance(suite.get("app"), dict) else {}
        checks.append({
            "id": item.get("databaseId", item.get("id")),
            "name": item.get("name", item.get("context")),
            "app_id": app.get("databaseId", item.get("app_id")),
            "status": item.get("status", item.get("state")),
            "conclusion": item.get("conclusion"),
        })
    projection["checks"] = checks
    policy = data.get("policy_discovery")
    if isinstance(policy, dict):
        projection["policy"] = {
            "status": policy.get("status"), "source": policy.get("source"),
            "required_status_checks": policy.get("required_status_checks"),
            "active_rule_types": policy.get("active_rule_types"),
        }
    hold = data.get("merge_hold")
    if isinstance(hold, dict):
        projection["hold"] = {key: hold.get(key) for key in (
            "kind", "active", "requester", "reason", "resume_authority",
        )}
    if context:
        projection["context"] = context
    return projection


def observation_key(client, repository: str, number: int, *, task_uid: str | None = None,
                    context: dict[str, Any] | None = None,
                    query_version: str = "oasis7-pr-snapshot/v1") -> str:
    identity = {
        "host": "api.github.com", "repository": repository, "pr_number": int(number),
        "credential_scope_digest": client.credential_scope_digest,
        "query_version": query_version, "task_uid": task_uid,
        "effective_context": context or {},
    }
    return _digest(identity)


def _result_from_state(state: dict[str, Any], *, cache_hit: bool, now: float,
                       wait_seconds: int = 0) -> dict[str, Any]:
    previous = state.get("observation") if isinstance(state.get("observation"), dict) else {}
    result = dict(previous)
    result.update({
        "evidence_mode": "observation", "ready_for_merge": False,
        "requires_live_gate": True, "cache_hit": cache_hit,
        "snapshot_digest": state.get("snapshot_digest"),
        "observed_at": state.get("observed_at"),
        "next_read_at": state.get("next_read_at"),
        "polls": int(state.get("polls") or 0),
        "unchanged_polls": int(state.get("unchanged_polls") or 0),
    })
    if wait_seconds > 0:
        result["retry_after_seconds"] = wait_seconds
    result.pop("readiness_receipt", None)
    return result


def observe_once(client, repository: str, number: int, fetch_snapshot: Callable[[], dict[str, Any]],
                 evaluate_candidate: Callable[[dict[str, Any]], bool], *,
                 task_uid: str | None = None, context: dict[str, Any] | None = None,
                 query_version: str = "oasis7-pr-snapshot/v1",
                 min_interval: int = DEFAULT_MIN_INTERVAL,
                 max_interval: int = DEFAULT_MAX_INTERVAL) -> dict[str, Any]:
    """Read at most once for a key and persist only derived observation data."""
    key = observation_key(client, repository, number, task_uid=task_uid,
                          context=context, query_version=query_version)
    now = float(client.clock())
    telemetry_context = dict(context or {})
    telemetry_context.setdefault("operation", "pr_observation")
    telemetry_context.setdefault("phase", "observation")
    telemetry_context.setdefault("api_family", "graphql")
    try:
        budget = client.rate_limit_snapshot()
        if budget and budget.get("status") == "external_wait":
            client.log_cache_event("pr_observation", context=telemetry_context,
                                   cache_hit=False, error_kind="rate_limit_pause")
            return {
                "evidence_mode": "observation", "status": "external_wait",
                "reason": budget.get("reason") or "github_rate_limit_pause",
                "ready_for_merge": False, "candidate_ready": False,
                "requires_live_gate": True, "cache_hit": False,
                "retry_after_seconds": budget.get("retry_after_seconds", 0),
            }
        with client.file_lock(f"pr-observation:{key}", timeout=2.0):
            cache_path = client.state_path("observations", key)
            state = client.read_state("observations", key, default=None, strict=False)
            cache_corrupt = cache_path.exists() and not isinstance(state, dict)
            if (isinstance(state, dict) and state.get("schema") == SCHEMA
                    and state.get("query_version") == query_version):
                next_read = float(state.get("next_read_at_epoch") or 0)
                if next_read > now:
                    client.log_cache_event("pr_observation", context=telemetry_context,
                                           cache_hit=True)
                    return _result_from_state(state, cache_hit=True, now=now,
                                              wait_seconds=max(1, int(next_read - now + 0.999)))
            else:
                state = None
            client.log_cache_event("pr_observation", context=telemetry_context,
                                   cache_hit=False,
                                   error_kind="observation_cache_corrupt" if cache_corrupt else None)
            data = fetch_snapshot()
            metadata = data.get("snapshot_metadata") if isinstance(data, dict) else None
            if not isinstance(data, dict) or not isinstance(metadata, dict) or metadata.get("complete") is not True:
                raise ValueError("PR observation snapshot is incomplete")
            projection = _business_projection(data, context)
            digest = _digest(projection)
            previous_digest = state.get("snapshot_digest") if state else None
            changed = previous_digest != digest
            interval = min_interval if changed else min(max_interval, max(min_interval, int(state.get("interval_seconds") or min_interval) * 2))
            candidate = bool(evaluate_candidate(data))
            observed_at = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
            candidate_output = {
                "status": "observed", "candidate_ready": candidate,
                "ready_for_merge": False, "requires_live_gate": True,
            }
            next_read_at = now + interval
            updated = {
                "schema": SCHEMA, "query_version": query_version,
                "snapshot_digest": digest, "observed_at": observed_at,
                "observed_at_epoch": now, "next_read_at": int(next_read_at),
                "next_read_at_epoch": next_read_at, "interval_seconds": interval,
                "polls": int((state or {}).get("polls") or 0) + 1,
                "unchanged_polls": 0 if changed else int((state or {}).get("unchanged_polls") or 0) + 1,
                "observation": candidate_output,
            }
            client.write_state("observations", key, updated, critical=True)
            result = _result_from_state(updated, cache_hit=False, now=now)
            result["changed"] = changed
            result["retry_after_seconds"] = interval
            return result
    except TimeoutError:
        client.log_cache_event("pr_observation", context=telemetry_context,
                               cache_hit=False, error_kind="pr_observation_in_progress")
        return {
            "evidence_mode": "observation", "status": "external_wait",
            "reason": "pr_observation_in_progress", "ready_for_merge": False,
            "candidate_ready": False, "requires_live_gate": True,
            "cache_hit": False, "retry_after_seconds": 2,
        }
    except Exception as exc:
        error_dict = exc.as_dict() if hasattr(exc, "as_dict") else None
        if error_dict and error_dict.get("status") == "external_wait":
            result = dict(error_dict)
            result.update({"evidence_mode": "observation", "ready_for_merge": False,
                           "candidate_ready": False, "requires_live_gate": True,
                           "cache_hit": False})
            result.pop("readiness_receipt", None)
            return result
        return {
            "evidence_mode": "observation", "status": "capability_blocked",
            "reason": getattr(exc, "kind", "observation_failed"),
            "error": str(exc), "ready_for_merge": False,
            "candidate_ready": False, "requires_live_gate": True,
            "cache_hit": False,
        }


def watch(client, repository: str, number: int, fetch_snapshot: Callable[[], dict[str, Any]],
          evaluate_candidate: Callable[[dict[str, Any]], bool], *, task_uid: str | None = None,
          context: dict[str, Any] | None = None, query_version: str = "oasis7-pr-snapshot/v1",
          min_interval: int = DEFAULT_MIN_INTERVAL, max_interval: int = DEFAULT_MAX_INTERVAL,
          max_polls: int = DEFAULT_MAX_POLLS,
          max_unchanged_polls: int = DEFAULT_MAX_UNCHANGED_POLLS,
          sleeper: Callable[[float], None] | None = None) -> dict[str, Any]:
    """Run the existing bounded exponential PR watch as derived observation."""
    sleeper = sleeper or client.sleeper or time.sleep
    previous_digest = None
    unchanged_polls = 0
    for _ in range(max_polls):
        result = observe_once(client, repository, number, fetch_snapshot, evaluate_candidate,
                              task_uid=task_uid, context=context, query_version=query_version,
                              min_interval=min_interval, max_interval=max_interval)
        if result.get("status") in {"external_wait", "capability_blocked", "uncertain"}:
            return result
        if result.get("candidate_ready") is True:
            result["formal_gate_command"] = f"python3 scripts/pm/pr-lifecycle-gate.py {number} --task-uid {task_uid or '<task_uid>'} --json"
            return result
        digest = result.get("snapshot_digest")
        if previous_digest is not None and digest != previous_digest:
            # A business change deserves prompt caller handling; the returned
            # candidate remains derived-only and still requires a live gate.
            result.update({"status": "observed", "changed": True,
                           "ready_for_merge": False, "requires_live_gate": True})
            result.pop("readiness_receipt", None)
            return result
        if previous_digest is not None and digest == previous_digest:
            unchanged_polls += 1
        else:
            unchanged_polls = 0
        previous_digest = digest
        if unchanged_polls >= max_unchanged_polls:
            result.update({"status": "external_wait",
                           "reason": "stable_pr_watch_unchanged_budget_exhausted",
                           "retry_after_seconds": int(result.get("retry_after_seconds") or min_interval)})
            return result
        if _ + 1 >= max_polls:
            result.update({"status": "external_wait", "reason": "stable_pr_watch_bound_exhausted",
                           "retry_after_seconds": int(result.get("retry_after_seconds") or min_interval)})
            return result
        sleeper(float(result.get("retry_after_seconds") or min_interval))
    return {"evidence_mode": "observation", "status": "external_wait",
            "reason": "stable_pr_watch_bound_exhausted", "ready_for_merge": False,
            "candidate_ready": False, "requires_live_gate": True, "cache_hit": False,
            "retry_after_seconds": min_interval}
