#!/usr/bin/env python3
"""Fail-closed PR watch/merge decision over all review and check surfaces."""
from __future__ import annotations

import argparse
import base64
import os
import datetime as dt
import hashlib
import json
import subprocess
import sys
import re
import fnmatch
from pathlib import Path
from typing import Any

SUCCESS = {"SUCCESS", "NEUTRAL", "SKIPPED"}
HOLDS = {"manual_packaging_ci_hold", "user_requested_merge_hold"}
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


def live_integration_admission(data, root, uid, tool_root, admission, integration_run_id=None):
    """Read selected CI and its frozen planner artifact; never trust a local receipt."""
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
    for name in ('ci-ready-receipt.py', 'ci_ready_receipt_identity.py', 'integration_ci.py'):
        relative = 'scripts/pm/' + name
        expected = subprocess.check_output(['git','-C',str(root),'show',commit + ':' + relative])
        path = effective / relative
        if path.is_symlink() or path.read_bytes() != expected:
            raise ValueError('effective CI authority helper bytes differ: ' + name)
    task = admission['task']
    if task.get('repository') != data['repository']:
        raise ValueError('CI task repository identity mismatch')
    request = {'root': str(effective), 'repository': data['repository'], 'uid': uid,
               'issue': task['issue_number'], 'pr': data['number'], 'app': next(iter(pins)),
               'base_ref': data['baseRefName'], 'integration_run_id': integration_run_id}
    # Isolated stdlib loader installs only the two byte-verified modules. No
    # candidate directory/PYTHONPATH is added to the import search path.
    program = """import importlib.util,json,sys
from pathlib import Path
request=json.loads(sys.argv[1]); directory=Path(request['root'])/'scripts/pm'
for name,filename in [('integration_ci','integration_ci.py'),('ci_ready_receipt_identity','ci_ready_receipt_identity.py'),('ci_live','ci-ready-receipt.py')]:
 spec=importlib.util.spec_from_file_location(name,directory/filename); module=importlib.util.module_from_spec(spec); sys.modules[name]=module; spec.loader.exec_module(module)
pr,run,base,head=module.selected_live(request['repository'],request['uid'],request['issue'],request['pr'],'required-gate',request['app'],allow_ready_pr=True,base_ref=request['base_ref'],integration_run_id=request.get('integration_run_id'))
planner=module.planner_for_run(request['repository'],run,base_oid=base,head_oid=head)
proof={'integration_base_oid':base,'head_oid':head,'check_run_id':run['id'],'check_app_id':run['app']['id'],'planner_digest':module.hashlib.sha256(json.dumps(planner,sort_keys=True,separators=(',',':')).encode()).hexdigest()}
if run.get('_integration'):
 proof.update({key:run['_integration'][key] for key in ('workflow_run_id','workflow_sha','tested_tree_oid','tested_commit_oid')})
print(json.dumps(proof))
"""
    completed = subprocess.run([sys.executable,'-I','-c',program,json.dumps(request)],text=True,capture_output=True)
    if completed.returncode:
        raise ValueError((completed.stderr or completed.stdout).strip() or 'fresh integration CI unavailable')
    proof = json.loads(completed.stdout)
    if proof.get('integration_base_oid') != data['baseRefOid'] or proof.get('head_oid') != data['headRefOid']:
        raise ValueError('stale integration CI base/head; rerun required CI against current target without rebasing source')
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
        integration = live_integration_admission(data, root, uid, tool_root, admission, integration_run_id) if integration_run_id is not None else live_integration_admission(data, root, uid, tool_root, admission)
        fresh = read_pr_identity(data['repository'], data['number'])
        # Admission may involve slow remote reads. Even unchanged commit OIDs
        # cannot preserve authority after a draft, body or branch transition.
        fields = ('number', 'state', 'isDraft', 'body', 'baseRefName', 'headRefName', 'baseRefOid', 'headRefOid')
        if (not isinstance(fresh, dict) or any(key not in fresh or key not in data or fresh[key] != data[key] for key in fields)
                or fresh['state'] != 'OPEN' or fresh['isDraft'] is not False):
            raise ValueError('PR admission identity or state changed during live loop admission; rerun gate')
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
