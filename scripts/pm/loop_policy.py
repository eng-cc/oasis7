"""Deterministic, immutable loop policy. No task mutations or scheduling.

Admission callers must validate_tool_root before executing trusted helpers.
validate_scope can also run on candidate code in offline tests; that is not
effective-policy activation or native filesystem isolation evidence.
"""
from __future__ import annotations

import fnmatch
import base64
import hashlib
import json
from datetime import datetime
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
from urllib.parse import quote

from loop_contracts import coordination_ref_errors, consumed_clause_ref_errors

SCHEMA = "oasis7.loop-task/v1"
POLICY_PATH = "scripts/pm/loop-policy.v1.json"
LOOPS = {"product", "system", "code"}
OID = re.compile(r"[0-9a-f]{40}\Z")
UID = re.compile(r"task_[0-9a-f]{32}\Z")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")
ADOPTION_SCHEMA = "oasis7.workflow-policy-adoption/v1"
ADOPTION_AUTH_SCHEMA = "oasis7.workflow-policy-adoption-authorization/v1"
ADOPTION_MARKER = "<!-- oasis7.workflow-policy-adoption/v1 -->"
ADOPTION_AUTH_MARKER = "<!-- oasis7.workflow-policy-adoption-authorization/v1 -->"
ADOPTION_FIELDS = frozenset({
    "schema", "task_uid", "bootstrap_epoch", "binding_identity_digest",
    "write_scope_digest", "predecessor_digest", "prior_policy_commit",
    "prior_policy_digest", "adopted_policy_commit", "adopted_policy_digest",
    "authorization", "task_identity", "target_proof", "record_digest",
})
ADOPTION_AUTH_FIELDS = frozenset({
    "schema", "task_uid", "bootstrap_epoch", "binding_identity_digest",
    "write_scope_digest", "target_policy_commit", "target_policy_digest",
})
ADOPTION_IDENTITY_FIELDS = frozenset({
    "repository", "issue_number", "task_uid", "bootstrap_epoch",
    "binding_identity_digest", "write_scope_digest", "owner_role",
    "canonical_worktree", "task_branch", "default_branch", "project_id",
    "project_number", "project_item_id", "pr_number", "pr_url", "status",
    "workflow_phase", "hold_active",
})
HOSTED_ADOPTION_IDENTITY_FIELDS = frozenset({
    "repository", "issue_number", "task_uid", "bootstrap_epoch",
    "binding_identity_digest", "write_scope_digest", "owner_role",
    "task_branch", "default_branch", "pr_number", "pr_url", "status",
    "workflow_phase", "hold_active",
})
_ADOPTION_STABLE_IDENTITY_FIELDS = frozenset(ADOPTION_IDENTITY_FIELDS - {
    "status", "workflow_phase", "hold_active",
})
_ADOPTION_TARGET_FIELDS = frozenset({
    "default_branch", "default_branch_oid", "policy_commit", "policy_digest",
    "workflow_source_digest",
})
_ADOPTION_ASSOCIATIONS = {"OWNER", "MEMBER", "COLLABORATOR"}
CORPUS_MODULE = "scripts/document_corpus.py"
TRUSTED_IMPORT_FILES = (
    CORPUS_MODULE,
    "scripts/product-doc-content-check.py",
    "scripts/product_doc_markdown.py",
)


def result(blockers, **fields):
    return {"status": "blocked" if blockers else "passed", "blockers": blockers, **fields}


def git(root, *args):
    process = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if process.returncode:
        raise ValueError("git read failed: " + process.stderr.decode("utf-8", "replace").strip())
    return process.stdout


def safe_path(value):
    return (isinstance(value, str) and bool(value) and "\\" not in value
            and not any(ord(c) < 32 for c in value) and not value.startswith("/")
            and not any(part in {"", ".", ".."} for part in value.split("/"))
            and not re.match(r"^[A-Za-z]:", value))


def validate_binding(binding):
    errors = []
    if not isinstance(binding, dict):
        return result(["loop_binding must be an object"])
    allowed = {
        "schema", "task_uid", "change_id", "loop", "owner_role",
        "bootstrap_epoch", "manual_request_ref", "request_key", "write_scope",
        "out_of_scope", "input_contracts", "acceptance_refs", "dependencies",
        "target_delivery", "policy_digest", "policy_commit", "coordination_ref",
        "consumed_clause_refs", "delivery_obligations",
    }
    if set(binding) - allowed:
        errors.append("loop binding contains unsupported fields")
    if binding.get("schema") != SCHEMA:
        errors.append("unsupported loop binding schema")
    for key in ("change_id", "owner_role", "manual_request_ref", "request_key", "target_delivery"):
        if not isinstance(binding.get(key), str) or not binding[key].strip():
            errors.append(f"{key} must be non-empty text")
    for key, regex in (("task_uid", UID), ("policy_commit", OID), ("policy_digest", DIGEST)):
        if not isinstance(binding.get(key), str) or not regex.fullmatch(binding[key]):
            errors.append(f"invalid {key}")
    if binding.get("loop") not in LOOPS:
        errors.append("invalid loop")
    if type(binding.get("bootstrap_epoch")) is not int or binding["bootstrap_epoch"] < 1:
        errors.append("bootstrap_epoch must be a positive integer")
    for key in ("write_scope", "out_of_scope", "acceptance_refs", "dependencies", "input_contracts"):
        if not isinstance(binding.get(key), list):
            errors.append(f"{key} must be a list")
    for key in ("write_scope", "acceptance_refs"):
        if not binding.get(key):
            errors.append(f"{key} must not be empty")
    for key in ("write_scope", "out_of_scope"):
        if isinstance(binding.get(key), list) and any(not safe_path(p) for p in binding[key]):
            errors.append(f"{key} contains unsafe path pattern")
    if isinstance(binding.get("acceptance_refs"), list):
        for acceptance in binding["acceptance_refs"]:
            if isinstance(acceptance, str) and acceptance.strip():
                continue
            if isinstance(acceptance, dict):
                errors.extend(consumed_clause_ref_errors(acceptance, require_identity=False))
                continue
            errors.append("invalid acceptance reference")
    if "coordination_ref" in binding and binding.get("coordination_ref") is not None:
        errors.extend(coordination_ref_errors(binding.get("coordination_ref")))
    if "consumed_clause_refs" in binding:
        refs = binding.get("consumed_clause_refs")
        if not isinstance(refs, list):
            errors.append("consumed_clause_refs must be a list")
        else:
            for reference in refs:
                errors.extend(consumed_clause_ref_errors(reference, require_identity=False))
    deps = binding.get("dependencies")
    if isinstance(deps, list):
        if any(not isinstance(d, str) or not UID.fullmatch(d) for d in deps):
            errors.append("dependencies must be task UIDs")
        elif len(set(deps)) != len(deps) or binding.get("task_uid") in deps:
            errors.append("duplicate or cyclic task dependency")
    if isinstance(binding.get("input_contracts"), list):
        for item in binding["input_contracts"]:
            if not isinstance(item, dict) or not item.get("contract_id") or type(item.get("revision")) is not int or item["revision"] < 1 or not isinstance(item.get("publication_ref"), dict) or not isinstance(item.get("contract_digest"),str) or not DIGEST.fullmatch(item["contract_digest"]):
                errors.append("invalid immutable input contract reference")
                continue
            ref=item["publication_ref"]
            clauses=item.get("consumed_clauses")
            clause_refs = item.get("consumed_clause_refs")
            has_clauses = isinstance(clauses, list) and bool(clauses)
            has_clause_refs = isinstance(clause_refs, list) and bool(clause_refs)
            if not has_clauses and not has_clause_refs:
                errors.append("immutable input contract needs consumed clauses or refs")
            if not isinstance(item["contract_id"],str) or any(type(ref.get(k)) is not int or ref[k]<1 for k in ("issue_number","comment_id")) or (clauses is not None and (not isinstance(clauses,list) or any(not isinstance(c,str) or not c.strip() for c in clauses))) or (clause_refs is not None and (not isinstance(clause_refs,list) or any(consumed_clause_ref_errors(c, require_identity=True) for c in clause_refs))):
                errors.append("invalid contract publication identity/clauses")
    obligations = binding.get("delivery_obligations", [])
    if not isinstance(obligations, list) or any(not isinstance(o, dict) or not o.get("id") or not isinstance(o.get("task_uid"), str) or not UID.fullmatch(o["task_uid"]) or type(o.get("issue_number")) is not int or o["issue_number"]<1 for o in obligations):
        errors.append("invalid delivery obligations")
    return result(errors)


