"""Deterministic, immutable loop policy. No task mutations or scheduling.

Admission callers must validate_tool_root before executing trusted helpers.
validate_scope can also run on candidate code in offline tests; that is not
effective-policy activation or native filesystem isolation evidence.
"""
from __future__ import annotations

import fnmatch
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

SCHEMA = "oasis7.loop-task/v1"
POLICY_PATH = "scripts/pm/loop-policy.v1.json"
LOOPS = {"product", "system", "code"}
OID = re.compile(r"[0-9a-f]{40}\Z")
UID = re.compile(r"task_[0-9a-f]{32}\Z")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}\Z")


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
    if isinstance(binding.get("acceptance_refs"), list) and any(not isinstance(x, str) or not x.strip() for x in binding["acceptance_refs"]):
        errors.append("invalid acceptance reference")
    deps = binding.get("dependencies")
    if isinstance(deps, list):
        if any(not isinstance(d, str) or not UID.fullmatch(d) for d in deps):
            errors.append("dependencies must be task UIDs")
        elif len(set(deps)) != len(deps) or binding.get("task_uid") in deps:
            errors.append("duplicate or cyclic task dependency")
    if isinstance(binding.get("input_contracts"), list):
        for item in binding["input_contracts"]:
            if not isinstance(item, dict) or not item.get("contract_id") or type(item.get("revision")) is not int or item["revision"] < 1 or not isinstance(item.get("publication_ref"), dict) or not item.get("consumed_clauses") or not isinstance(item.get("contract_digest"),str) or not DIGEST.fullmatch(item["contract_digest"]):
                errors.append("invalid immutable input contract reference")
                continue
            ref=item["publication_ref"]
            clauses=item["consumed_clauses"]
            if not isinstance(item["contract_id"],str) or any(type(ref.get(k)) is not int or ref[k]<1 for k in ("issue_number","comment_id")) or not isinstance(clauses,list) or any(not isinstance(c,str) or not c.strip() for c in clauses):
                errors.append("invalid contract publication identity/clauses")
    obligations = binding.get("delivery_obligations", [])
    if not isinstance(obligations, list) or any(not isinstance(o, dict) or not o.get("id") or not isinstance(o.get("task_uid"), str) or not UID.fullmatch(o["task_uid"]) or type(o.get("issue_number")) is not int or o["issue_number"]<1 for o in obligations):
        errors.append("invalid delivery obligations")
    return result(errors)


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


def validate_tool_root(tool_root, target_repo_root, binding):
    """Validate explicit effective checkout; caller must refresh origin first.

    No candidate checkout may bootstrap its own admission. This local check
    cannot establish that a remote tracking ref is fresh; live gate callers
    retain their existing refresh/readback responsibility.
    """
    errors = list(validate_binding(binding)["blockers"])
    if errors:
        return result(errors)
    try:
        commit = binding["policy_commit"]
        target = Path(target_repo_root).resolve()
        tool = Path(tool_root).resolve()
        def common(root):
            return Path(git(root, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip()).resolve()
        if common(tool) != common(target):
            errors.append("tool and target Git common-directory mismatch")
        if git(tool, "rev-parse", "HEAD").decode().strip() != commit:
            errors.append("tool_root is not the pinned effective policy commit")
        git(target, "merge-base", "--is-ancestor", commit, "refs/remotes/origin/main")
        git(tool, "diff", "--exit-code", commit, "--", "scripts/pm", "scripts/prepare-task-pr.sh", "scripts/plan-rust-required-scope.py")
        # Untracked import shadows are executable authority too.
        untracked = git(tool, "ls-files", "--others", "--", "scripts/pm", ":(exclude)**/__pycache__/**")
        if untracked.strip():
            errors.append("untracked files in trusted helper directory")
        for raw in git(tool,"ls-files","-z","--","scripts/pm").split(b"\0"):
            if raw:
                path=tool / raw.decode("utf-8")
                if path.is_symlink() or not path.resolve().is_relative_to(tool):
                    errors.append("trusted helper path escapes tool_root")
        load_policy(tool, binding)
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as exc:
        errors.append(str(exc))
    return result(errors, tool_root=str(tool_root), target_repo_root=str(target_repo_root))


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


def scope_context(root, integration_base, head):
    """Bind task-owned diff separately from current integration composition."""
    if not all(isinstance(value, str) and OID.fullmatch(value) for value in (integration_base, head)):
        raise ValueError('scope context requires immutable integration/head OIDs')
    bases = git(root, 'merge-base', '--all', integration_base, head).decode().splitlines()
    if len(bases) != 1:
        raise ValueError('scope comparison requires one unambiguous merge-base')
    tree = git(root, 'merge-tree', '--write-tree', integration_base, head).decode().splitlines()[0]
    return {'scope_base_oid': bases[0], 'integration_base_oid': integration_base, 'source_head_oid': head, 'integration_tree_oid': tree}


def validate_scope(tool_root, target_repo_root, binding, base, head):
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
        for raw in changed:
            if not raw:
                continue
            path = raw.decode("utf-8", "strict")
            if path in policy.get("mixed_documents", []):
                errors.append(f"mixed document requires separately authorized semantic split/migration: {path}")
            owner = classify_path(path, policy)
            paths.append({"path": path, "loop": owner})
            if owner != binding["loop"]:
                errors.append(f"loop ownership mismatch or unknown path: {path}")
            if not any(path_matches(path, p) for p in binding["write_scope"]) or any(path_matches(path, p) for p in binding["out_of_scope"]):
                errors.append(f"outside declared write scope: {path}")
            for revision in (base, head):
                entry = git(target_repo_root, "ls-tree", "-z", revision, "--", path)
                if not entry:
                    continue
                mode = entry.split(b" ", 1)[0].decode()
                allowed = {"100644", "100755"} if owner == "code" else {"100644"}
                if mode not in allowed:
                    errors.append(f"unsupported asset mode {mode}: {path}@{revision}")
    except (ValueError, OSError, UnicodeError, KeyError, json.JSONDecodeError) as exc:
        errors.append(str(exc))
    return result(errors, paths=paths, execution_scope="execution_scope_unverified", policy_commit=binding.get("policy_commit"))
