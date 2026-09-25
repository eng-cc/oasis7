#!/usr/bin/env python3
"""Fail-closed PR watch/merge decision over all review and check surfaces."""
from __future__ import annotations

import argparse
import base64
import os
import datetime as dt
import hashlib
import importlib.util
from contextlib import contextmanager
import json
import subprocess
import sys
import re
import fnmatch
import tempfile
from pathlib import Path
from typing import Any

SUCCESS = {"SUCCESS", "NEUTRAL", "SKIPPED"}
HOLDS = {"manual_packaging_ci_hold", "user_requested_merge_hold"}
KEYED_Q_APPLICABILITY_SCHEMA = "oasis7-ci-keyed-q-applicability/v1"
LOCAL_TARGET_INVENTORY_SCHEMA = "oasis7-ci-local-target-inventory/v1"
# Canonical recovery surface: oasis7-pr-disposition records are rebuilt from
# paginated GitHub task issueComments; the optional local cache is never truth.
issueComments = "GitHub task issueComments"


def actionable(body: str) -> bool:
    text = body.strip().lower()
    benign = {"lgtm", "looks good", "thanks", "thank you", "approved", "+1", "👍"}
    return bool(text) and text.rstrip(".! ") not in benign and not text.startswith(("resolved:", "non-actionable:", "acknowledged:", "thanks, acknowledged", "status:"))


def benign_bot_comment(body: str) -> bool:
    """Recognize bot status chatter without granting bots a blanket exemption."""
    text = " ".join(body.strip().lower().split())
    status_prefix = text.startswith((
        "automated build summary:",
        "build summary:",
        "deployment preview:",
        "preview deployment:",
    ))
    action_terms = ("fix ", "must ", "required", "vulnerability", "before merge", "action needed", "failed", "failure")
    return status_prefix and not any(term in text for term in action_terms)

def login_of(item: dict[str, Any]) -> str:
    author = item.get("author")
    return str(author.get("login") or "") if isinstance(author, dict) else str(author or "")

def latest_reviews(reviews: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_reviewer: dict[str, dict[str, Any]] = {}
    for review in reviews:
        author = login_of(review) or "unknown"
        current = by_reviewer.get(author)
        stamp = str(review.get("submittedAt") or review.get("createdAt") or "")
        old = str((current or {}).get("submittedAt") or (current or {}).get("createdAt") or "")
        if current is None or stamp >= old:
            by_reviewer[author] = review
    return list(by_reviewer.values())


def verified_evidence(receipt: Any, data: dict[str, Any], head_oid: str, *, task_uid: str|None=None, issue_number: int|None=None, node_id: str|None=None, kind: str|None=None, disposition: str|None=None) -> bool:
    if not isinstance(receipt, dict): return False
    bound = (receipt.get("source") == "github_task_issue_comment" and receipt.get("runtime_verified") is True
            and bool(receipt.get("task_uid")) and bool(receipt.get("issue_number"))
            and str(receipt.get("repository")) == str(data.get("repository"))
            and str(receipt.get("pr_number")) == str(data.get("number"))
            and str(receipt.get("head_oid")) == head_oid
            and bool(str(receipt.get("github_node_id") or ""))
            and str(receipt.get("url") or "").startswith("https://github.com/")
            and bool(receipt.get("author")) and bool(receipt.get("observed_at"))
            and bool(re.fullmatch(r"[0-9a-f]{64}", str(receipt.get("digest") or ""))))
    if not bound: return False
    if receipt.get("live_rebuilt") is True:
        expected_receipt={"task_uid":task_uid,"issue_number":issue_number,"node_id":node_id,"kind":kind,"disposition":disposition}
        return not any(value is not None and str(receipt.get(key)) != str(value) for key,value in expected_receipt.items())
    try:
        comment = json.loads(subprocess.check_output(["gh","api",f"repos/{receipt['repository']}/issues/comments/{str(receipt['github_node_id']).split('-')[-1]}"], text=True, stderr=subprocess.PIPE))
    except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError): return False
    body = str(comment.get("body") or "")
    fields = dict(re.findall(r"^- ([a-z_]+): `?([^`\n]+)`?$", body, re.M))
    expected = {"task_uid":task_uid,"issue_number":issue_number,"repository":data.get("repository") if task_uid is not None else None,"pr_number":data.get("number") if task_uid is not None else None,"head_oid":head_oid if task_uid is not None else None,"node_id":node_id,"kind":kind,"disposition":disposition}
    if any(value is not None and str(fields.get(key)) != str(value) for key,value in expected.items()): return False
    created = str(comment.get("created_at") or comment.get("createdAt") or "")
    if not created or str(receipt.get("observed_at")) != created: return False
    return (hashlib.sha256(body.encode()).hexdigest() == receipt.get("digest")
            and str((comment.get("user") or {}).get("login") or "") == str(receipt.get("author"))
            and str(comment.get("html_url") or "") == str(receipt.get("url")))


def rebuild_issue_evidence(repo: str, issue_number: int, task_uid: str, data: dict[str, Any]) -> dict[str, Any]:
    raw = _run_json(["gh","api",f"repos/{repo}/issues/{issue_number}/comments","--paginate","--slurp"])
    comments = [x for page in raw for x in page] if raw and isinstance(raw[0], list) else raw
    result: dict[str, Any] = {"comment_dispositions":[],"review_dispositions":[],"admin_merge_authority":None}
    for comment in comments or []:
        body = str(comment.get("body") or "")
        fields = dict(re.findall(r"^- ([a-z_]+): `?([^`\n]+)`?$", body, re.M))
        if fields.get("task_uid") != task_uid or fields.get("repository") != repo or str(fields.get("issue_number")) != str(issue_number): continue
        if str(fields.get("pr_number")) != str(data.get("number")) or fields.get("head_oid") != str(data.get("headRefOid")): continue
        receipt = {"source":"github_task_issue_comment","runtime_verified":True,"live_rebuilt":True,"task_uid":task_uid,"repository":repo,"issue_number":issue_number,"pr_number":data.get("number"),"head_oid":data.get("headRefOid"),"node_id":fields.get("node_id"),"kind":fields.get("kind"),"disposition":fields.get("disposition"),"github_node_id":str(comment.get("id")),"url":comment.get("html_url"),"author":(comment.get("user") or {}).get("login"),"observed_at":comment.get("created_at"),"digest":hashlib.sha256(body.encode()).hexdigest()}
        if "<!-- oasis7-pr-disposition -->" in body:
            record={"node_id":fields.get("node_id"),"head_oid":fields.get("head_oid"),"disposition":fields.get("disposition"),"evidence_receipt":receipt}
            result["review_dispositions" if fields.get("kind")=="review" else "comment_dispositions"].append(record)
        elif "<!-- oasis7-merge-hold -->" in body:
            result["merge_hold"]={"kind":fields.get("hold_kind"),"active":fields.get("active")=="true","requester":fields.get("requester"),"reason":fields.get("reason"),"resume_authority":fields.get("resume_authority"),"evidence_receipt":receipt}
        elif "<!-- oasis7-admin-merge-authority -->" in body:
            result["admin_merge_authority"]={"requester":fields.get("requester"),"scope":fields.get("scope"),"reason":fields.get("reason"),"disposition":fields.get("disposition"),"evidence_receipt":receipt}
    return result


def graphql_pages(repo: str, number: int, surface: str) -> list[dict[str, Any]]:
    owner, name = repo.split("/", 1)
    cursor = ""
    collected: list[dict[str, Any]] = []
    while True:
        if surface in {"comments", "reviews", "reviewThreads"}:
            fields = "id body url createdAt author{login} authorAssociation" if surface == "comments" else (
                "id body url submittedAt createdAt state author{login}" if surface == "reviews" else "id isResolved"
            )
            query = (
                "query($owner:String!,$repo:String!,$number:Int!,$cursor:String){"
                "repository(owner:$owner,name:$repo){pullRequest(number:$number){"
                + surface + "(first:100,after:$cursor){pageInfo{hasNextPage endCursor} nodes{" + fields + "}}"
                "}}}"
            )
            path = ["data", "repository", "pullRequest", surface]
        else:
            query = (
                "query($owner:String!,$repo:String!,$number:Int!,$cursor:String){"
                "repository(owner:$owner,name:$repo){pullRequest(number:$number){"
                "commits(last:1){nodes{commit{statusCheckRollup{"
                "contexts(first:100,after:$cursor){pageInfo{hasNextPage endCursor} nodes{"
                "__typename ... on CheckRun{name conclusion status checkSuite{app{databaseId}}} "
                "... on StatusContext{context state}"
                "}}}}}}}}}"
            )
            path = ["data", "repository", "pullRequest", "commits", "nodes"]
        cmd = ["gh", "api", "graphql", "-f", f"query={query}", "-F", f"owner={owner}", "-F", f"repo={name}", "-F", f"number={number}"]
        if cursor:
            cmd += ["-F", f"cursor={cursor}"]
        payload = json.loads(subprocess.check_output(cmd, text=True))
        node: Any = payload
        for key in path:
            node = node[key]
        if surface == "checks":
            if not node:
                return []
            node = node[0]["commit"].get("statusCheckRollup")
            if node is None:
                return []
            node = node["contexts"]
        collected.extend(node.get("nodes") or [])
        page = node.get("pageInfo") or {}
        if not page.get("hasNextPage"):
            return collected
        next_cursor = str(page.get("endCursor") or "")
        if not next_cursor or next_cursor == cursor:
            raise SystemExit(f"pr-lifecycle-gate: {surface} pagination did not advance")
        cursor = next_cursor


def graphql_pr_snapshot(repo: str, number: int) -> dict[str, list[dict[str, Any]]]:
    """Load all hot PR-watch surfaces in one bounded GraphQL request.

    A watch poll intentionally fails closed when any surface exceeds 100 nodes;
    silently issuing pagination reads would make the per-poll budget unbounded.
    """
    owner, name = repo.split("/", 1)
    query = """query($owner:String!,$repo:String!,$number:Int!){
      repository(owner:$owner,name:$repo){pullRequest(number:$number){
        comments(first:100){pageInfo{hasNextPage} nodes{id body url createdAt author{login} authorAssociation}}
        reviews(first:100){pageInfo{hasNextPage} nodes{id body url submittedAt createdAt state author{login}}}
        reviewThreads(first:100){pageInfo{hasNextPage} nodes{id isResolved}}
        commits(last:1){nodes{commit{statusCheckRollup{contexts(first:100){pageInfo{hasNextPage} nodes{
          __typename ... on CheckRun{name conclusion status checkSuite{app{databaseId}}}
          ... on StatusContext{context state}
        }}}}}}
      }}
    }"""
    payload = _run_json(["gh", "api", "graphql", "-f", f"query={query}",
                         "-F", f"owner={owner}", "-F", f"repo={name}", "-F", f"number={number}"])
    pr = (((payload.get("data") or {}).get("repository") or {}).get("pullRequest") or {})
    surfaces = {"comments": pr.get("comments") or {}, "reviews": pr.get("reviews") or {},
                "threads": pr.get("reviewThreads") or {}}
    commits = ((pr.get("commits") or {}).get("nodes") or [])
    rollup = ((commits[0].get("commit") or {}).get("statusCheckRollup") or {}) if commits else {}
    surfaces["statusCheckRollup"] = rollup.get("contexts") or {}
    oversized = [name for name, connection in surfaces.items()
                 if (connection.get("pageInfo") or {}).get("hasNextPage")]
    if oversized:
        raise SystemExit("pr-lifecycle-gate: bounded PR snapshot exceeded 100 nodes for: " + ", ".join(oversized))
    return {name: list(connection.get("nodes") or []) for name, connection in surfaces.items()}