def _canonical_digest(value):
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True,
                         separators=(",", ":")).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def binding_identity_digest(binding):
    """Digest the immutable original binding, including its original pin."""
    if validate_binding(binding)["status"] != "passed":
        raise ValueError("cannot identify an invalid immutable loop binding")
    return _canonical_digest(binding)


def write_scope_digest(binding):
    if validate_binding(binding)["status"] != "passed":
        raise ValueError("cannot identify an invalid loop write scope")
    return _canonical_digest({
        "write_scope": binding["write_scope"],
        "out_of_scope": binding["out_of_scope"],
        "target_delivery": binding["target_delivery"],
    })


def _unique_json(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate JSON field")
        value[key] = item
    return value


def _parse_marked_record(body, marker, schema, fields):
    if not isinstance(body, str) or body.count(marker) != 1 or not body.startswith(marker + "\n"):
        raise ValueError("machine evidence marker is missing or duplicated")
    payload = body[len(marker) + 1:]
    decoder = json.JSONDecoder(object_pairs_hook=_unique_json)
    value, end = decoder.raw_decode(payload)
    if payload[end:].strip() or not isinstance(value, dict):
        raise ValueError("machine evidence has trailing or non-object content")
    if set(value) != fields or value.get("schema") != schema:
        raise ValueError("machine evidence fields or schema are unsupported")
    return value


def policy_adoption_authorization_comment(payload):
    if not isinstance(payload, dict) or set(payload) != ADOPTION_AUTH_FIELDS - {"schema"}:
        raise ValueError("policy adoption authorization fields are not closed")
    value = {"schema": ADOPTION_AUTH_SCHEMA, **payload}
    _validate_adoption_auth(value)
    return ADOPTION_AUTH_MARKER + "\n" + json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    )


def parse_policy_adoption_authorization(body):
    value = _parse_marked_record(body, ADOPTION_AUTH_MARKER,
                                 ADOPTION_AUTH_SCHEMA, ADOPTION_AUTH_FIELDS)
    _validate_adoption_auth(value)
    return value


def _validate_adoption_auth(value):
    if (not UID.fullmatch(str(value.get("task_uid") or ""))
            or type(value.get("bootstrap_epoch")) is not int or value["bootstrap_epoch"] < 1
            or not DIGEST.fullmatch(str(value.get("binding_identity_digest") or ""))
            or not DIGEST.fullmatch(str(value.get("write_scope_digest") or ""))
            or not OID.fullmatch(str(value.get("target_policy_commit") or ""))
            or not DIGEST.fullmatch(str(value.get("target_policy_digest") or ""))):
        raise ValueError("policy adoption authorization identity is invalid")


def _validate_adoption_identity(identity, *, allow_hosted=False):
    valid_keys = {ADOPTION_IDENTITY_FIELDS}
    if allow_hosted:
        valid_keys.add(HOSTED_ADOPTION_IDENTITY_FIELDS)
    if not isinstance(identity, dict) or frozenset(identity) not in valid_keys:
        raise ValueError("policy adoption Task identity fields are not closed")
    is_hosted = frozenset(identity) == HOSTED_ADOPTION_IDENTITY_FIELDS
    if (not isinstance(identity.get("repository"), str)
            or not re.fullmatch(r"[^/\s]+/[^/\s]+", identity["repository"])
            or type(identity.get("issue_number")) is not int or identity["issue_number"] < 1
            or not UID.fullmatch(str(identity.get("task_uid") or ""))
            or type(identity.get("bootstrap_epoch")) is not int or identity["bootstrap_epoch"] < 1
            or not DIGEST.fullmatch(str(identity.get("binding_identity_digest") or ""))
            or not DIGEST.fullmatch(str(identity.get("write_scope_digest") or ""))
            or not isinstance(identity.get("owner_role"), str) or not identity["owner_role"]
            or not isinstance(identity.get("task_branch"), str)
            or (not is_hosted and not identity["task_branch"])
            or not isinstance(identity.get("default_branch"), str) or not identity["default_branch"]
            or (identity.get("pr_number") is not None and
                (type(identity["pr_number"]) is not int or identity["pr_number"] < 1))
            or (identity.get("pr_url") is not None and not isinstance(identity["pr_url"], str))
            or not isinstance(identity.get("status"), str)
            or not isinstance(identity.get("workflow_phase"), str)
            or type(identity.get("hold_active")) is not bool):
        raise ValueError("policy adoption Task identity is malformed")
    if not is_hosted and (
            not isinstance(identity.get("canonical_worktree"), str) or not identity["canonical_worktree"]
            or not isinstance(identity.get("project_id"), str) or not identity["project_id"]
            or type(identity.get("project_number")) is not int or identity["project_number"] < 1
            or not isinstance(identity.get("project_item_id"), str) or not identity["project_item_id"]):
        raise ValueError("strict local policy adoption Task identity is incomplete")
    if (identity["pr_number"] is None) != (identity["pr_url"] is None):
        raise ValueError("policy adoption PR identity is incomplete")
    if identity["pr_url"] is not None and identity["pr_url"] != (
            f"https://github.com/{identity['repository']}/pull/{identity['pr_number']}"):
        raise ValueError("policy adoption PR URL identity is inconsistent")