def _run_json(cmd: list[str]) -> Any:
    return json.loads(subprocess.check_output(cmd, text=True, stderr=subprocess.PIPE))


def discover_required_policy(repo: str, branch: str) -> dict[str, Any]:
    classic_error = ""
    checks: list[dict[str, Any]] = []
    active_rule_types: set[str] = set()
    try:
        protection = _run_json(["gh", "api", f"repos/{repo}/branches/{branch}/protection"])
        required = protection.get("required_status_checks") or {}
        checks = [{"context": str(x), "app_id": None} for x in required.get("contexts") or []]
        checks += [{"context": str(x.get("context") or ""), "app_id": x.get("app_id")} for x in required.get("checks") or [] if x.get("context")]
        if checks:
            active_rule_types.add("required_status_checks")
        reviews = protection.get("required_pull_request_reviews") or {}
        if int(reviews.get("required_approving_review_count") or 0) > 0:
            active_rule_types.add("required_pull_request_reviews")
        for field in ("required_signatures", "required_linear_history", "required_conversation_resolution", "lock_branch"):
            if (protection.get(field) or {}).get("enabled") is True:
                active_rule_types.add(field)
    except subprocess.CalledProcessError as exc:
        classic_error = str(exc.stderr or exc)
    except json.JSONDecodeError as exc:
        return {"status":"capability_blocked","source":"classic_branch_protection","reason":"malformed_classic_policy","resume":"restore policy read access and rerun","required_status_checks":[],"error":str(exc)}
    classic_missing = "404" in classic_error or "Not Found" in classic_error
    if classic_error and not classic_missing:
        return {"status":"capability_blocked","source":"classic_branch_protection","reason":"policy_read_error","resume":"restore classic branch protection read access and rerun","required_status_checks":[],"error":classic_error}
    try:
        raw_rulesets = _run_json(["gh", "api", f"repos/{repo}/rulesets", "--paginate", "--slurp"])
        rulesets = [item for page in raw_rulesets for item in page] if raw_rulesets and isinstance(raw_rulesets[0], list) else raw_rulesets
    except (subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        return {"status": "capability_blocked", "source": "repository_rulesets", "reason": "permission_or_transport_failure", "resume": "restore GitHub ruleset read access and rerun", "required_status_checks": [], "error": str(exc)}
    checks = checks if not classic_error else []
    expanded_rulesets = []
    for summary in rulesets if isinstance(rulesets, list) else []:
        if "rules" in summary:
            expanded_rulesets.append(summary)
            continue
        try:
            expanded_rulesets.append(_run_json(["gh", "api", f"repos/{repo}/rulesets/{summary['id']}"]))
        except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError) as exc:
            return {"status": "capability_blocked", "source": "repository_rulesets", "reason": "ruleset_detail_unavailable", "resume": "restore GitHub ruleset detail access and rerun", "required_status_checks": [], "error": str(exc)}
    needs_default = any("~DEFAULT_BRANCH" in (((x.get("conditions") or {}).get("ref_name") or {}).get("include") or []) for x in expanded_rulesets)
    try:
        default_branch = str(_run_json(["gh","api",f"repos/{repo}"]).get("default_branch") or "") if needs_default else ""
    except Exception as exc:
        return {"status":"capability_blocked","source":"repository_metadata","reason":"default_branch_read_error","resume":"restore repository metadata read access and rerun","required_status_checks":[],"error":str(exc)}
    if needs_default and not default_branch:
        return {"status":"capability_blocked","source":"repository_metadata","reason":"default_branch_read_error","resume":"repository default_branch was empty; repair metadata access and rerun","required_status_checks":[]}
    for ruleset in expanded_rulesets:
        if str(ruleset.get("enforcement") or "").lower() != "active":
            continue
        if str(ruleset.get("target") or "branch").lower() != "branch":
            continue
        refs = (ruleset.get("conditions") or {}).get("ref_name") or {}
        includes = refs.get("include") or ["~ALL"]
        excludes = refs.get("exclude") or []
        ref = f"refs/heads/{branch}"
        def matches(value: Any) -> bool:
            value = str(value)
            return value == "~ALL" or (value == "~DEFAULT_BRANCH" and branch == default_branch) or fnmatch.fnmatch(ref, value)
        if not any(matches(value) for value in includes) or any(matches(value) for value in excludes):
            continue
        for rule in ruleset.get("rules") or []:
            rule_type = str(rule.get("type") or "")
            if rule_type == "pull_request":
                parameters = rule.get("parameters") or {}
                if int(parameters.get("required_approving_review_count") or 0) > 0:
                    active_rule_types.add("required_pull_request_reviews")
                if parameters.get("required_review_thread_resolution") is True:
                    active_rule_types.add("required_conversation_resolution")
                known = {
                    "dismiss_stale_reviews_on_push", "require_code_owner_review",
                    "require_last_push_approval", "required_approving_review_count",
                    "required_review_thread_resolution", "allowed_merge_methods",
                }
                if set(parameters) - known:
                    active_rule_types.add("unsupported_pull_request_policy")
                allowed_methods = parameters.get("allowed_merge_methods") or []
                if allowed_methods and "squash" not in allowed_methods:
                    active_rule_types.add("unsupported_pull_request_policy")
            elif rule_type:
                active_rule_types.add(rule_type)
            if rule.get("type") != "required_status_checks":
                continue
            for item in (rule.get("parameters") or {}).get("required_status_checks") or []:
                if item.get("context"):
                    checks.append({"context": str(item["context"]), "app_id": item.get("integration_id")})
    unique = {(x["context"], x.get("app_id")): x for x in checks}
    return {"status": "resolved", "source": "classic_and_repository_rulesets" if not classic_error and rulesets else ("repository_rulesets" if rulesets else ("classic_branch_protection" if not classic_error else "explicit_no_policy")), "required_status_checks": list(unique.values()), "active_rule_types": sorted(active_rule_types)}


def load_live(selector: str) -> dict[str, Any]:
    fields = "number,url,state,isDraft,body,mergeable,mergeStateStatus,reviewDecision,headRefName,headRefOid,baseRefName,baseRefOid"
    raw = subprocess.check_output(["gh", "pr", "view", selector, "--json", fields], text=True)
    payload = json.loads(raw)
    repo = json.loads(subprocess.check_output(["gh", "repo", "view", "--json", "nameWithOwner"], text=True))["nameWithOwner"]
    payload["repository"] = repo
    payload.update(graphql_pr_snapshot(repo, int(payload["number"])))
    payload["policy_discovery"] = discover_required_policy(repo, str(payload["baseRefName"]))
    payload["required_status_checks"] = payload["policy_discovery"]["required_status_checks"]
    return payload