def _validate_target_proof(proof):
    if not isinstance(proof, dict) or set(proof) != _ADOPTION_TARGET_FIELDS:
        raise ValueError("trusted current-policy proof fields are not closed")
    if (not isinstance(proof.get("default_branch"), str) or not proof["default_branch"]
            or not OID.fullmatch(str(proof.get("default_branch_oid") or ""))
            or proof["policy_commit"] != proof["default_branch_oid"]
            or not DIGEST.fullmatch(str(proof.get("policy_digest") or ""))
            or not DIGEST.fullmatch(str(proof.get("workflow_source_digest") or ""))):
        raise ValueError("trusted current-policy proof is invalid")


def build_policy_adoption_record(binding, task_identity, *, target_commit,
                                 target_digest, target_proof,
                                 authorization_comment, predecessor_digest,
                                 prior_policy=None):
    """Build one closed append-only record; caller must revalidate live inputs before POST."""
    if validate_binding(binding)["status"] != "passed":
        raise ValueError("policy adoption requires a valid immutable binding")
    _validate_adoption_identity(task_identity)
    _validate_target_proof(target_proof)
    if (task_identity["task_uid"] != binding["task_uid"]
            or task_identity["bootstrap_epoch"] != binding["bootstrap_epoch"]
            or task_identity["binding_identity_digest"] != binding_identity_digest(binding)
            or task_identity["write_scope_digest"] != write_scope_digest(binding)
            or task_identity["owner_role"] != binding["owner_role"]
            or task_identity["hold_active"]):
        raise ValueError("policy adoption Task identity or hold does not match immutable authority")
    if (target_commit != target_proof["default_branch_oid"]
            or target_proof["policy_commit"] != target_commit
            or target_digest != target_proof["policy_digest"]
            or target_proof["default_branch"] != task_identity["default_branch"]):
        raise ValueError("target policy must equal the exact trusted effective default-branch tip")
    try:
        authorization = parse_policy_adoption_authorization(authorization_comment["body"])
    except (TypeError, KeyError) as exc:
        raise ValueError("explicit pre-existing Task authorization is unavailable") from exc
    user = authorization_comment.get("user")
    association = authorization_comment.get("author_association")
    if (type(authorization_comment.get("id")) is not int or authorization_comment["id"] < 1
            or not isinstance(user, dict) or user.get("type") != "User"
            or not isinstance(user.get("login"), str) or not user["login"]
            or association not in _ADOPTION_ASSOCIATIONS):
        raise ValueError("authorization comment is not trusted user Task evidence")
    if (authorization["task_uid"] != task_identity["task_uid"]
            or authorization["bootstrap_epoch"] != task_identity["bootstrap_epoch"]
            or authorization["binding_identity_digest"] != task_identity["binding_identity_digest"]
            or authorization["write_scope_digest"] != task_identity["write_scope_digest"]
            or authorization["target_policy_commit"] != target_commit
            or authorization["target_policy_digest"] != target_digest):
        raise ValueError("pre-existing user authorization does not bind this exact adoption")
    prior = prior_policy or {
        "policy_commit": binding["policy_commit"],
        "policy_digest": binding["policy_digest"],
    }
    if (not isinstance(prior, dict)
            or set(prior) != {"policy_commit", "policy_digest"}
            or not OID.fullmatch(str(prior.get("policy_commit") or ""))
            or not DIGEST.fullmatch(str(prior.get("policy_digest") or ""))):
        raise ValueError("active prior policy pin is malformed")
    prior_commit = str(prior["policy_commit"])
    prior_digest = str(prior["policy_digest"])
    if predecessor_digest is not None and not DIGEST.fullmatch(str(predecessor_digest)):
        raise ValueError("policy adoption predecessor digest is malformed")
    value = {
        "schema": ADOPTION_SCHEMA, "task_uid": task_identity["task_uid"],
        "bootstrap_epoch": task_identity["bootstrap_epoch"],
        "binding_identity_digest": task_identity["binding_identity_digest"],
        "write_scope_digest": task_identity["write_scope_digest"],
        "predecessor_digest": predecessor_digest,
        "prior_policy_commit": prior_commit, "prior_policy_digest": prior_digest,
        "adopted_policy_commit": target_commit, "adopted_policy_digest": target_digest,
        "authorization": {
            "repository": task_identity["repository"],
            "issue_number": task_identity["issue_number"],
            "comment_id": authorization_comment["id"],
            "body_digest": "sha256:" + hashlib.sha256(
                authorization_comment["body"].encode("utf-8"),
            ).hexdigest(),
            "author_login": user["login"],
        },
        "task_identity": dict(task_identity),
        "target_proof": dict(target_proof),
    }
    value["record_digest"] = _canonical_digest(value)
    return validate_policy_adoption_record(value)


def validate_policy_adoption_record(value):
    if not isinstance(value, dict) or set(value) != ADOPTION_FIELDS or value.get("schema") != ADOPTION_SCHEMA:
        raise ValueError("policy adoption record fields or schema are unsupported")
    identity = value.get("task_identity")
    _validate_adoption_identity(identity)
    _validate_target_proof(value.get("target_proof"))
    authorization = value.get("authorization")
    if (not isinstance(authorization, dict)
            or set(authorization) != {"repository", "issue_number", "comment_id", "body_digest", "author_login"}
            or authorization.get("repository") != identity["repository"]
            or authorization.get("issue_number") != identity["issue_number"]
            or type(authorization.get("comment_id")) is not int or authorization["comment_id"] < 1
            or not DIGEST.fullmatch(str(authorization.get("body_digest") or ""))
            or not isinstance(authorization.get("author_login"), str) or not authorization["author_login"]):
        raise ValueError("policy adoption authorization reference is malformed")
    if (value.get("task_uid") != identity["task_uid"]
            or value.get("bootstrap_epoch") != identity["bootstrap_epoch"]
            or value.get("binding_identity_digest") != identity["binding_identity_digest"]
            or value.get("write_scope_digest") != identity["write_scope_digest"]
            or not OID.fullmatch(str(value.get("prior_policy_commit") or ""))
            or not DIGEST.fullmatch(str(value.get("prior_policy_digest") or ""))
            or not OID.fullmatch(str(value.get("adopted_policy_commit") or ""))
            or not DIGEST.fullmatch(str(value.get("adopted_policy_digest") or ""))
            or (value.get("predecessor_digest") is not None and
                not DIGEST.fullmatch(str(value["predecessor_digest"])))):
        raise ValueError("policy adoption record identity is invalid")
    if (value["adopted_policy_commit"] != value["target_proof"]["default_branch_oid"]
            or value["adopted_policy_digest"] != value["target_proof"]["policy_digest"]
            or value["task_identity"]["default_branch"] != value["target_proof"]["default_branch"]):
        raise ValueError("policy adoption record does not bind its trusted target proof")
    digest_value = dict(value)
    supplied = digest_value.pop("record_digest")
    if supplied != _canonical_digest(digest_value):
        raise ValueError("policy adoption record digest mismatch")
    return value


def policy_adoption_comment(record):
    value = validate_policy_adoption_record(record)
    return ADOPTION_MARKER + "\n" + json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
    )


def parse_policy_adoption_comment(body):
    return validate_policy_adoption_record(
        _parse_marked_record(body, ADOPTION_MARKER, ADOPTION_SCHEMA, ADOPTION_FIELDS),
    )