def decision(data: dict[str, Any], admin_authorized: bool, *, evidence_mode: str = "production") -> dict[str, Any]:
    blockers: list[str] = []
    hold_truth = data.get("merge_hold")
    if not isinstance(hold_truth, dict) or not hold_truth.get("kind"):
        hold = "missing"
        blockers.append("merge hold task truth is missing")
    else:
        hold = str(hold_truth.get("kind"))
        if hold_truth.get("active") and not all(str(hold_truth.get(k) or "").strip() for k in ("requester","reason","resume_authority")):
            blockers.append("active merge hold lacks requester/reason/resume authority")
    if isinstance(hold_truth, dict) and hold_truth.get("active") and hold in HOLDS:
        blockers.append(f"active merge hold: {hold}")
    elif hold not in {"normal_pr_ci_watch", "missing"}:
        blockers.append(f"unknown merge hold: {hold}")
    if str(data.get("state") or "OPEN").upper() != "OPEN":
        blockers.append("PR is not open")
    checks = data.get("statusCheckRollup") or data.get("checks") or []
    policy = data.get("policy_discovery")
    if isinstance(policy, dict) and policy.get("status") != "resolved":
        blockers.append(f"required-check policy capability blocked: {policy.get('reason') or 'unknown'}; {policy.get('resume') or 'rerun'}")
    required_checks = (
        policy.get("required_status_checks")
        if isinstance(policy, dict) and "required_status_checks" in policy
        else data.get("required_status_checks")
    )
    if required_checks is None:
        contexts = data.get("required_status_contexts")
        required_checks = ([{"context": str(x), "app_id": None} for x in contexts] if contexts is not None else
                           [{"context": str(item.get("name") or item.get("context") or ""), "app_id": item.get("app_id")} for item in checks])
    def check_identity(item: dict[str, Any]) -> tuple[str, Any]:
        app = item.get("app_id")
        if app is None and isinstance(item.get("checkSuite"), dict):
            app = ((item.get("checkSuite") or {}).get("app") or {}).get("databaseId")
        return str(item.get("name") or item.get("context") or ""), app
    checks_by_identity = {check_identity(item): item for item in checks}
    for required in required_checks:
        name = str(required.get("context") if isinstance(required, dict) else required)
        app_id = required.get("app_id") if isinstance(required, dict) else None
        item = checks_by_identity.get((name, app_id))
        if item is None and app_id is None:
            item = next((value for (context, _app), value in checks_by_identity.items() if context == name), None)
        if item is None:
            blockers.append(f"required check is missing: {name} app_id={app_id}")
            continue
        state = str(item.get("conclusion") or item.get("state") or item.get("status") or "").upper()
        if state not in SUCCESS:
            blockers.append(f"required check not successful: {name} app_id={app_id}={state or 'UNKNOWN'}")
    mergeable = str(data.get("mergeable") or "UNKNOWN").upper()
    if mergeable not in {"MERGEABLE", "TRUE"}:
        blockers.append(f"PR is not mergeable: {mergeable}")
    reviews = latest_reviews(data.get("reviews") or [])
    if str(data.get("reviewDecision") or "").upper() == "CHANGES_REQUESTED" or any(str(r.get("state") or "").upper() == "CHANGES_REQUESTED" for r in reviews):
        blockers.append("requested changes remain")
    dispositions = {(str(x.get("node_id") or ""), str(x.get("head_oid") or "")): x for x in data.get("comment_dispositions") or []}
    head_oid = str(data.get("headRefOid") or data.get("head_oid") or "")
    for item in data.get("comments") or []:
        login = login_of(item)
        is_bot = login.endswith("[bot]") or str(item.get("authorAssociation") or "").upper() == "BOT"
        body = str(item.get("body") or "")
        disposition = dispositions.get((str(item.get("id") or ""), head_oid))
        legacy_fixture = not data.get("repository") and bool(disposition and str(disposition.get("evidence") or "").strip())
        disposition_ok = bool(disposition and disposition.get("disposition") in {"addressed", "rejected_with_evidence", "non_actionable"} and (legacy_fixture or verified_evidence(disposition.get("evidence_receipt"), data, head_oid)))
        if actionable(body) and not disposition_ok and not (is_bot and benign_bot_comment(body)):
            blockers.append(f"actionable PR conversation comment: {item.get('url') or item.get('id') or 'unknown'}")
    review_dispositions = {(str(x.get("node_id") or ""), str(x.get("head_oid") or "")): x for x in data.get("review_dispositions") or []}
    for item in reviews:
        disposition = review_dispositions.get((str(item.get("id") or ""), head_oid))
        disposed = bool(disposition and disposition.get("disposition") in {"addressed","rejected_with_evidence","non_actionable"} and verified_evidence(disposition.get("evidence_receipt"), data, head_oid))
        if actionable(str(item.get("body") or "")) and str(item.get("state") or "").upper() != "APPROVED" and not disposed:
            blockers.append(f"actionable top-level review body: {item.get('url') or item.get('id') or 'unknown'}")
    if any(not bool(item.get("isResolved", item.get("is_resolved", False))) for item in data.get("threads") or []):
        blockers.append("unresolved review threads remain")
    merge_state = str(data.get("mergeStateStatus") or "").upper()
    allowed_admin_rule_types = {"required_status_checks", "required_pull_request_reviews", "required_conversation_resolution", "deletion", "non_fast_forward"}
    policy_rule_types = set((policy or {}).get("active_rule_types") or []) if isinstance(policy, dict) else set()
    policy_proves_approval_only = bool(
        isinstance(policy, dict)
        and policy.get("status") == "resolved"
        and "required_pull_request_reviews" in policy_rule_types
        and policy_rule_types <= allowed_admin_rule_types
    )
    approval_only = (
        merge_state in {"BLOCKED", "BEHIND"}
        and mergeable in {"MERGEABLE", "TRUE"}
        and str(data.get("reviewDecision") or "").upper() == "REVIEW_REQUIRED"
        and policy_proves_approval_only
    )
    # Standing repository policy selects admin merge only when approval absence
    # (plus the informational up-to-date state BEHIND) is the entire remaining
    # protection state. Existing blockers remain
    # authoritative and can never be bypassed by this selection.
    use_admin = approval_only and not blockers
    if merge_state == "BLOCKED" and not approval_only:
        blockers.append("BLOCKED is not a proven review-approval-only state")
    elif merge_state in {"DIRTY", "UNKNOWN", "UNSTABLE"}:
        blockers.append(f"blocking merge state: {merge_state}")
    observed_at = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    epoch_input = {"repository": data.get("repository") or "fixture", "pr_number": data.get("number"), "head_oid": head_oid or "fixture-head", "blockers": blockers, "policy":policy, "hold":hold}
    if data.get('integration_ci') is not None:
        epoch_input['integration_ci'] = data['integration_ci']
        integration = data['integration_ci']
        if isinstance(integration, dict) and integration.get('keyed_target_applicability_epoch') is not None:
            epoch_input['keyed_target_applicability_epoch'] = integration['keyed_target_applicability_epoch']
    gate_epoch = hashlib.sha256(json.dumps(epoch_input, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    result = {
        "ready_for_merge": not blockers,
        "status": "ready" if not blockers else ("held" if isinstance(hold_truth, dict) and hold_truth.get("active") and hold in HOLDS else "blocked"),
        "merge_hold": hold,
        "use_admin_merge": use_admin,
        "merge_path": "admin_review_approval_only" if use_admin else "ordinary",
        "merge_path_reason": "repository standing policy for proven approval-only protection" if use_admin else None,
        "blockers": blockers,
        "pr_number": data.get("number"),
        "pr_url": data.get("url"),
        "policy_discovery": policy,
    }
    if not blockers and evidence_mode == "production":
        result["readiness_receipt"] = {"receipt_type": "oasis7_pr_lifecycle_ready", "issuer": "oasis7_pr_lifecycle_gate/v1", "repository": epoch_input["repository"], "pr_number": data.get("number"), "head_oid": epoch_input["head_oid"], "observed_at": observed_at, "gate_epoch": gate_epoch}
    elif not blockers:
        # Fixture evaluation is deliberately untrusted decision evidence.  It
        # must never mint the production readiness_receipt consumed by merge.
        result["evidence_mode"] = evidence_mode
    return result


def read_pr_identity(repository, number):
    return json.loads(subprocess.check_output([
        'gh', 'pr', 'view', str(number), '--repo', repository,
        '--json', 'number,state,isDraft,body,baseRefName,headRefName,baseRefOid,headRefOid'], text=True))


def local_loop_admission(root, uid, base, head, tool_root):
    """Verify the effective ingress bytes before executing any loop helper."""
    root = Path(root).resolve()
    task = json.loads((root / '.pm/github-project-sync/tasks.json').read_text())['tasks'][uid]
    issue = json.loads(subprocess.check_output([
        'gh', 'api', f"repos/{task['repository']}/issues/{task['issue_number']}"], text=True))
    body = issue.get('body', '')
    matches = re.findall(r'^- loop_binding_b64: `([^`]+)`$', body, re.MULTILINE)
    binding = task.get('loop_binding')
    if matches:
        if len(matches) != 1: raise ValueError('ambiguous live loop binding')
        binding = json.loads(base64.urlsafe_b64decode(matches[0] + '=' * (-len(matches[0]) % 4)))
    effective = Path(tool_root or os.environ.get('OASIS7_LOOP_TOOL_ROOT') or Path(__file__).resolve().parents[2]).resolve()
    relative = 'scripts/pm/loop-local-gate.py'
    helper = effective / relative
    if binding is not None:
        commit = binding.get('policy_commit', '')
        if not re.fullmatch(r'[0-9a-f]{40}', commit): raise ValueError('invalid effective policy commit')
        def git(checkout, *arguments):
            return subprocess.check_output(['git', '-C', str(checkout), *arguments], text=True).strip()
        if git(effective, 'rev-parse', 'HEAD') != commit:
            raise ValueError('effective tool root HEAD differs from policy commit')
        if git(effective, 'rev-parse', '--path-format=absolute', '--git-common-dir') != git(root, 'rev-parse', '--path-format=absolute', '--git-common-dir'):
            raise ValueError('effective helper belongs to a different repository')
        subprocess.run(['git', '-C', str(root), 'fetch', '--no-tags', 'origin', 'main:refs/remotes/origin/main'], check=True, capture_output=True)
        subprocess.run(['git', '-C', str(root), 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main'], check=True, capture_output=True)
        expected = subprocess.check_output(['git', '-C', str(effective), 'show', f'{commit}:{relative}'])
        if helper.read_bytes() != expected: raise ValueError('effective local gate bytes differ from policy commit')
        if subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() != head:
            raise ValueError('canonical worktree HEAD differs from current PR head')
    command = [sys.executable, '-I', str(helper), '--root', str(root), '--task-uid', uid,
               '--base', base, '--head', head, '--tool-root', str(effective), '--json']
    completed = subprocess.run(command, text=True, capture_output=True)
    if completed.returncode:
        raise ValueError((completed.stdout or completed.stderr).strip() or 'live loop admission failed')
    result = json.loads(completed.stdout)
    if result.get('status') not in ('passed', 'legacy'):
        raise ValueError('live loop admission did not pass')
    return {'status': result['status'], 'tool_root': str(effective),
            'policy_commit': (binding or {}).get('policy_commit'), 'task': task}


def requires_strict_integration(data: dict[str, Any]) -> bool:
    """Fail closed when no trusted projection context is available.

    Production callers use ``trusted_requires_strict_integration`` after
    byte-loading and validating the published projection.  This compatibility
    wrapper deliberately never relaxes from ad-hoc PR data.
    """
    return True


def _load_trusted_projection(data: dict[str, Any], root: Path, effective: Path, uid: str, policy_commit: str) -> dict[str, Any]:
    body = str(data.get("body") or "")
    marker = re.search(r"<!--\s*oasis7-impact-projection-b64:\s*([A-Za-z0-9+/=]+)\s*-->", body)
    if not marker:
        raise ValueError("trusted impact projection marker is missing")
    try:
        raw = base64.b64decode(marker.group(1), validate=True)
        value = json.loads(raw.decode("utf-8"))
    except (ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError("trusted impact projection marker is malformed") from exc
    helper_path = effective / "scripts/pm/workflow-impact-projection.py"
    if not helper_path.is_file() or helper_path.is_symlink():
        raise ValueError("trusted impact projection helper is unavailable")
    expected_helper = subprocess.check_output(
        ["git", "-C", str(effective), "show", f"{policy_commit}:scripts/pm/workflow-impact-projection.py"]
    )
    if helper_path.read_bytes() != expected_helper:
        raise ValueError("effective impact projection helper bytes differ from policy authority")
    spec = importlib.util.spec_from_file_location("trusted_workflow_impact_projection", helper_path)
    if spec is None or spec.loader is None:
        raise ValueError("trusted impact projection helper cannot be loaded")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    scope_base = value.get("scope_base_oid")
    source_head = data.get("headRefOid")
    try:
        subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", str(scope_base), str(source_head)],
            check=True, capture_output=True,
        )
        changed_paths = subprocess.check_output(
            ["git", "-C", str(root), "diff", "--name-only", f"{scope_base}..{source_head}"],
            text=True,
        ).splitlines()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ValueError("trusted impact projection source scope cannot be verified") from exc
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", suffix=".json") as handle:
        json.dump(value, handle)
        handle.flush()
        try:
            return helper.load_verified_projection(
                handle.name,
                expected={"task_uid": uid, "source_head_oid": source_head,
                          "scope_base_oid": scope_base, "changed_paths": changed_paths},
                repo_root=root,
            )
        except (OSError, TypeError, ValueError) as exc:
            raise ValueError(f"trusted impact projection is invalid: {exc}") from exc


def _related_path(path: str, other: str) -> bool:
    left, right = path.strip("/"), other.strip("/")
    return left == right or left.startswith(right + "/") or right.startswith(left + "/")


def _relation_path(value: Any) -> str | None:
    """Return a safe repository path from a contract/consumer locator.

    Contract references may carry a stable ``#fragment`` after the path.  The
    fragment identifies the clause, while target-drift comparison is against
    the repository path.  A missing or unsafe path is deliberately represented
    as ``None`` so callers can fail closed when the target has advanced.
    """
    if not isinstance(value, str) or not value.strip():
        return None
    path = value.strip().split("#", 1)[0].strip("/")
    if (not path or path.startswith(".") or "\x00" in path
            or "\\" in path or any(part in {"", ".", ".."} for part in path.split("/"))):
        return None
    return path


def _source_path_exists(root: Path, source_head: str, path: str) -> bool:
    try:
        return subprocess.run(
            ["git", "-C", str(root), "cat-file", "-e", f"{source_head}:{path}"],
            check=False, capture_output=True,
        ).returncode == 0
    except OSError:
        return False


def _mapped_relation_paths(
    projection: dict[str, Any], *, root: Path, source_head: str,
) -> tuple[list[str], bool]:
    """Collect trusted target-drift paths and report unmapped declarations.

    ``consumed_contracts`` describes stable inputs and therefore does not make
    every ordinary PR strict.  Once the target advances, however, each
    consumed contract must expose a repository-relative path (directly or via
    its clause/content references).  Without that mapping the gate cannot
    prove the target change is unrelated, so it escalates conservatively.
    Closure evidence is included because it is the trusted projection's
    explicit proof of the contract/consumer surface.
    """
    relations = [
        path for path in (projection.get("changed_paths") or [])
        if isinstance(path, str)
    ]
    unmapped = False

    def mapped_candidates(candidates: list[Any]) -> list[str]:
        return [
            path for value in candidates
            if (path := _relation_path(value)) and _source_path_exists(root, source_head, path)
        ]

    for item in projection.get("affected_consumers") or []:
        candidates: list[Any] = []
        if isinstance(item, str):
            candidates.append(item)
        elif isinstance(item, dict):
            candidates.extend(item.get(key) for key in ("path", "consumer_path", "contract_path"))
            refs = item.get("references")
            if isinstance(refs, list):
                candidates.extend(
                    ref.get("path") for ref in refs if isinstance(ref, dict)
                )
        mapped = mapped_candidates(candidates)
        if mapped:
            relations.extend(mapped)
        else:
            unmapped = True

    for item in projection.get("consumed_contracts") or []:
        candidates = []
        if isinstance(item, dict):
            candidates.extend(item.get(key) for key in ("path", "contract_path"))
            for container_key in ("consumed_clause_refs", "content_refs", "clauses"):
                refs = item.get(container_key)
                if isinstance(refs, list):
                    candidates.extend(
                        ref.get("path") for ref in refs if isinstance(ref, dict)
                    )
            nested = item.get("contract")
            if isinstance(nested, dict):
                candidates.append(nested.get("path"))
                refs = nested.get("content_refs")
                if isinstance(refs, list):
                    candidates.extend(
                        ref.get("path") for ref in refs if isinstance(ref, dict)
                    )
        # An identifier/revision pair is not a path mapping.  It remains valid
        # source metadata, but cannot establish unrelated target drift.
        mapped = mapped_candidates(candidates)
        if mapped:
            relations.extend(mapped)
        else:
            unmapped = True

    closure = projection.get("closure_status") or {}
    evidence = closure.get("evidence") if isinstance(closure, dict) else None
    if isinstance(evidence, list):
        for item in evidence:
            path = _relation_path(item.get("path")) if isinstance(item, dict) else None
            if path:
                if _source_path_exists(root, source_head, path):
                    relations.append(path)
                else:
                    unmapped = True
            else:
                unmapped = True

    return relations, unmapped


def trusted_requires_strict_integration(data: dict[str, Any], root: Path, effective: Path, uid: str, policy_commit: str) -> bool:
    """Derive strict escalation only from the verified published projection."""
    projection = _load_trusted_projection(data, root, effective, uid, policy_commit)
    if projection.get("review_escalated") is True or projection.get("verification_affected") is True:
        return True
    if projection.get("change_class") in {"workflow-doc", "unknown", "mixed"}:
        return True
    closure = projection.get("closure_status") or {}
    if not isinstance(closure, dict) or closure.get("status") != "complete":
        return True
    # Search only the projection values.  Including field names here would
    # make every well-formed ``consumed_contracts`` field appear risky merely
    # because the schema contains the word ``contract``.
    risk_text = json.dumps(
        [reason for reason in projection.get("review_reasons", [])
         if isinstance(reason, str) and not reason.startswith("input:")],
        sort_keys=True,
    ).lower()
    risk_terms = ("api", "abi", "persistence", "serialization", "state-root", "consensus",
                  "security", "dependency", "permission", "workflow", "validation", "contract",
                  "schema", "migration", "wasm", "replay", "recovery", "critical")
    if any(term in risk_text for term in risk_terms):
        return True
    high_risk_paths = (".github/workflows/", ".codex/", "scripts/pm/", "cargo.toml", "cargo.lock")
    if any(any(path.lower().startswith(prefix) or path.lower() == prefix.rstrip("/")
               for prefix in high_risk_paths) for path in projection.get("changed_paths", [])):
        return True
    if projection.get("public_semantics"):
        return True
    scope_base = projection.get("scope_base_oid")
    current_base = data.get("baseRefOid")
    if scope_base != current_base:
        try:
            subprocess.run(
                ["git", "-C", str(root), "merge-base", "--is-ancestor", str(scope_base), str(current_base)],
                check=True, capture_output=True,
            )
            changed = subprocess.check_output(
                ["git", "-C", str(root), "diff", "--name-only", f"{scope_base}..{current_base}"],
                text=True,
            ).splitlines()
        except (OSError, subprocess.CalledProcessError) as exc:
            raise ValueError("cannot verify target-only changes for ordinary integration policy") from exc
        relations, unmapped = _mapped_relation_paths(
            projection, root=root, source_head=str(data.get("headRefOid")),
        )
        if unmapped or not relations or any(
            _related_path(path, related) for path in changed for related in relations
        ):
            return True
    return False


def _validate_live_integration_proof(
    proof: dict[str, Any], data: dict[str, Any], *, strict: bool,
    allow_legacy_strict_fallback: bool = False,
) -> None:
    """Enforce the selected evidence mode after isolated CI discovery.

    ``selected_live`` retains a compatibility fallback for direct legacy
    callers. The production gate must never accept that ordinary fallback when
    the trusted classifier selected strict integration.
    """
    if not isinstance(proof, dict):
        raise ValueError('live integration proof is not an object')
    if strict and not allow_legacy_strict_fallback and proof.get('ci_validation_mode') != 'trusted_integration':
        raise ValueError('strict integration requires a trusted integration receipt')
    if proof.get('head_oid') != data['headRefOid']:
        raise ValueError('source-bound PR CI head changed; rerun required CI for the current source')
    if proof.get('base_ref') != data['baseRefName']:
        raise ValueError('source-bound PR CI target ref changed; rerun required CI for the current target ref')
    if 'assessed_target_oid' in proof and not re.fullmatch(
            r'[0-9a-f]{40,64}', str(proof.get('assessed_target_oid') or '')):
        raise ValueError('fresh default-branch target identity is invalid')
    if proof.get('check_name') is not None:
        if proof.get('check_name') != 'required-gate':
            raise ValueError('selected integration check is not required-gate')
        policy = data.get('policy_discovery') or {}
        required = policy.get('required_status_checks') or []
        pins = {str(item['app_id']) for item in required
                if isinstance(item, dict) and item.get('context') == 'required-gate'
                and item.get('app_id') is not None}
        if len(pins) != 1 or str(proof.get('check_app_id')) != next(iter(pins), None):
            raise ValueError('selected integration check app differs from live required-check policy')
    if strict and proof.get('integration_base_oid') != data['baseRefOid']:
        _validate_keyed_q_applicability(proof, data)


def _positive_bootstrap_epoch(value: Any, label: str = 'bootstrap_epoch') -> int:
    if type(value) is not int or value <= 0:
        raise ValueError(f'{label} must be a positive integer')
    return value


def validate_keyed_integration_identity(
    proof: dict[str, Any], data: dict[str, Any], task_uid: str, task: dict[str, Any],
) -> bool:
    """Bind keyed workflow evidence to the current Task, PR, H/S, policy and check.

    The live receipt reader authenticates the journal, W policy, request key,
    artifacts, and exact attempt. These comparisons bind that proof to this
    lifecycle caller's admitted Task and current PR identity.
    """
    request_key = proof.get('request_key')
    request_identity = proof.get('request_identity')
    if request_key is None:
        if request_identity is not None:
            raise ValueError('unkeyed integration proof carries a request identity')
        return False
    if not isinstance(request_key, str) or not re.fullmatch(r'sha256:[0-9a-f]{64}', request_key):
        raise ValueError('keyed integration request key is malformed')
    if not isinstance(request_identity, dict):
        raise ValueError('keyed integration request identity is missing')
    assessed_target_oid = proof.get('assessed_target_oid')
    if not isinstance(assessed_target_oid, str) or not re.fullmatch(r'[0-9a-f]{40,64}', assessed_target_oid):
        raise ValueError('keyed integration assessed target identity is missing or invalid')
    epoch = _positive_bootstrap_epoch(task.get('bootstrap_epoch'))
    binding = task.get('loop_binding')
    if isinstance(binding, dict):
        binding_epoch = _positive_bootstrap_epoch(
            binding.get('bootstrap_epoch'), 'Task loop binding bootstrap_epoch',
        )
        if binding_epoch != epoch:
            raise ValueError('keyed integration Task binding bootstrap epoch mismatch')
    expected = {
        'repository': data.get('repository'),
        'task_uid': task_uid,
        'pr_number': data.get('number'),
        'bootstrap_epoch': epoch,
        'source_head_oid': data.get('headRefOid'),
    }
    if any(type(request_identity.get(field)) is not type(value)
           or request_identity.get(field) != value for field, value in expected.items()):
        raise ValueError('keyed integration request identity differs from Task or PR')
    projection_digest = request_identity.get('source_projection_digest')
    if not isinstance(projection_digest, str) or not re.fullmatch(r'sha256:[0-9a-f]{64}', projection_digest):
        raise ValueError('keyed integration source projection digest is invalid')

    source_scope = proof.get('source_scope_oid')
    if not isinstance(source_scope, str) or not re.fullmatch(r'[0-9a-f]{40,64}', source_scope):
        raise ValueError('keyed integration source scope is missing or invalid')
    plan = proof.get('required_plan_v2_payload')
    planner_output = plan.get('planner_output') if isinstance(plan, dict) else None
    if isinstance(plan, dict):
        plan_epoch = _positive_bootstrap_epoch(
            plan.get('bootstrap_epoch'), 'required plan bootstrap_epoch',
        )
        plan_pr_number = plan.get('pr_number')
        if type(plan_pr_number) is not int or plan_pr_number <= 0:
            raise ValueError('required plan PR number is invalid')
    if (not isinstance(plan, dict)
            or plan.get('schema') != 'oasis7-required-plan-v2'
            or plan.get('request_key') != request_key
            or plan.get('request_identity') != request_identity
            or plan.get('repository') != expected['repository']
            or plan.get('task_uid') != task_uid
            or plan_pr_number != expected['pr_number']
            or plan_epoch != epoch
            or plan.get('source_head_oid') != expected['source_head_oid']
            or plan.get('source_scope_oid') != source_scope
            or not isinstance(planner_output, dict)
            or planner_output.get('source_scope_base') != source_scope
            or planner_output.get('impact_projection_digest') != projection_digest):
        raise ValueError('keyed required-plan source identity differs from verified Task projection')

    effective_identity = proof.get('effective_policy_identity')
    policy_context = proof.get('trusted_policy_context')
    planner_authority = proof.get('planner_inventory_authority')
    if (not isinstance(effective_identity, dict)
            or effective_identity.get('schema') != 'oasis7-ci-effective-policy-identity/v1'
            or not re.fullmatch(r'sha256:[0-9a-f]{64}', str(effective_identity.get('digest') or ''))
            or not isinstance(policy_context, dict)
            or policy_context.get('effective_policy_identity') != effective_identity
            or request_identity.get('effective_policy_digest') != effective_identity.get('digest')
            or policy_context.get('planner_inventory_authority') != planner_authority
            or plan.get('effective_policy_identity') != effective_identity
            or plan.get('planner_inventory_authority') != planner_authority):
        raise ValueError('keyed integration effective policy or planner authority mismatch')

    check_app_id = proof.get('check_app_id')
    check_run_id = proof.get('check_run_id')
    if (proof.get('check_name') != 'required-gate'
            or type(check_app_id) is not int or check_app_id <= 0
            or type(check_run_id) is not int or check_run_id <= 0
            or plan.get('check_name') != 'required-gate'
            or type(plan.get('check_app_id')) is not int
            or type(plan.get('check_run_id')) is not int
            or plan.get('check_app_id') != check_app_id
            or plan.get('check_run_id') != check_run_id):
        raise ValueError('keyed integration check identity is invalid')
    required = (data.get('policy_discovery') or {}).get('required_status_checks') or []
    pinned_apps = {str(item['app_id']) for item in required
                   if isinstance(item, dict) and item.get('context') == 'required-gate'
                   and item.get('app_id') is not None}
    if len(pinned_apps) != 1 or str(check_app_id) != next(iter(pinned_apps), None):
        raise ValueError('keyed integration app differs from the current required-gate pin')
    run_id = proof.get('workflow_run_id')
    run_attempt = proof.get('run_attempt')
    if (type(run_id) is not int or run_id <= 0
            or type(run_attempt) is not int or run_attempt <= 0
            or type(proof.get('run_id')) is not int
            or type(proof.get('request_id')) is not int
            or proof.get('run_id') != run_id or proof.get('request_id') != run_id
            or type(plan.get('workflow_run_id')) is not int
            or type(plan.get('run_attempt')) is not int
            or plan.get('workflow_run_id') != run_id
            or plan.get('run_attempt') != run_attempt):
        raise ValueError('keyed integration workflow attempt identity is invalid')
    return True


def _load_effective_helper(effective: Path, name: str):
    path = effective / 'scripts/pm' / (name + '.py')
    if path.is_symlink() or not path.is_file():
        raise ValueError('effective integration helper is unavailable: ' + name)
    spec = importlib.util.spec_from_file_location('lifecycle_' + name, path)
    if spec is None or spec.loader is None:
        raise ValueError('effective integration helper cannot be loaded: ' + name)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@contextmanager
def _effective_ci_modules(effective: Path):
    """Load the C4/C0 reader stack only from byte-verified effective helpers."""
    names = (
        "integration_executor_contract", "ci_ready_receipt_identity",
        "ci_input_scope", "ci_required_artifact_v2",
        "ci_evidence_applicability", "integration_ci",
    )
    previous = {name: sys.modules.get(name) for name in names}
    loaded = {}
    try:
        for name in names:
            path = effective / "scripts/pm" / (name + ".py")
            if path.is_symlink() or not path.is_file():
                raise ValueError("effective CI helper is unavailable: " + name)
            spec = importlib.util.spec_from_file_location(name, path)
            if spec is None or spec.loader is None:
                raise ValueError("effective CI helper cannot be loaded: " + name)
            module = importlib.util.module_from_spec(spec)
            sys.modules[name] = module
            spec.loader.exec_module(module)
            loaded[name] = module
        yield loaded
    finally:
        for name, module in previous.items():
            if module is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = module


def _canonical_digest(value: Any) -> str:
    raw = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False,
    ).encode("utf-8")
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def evaluate_keyed_q_applicability(
    source_proof: dict[str, Any], target_inventory: dict[str, Any],
    applicability_module: Any, live_pr_data: dict[str, Any],
) -> dict[str, Any]:
    """Apply C0's test-evidence dimension to a separate fresh-Q observation.

    Role-review applicability is intentionally outside this CI-only projection;
    the existing live PR review and conversation checks remain authoritative.
    """
    if not isinstance(source_proof, dict) or not isinstance(target_inventory, dict):
        raise ValueError("keyed target applicability inputs are malformed")
    plan = source_proof.get("required_plan_v2_payload")
    request_identity = source_proof.get("request_identity")
    source_inventory = source_proof.get("trusted_planner_inventory")
    source_attempt = source_proof.get("trusted_source_attempt")
    if (not isinstance(plan, dict) or not isinstance(request_identity, dict)
            or not isinstance(source_inventory, dict) or not isinstance(source_attempt, dict)):
        raise ValueError("keyed source plan or trusted inventory is missing")
    if target_inventory.get("schema") != LOCAL_TARGET_INVENTORY_SCHEMA:
        raise ValueError("fresh Q target inventory schema is unsupported")
    if target_inventory.get("closure_status") != "complete":
        raise ValueError("fresh Q target input closure is unknown or partial")
    observation = target_inventory.get("target_observation")
    input_scope = target_inventory.get("input_scope")
    input_closure = input_scope.get("closure_status") if isinstance(input_scope, dict) else None
    if (not isinstance(observation, dict) or not isinstance(input_scope, dict)
            or input_scope.get("target_observation") != observation
            or not isinstance(input_closure, dict)
            or input_closure.get("status") != "complete"):
        raise ValueError("fresh Q target observation or complete input scope is missing")

    identity_fields = {
        "repository": live_pr_data.get("repository"),
        "task_uid": request_identity.get("task_uid"),
        "pr_number": live_pr_data.get("number"),
        "source_head_oid": live_pr_data.get("headRefOid"),
        "source_scope_oid": source_proof.get("source_scope_oid"),
    }
    for field, expected in identity_fields.items():
        if target_inventory.get(field) != expected or plan.get(field) != expected:
            raise ValueError("fresh Q target observation source identity mismatch: " + field)
        if field != "source_scope_oid" and request_identity.get(field) != expected:
            raise ValueError("fresh Q target observation request identity mismatch: " + field)
    base = source_proof.get("integration_base_oid")
    target_oid = target_inventory.get("assessed_target_oid")
    if (not isinstance(base, str) or not re.fullmatch(r"[0-9a-f]{40,64}", base)
            or plan.get("integration_base_oid") != base
            or target_oid != live_pr_data.get("baseRefOid")
            or source_proof.get("assessed_target_oid") != target_oid
            or target_inventory.get("source_scope_oid") != source_proof.get("source_scope_oid")):
        raise ValueError("fresh Q target observation differs from immutable B or live PR Q")
    commit_oid = target_inventory.get("input_scope_commit_oid")
    tree_oid = target_inventory.get("input_scope_tree_oid")
    authority_oid = target_inventory.get("planner_authority_oid")
    config_digest = target_inventory.get("planner_config_sha256")
    policy_identity = target_inventory.get("effective_policy_identity")
    if (not isinstance(commit_oid, str) or not re.fullmatch(r"[0-9a-f]{40,64}", commit_oid)
            or not isinstance(tree_oid, str) or not re.fullmatch(r"[0-9a-f]{40,64}", tree_oid)
            or authority_oid != target_oid
            or not isinstance(config_digest, str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", config_digest)
            or not isinstance(policy_identity, dict)
            or observation.get("effective_policy_identity") != policy_identity
            or observation.get("input_scope_commit_oid") != commit_oid
            or observation.get("input_scope_tree_oid") != tree_oid):
        raise ValueError("fresh Q W authority, M/T, or effective policy identity is invalid")
    unit_ids = target_inventory.get("required_test_units")
    if (not isinstance(unit_ids, list) or not unit_ids
            or any(not isinstance(unit, str) or not unit for unit in unit_ids)
            or unit_ids != sorted(set(unit_ids))
            or input_scope.get("required_test_units") != unit_ids):
        raise ValueError("fresh Q complete test-unit inventory is malformed")

    # C0 owns the execution test dimension here. Professional-role evidence is
    # not synthesized from GitHub approval state; the existing lifecycle review
    # gates remain independently required.
    review_digest = _canonical_digest({
        "schema": "oasis7-ci-lifecycle-test-only-review-dimension/v1",
        "required_review_roles": [],
    })
    c0_source_plan = dict(plan)
    c0_source_plan["required_review_roles"] = []
    c0_source_plan["review_applicability_digest"] = review_digest
    c0_target = {
        "repository": identity_fields["repository"],
        "task_uid": identity_fields["task_uid"],
        "pr_number": identity_fields["pr_number"],
        "source_head_oid": identity_fields["source_head_oid"],
        "source_scope_oid": identity_fields["source_scope_oid"],
        "target_oid": target_oid,
        "prior_assessed_target_oid": None,
        "input_scope_commit_oid": commit_oid,
        "input_scope_tree_oid": tree_oid,
        "review_applicability_digest": review_digest,
        "required_test_units": unit_ids,
        "required_review_roles": [],
        "input_scope": input_scope,
        "unit_specs": target_inventory.get("unit_specs"),
        "product_corpus": target_inventory.get("product_corpus"),
    }
    raw_results = source_proof.get("required_result_v2_artifacts")
    if not isinstance(raw_results, list):
        raise ValueError("keyed source result artifact set is missing")
    expected_attempt_identity = {
        "workflow_run_id": source_proof.get("workflow_run_id"),
        "run_attempt": source_proof.get("run_attempt"),
        "check_app_id": source_proof.get("check_app_id"),
        "check_run_id": source_proof.get("check_run_id"),
    }
    if any(type(value) is not int or value <= 0
           for value in expected_attempt_identity.values()):
        raise ValueError("keyed source R/A/check identity is malformed")
    expected_trusted_attempt = {
        **expected_attempt_identity,
        "request_key": source_proof.get("request_key"),
        "job_id": source_proof.get("job_id"),
        "job_name": source_proof.get("job_name"),
        "plan_artifact_id": source_proof.get("required_plan_v2_artifact_id"),
        "plan_artifact_name": source_proof.get("required_plan_v2_artifact_name"),
    }
    if (source_attempt.get("schema") != "oasis7-ci-trusted-source-attempt/v1"
            or any(type(value) is not int or value <= 0
                   for field, value in expected_trusted_attempt.items()
                   if field != "job_name" and field != "request_key"
                   and field != "plan_artifact_name")
            or any(source_attempt.get(field) != value
                   for field, value in expected_trusted_attempt.items())):
        raise ValueError("trusted source attempt differs from the selected exact R/A/check/artifact")
    if (not isinstance(expected_trusted_attempt["request_key"], str)
            or not isinstance(expected_trusted_attempt["job_name"], str)
            or not isinstance(expected_trusted_attempt["plan_artifact_name"], str)):
        raise ValueError("trusted source attempt locator identity is malformed")
    trusted_results = source_attempt.get("result_artifacts")
    if not isinstance(trusted_results, list):
        raise ValueError("trusted source attempt result artifact set is malformed")
    trusted_by_unit = {}
    for item in trusted_results:
        if (not isinstance(item, dict) or set(item) != {"unit_id", "artifact_id", "name"}
                or not isinstance(item.get("unit_id"), str) or not item["unit_id"]
                or type(item.get("artifact_id")) is not int or item["artifact_id"] <= 0
                or not isinstance(item.get("name"), str) or not item["name"]
                or item["unit_id"] in trusted_by_unit):
            raise ValueError("trusted source attempt result locator is malformed")
        trusted_by_unit[item["unit_id"]] = item
    tests = []
    result_units = []
    for artifact in raw_results:
        if (not isinstance(artifact, dict)
                or set(artifact) != {"artifact_id", "name", "payload"}
                or type(artifact.get("artifact_id")) is not int
                or artifact["artifact_id"] <= 0
                or not isinstance(artifact.get("payload"), dict)):
            raise ValueError("keyed source result artifact locator is malformed")
        result = artifact["payload"]
        if any(result.get(field) != expected
               for field, expected in expected_attempt_identity.items()):
            raise ValueError("keyed source result differs from the exact R/A/check attempt")
        unit_id = result.get("unit_id")
        trusted_result = trusted_by_unit.get(unit_id)
        if (trusted_result is None
                or artifact["artifact_id"] != trusted_result["artifact_id"]
                or artifact["name"] != trusted_result["name"]
                or unit_id in result_units):
            raise ValueError("keyed source result locators differ from trusted source attempt")
        result_units.append(unit_id)
        tests.append({
            "unit_id": unit_id,
            "obligation_ids": result.get("obligation_ids"),
            "status": result.get("status"),
            "input_digest": result.get("input_digest"),
            "inventory_digest": result.get("planner_inventory_digest"),
            "effective_policy_identity": result.get("effective_policy_identity"),
            "repository": result.get("repository"),
            "task_uid": result.get("task_uid"),
            "pr_number": result.get("pr_number"),
            "source_head_oid": result.get("source_head_oid"),
            "source_scope_oid": result.get("source_scope_oid"),
            "run_id": result.get("workflow_run_id"),
            "run_attempt": result.get("run_attempt"),
            "check_app_id": result.get("check_app_id"),
            "check_run_id": result.get("check_run_id"),
            "artifact_id": artifact["artifact_id"],
            "artifact_name": artifact["name"],
        })
    if result_units != sorted(trusted_by_unit):
        raise ValueError("keyed source result artifact set is incomplete or out of order")
    try:
        decision = applicability_module.evaluate_evidence_applicability(
            c0_source_plan, {"reviews": [], "tests": tests}, c0_target,
            target_inventory.get("effective_policy"),
            trusted_source_inventory=source_inventory,
            trusted_source_attempt=source_attempt,
            trusted_target_observation=observation,
        ).to_dict()
    except (AttributeError, KeyError, TypeError, ValueError) as exc:
        raise ValueError("C0 could not evaluate keyed source evidence against fresh Q") from exc

    assessment = {
        "schema": KEYED_Q_APPLICABILITY_SCHEMA,
        "request_key": source_proof.get("request_key"),
        "workflow_run_id": source_proof.get("workflow_run_id"),
        "run_attempt": source_proof.get("run_attempt"),
        "check_app_id": source_proof.get("check_app_id"),
        "check_run_id": source_proof.get("check_run_id"),
        "trusted_source_attempt": source_attempt,
        "integration_base_oid": base,
        "source_head_oid": identity_fields["source_head_oid"],
        "source_scope_oid": identity_fields["source_scope_oid"],
        "assessed_target_oid": target_oid,
        "input_scope_commit_oid": commit_oid,
        "input_scope_tree_oid": tree_oid,
        "planner_authority_oid": authority_oid,
        "planner_config_sha256": config_digest,
        "effective_policy_identity": policy_identity,
        "inventory_digest": observation.get("inventory_digest"),
        "required_test_units": unit_ids,
        "closure_status": "complete",
        "test_evidence": decision.get("test_evidence"),
        "decision": decision,
    }
    assessment["decision_digest"] = _canonical_digest(decision)
    _validate_keyed_q_applicability(
        {**source_proof, "keyed_q_applicability": assessment}, live_pr_data,
    )
    return assessment


def _validate_keyed_q_applicability(
    proof: dict[str, Any], data: dict[str, Any],
) -> dict[str, Any]:
    """Validate the C0 decision and produce the exact gate-epoch identity."""
    assessment = proof.get("keyed_q_applicability")
    fields = {
        "schema", "request_key", "workflow_run_id", "run_attempt", "check_app_id",
        "check_run_id", "integration_base_oid", "source_head_oid", "source_scope_oid",
        "trusted_source_attempt",
        "assessed_target_oid", "input_scope_commit_oid", "input_scope_tree_oid",
        "planner_authority_oid", "planner_config_sha256", "effective_policy_identity",
        "inventory_digest", "required_test_units", "closure_status", "test_evidence",
        "decision", "decision_digest",
    }
    if not isinstance(assessment, dict) or set(assessment) != fields:
        raise ValueError("keyed target Q applicability assessment is missing or malformed")
    if assessment.get("schema") != KEYED_Q_APPLICABILITY_SCHEMA:
        raise ValueError("keyed target Q applicability schema is unsupported")
    request_key = proof.get("request_key")
    if (not isinstance(request_key, str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", request_key)
            or assessment.get("request_key") != request_key):
        raise ValueError("keyed target Q request identity is invalid")
    for field in ("workflow_run_id", "run_attempt", "check_app_id", "check_run_id"):
        value = proof.get(field)
        if type(value) is not int or value <= 0 or assessment.get(field) != value:
            raise ValueError("keyed target Q latest run/check identity mismatch: " + field)
    plan = proof.get("required_plan_v2_payload")
    if not isinstance(plan, dict):
        raise ValueError("keyed target Q source plan is missing")
    identity_pairs = {
        "integration_base_oid": proof.get("integration_base_oid"),
        "source_head_oid": data.get("headRefOid"),
        "source_scope_oid": proof.get("source_scope_oid"),
        "assessed_target_oid": data.get("baseRefOid"),
    }
    for field, expected in identity_pairs.items():
        value = assessment.get(field)
        if (not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{40,64}", expected)
                or value != expected):
            raise ValueError("keyed target Q identity drift: " + field)
    if (assessment["assessed_target_oid"] != proof.get("assessed_target_oid")
            or assessment["integration_base_oid"] != plan.get("integration_base_oid")
            or assessment["source_head_oid"] != plan.get("source_head_oid")
            or assessment["source_scope_oid"] != plan.get("source_scope_oid")
            or assessment["workflow_run_id"] != plan.get("workflow_run_id")
            or assessment["run_attempt"] != plan.get("run_attempt")
            or assessment["check_app_id"] != plan.get("check_app_id")
            or assessment["check_run_id"] != plan.get("check_run_id")
            or assessment["trusted_source_attempt"] != proof.get("trusted_source_attempt")):
        raise ValueError("keyed target Q assessment differs from exact source attempt")
    for field in ("input_scope_commit_oid", "input_scope_tree_oid", "planner_authority_oid"):
        if not isinstance(assessment.get(field), str) or not re.fullmatch(r"[0-9a-f]{40,64}", assessment[field]):
            raise ValueError("keyed target Q identity is invalid: " + field)
    if (assessment["planner_authority_oid"] != assessment["assessed_target_oid"]
            or not isinstance(assessment.get("planner_config_sha256"), str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", assessment["planner_config_sha256"])
            or not isinstance(assessment.get("inventory_digest"), str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", assessment["inventory_digest"])
            or not isinstance(assessment.get("effective_policy_identity"), dict)
            or assessment["effective_policy_identity"].get("schema")
               != "oasis7-ci-effective-policy-identity/v1"
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", str(
                assessment["effective_policy_identity"].get("digest") or ""))):
        raise ValueError("keyed target Q W, inventory, or policy identity is malformed")
    required_units = assessment.get("required_test_units")
    if (assessment.get("closure_status") != "complete"
            or not isinstance(required_units, list)
            or not required_units
            or any(not isinstance(unit, str) or not unit for unit in required_units)
            or required_units != sorted(set(required_units))):
        raise ValueError("keyed target Q input closure is unknown, partial, or malformed")

    decision = assessment.get("decision")
    if (not isinstance(decision, dict)
            or assessment.get("decision_digest") != _canonical_digest(decision)
            or decision.get("test_evidence") != assessment.get("test_evidence")
            or decision.get("effective_policy_identity") != assessment.get("effective_policy_identity")):
        raise ValueError("keyed target Q C0 decision digest or policy identity is invalid")
    trusted_attempt = proof.get("trusted_source_attempt")
    locators = decision.get("evidence_locators")
    expected_attempt_locator = {"kind": "trusted-source-attempt", "id": trusted_attempt}
    if (not isinstance(trusted_attempt, dict) or not isinstance(locators, list)
            or locators.count(expected_attempt_locator) != 1):
        raise ValueError("keyed target Q C0 decision omits the exact trusted source attempt locator")
    decision_identity = decision.get("identity")
    if (not isinstance(decision_identity, dict)
            or decision_identity.get("source_head_oid") != assessment["source_head_oid"]
            or decision_identity.get("source_scope_oid") != assessment["source_scope_oid"]
            or decision_identity.get("assessed_target_oid") != assessment["assessed_target_oid"]):
        raise ValueError("keyed target Q C0 decision identity mismatch")
    status = assessment.get("test_evidence")
    decision_blockers = decision.get("blockers")
    reused = decision.get("reused_units")
    required = decision.get("required_test_units")
    if (not isinstance(decision_blockers, list) or not isinstance(reused, list)
            or not isinstance(required, list)):
        raise ValueError("keyed target Q C0 decision lists are malformed")
    if status == "disabled":
        if assessment["integration_base_oid"] != assessment["assessed_target_oid"]:
            raise ValueError("input-scope reuse is disabled and source B is stale for current Q")
        if decision_blockers or reused or required:
            raise ValueError("disabled keyed target Q decision contains reuse or blockers")
    elif status == "reusable":
        if (decision_blockers or required
                or reused != assessment["required_test_units"]
                or decision_identity.get("input_scope_commit_oid") != assessment["input_scope_commit_oid"]
                or decision_identity.get("input_scope_tree_oid") != assessment["input_scope_tree_oid"]
                or decision_identity.get("target_inventory_digest") != assessment["inventory_digest"]):
            raise ValueError("keyed target Q C0 test decision is incomplete or drifted")
        result_locators = {
            item["unit_id"]: item for item in trusted_attempt.get("result_artifacts", [])
            if isinstance(item, dict) and isinstance(item.get("unit_id"), str)
        }
        item_decisions = decision.get("item_decisions")
        if not isinstance(item_decisions, list):
            raise ValueError("keyed target Q C0 per-item decisions are malformed")
        for unit in assessment["required_test_units"]:
            rows = [item for item in item_decisions
                    if isinstance(item, dict) and item.get("kind") == "test"
                    and item.get("id") == unit]
            locator = result_locators.get(unit)
            expected_locator = {
                "kind": "github-check-artifact",
                "id": {
                    "unit_id": unit,
                    "run_id": trusted_attempt.get("workflow_run_id"),
                    "run_attempt": trusted_attempt.get("run_attempt"),
                    "check_app_id": str(trusted_attempt.get("check_app_id")),
                    "check_run_id": trusted_attempt.get("check_run_id"),
                    "artifact_id": locator.get("artifact_id") if isinstance(locator, dict) else None,
                },
            }
            if (len(rows) != 1 or rows[0].get("disposition") != "reusable"
                    or rows[0].get("evidence_locator") != expected_locator):
                raise ValueError("keyed target Q C0 per-unit locator differs from trusted source attempt")
    elif status in ("revalidate", "blocked"):
        raise ValueError("keyed target Q requires revalidation or is blocked: " + status)
    else:
        raise ValueError("keyed target Q C0 test decision status is unsupported")
    return {
        "schema": KEYED_Q_APPLICABILITY_SCHEMA,
        "request_key": request_key,
        "workflow_run_id": proof["workflow_run_id"],
        "run_attempt": proof["run_attempt"],
        "check_app_id": proof["check_app_id"],
        "check_run_id": proof["check_run_id"],
        "trusted_source_attempt": assessment["trusted_source_attempt"],
        "integration_base_oid": assessment["integration_base_oid"],
        "source_head_oid": assessment["source_head_oid"],
        "source_scope_oid": assessment["source_scope_oid"],
        "assessed_target_oid": assessment["assessed_target_oid"],
        "input_scope_commit_oid": assessment["input_scope_commit_oid"],
        "input_scope_tree_oid": assessment["input_scope_tree_oid"],
        "planner_authority_oid": assessment["planner_authority_oid"],
        "planner_config_sha256": assessment["planner_config_sha256"],
        "inventory_digest": assessment["inventory_digest"],
        "effective_policy_identity": assessment["effective_policy_identity"],
        "test_evidence": status,
        "decision_digest": assessment["decision_digest"],
    }


def _latest_local_keyed_request_key(root: Path, effective: Path, *, repository: str,
                                    uid: str, pr_number: int, base_oid: str,
                                    head_oid: str, branch: str, task: dict[str, Any],
                                    projection_digest: str,
                                    allow_advanced_target: bool = False) -> str | None:
    """Select the latest durable intent before applying its state barrier.

    Local intent order is persisted before dispatch and is the only authority
    for comparing distinct request keys. The newest matching prepared,
    uncertain, or observed intent controls; an older uncertain intent cannot
    contaminate a later request, and a newer unresolved intent cannot fall
    back to older green evidence.

    For a fresh-Q assessment, ``base_oid`` is the observed live target Q but
    the selected request is still read back using its journaled immutable B.
    The caller's trusted target replay proves B is an ancestor of Q.
    """
    binding = task.get('loop_binding')
    if not isinstance(binding, dict):
        return None
    epoch = _positive_bootstrap_epoch(task.get('bootstrap_epoch'))
    if binding.get('bootstrap_epoch') != epoch:
        raise ValueError('Task loop binding bootstrap epoch mismatch')
    if not isinstance(projection_digest, str) or not re.fullmatch(r'sha256:[0-9a-f]{64}', projection_digest):
        raise ValueError('trusted source projection digest is invalid')
    integration = _load_effective_helper(effective, 'integration_ci')
    contract = _load_effective_helper(effective, 'integration_executor_contract')
    directory = Path(integration.git_common_dir(root))
    if not directory.exists():
        return None
    expected = {
        'repository': repository,
        'task_uid': uid,
        'pr_number': pr_number,
        'bootstrap_epoch': epoch,
        'source_head_oid': head_oid,
        'source_projection_digest': projection_digest,
    }
    def latest_matching_intent() -> tuple[str, dict[str, Any]] | None:
        """Read the newest matching durable intent while holding its journal lock."""
        contract.validate_validation_intent_order_state(directory)
        intents: list[tuple[int | None, str, dict[str, Any]]] = []
        for path in sorted(directory.glob('*.json')):
            if not re.fullmatch(r'[0-9a-f]{64}', path.stem):
                raise ValueError('validation request journal has a malformed key filename')
            key = 'sha256:' + path.stem
            record = contract._read_request_record(path, key)
            identity = record['identity']
            if any(identity.get(field) != value for field, value in expected.items()):
                continue
            if (not allow_advanced_target
                    and record.get('integration_base_oid') != base_oid):
                continue
            intent_order = record.get('intent_order')
            if intent_order is not None and (type(intent_order) is not int or intent_order < 1):
                raise ValueError('matching keyed request has an invalid immutable intent order')
            record_base = record.get('integration_base_oid')
            if not isinstance(record_base, str) or not re.fullmatch(r'[0-9a-f]{40,64}', record_base):
                raise ValueError('validation request journal immutable integration base is invalid')
            intents.append((intent_order, key, record))
        if not intents:
            return None
        legacy = [item for item in intents if item[0] is None]
        if legacy:
            if len(intents) != 1:
                raise ValueError('matching legacy keyed request has ambiguous cross-key intent order')
            _intent_order, selected_key, selected_record = legacy[0]
        else:
            intents.sort(key=lambda item: item[0])
            if len({item[0] for item in intents}) != len(intents):
                raise ValueError('multiple matching validation intents share one immutable order')
            _intent_order, selected_key, selected_record = intents[-1]
        return selected_key, selected_record

    def require_observed(record: dict[str, Any]) -> None:
        selected_status = record.get('status')
        if selected_status != 'observed':
            if selected_status == 'dispatch_uncertain':
                raise ValueError('latest keyed validation request dispatch is unresolved for the current Task/PR/source')
            raise ValueError('latest keyed validation request intent has not been observed')

    # Select and snapshot local authority under the journal lock, then release
    # it before the GitHub readback. Request reservation must not wait on a slow
    # network operation.
    with contract.validation_intent_order_lock(directory):
        selected_intent = latest_matching_intent()
        if selected_intent is None:
            return None
        selected_key, selected_record = selected_intent
        require_observed(selected_record)
        selected_run_id = selected_record['run_id']
        selected_attempt = selected_record['run_attempt']
        selected_base_oid = selected_record['integration_base_oid']
        selected_order = selected_record.get('intent_order')

    selected = integration.current_request(
        repository, uid, pr_number, selected_base_oid, head_oid, branch,
        request_key=selected_key,
    )
    if (not isinstance(selected, dict) or selected.get('id') != selected_run_id
            or type(selected.get('run_attempt')) is not int
            or selected['run_attempt'] < selected_attempt):
        raise ValueError('durable keyed validation request is absent from complete current-run readback')

    # The journal may have advanced while remote evidence was being fetched.
    # Recheck under the lock and never return a key that is no longer the
    # newest matching intent. A newly reserved prepared/uncertain row retains
    # its normal fail-closed barrier here.
    with contract.validation_intent_order_lock(directory):
        latest_intent = latest_matching_intent()
        if latest_intent is None:
            raise ValueError('latest keyed validation request intent changed during live readback')
        latest_key, latest_record = latest_intent
        require_observed(latest_record)
        if latest_key != selected_key:
            raise ValueError('latest keyed validation request intent changed during live readback')
        if (latest_record.get('intent_order') != selected_order
                or latest_record.get('run_id') != selected_run_id
                or latest_record.get('run_attempt') != selected_attempt):
            raise ValueError('durable keyed validation request changed during complete current-run readback')
    return selected_key


def live_integration_admission(data, root, uid, tool_root, admission, integration_run_id=None, *, require_strict=None):
    """Read trusted source-bound PR CI or strict integration evidence."""
    policy = data.get('policy_discovery') or {}
    required = policy.get('required_status_checks')
    legacy = isinstance(admission, dict) and admission.get('status') == 'legacy'
    if legacy and policy.get('status') == 'resolved' and required == []:
        return None
    pins = {str(item['app_id']) for item in (required or [])
            if isinstance(item, dict) and item.get('context') == 'required-gate' and item.get('app_id') is not None}
    if len(pins) != 1 or not next(iter(pins), '').isdigit():
        raise ValueError('required-gate needs one unambiguous app pin in live required-check policy; restore the policy binding and rerun')
    if not isinstance(admission, dict) or not isinstance(admission.get('task'), dict):
        raise ValueError('trusted local task admission context missing')
    effective = Path(admission['tool_root'])
    commit = admission.get('policy_commit')
    if not commit:
        # Legacy tasks still use immutable main helper bytes, not candidate
        # helpers or a caller-authored receipt as CI authority.
        subprocess.run(['git','-C',str(root),'fetch','--no-tags','origin','main:refs/remotes/origin/main'],check=True,capture_output=True)
        commit = subprocess.check_output(['git','-C',str(root),'rev-parse','refs/remotes/origin/main'],text=True).strip()
    task = admission['task']
    authority_helpers = (
        'ci-ready-receipt.py', 'ci_ready_receipt_identity.py', 'integration_ci.py',
        'integration_executor_contract.py',
    )
    keyed_helpers = (
        'ci_input_scope.py', 'ci_required_artifact_v2.py',
        'ci_evidence_applicability.py', 'ci_required_inventory.py',
        'ci_reuse_policy.py',
    )
    for name in authority_helpers:
        relative = 'scripts/pm/' + name
        expected = subprocess.check_output(['git','-C',str(root),'show',commit + ':' + relative])
        path = effective / relative
        if path.is_symlink() or path.read_bytes() != expected:
            raise ValueError('effective CI authority helper bytes differ: ' + name)
    if require_strict == "auto":
        projection_relative = 'scripts/pm/workflow-impact-projection.py'
        projection_expected = subprocess.check_output(['git','-C',str(effective),'show',commit + ':' + projection_relative])
        projection_path = effective / projection_relative
        if projection_path.is_symlink() or projection_path.read_bytes() != projection_expected:
            raise ValueError('effective CI authority helper bytes differ: workflow-impact-projection.py')
    if task.get('repository') != data['repository']:
        raise ValueError('CI task repository identity mismatch')
    strict = (trusted_requires_strict_integration(data, root, effective, uid, commit)
              if require_strict == "auto" else True if require_strict is None else bool(require_strict))
    if integration_run_id is not None:
        strict = True
    projection = None
    request_key = None
    if isinstance(task.get('loop_binding'), dict):
        # The same W-validated projection used by the strictness classifier
        # selects a local journal key; the key is never taken from PR text or
        # an artifact payload.
        projection = _load_trusted_projection(data, root, effective, uid, commit)
        request_key = _latest_local_keyed_request_key(
            Path(root), effective, repository=data['repository'], uid=uid,
            pr_number=int(data['number']), base_oid=data['baseRefOid'],
            head_oid=data['headRefOid'], branch=data['baseRefName'], task=task,
            projection_digest=projection['projection_digest'],
            # The source attempt's B is immutable while the default branch may
            # advance.  The local W replay below separately proves B -> fresh Q.
            allow_advanced_target=True,
        )
        if request_key is not None:
            for name in keyed_helpers:
                relative = 'scripts/pm/' + name
                expected = subprocess.check_output(['git', '-C', str(root), 'show', commit + ':' + relative])
                path = effective / relative
                if path.is_symlink() or path.read_bytes() != expected:
                    raise ValueError('effective keyed CI authority helper bytes differ: ' + name)
    request = {'root': str(effective), 'repository': data['repository'], 'uid': uid,
               'canonical_root': str(root), 'issue': task['issue_number'], 'pr': data['number'],
               'app': next(iter(pins)), 'request_key': request_key,
               'base_ref': data['baseRefName'], 'integration_run_id': integration_run_id,
               'require_strict': strict,
               # ``None`` is the direct compatibility API: it retains the
               # legacy strict check fallback used by existing callers. Every
               # production-selected strict mode carries an explicit policy
               # selector and must have a matching manual dispatch.
               'require_dispatch': bool(strict and require_strict is not None)}
    # The isolated loader installs only byte-verified authority helpers. No
    # candidate directory/PYTHONPATH is added to the import search path.
    program = """import importlib.util,json,sys
from pathlib import Path
request=json.loads(sys.argv[1]); directory=Path(request['root'])/'scripts/pm'
loaded={}
for name,filename in [('integration_ci','integration_ci.py'),('integration_executor_contract','integration_executor_contract.py'),('ci_ready_receipt_identity','ci_ready_receipt_identity.py'),('ci_live','ci-ready-receipt.py')]:
 spec=importlib.util.spec_from_file_location(name,directory/filename); item=importlib.util.module_from_spec(spec); sys.modules[name]=item; spec.loader.exec_module(item); loaded[name]=item
integration=loaded['integration_ci']; module=loaded['ci_live']
repository=integration.gh('api',f"repos/{request['repository']}")
if repository.get('full_name')!=request['repository'] or repository.get('default_branch')!=request['base_ref']:
 raise ValueError('PR target ref is not the live repository default branch')
assessed_target=integration.default_branch_head(request['repository'],request['base_ref'])
if request['require_strict']:
 pr,run,base,head=module.selected_live(request['repository'],request['uid'],request['issue'],request['pr'],'required-gate',request['app'],allow_ready_pr=True,base_ref=request['base_ref'],integration_run_id=request.get('integration_run_id'),require_integration=True,require_dispatch=request.get('require_dispatch',False),request_key=request.get('request_key'))
else:
 pr,run,base,head=module.selected_live(request['repository'],request['uid'],request['issue'],request['pr'],'required-gate',request['app'],allow_ready_pr=True,base_ref=request['base_ref'],request_key=request.get('request_key'))
if request.get('request_key'):
 body=(pr.get('body') or '').replace('\\r\\n','\\n')
 import re
 if (re.findall(r'^Task:[^\\n]*$',body,re.M)!=['Task: '+request['uid']]
     or re.findall(r'^Refs #[1-9][0-9]*$',body,re.M)!=['Refs #'+str(request['issue'])]):
  raise ValueError('keyed PR Task/Refs identity is not canonical')
planner=module.planner_for_run(request['repository'],run,base_oid=base,head_oid=head)
proof={'integration_base_oid':base,'base_ref':pr.get('base',{}).get('ref'),'head_oid':head,'check_name':run.get('name'),'check_run_id':run['id'],'check_app_id':run['app']['id'],'planner_digest':module.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'ci_validation_mode':'trusted_integration' if run.get('_integration') else 'ordinary_pr','assessed_target_oid':assessed_target}
if run.get('_integration'):
 proof.update({key:run['_integration'][key] for key in ('workflow_run_id','workflow_sha','tested_tree_oid','tested_commit_oid')})
 for key in ('request_key','request_identity','source_scope_oid','trusted_policy_context','effective_policy_identity','planner_inventory_authority','required_plan_v2_artifact_id','required_plan_v2_artifact_name','required_plan_v2_payload','required_result_v2_artifacts','trusted_planner_inventory','trusted_source_attempt','execution_jobs','run_id','run_attempt','request_id','job_id','job_name'):
  if key in (run.get('_integration') or {}): proof[key]=run['_integration'][key]
if integration.default_branch_head(request['repository'],request['base_ref'])!=assessed_target:
 raise ValueError('default-branch target moved during live CI and Task verification')
print(json.dumps(proof))
"""
    completed = subprocess.run([sys.executable,'-I','-c',program,json.dumps(request)],text=True,capture_output=True)
    if completed.returncode:
        raise ValueError((completed.stderr or completed.stdout).strip() or 'fresh integration CI unavailable')
    proof = json.loads(completed.stdout)
    if proof.get('request_key') is not None:
        validate_keyed_integration_identity(proof, data, uid, task)
        # A keyed source result is only a candidate.  Recompute the current-Q
        # inventory from trusted W and ask the shared C0 evaluator to bind it
        # to the exact source attempt before the lifecycle can consume it.
        with _effective_ci_modules(effective) as modules:
            target_inventory = modules['integration_ci'].trusted_local_target_inventory(
                data['repository'], uid, int(data['number']), proof,
            )
            proof['keyed_q_applicability'] = evaluate_keyed_q_applicability(
                proof, target_inventory,
                modules['ci_evidence_applicability'], data,
            )
    _validate_live_integration_proof(
        proof, data, strict=strict, allow_legacy_strict_fallback=require_strict is None,
    )
    return proof


def production_decision(data, admin_authorized, root, uid, tool_root, integration_run_id=None):
    # Never create a production receipt before fresh local authority admission.
    result = decision(data, admin_authorized, evidence_mode='pending_live_loop')
    if not result['ready_for_merge']: return result
    try:
        base, head = data.get('baseRefOid', ''), data.get('headRefOid', '')
        if not all(re.fullmatch(r'[0-9a-f]{40}', value) for value in (base, head)):
            raise ValueError('current PR base/head OIDs unavailable')
        admission = local_loop_admission(root, uid, base, head, tool_root)
        legacy_admission = isinstance(admission, dict) and admission.get('status') == 'legacy'
        integration = live_integration_admission(
            data, root, uid, tool_root, admission, integration_run_id,
            require_strict=True if integration_run_id is not None or legacy_admission else "auto",
        )
        fresh = read_pr_identity(data['repository'], data['number'])
        # Admission may involve slow remote reads. Even unchanged commit OIDs
        # cannot preserve authority after a draft, body or branch transition.
        fields = ('number', 'state', 'isDraft', 'body', 'baseRefName', 'headRefName', 'baseRefOid', 'headRefOid')
        if (not isinstance(fresh, dict) or any(key not in fresh or key not in data or fresh[key] != data[key] for key in fields)
                or fresh['state'] != 'OPEN' or fresh['isDraft'] is not False):
            raise ValueError('PR admission identity or state changed during live loop admission; rerun gate')
        if integration is not None and integration.get('request_key') is not None:
            # Bind the lifecycle readiness epoch to the exact source attempt
            # and the fresh-Q C0 decision.  This prevents a receipt from being
            # replayed after Q, M/T, policy, or the selected run changes.
            integration = dict(integration)
            integration['keyed_target_applicability_epoch'] = _validate_keyed_q_applicability(
                integration, data,
            )
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as exc:
        result.update(ready_for_merge=False, status='blocked', use_admin_merge=False)
        result['blockers'].append('live loop admission: ' + str(exc))
        return result
    result = decision({**data, 'integration_ci': integration}, admin_authorized, evidence_mode='production')
    if integration is not None:
        result['readiness_receipt']['integration_ci'] = integration
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("pr", nargs="?", default="")
    parser.add_argument("--fixture")
    parser.add_argument("--root", default=".")
    parser.add_argument("--task-uid")
    parser.add_argument("--integration-run-id", type=int, help="manual run locator; latest matching request still revalidated live")
    parser.add_argument("--tool-root", help="effective loop helper checkout (default: OASIS7_LOOP_TOOL_ROOT or this script checkout)")
    parser.add_argument("--merge-hold", choices=["normal_pr_ci_watch", *sorted(HOLDS)])
    parser.add_argument("--admin-merge-authorized", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    data = json.loads(Path(args.fixture).read_text(encoding="utf-8")) if args.fixture else load_live(args.pr)
    if args.fixture:
        evidence_mode = "fixture"
        if args.merge_hold:
            data["merge_hold"] = {"kind": args.merge_hold, "active": args.merge_hold in HOLDS, "requester":"fixture","reason":"fixture","resume_authority":"fixture"}
    if not args.fixture:
        if not args.task_uid:
            parser.error("live gate requires --task-uid so merge hold is read from task truth")
        mapping = json.loads((Path(args.root) / ".pm/github-project-sync/tasks.json").read_text(encoding="utf-8"))
        record = (mapping.get("tasks") or {}).get(args.task_uid) or {}
        rebuilt = rebuild_issue_evidence(str(data["repository"]), int(record["issue_number"]), args.task_uid, data)
        rebuilt_hold = rebuilt.get("merge_hold")
        recorded_hold = record.get("merge_hold")
        selected_pr_matches_live = str(args.pr or "") == str(data.get("number") or "")
        recorded_pr = str(record.get("pr_number") or "")
        default_hold_matches_live_pr = (
            isinstance(recorded_hold, dict)
            and recorded_hold.get("kind") == "normal_pr_ci_watch"
            and recorded_hold.get("active") is False
            and selected_pr_matches_live
            and bool(recorded_pr)
            and recorded_pr == str(data.get("number") or "")
        )
        # An explicit head-bound issue comment always wins.  The only local
        # fallback is record-pr's canonical inactive default for this exact PR;
        # caller-authored active holds never gain authority from cache shape.
        data["merge_hold"] = rebuilt_hold if rebuilt_hold is not None else (
            recorded_hold if default_hold_matches_live_pr else None
        )
        data["comment_dispositions"] = rebuilt.get("comment_dispositions") or []
        data["review_dispositions"] = rebuilt.get("review_dispositions") or []
        data["admin_merge_authority"] = rebuilt.get("admin_merge_authority")
        evidence_mode = "production"
        if args.merge_hold:
            parser.error("--merge-hold is fixture-only; live hold truth is rebuilt from the GitHub task issue")
    result = (decision(data, args.admin_merge_authorized, evidence_mode=evidence_mode) if args.fixture else
              production_decision(data, args.admin_merge_authorized, Path(args.root), args.task_uid, args.tool_root, args.integration_run_id))
    print(json.dumps(result, indent=2, sort_keys=True) if args.json else ("ready_for_merge" if result["ready_for_merge"] else "\n".join(result["blockers"])))
    return 0 if result["ready_for_merge"] else 3


if __name__ == "__main__":
    raise SystemExit(main())