def comment_timestamps_are_unchanged(comment):
    """Require an unedited GitHub comment with valid server timestamps."""
    if not isinstance(comment, dict):
        return False
    created_raw = comment.get("created_at")
    updated_raw = comment.get("updated_at")
    if (not isinstance(created_raw, str) or not created_raw
            or not isinstance(updated_raw, str) or not updated_raw
            or created_raw != updated_raw):
        return False
    try:
        created = datetime.fromisoformat(created_raw.replace("Z", "+00:00"))
        updated = datetime.fromisoformat(updated_raw.replace("Z", "+00:00"))
    except ValueError:
        return False
    return created.tzinfo is not None and updated.tzinfo is not None and created == updated


def _trusted_policy_identity(value):
    _validate_target_proof(value)
    return value


def _canonical_github_repository_from_origin(repo_root):
    try:
        remote = git(repo_root, "config", "--get", "remote.origin.url").decode("utf-8").strip()
    except (OSError, ValueError, UnicodeDecodeError) as exc:
        raise ValueError("canonical repository origin cannot be verified") from exc
    normalized = re.sub(r"\.git\Z", "", remote)
    if normalized.startswith("git@github.com:"):
        normalized = normalized.removeprefix("git@github.com:")
    elif normalized.startswith("https://github.com/"):
        normalized = normalized.removeprefix("https://github.com/")
    elif normalized.startswith("ssh://git@github.com/"):
        normalized = normalized.removeprefix("ssh://git@github.com/")
    else:
        raise ValueError("canonical repository origin is not a GitHub remote")
    if not re.fullmatch(r"[^/\s]+/[^/\s]+", normalized):
        raise ValueError("canonical GitHub repository origin is malformed")
    return normalized


def current_effective_policy_identity(repo_root, repository):
    """Read exact protected-default-branch policy/source blobs from live GitHub.

    Local refs, caller-supplied OIDs, and candidate worktree contents are not
    authority.  The returned pair is tied to GitHub's current repository
    default branch tip; callers that need ancestry additionally verify the
    commit graph in the canonical repository.
    """
    if not isinstance(repository, str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", repository):
        raise ValueError("repository identity is invalid")

    def gh_json(endpoint):
        process = subprocess.run(["gh", "api", endpoint], stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, check=False)
        if process.returncode:
            raise ValueError("live GitHub authority read failed: " +
                             process.stderr.decode("utf-8", "replace").strip())
        try:
            value = json.loads(process.stdout)
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ValueError("live GitHub authority response is malformed") from exc
        if not isinstance(value, dict):
            raise ValueError("live GitHub authority response is not an object")
        return value

    normalized = _canonical_github_repository_from_origin(repo_root)
    if normalized.casefold() != repository.casefold():
        raise ValueError("canonical repository origin differs from requested repository")

    repository_info = gh_json(f"repos/{repository}")
    branch = repository_info.get("default_branch")
    if not isinstance(branch, str) or not branch or branch.startswith("-"):
        raise ValueError("live repository default branch is unavailable")
    branch_info = gh_json(f"repos/{repository}/branches/{quote(branch, safe='')}")
    if (branch_info.get("name") != branch or branch_info.get("protected") is not True
            or not isinstance(branch_info.get("commit"), dict)):
        raise ValueError("canonical default branch is not live and protected")
    commit = branch_info["commit"].get("sha")
    if not isinstance(commit, str) or not OID.fullmatch(commit):
        raise ValueError("live default-branch tip OID is malformed")

    def read_blob(path):
        endpoint = f"repos/{repository}/contents/{path}?ref={commit}"
        blob = gh_json(endpoint)
        if blob.get("encoding") != "base64" or not isinstance(blob.get("content"), str):
            raise ValueError("trusted default-branch source blob is unavailable: " + path)
        try:
            raw = base64.b64decode(blob["content"], validate=True)
        except (ValueError, TypeError) as exc:
            raise ValueError("trusted default-branch source blob is malformed: " + path) from exc
        return raw

    policy_raw = read_blob(POLICY_PATH)
    source_raw = read_blob("doc/engineering/workflow/source-of-truth.md")
    try:
        policy = json.loads(policy_raw, object_pairs_hook=_unique_json)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise ValueError("trusted effective policy JSON is malformed") from exc
    if (not isinstance(policy, dict) or policy.get("schema") != "oasis7.loop-policy/v1"
            or not isinstance(policy.get("rules"), list) or not policy["rules"]
            or not isinstance(policy.get("denied"), list)
            or not isinstance(policy.get("document_extensions"), list)):
        raise ValueError("trusted effective policy schema is unsupported")
    proof = {
        "default_branch": branch, "default_branch_oid": commit,
        "policy_commit": commit,
        "policy_digest": "sha256:" + hashlib.sha256(policy_raw).hexdigest(),
        "workflow_source_digest": "sha256:" + hashlib.sha256(source_raw).hexdigest(),
    }
    _validate_target_proof(proof)
    return proof


def validate_new_adoption_target(target_commit, target_digest, trusted_current_policy):
    """Only a fresh exact current trusted tip can start a new adoption step."""
    try:
        current = _trusted_policy_identity(trusted_current_policy)
    except (TypeError, ValueError) as exc:
        return result([f"trusted current policy read is invalid: {exc}"])
    if (target_commit != current["default_branch_oid"]
            or target_digest != current["policy_digest"]):
        return result(["adoption target is not the exact current effective default-branch policy"])
    return result([], **current)


def resolve_effective_policy(repo_root, binding, live_comment_read,
                             live_task_identity, trusted_current_policy):
    """Resolve the active pin from immutable binding plus the complete live Issue chain.

    Main advancement does not invalidate a previously proven pin: each stored
    target must remain an ancestor of the freshly read trusted branch tip, while
    only a *new* adoption target must equal that current tip.
    """
    errors = list(validate_binding(binding).get("blockers", []))
    if errors:
        return result(errors, policy_commit=None, policy_digest=None, adoption_chain_tip=None)
    try:
        current = _trusted_policy_identity(trusted_current_policy)
        _validate_adoption_identity(live_task_identity, allow_hosted=True)
    except (TypeError, ValueError) as exc:
        return result([f"effective policy authority is unresolved: {exc}"],
                      policy_commit=None, policy_digest=None, adoption_chain_tip=None)
    if (live_task_identity["task_uid"] != binding["task_uid"]
            or live_task_identity["bootstrap_epoch"] != binding["bootstrap_epoch"]
            or live_task_identity["binding_identity_digest"] != binding_identity_digest(binding)
            or live_task_identity["write_scope_digest"] != write_scope_digest(binding)
            or live_task_identity["owner_role"] != binding["owner_role"]):
        errors.append("live Task identity differs from immutable loop binding")
    if current["default_branch"] != live_task_identity["default_branch"]:
        errors.append("trusted policy branch differs from live Task repository identity")
    if not isinstance(live_comment_read, dict) or live_comment_read.get("complete") is not True:
        return result(errors + ["Task Issue comment read is incomplete"], status="pending",
                      policy_commit=None, policy_digest=None, adoption_chain_tip=None)
    comments = live_comment_read.get("comments")
    if (live_comment_read.get("repository") != live_task_identity["repository"]
            or live_comment_read.get("issue_number") != live_task_identity["issue_number"]
            or not isinstance(comments, list)):
        return result(errors + ["Task Issue comment read identity is incomplete"], status="pending",
                      policy_commit=None, policy_digest=None, adoption_chain_tip=None)
    auth_by_id = {}
    adoptions = []
    malformed_auth = False
    malformed_adoption = False
    comment_ids = set()
    for comment in comments:
        if not isinstance(comment, dict) or type(comment.get("id")) is not int or comment["id"] < 1:
            errors.append("Task Issue comment enumeration contains malformed identity")
            continue
        if comment["id"] in comment_ids:
            errors.append("Task Issue comment pagination contains duplicate IDs")
        comment_ids.add(comment["id"])
        body = comment.get("body")
        if not isinstance(body, str):
            errors.append("Task Issue comment body is malformed")
            continue
        if ADOPTION_AUTH_MARKER in body:
            if not comment_timestamps_are_unchanged(comment):
                malformed_auth = True
                continue
            try:
                auth = parse_policy_adoption_authorization(body)
            except (TypeError, ValueError, json.JSONDecodeError):
                malformed_auth = True
                continue
            auth_by_id.setdefault(comment["id"], []).append((comment, auth))
        if ADOPTION_MARKER in body:
            if not comment_timestamps_are_unchanged(comment):
                malformed_adoption = True
                continue
            try:
                record_value = parse_policy_adoption_comment(body)
            except (TypeError, ValueError, json.JSONDecodeError):
                malformed_adoption = True
                continue
            adoptions.append((comment, record_value))
    if malformed_auth or malformed_adoption:
        errors.append("Task Issue contains malformed workflow policy adoption evidence")
    if errors:
        return result(errors, policy_commit=None, policy_digest=None, adoption_chain_tip=None)
    # Old bindings remain authoritative when the complete live Issue has no
    # adoption records.  No derived value is written back into the binding.
    if not adoptions:
        return result([], policy_commit=binding["policy_commit"],
                      policy_digest=binding["policy_digest"], adoption_chain_tip=None,
                      pin_source="immutable_binding", binding=dict(binding))
    adoptions.sort(key=lambda item: item[0]["id"])
    expected_prior_commit = binding["policy_commit"]
    expected_prior_digest = binding["policy_digest"]
    expected_predecessor = None
    seen_record_digests = set()
    previous_comment_id = 0
    for comment, adopted in adoptions:
        user = comment.get("user") if isinstance(comment.get("user"), dict) else {}
        if (comment["id"] <= previous_comment_id
                or user.get("type") != "User" or not isinstance(user.get("login"), str)
                or comment.get("author_association") not in _ADOPTION_ASSOCIATIONS):
            errors.append("policy adoption record lacks ordered trusted publisher provenance")
        previous_comment_id = comment["id"]
        if adopted["record_digest"] in seen_record_digests:
            errors.append("policy adoption chain contains a duplicate record")
        seen_record_digests.add(adopted["record_digest"])
        if (adopted["task_uid"] != binding["task_uid"]
                or adopted["bootstrap_epoch"] != binding["bootstrap_epoch"]
                or adopted["binding_identity_digest"] != binding_identity_digest(binding)
                or adopted["write_scope_digest"] != write_scope_digest(binding)
                or adopted["prior_policy_commit"] != expected_prior_commit
                or adopted["prior_policy_digest"] != expected_prior_digest
                or adopted["predecessor_digest"] != expected_predecessor):
            errors.append("policy adoption chain forks or changes immutable Task/pin lineage")
        recorded_identity = adopted["task_identity"]
        if any(recorded_identity[key] != live_task_identity[key]
               for key in _ADOPTION_STABLE_IDENTITY_FIELDS.intersection(live_task_identity)):
            errors.append("policy adoption chain Task/worktree/branch/PR/scope identity drifted")
        auth_ref = adopted["authorization"]
        referenced = auth_by_id.get(auth_ref["comment_id"], [])
        if len(referenced) != 1:
            errors.append("policy adoption authorization reference is missing or ambiguous")
        else:
            auth_comment, auth_value = referenced[0]
            auth_user = auth_comment.get("user") if isinstance(auth_comment.get("user"), dict) else {}
            if (auth_comment["id"] >= comment["id"]
                    or auth_comment.get("author_association") not in _ADOPTION_ASSOCIATIONS
                    or auth_user.get("type") != "User"
                    or auth_user.get("login") != auth_ref["author_login"]
                    or not isinstance(auth_comment.get("body"), str)
                    or "sha256:" + hashlib.sha256(auth_comment["body"].encode("utf-8")).hexdigest()
                       != auth_ref["body_digest"]
                    or auth_value["task_uid"] != adopted["task_uid"]
                    or auth_value["bootstrap_epoch"] != adopted["bootstrap_epoch"]
                    or auth_value["binding_identity_digest"] != adopted["binding_identity_digest"]
                    or auth_value["write_scope_digest"] != adopted["write_scope_digest"]
                    or auth_value["target_policy_commit"] != adopted["adopted_policy_commit"]
                    or auth_value["target_policy_digest"] != adopted["adopted_policy_digest"]):
                errors.append("policy adoption authorization does not prove pre-existing user approval")
        proof = adopted["target_proof"]
        if (proof["default_branch"] != current["default_branch"]
                or proof["default_branch_oid"] != adopted["adopted_policy_commit"]
                or proof["policy_digest"] != adopted["adopted_policy_digest"]):
            errors.append("policy adoption target proof conflicts with its immutable record")
        try:
            git(repo_root, "merge-base", "--is-ancestor", proof["default_branch_oid"],
                current["default_branch_oid"])
            raw_policy = git(repo_root, "show", f"{proof['default_branch_oid']}:{POLICY_PATH}")
            raw_source = git(repo_root, "show", f"{proof['default_branch_oid']}:doc/engineering/workflow/source-of-truth.md")
            if "sha256:" + hashlib.sha256(raw_policy).hexdigest() != proof["policy_digest"]:
                errors.append("adopted policy content digest does not match trusted source commit")
            if "sha256:" + hashlib.sha256(raw_source).hexdigest() != proof["workflow_source_digest"]:
                errors.append("adopted workflow source digest does not match trusted source commit")
        except (OSError, ValueError) as exc:
            errors.append(f"adopted policy trust ancestry/content cannot be verified: {exc}")
        expected_prior_commit = adopted["adopted_policy_commit"]
        expected_prior_digest = adopted["adopted_policy_digest"]
        expected_predecessor = adopted["record_digest"]
    if errors:
        return result(errors, policy_commit=None, policy_digest=None,
                      adoption_chain_tip=None)
    active = dict(binding)
    active["policy_commit"] = expected_prior_commit
    active["policy_digest"] = expected_prior_digest
    return result([], policy_commit=expected_prior_commit, policy_digest=expected_prior_digest,
                  adoption_chain_tip=expected_predecessor, pin_source="task_issue_adoption_chain",
                  binding=active)


def validate_dependencies(binding, bindings):
    """Require a complete selected dependency closure; never infer missing nodes."""
    errors = list(validate_binding(binding)["blockers"])
    visiting, visited = set(), set()
    def walk(uid):
        if uid in visiting:
            errors.append("cyclic dependency: " + uid)
            return
        if uid in visited:
            return
        item = binding if uid == binding.get("task_uid") else bindings.get(uid)
        if not isinstance(item, dict):
            errors.append("dependency authority missing: " + str(uid))
            return
        errors.extend(validate_binding(item)["blockers"])
        visiting.add(uid)
        for dependency in item.get("dependencies", []):
            if isinstance(dependency, str):
                walk(dependency)
        visiting.remove(uid)
        visited.add(uid)
    if not errors:
        walk(binding["task_uid"])
    return result(errors)


def validate_tool_root(tool_root, target_repo_root, binding, *, maintenance=None):
    """Validate the pinned helper against live protected-default ancestry.

    No candidate checkout may bootstrap its own admission. This local check
    derives the canonical repository from its GitHub origin and independently
    reads the protected default-branch OID through the trusted policy helper.
    It never fetches or mutates a ref: if the live tip object is not already
    available in this repository, the result remains pending.
    """
    errors = list(validate_binding(binding)["blockers"])
    if errors:
        return result(errors)
    try:
        commit = maintenance["tool_revision"] if maintenance else binding["policy_commit"]
        target = Path(target_repo_root).resolve()
        tool = Path(tool_root).resolve()
        def common(root):
            return Path(git(root, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip()).resolve()
        if common(tool) != common(target):
            errors.append("tool and target Git common-directory mismatch")
        if git(tool, "rev-parse", "HEAD").decode().strip() != commit:
            errors.append("tool_root is not the pinned effective policy commit")
        repository = _canonical_github_repository_from_origin(target)
        try:
            current = _trusted_policy_identity(current_effective_policy_identity(target, repository))
        except (OSError, ValueError, subprocess.SubprocessError) as exc:
            message = str(exc)
            if any(token in message.lower() for token in (
                "authority read failed", "response is malformed", "response is not an object",
                "default branch is unavailable", "source blob is unavailable",
                "origin cannot be verified",
            )):
                return {"status": "pending", "blockers": [
                    "live protected-default policy authority is unavailable: " + message,
                ]}
            errors.append("live protected-default policy authority is invalid: " + message)
            return result(errors)
        current_oid = current["default_branch_oid"]
        object_read = subprocess.run(
            ["git", "-C", str(target), "cat-file", "-e", f"{current_oid}^{{commit}}"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )
        if object_read.returncode:
            return {"status": "pending", "blockers": [
                "live protected-default tip object is unavailable locally; no fetch was attempted",
            ]}
        ancestry = subprocess.run(
            ["git", "-C", str(target), "merge-base", "--is-ancestor", binding["policy_commit"], current_oid],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )
        if ancestry.returncode == 1:
            errors.append("pinned effective policy commit is not an ancestor of the live protected default tip")
        elif ancestry.returncode:
            return {"status": "pending", "blockers": [
                "live protected-default ancestry could not be verified locally",
            ]}
        if maintenance:
            from workflow_maintenance import read_maintenance_authority, validate_candidate_tool_root
            fresh = read_maintenance_authority(repository, maintenance['comment_id'],
                                              binding['task_uid'], maintenance['pr_number'],
                                              maintenance['tool_revision'],
                                              required_tool_paths=maintenance['allowed_tool_paths'])
            if fresh != maintenance:
                raise ValueError("maintenance scope changed during content validation")
            validate_candidate_tool_root(tool, target, fresh)
        git(tool, "diff", "--exit-code", commit, "--", "scripts/pm", *TRUSTED_IMPORT_FILES,
            "scripts/prepare-task-pr.sh", "scripts/plan-rust-required-scope.py")
        # Untracked import shadows are executable authority too.
        untracked = git(tool, "ls-files", "--others", "--", "scripts/pm", *TRUSTED_IMPORT_FILES, ":(exclude)**/__pycache__/**")
        if untracked.strip():
            errors.append("untracked files in trusted helper directory")
        for raw in git(tool,"ls-files","-z","--","scripts/pm").split(b"\0"):
            if raw:
                path=tool / raw.decode("utf-8")
                if path.is_symlink() or not path.resolve().is_relative_to(tool):
                    errors.append("trusted helper path escapes tool_root")
        existing_imports = tuple(
            path for path in TRUSTED_IMPORT_FILES
            if git(tool, "ls-tree", "-r", "--name-only", commit, "--", path).strip()
        )
        errors.extend(_trusted_file_errors(tool, commit, existing_imports))
        load_policy(tool, binding)
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as exc:
        errors.append(str(exc))
    return result(errors, tool_root=str(tool_root), target_repo_root=str(target_repo_root))


def _trusted_file_errors(tool_root, commit, relative_paths):
    """Verify exact imported repository modules without following shadow paths."""
    errors = []
    tool = Path(tool_root).resolve()
    for relative in relative_paths:
        try:
            tree = git(tool, "ls-tree", "-z", commit, "--", relative)
            records = [item for item in tree.split(b"\0") if item]
            if len(records) != 1:
                errors.append("trusted module missing or ambiguous: " + relative)
                continue
            metadata, raw_path = records[0].split(b"\t", 1)
            mode, object_type, _oid = metadata.decode("ascii").split()
            if raw_path.decode("utf-8", "strict") != relative or mode != "100644" or object_type != "blob":
                errors.append("trusted module has unsafe Git mode: " + relative)
                continue
            path = tool / relative
            if path.is_symlink() or not path.resolve().is_relative_to(tool) or not path.is_file():
                errors.append("trusted module path escapes tool_root: " + relative)
                continue
            expected = git(tool, "show", commit + ":" + relative)
            if path.read_bytes() != expected:
                errors.append("effective helper bytes differ: " + relative)
            if git(tool, "ls-files", "--others", "--", relative).strip():
                errors.append("untracked trusted module shadow: " + relative)
        except (ValueError, OSError, UnicodeError, subprocess.CalledProcessError) as exc:
            errors.append("trusted module unavailable: " + relative + ": " + str(exc))
    return errors


def load_trusted_corpus_module(tool_root, target_repo_root, binding, *, maintenance=None):
    """Execute only the corpus parser bytes from the pinned effective helper commit.

    This intentionally avoids normal import resolution: target ``sys.path``,
    ``sys.modules`` entries, and a candidate ``scripts/document_corpus.py``
    cannot select or shadow admission code.
    """
    blockers = list(validate_binding(binding)["blockers"])
    if blockers:
        raise ValueError("invalid loop binding: " + "; ".join(blockers))
    tool = Path(tool_root).resolve()
    target = Path(target_repo_root).resolve()
    commit = binding["policy_commit"]
    if git(tool, "rev-parse", "HEAD").decode().strip() != (maintenance['tool_revision'] if maintenance else commit):
        raise ValueError("tool root is not the pinned effective policy commit")
    if git(tool, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip() != git(target, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip():
        raise ValueError("tool and target Git common-directory mismatch")
    repository = _canonical_github_repository_from_origin(target)
    try:
        current = _trusted_policy_identity(current_effective_policy_identity(target, repository))
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        raise ValueError("live protected-default policy authority is unavailable: " + str(exc)) from exc
    current_oid = current["default_branch_oid"]
    object_read = subprocess.run(
        ["git", "-C", str(target), "cat-file", "-e", f"{current_oid}^{{commit}}"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if object_read.returncode:
        raise ValueError("live protected-default tip object is unavailable locally; no fetch was attempted")
    ancestry = subprocess.run(
        ["git", "-C", str(target), "merge-base", "--is-ancestor", commit, current_oid],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if ancestry.returncode == 1:
        raise ValueError("pinned effective policy commit is not an ancestor of the live protected default tip")
    if ancestry.returncode:
        raise ValueError("live protected-default ancestry could not be verified locally")
    errors = _trusted_file_errors(tool, commit, (CORPUS_MODULE,))
    if errors:
        raise ValueError("; ".join(errors))
    source = git(tool, "show", commit + ":" + CORPUS_MODULE)
    module_name = "_oasis7_effective_document_corpus_" + commit
    import types
    module = types.ModuleType(module_name)
    module.__file__ = str(tool / CORPUS_MODULE)
    module.__package__ = ""
    # Replace even a correctly named/pre-seeded entry; its origin is not proof
    # that its module object contains the pinned source bytes.
    sys.modules[module_name] = module
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def load_policy(tool_root, binding):
    raw = git(tool_root, "show", binding["policy_commit"] + ":" + POLICY_PATH)
    if "sha256:" + hashlib.sha256(raw).hexdigest() != binding["policy_digest"]:
        raise ValueError("effective policy digest mismatch")
    policy = json.loads(raw)
    if policy.get("schema") != "oasis7.loop-policy/v1" or not isinstance(policy.get("rules"), list) or not policy["rules"]:
        raise ValueError("invalid effective policy")
    for rule in policy["rules"]:
        if not isinstance(rule, dict) or not safe_path(rule.get("pattern")) or rule.get("loop") not in LOOPS:
            raise ValueError("invalid classification rule")
    if not isinstance(policy.get("denied"), list) or any(not safe_path(p) for p in policy["denied"]):
        raise ValueError("invalid denied paths")
    if not isinstance(policy.get("document_extensions"),list) or any(not isinstance(x,str) or not x.startswith(".") for x in policy["document_extensions"]):
        raise ValueError("invalid document type policy")
    if not isinstance(policy.get("mixed_documents", []), list) or any(not safe_path(p) for p in policy.get("mixed_documents", [])):
        raise ValueError("invalid mixed document policy")
    return policy


def path_matches(path, pattern):
    """Match a whole POSIX path; only a standalone ** crosses components."""
    parts = path.split("/")
    positions = {0}
    for component in pattern.split("/"):
        if component == "**":
            positions = set(range(min(positions), len(parts) + 1))
        else:
            positions = {i + 1 for i in positions if i < len(parts)
                         and fnmatch.fnmatchcase(parts[i], component)}
        if not positions:
            return False
    return len(parts) in positions


def classify_path(path, policy):
    if not safe_path(path) or any(path_matches(path, p) for p in policy["denied"]):
        return None
    if path in policy.get("mixed_documents", []):
        return None
    for rule in policy["rules"]:
        if path_matches(path, rule["pattern"]):
            if rule["loop"] in {"product","system"} and PurePosixPath(path).suffix.lower() not in policy["document_extensions"]:
                return None
            return rule["loop"]
    return None


def _corpus_locations(core, model):
    """Index source records by their exact endpoint without trusting path names."""
    indexed = {}
    sources = set(model.objects_by_path)
    sources.update(model.semantic_entries_by_path)
    sources.update(model.path_to_bundle)
    sources.update(model.evidence_entries_by_path)
    sources.update(model.path_to_group)
    for source in sorted(sources):
        for location in core.locate(model, source):
            if location.kind != "static-control":
                indexed.setdefault(location.record_path, []).append(location)
    return indexed


def _corpus_owner(core, policy, model, source):
    """Apply narrow v3 source/evidence ownership before legacy path policy."""
    controls = {
        core.CORPUS_ROOT,
        core.EVIDENCE_ROOT,
        core.REGISTRY_PATH,
        core.LEGACY_SEMANTIC_ROOT,
    }
    if source in controls or source in model.metadata_paths:
        return "code"
    if source.startswith("doc/testing/evidence/"):
        if source == core.EVIDENCE_ROOT:
            return "code"
        if PurePosixPath(source).suffix.lower() in core.EVIDENCE_SUFFIXES:
            return "system"
        return None
    return classify_path(source, policy)


def _declared_path(path, patterns):
    return any(path_matches(path, pattern) for pattern in patterns)


def _check_scope_membership(errors, binding, required, changed_path):
    for path in sorted(required):
        if not _declared_path(path, binding["write_scope"]) or _declared_path(path, binding["out_of_scope"]):
            errors.append(f"outside declared write scope: {changed_path} requires {path}")


def _corpus_scope_errors(core, policy, root, binding, base, head, changed_paths):
    """Resolve every changed doc path against immutable base/head corpus models."""
    base_view = core.GitCorpusView(root, base)
    head_view = core.GitCorpusView(root, head)
    base_model = core.load_corpus(base_view)
    head_model = core.load_corpus(head_view)
    snapshots = ((base_view, base_model, set(base_view.list_doc_paths())),
                 (head_view, head_model, set(head_view.list_doc_paths())))
    endpoint_indexes = [_corpus_locations(core, model) for _, model, _paths in snapshots]
    errors = []
    resolved = []

    def endpoint_locations(path, model, endpoint_index):
        if path in endpoint_index:
            return endpoint_index[path]
        if path in model.metadata_paths or path in {
            core.CORPUS_ROOT, core.EVIDENCE_ROOT, core.REGISTRY_PATH,
            core.LEGACY_SEMANTIC_ROOT,
        }:
            return [core.RecordLocation("static-control", path, path, (path,))]
        return []

    for path in changed_paths:
        if not path.startswith("doc/"):
            continue
        required = {path}
        owners = []
        object_sources = set()
        # A changed shard endpoint inherits from every exact source/group
        # location in both immutable snapshots. Static controls remain code.
        matched_endpoint = False
        for (_view, model, view_paths), index in zip(snapshots, endpoint_indexes):
            locations = endpoint_locations(path, model, index)
            if locations:
                matched_endpoint = True
            for location in locations:
                if location.kind == "static-control":
                    owners.append("code")
                    continue
                owners.extend(_corpus_owner(core, policy, model, member) if member in view_paths else None
                              for member in location.member_paths)
                object_sources.update(location.member_paths)
                required.update(location.member_paths)
                # A group/bundle scope includes every member's own endpoint
                # locations as well as the atomic aggregate record.
                for member in location.member_paths:
                    for member_location in core.locate(model, member):
                        if member_location.kind != "static-control":
                            required.add(member_location.record_path)

        # Ordinary sources need their prospective exact record even when the
        # record is newly added, already missing, or being deleted.
        source_found = False
        for (_view, model, view_paths), index in zip(snapshots, endpoint_indexes):
            is_metadata = path in model.metadata_paths
            is_sidecar_endpoint = path in index
            if is_metadata and not is_sidecar_endpoint:
                matched_endpoint = True
                owners.append("code")
                continue
            if is_metadata:
                # Its ownership was inherited from the exact record members
                # above; never reclassify a valid shard as a control file.
                continue
            owner = _corpus_owner(core, policy, model, path) if path in view_paths else None
            if owner is None:
                continue
            source_found = True
            owners.append(owner)
            locations = core.locate(model, path)
            for location in locations:
                if location.kind != "static-control":
                    required.add(location.record_path)
                    required.update(location.member_paths)
                    owners.extend(_corpus_owner(core, policy, model, member) if member in view_paths else None
                                  for member in location.member_paths)
                    object_sources.update(location.member_paths)
            if owner in {"product", "system"}:
                object_sources.add(path)
            if path.startswith("doc/testing/evidence/") and path != core.EVIDENCE_ROOT:
                required.add(core.record_path("evidence", path))
            elif owner in {"product", "system"}:
                required.add(core.record_path("object", path))

        # New files beneath the corpus metadata root must be known records or
        # fixed controls; the shared loader rejects unknown metadata above.
        if path.startswith("doc/.governance/document-corpus/") and not matched_endpoint:
            errors.append(f"unregistered corpus metadata endpoint: {path}")
        concrete_owners = {owner for owner in owners if owner is not None}
        if not source_found and not matched_endpoint:
            concrete_owners.add(classify_path(path, policy))
        if len(concrete_owners) != 1 or None in concrete_owners or any(owner is None for owner in owners):
            errors.append(f"corpus source or endpoint ownership is unknown or crosses loops: {path}")
            resolved.append({"path": path, "loop": None})
        elif next(iter(concrete_owners)) != binding["loop"]:
            errors.append(f"loop ownership mismatch: {path}")
            resolved.append({"path": path, "loop": next(iter(concrete_owners))})
        else:
            owner = next(iter(concrete_owners))
            resolved.append({"path": path, "loop": owner})
            for view, _model, _view_paths in snapshots:
                mode = view.file_mode(path)
                if mode == "000000":
                    continue
                allowed = {"100644", "100755"} if owner == "code" else {"100644"}
                if mode not in allowed:
                    errors.append(f"unsupported asset mode {mode}: {path}@{view.snapshot_id}")
        for source in sorted(object_sources):
            if source.startswith("doc/testing/evidence/") and source != core.EVIDENCE_ROOT:
                continue
            for view, model, _view_paths in snapshots:
                mode = view.file_mode(source)
                record = model.objects_by_path.get(source)
                if mode == "000000":
                    if record is not None:
                        errors.append(f"generated object has no source: {source}@{view.snapshot_id}")
                    continue
                try:
                    expected_record = core.expected_object(view, source)
                except (ValueError, OSError) as exc:
                    errors.append(f"cannot derive object endpoint for {source}@{view.snapshot_id}: {exc}")
                    continue
                if record != expected_record:
                    errors.append(f"generated object mismatch: {source}@{view.snapshot_id}")
        _check_scope_membership(errors, binding, required, path)

    return errors, resolved


def scope_context(root, integration_base, head):
    """Bind task-owned diff separately from current integration composition."""
    if not all(isinstance(value, str) and OID.fullmatch(value) for value in (integration_base, head)):
        raise ValueError('scope context requires immutable integration/head OIDs')
    bases = git(root, 'merge-base', '--all', integration_base, head).decode().splitlines()
    if len(bases) != 1:
        raise ValueError('scope comparison requires one unambiguous merge-base')
    tree = git(root, 'merge-tree', '--write-tree', integration_base, head).decode().splitlines()[0]
    return {'scope_base_oid': bases[0], 'integration_base_oid': integration_base, 'source_head_oid': head, 'integration_tree_oid': tree}


def validate_scope(tool_root, target_repo_root, binding, base, head, *, maintenance=None):
    errors = list(validate_binding(binding)["blockers"])
    paths = []
    if errors:
        return result(errors, execution_scope="execution_scope_unverified")
    try:
        if not isinstance(base, str) or not OID.fullmatch(base) or not isinstance(head, str) or not OID.fullmatch(head):
            raise ValueError("scope requires immutable base/head OIDs")
        git(target_repo_root, "merge-base", "--is-ancestor", base, head)
        policy = load_policy(tool_root, binding)
        # --no-renames checks both endpoints as deletion/addition even when Git
        # rename similarity heuristics would miss a move or copy.
        changed = git(target_repo_root, "diff", "--no-renames", "--name-only", "-z", base, head).split(b"\0")
        changed_paths = []
        for raw in changed:
            if not raw:
                continue
            path = raw.decode("utf-8", "strict")
            changed_paths.append(path)
            if path in policy.get("mixed_documents", []):
                errors.append(f"mixed document requires separately authorized semantic split/migration: {path}")
            if not path.startswith("doc/"):
                owner = classify_path(path, policy)
                if owner != binding["loop"]:
                    errors.append(f"loop ownership mismatch or unknown path: {path}")
                paths.append({"path": path, "loop": owner})
            allowed_scope = maintenance['allowed_write_paths'] if maintenance else binding['write_scope']
            if not any(path_matches(path, p) for p in allowed_scope) or (not maintenance and any(path_matches(path, p) for p in binding["out_of_scope"])):
                errors.append(f"outside declared write scope: {path}")
            if not path.startswith("doc/"):
                for revision in (base, head):
                    entry = git(target_repo_root, "ls-tree", "-z", revision, "--", path)
                    if not entry:
                        continue
                    mode = entry.split(b" ", 1)[0].decode()
                    allowed = {"100644", "100755"} if owner == "code" else {"100644"}
                    if mode not in allowed:
                        errors.append(f"unsupported asset mode {mode}: {path}@{revision}")
        if any(path.startswith("doc/") for path in changed_paths):
            core = load_trusted_corpus_module(tool_root, target_repo_root, binding, maintenance=maintenance)
            scope_binding = dict(binding)
            if maintenance:
                scope_binding['write_scope'] = list(maintenance['allowed_write_paths'])
                scope_binding['out_of_scope'] = []
            corpus_errors, corpus_paths = _corpus_scope_errors(
                core, policy, target_repo_root, scope_binding, base, head, changed_paths,
            )
            errors.extend(corpus_errors)
            paths.extend(corpus_paths)
    except (ValueError, OSError, UnicodeError, KeyError, json.JSONDecodeError) as exc:
        errors.append(str(exc))
    return result(errors, paths=paths, execution_scope="execution_scope_unverified", policy_commit=binding.get("policy_commit"))
