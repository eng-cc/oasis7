"""Strict Task primary-package history and its current evidence identity.

The Issue field is canonical truth; the local mapping is a cache. A completion
is effective only after the immutable server comment and Issue field agree.
Readers remain available independently of the completion writer for rollback.
"""
from __future__ import annotations

import hashlib
import base64
import json
from pathlib import Path
import re
import subprocess
import sys
import os
import importlib.util
from contextlib import contextmanager
from datetime import datetime
from pathlib import PurePosixPath
from typing import Any

SCHEMA = "oasis7-task-primary-package-completion/v1"
SCOPE_SCHEMA = "oasis7-task-primary-package-completion/v2"
SELECTOR_SCHEMA = "oasis7-existing-task-scope-selector/v1"
PROOF_SCHEMA = "oasis7-existing-task-scope-proof/v1"
LOOP_PROOF_SCHEMA = "oasis7-task-loop-scope-proof/v1"
REFERENCE_TYPE = "task-primary-package-completion"
MARKER = "<!-- oasis7-task-primary-package-completion/v1 -->"
FIELD = "primary_package_completion"
PACKAGE_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*\Z")
DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
IDENTITY_FIELDS = ("task_uid", "repository", "issue_number", "issue_url",
                   "project_item_id", "owner_role", "canonical_worktree",
                   "task_branch", "acceptance", "loop_binding", "bootstrap_epoch")


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False).encode("utf-8")


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def strict_primary_package(value: Any, present: bool = True) -> str | None:
    if not present or value is None:
        return None
    if not isinstance(value, str) or not PACKAGE_RE.fullmatch(value):
        raise ValueError("primary_package must be absent/null or one exact Cargo package name")
    return value


def immutable_identity(task: dict[str, Any]) -> dict[str, Any]:
    return {key: task.get(key, 1 if key == "bootstrap_epoch" else None)
            for key in IDENTITY_FIELDS}


def issue_primary_fields(body: str) -> dict[str, Any]:
    fields: dict[str, Any] = {}
    for key in ("primary_package", "primary_package_completion_b64"):
        lines = re.findall(rf"^[ \t]*(?:-[ \t]+)?{key}\b[^\n]*$", body, re.M)
        if not lines:
            continue
        if len(lines) != 1:
            raise ValueError(f"Task Issue {key} is duplicated")
        match = re.fullmatch(rf"- {key}: `([^`\n]+)`", lines[0])
        if match is None:
            raise ValueError(f"Task Issue {key} is malformed")
        if key == "primary_package":
            fields[key] = strict_primary_package(match.group(1))
        else:
            encoded = match.group(1)
            raw = base64.b64decode(encoded + "=" * (-len(encoded) % 4), altchars=b"-_", validate=True)
            fields[FIELD] = strict_json(raw.decode())
    return fields


def completion_body(payload: dict[str, Any]) -> str:
    return MARKER + "\n" + canonical_bytes(payload).decode() + "\n"


def strict_json(raw: str) -> Any:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate completion JSON key: " + key)
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs)


def parse_completion_body(body: str) -> dict[str, Any]:
    if not isinstance(body, str) or not body.startswith(MARKER + "\n"):
        raise ValueError("completion comment marker is missing")
    value = strict_json(body[len(MARKER) + 1:])
    if not isinstance(value, dict) or completion_body(value) != body:
        raise ValueError("completion comment is not canonical")
    return value


def validate_completion(task: dict[str, Any]) -> dict[str, Any] | None:
    record = task.get(FIELD)
    if record is None:
        return None
    if not isinstance(record, dict) or set(record) != {"payload", "server"}:
        raise ValueError("primary completion record fields are invalid")
    payload, server = record["payload"], record["server"]
    if (not isinstance(payload, dict)
            or payload.get("schema") not in {SCHEMA, SCOPE_SCHEMA}
            or set(payload) != ({"schema", "action_id", "before", "primary_package"}
                | ({"scope_evidence"} if payload.get("schema") == SCOPE_SCHEMA else set()))):
        raise ValueError("primary completion payload schema/fields are invalid")
    package = strict_primary_package(payload.get("primary_package"))
    if package is None:
        raise ValueError("completion primary must be nonempty")
    before = payload["before"]
    if not isinstance(before, dict) or set(before) != {"identity", "primary_present", "primary_package"}:
        raise ValueError("completion before image is invalid")
    if type(before["primary_present"]) is not bool or before["primary_package"] is not None:
        raise ValueError("completion predecessor must be strictly absent/null")
    if before["identity"] != immutable_identity(task):
        raise ValueError("completion immutable Task identity differs")
    identity = before["identity"]
    if (not isinstance(identity.get("task_uid"), str) or not re.fullmatch(r"task_[0-9a-f]{32}", identity["task_uid"])
            or not isinstance(identity.get("repository"), str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", identity["repository"])
            or type(identity.get("issue_number")) is not int or identity["issue_number"] < 1
            or identity.get("issue_url") != f"https://github.com/{identity['repository']}/issues/{identity['issue_number']}"
            or not isinstance(identity.get("project_item_id"), str) or not identity["project_item_id"]
            or not isinstance(identity.get("canonical_worktree"), str) or not Path(identity["canonical_worktree"]).is_absolute()
            or not isinstance(identity.get("task_branch"), str) or not identity["task_branch"]
            or not isinstance(identity.get("owner_role"), str) or not identity["owner_role"]
            or not isinstance(identity.get("acceptance"), list) or not identity["acceptance"]
            or any(not isinstance(item, str) or not item for item in identity["acceptance"])
            or type(identity.get("bootstrap_epoch")) is not int or identity["bootstrap_epoch"] < 1):
        raise ValueError("completion immutable Task identity fields/types are invalid")
    expected_action = digest({"schema": payload["schema"], "before": before, "primary_package": package})
    if payload["action_id"] != expected_action:
        raise ValueError("completion action identity differs")
    if strict_primary_package(task.get("primary_package"), "primary_package" in task) != package:
        raise ValueError("canonical primary and completion record disagree")
    if (not isinstance(server, dict)
            or set(server) != {"repository", "issue_number", "comment_id", "comment_url", "body_sha256"}
            or server["repository"] != task.get("repository")
            or server["issue_number"] != task.get("issue_number")
            or type(server["comment_id"]) is not int or server["comment_id"] <= 0):
        raise ValueError("completion server locator is invalid")
    url = f"https://github.com/{server['repository']}/issues/{server['issue_number']}#issuecomment-{server['comment_id']}"
    if server["comment_url"] != url:
        raise ValueError("completion comment URL differs")
    expected_digest = "sha256:" + hashlib.sha256(completion_body(payload).encode()).hexdigest()
    if server["body_sha256"] != expected_digest:
        raise ValueError("completion server content digest differs")
    if payload["schema"] == SCOPE_SCHEMA:
        proof = payload["scope_evidence"]
        validate_scope_proof_shape(proof)
        if proof["schema"] == LOOP_PROOF_SCHEMA:
            if proof["loop_binding"] != payload["before"]["identity"].get("loop_binding") or proof["primary_package"] != package:
                raise ValueError("completion loop proof differs from immutable binding/primary")
        elif proof["source_refs"] != task.get("source_refs", []):
            raise ValueError("completion source references differ from the durable before image")
    return record


def effective_primary_package(task: dict[str, Any]) -> str | None:
    package = strict_primary_package(task.get("primary_package"), "primary_package" in task)
    validate_completion(task)
    return package


def completion_reference(task: dict[str, Any]) -> dict[str, Any] | None:
    record = validate_completion(task)
    if record is None:
        return None
    return {"type": REFERENCE_TYPE, "schema": record["payload"]["schema"], "task_uid": task["task_uid"],
            "primary_package": record["payload"]["primary_package"],
            **record["server"]}


def validate_completion_reference(item: Any, task: dict[str, Any]) -> None:
    validate_reference_shape(item)
    expected = completion_reference(task)
    if expected is None or not isinstance(item, dict) or item != expected:
        raise ValueError("consumed primary completion differs from the current effective record")


def validate_reference_shape(item: Any) -> None:
    required = {"type", "schema", "task_uid", "primary_package", "repository", "issue_number", "comment_id", "comment_url", "body_sha256"}
    if (not isinstance(item, dict) or set(item) != required
            or item.get("type") != REFERENCE_TYPE or item.get("schema") not in {SCHEMA, SCOPE_SCHEMA}
            or not isinstance(item.get("task_uid"), str) or not re.fullmatch(r"task_[0-9a-f]{32}", item["task_uid"])
            or not isinstance(item.get("repository"), str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", item["repository"])
            or type(item.get("issue_number")) is not int or item["issue_number"] < 1
            or type(item.get("comment_id")) is not int or item["comment_id"] < 1
            or not isinstance(item.get("body_sha256"), str) or not DIGEST_RE.fullmatch(item["body_sha256"])):
        raise ValueError("primary completion reference strict fields/types are invalid")
    if strict_primary_package(item.get("primary_package")) is None:
        raise ValueError("primary completion reference package is absent")
    if item["comment_url"] != f"https://github.com/{item['repository']}/issues/{item['issue_number']}#issuecomment-{item['comment_id']}":
        raise ValueError("primary completion reference server URL is invalid")


def _github(repository: str, endpoint: str) -> Any:
    result = subprocess.run(["gh", "api", endpoint], capture_output=True, text=True, check=False)
    if result.returncode:
        raise ValueError("primary completion server readback unavailable: " + result.stderr.strip())
    return strict_json(result.stdout)


def validate_current_completion(root: Path, task: dict[str, Any]) -> None:
    record = validate_completion(task)
    if record is None:
        return
    server = record["server"]
    comment = _github(server["repository"], f"repos/{server['repository']}/issues/comments/{server['comment_id']}")
    if (not isinstance(comment, dict) or comment.get("id") != server["comment_id"]
            or comment.get("html_url") != server["comment_url"]
            or comment.get("issue_url") != f"https://api.github.com/repos/{server['repository']}/issues/{server['issue_number']}"
            or not isinstance(comment.get("created_at"), str) or not comment["created_at"]
            or comment.get("created_at") != comment.get("updated_at")
            or comment.get("body") != completion_body(record["payload"])):
        raise ValueError("primary completion server comment identity/content differs")
    author = (comment.get("user") or {}).get("login")
    if not isinstance(author, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*", author):
        raise ValueError("primary completion server author identity is invalid")
    permission = _github(server["repository"], f"repos/{server['repository']}/collaborators/{author}/permission")
    if not isinstance(permission, dict) or permission.get("permission") not in {"admin", "maintain", "write"}:
        raise ValueError("primary completion server author lacks repository writer permission")
    issue = _github(server["repository"], f"repos/{server['repository']}/issues/{server['issue_number']}")
    if (not isinstance(issue, dict) or issue.get("number") != task["issue_number"]
            or issue.get("html_url") != task["issue_url"]):
        raise ValueError("primary completion canonical Issue identity differs")
    body = str(issue.get("body") or "")
    if re.findall(r"^task_uid:[^\n]*$", body, re.M) != ["task_uid: " + task["task_uid"]]:
        raise ValueError("primary completion canonical Issue UID differs")
    fields = issue_primary_fields(body)
    if fields.get("primary_package") != task["primary_package"] or fields.get(FIELD) != record:
        raise ValueError("primary completion canonical Issue fields differ")
    if record["payload"]["schema"] == SCOPE_SCHEMA:
        saved = record["payload"]["scope_evidence"]
        current = (loop_scope_proof(root, task, package=saved["primary_package"], current_authority=True)
            if saved["schema"] == LOOP_PROOF_SCHEMA else existing_scope_proof(root, task, saved["selector"],
                saved["base_oid"], saved["head_oid"], issue_body=body))
        if current != saved:
            raise ValueError("completion current scope evidence differs")


def load_task(root: Path, task_uid: str) -> dict[str, Any] | None:
    path = Path(root) / ".pm/github-project-sync/tasks.json"
    if not path.exists():
        return None
    mapping = strict_json(path.read_text())
    task = (mapping.get("tasks") or {}).get(task_uid)
    if task is not None and not isinstance(task, dict):
        raise ValueError("Task mapping record is invalid")
    if task is not None:
        task = {**task, "task_uid": task_uid}
    return task


def validate_consumed_contracts(items: Any, task: dict[str, Any] | None, *, root: Path | None = None) -> None:
    if not isinstance(items, list):
        raise ValueError("consumed_contracts must be an array")
    refs = [item for item in items if isinstance(item, dict)
            and (item.get("type") == REFERENCE_TYPE or item.get("schema") in {SCHEMA, SCOPE_SCHEMA})]
    expected = completion_reference(task) if task is not None else None
    if len(refs) != (1 if expected is not None else 0):
        raise ValueError("current primary completion reference is missing, duplicated or unexpected")
    if refs:
        validate_completion_reference(refs[0], task)
        if root is not None:
            validate_current_completion(root, task)


def snapshot_primary_compatible(saved_task: dict[str, Any], current_task: dict[str, Any],
                                task_record: dict[str, Any]) -> bool:
    saved = strict_primary_package(saved_task.get("primary_package"), "primary_package" in saved_task)
    current = strict_primary_package(current_task.get("primary_package"), "primary_package" in current_task)
    if saved == current:
        return True
    record = validate_completion(task_record)
    return saved is None and record is not None and current == record["payload"]["primary_package"]


def validate_scope_selector(value: Any, task_uid: str) -> dict[str, Any]:
    if (not isinstance(value, dict) or set(value) != {"schema", "task_uid", "freeze", "authorizations"}
            or value.get("schema") != SELECTOR_SCHEMA or value.get("task_uid") != task_uid
            or not isinstance(value.get("authorizations"), list) or not value["authorizations"]):
        raise ValueError("existing scope selector schema/Task/fields are invalid")
    ids = set()
    for locator in [value["freeze"], *value["authorizations"]]:
        if (not isinstance(locator, dict) or set(locator) != {"comment_id", "body_sha256"}
                or type(locator.get("comment_id")) is not int or locator["comment_id"] <= 0
                or not isinstance(locator.get("body_sha256"), str)
                or not DIGEST_RE.fullmatch(locator["body_sha256"]) or locator["comment_id"] in ids):
            raise ValueError("existing scope selector locator is malformed or duplicated")
        ids.add(locator["comment_id"])
    return value


def _path(path: str) -> str:
    if (not isinstance(path, str) or not path or path != path.strip()
            or PurePosixPath(path).is_absolute() or str(PurePosixPath(path)) != path
            or any(part in {".", ".."} for part in path.split("/"))
            or any(c in path for c in "\\*?[]{}\0") or any(ord(c) < 33 for c in path)):
        raise ValueError("scope path is not a normalized exact repository path")
    return path


def _git(root: Path, *args: str) -> str:
    try:
        return subprocess.check_output(["git", "-C", str(root), *args], text=True, stderr=subprocess.PIPE).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ValueError("scope immutable Git proof is unavailable") from exc


def _oid(root: Path, value: str) -> str:
    result = _git(root, "rev-parse", "--verify", value + "^{commit}")
    if not re.fullmatch(r"[0-9a-f]{40,64}", result):
        raise ValueError("scope commit identity is invalid")
    return result


def _endpoints(root: Path, base: str, head: str) -> list[dict[str, Any]]:
    raw = subprocess.check_output(["git", "-C", str(root), "diff", "--name-status", "-z", "--find-renames", base, head])
    fields = raw.decode("utf-8", "strict").split("\0")
    result = []
    index = 0
    while fields[index]:
        status = fields[index]
        index += 1
        if status.startswith("R"):
            old, new = _path(fields[index]), _path(fields[index + 1])
            index += 2
            operation = "R"
        elif status in {"A", "D", "M"}:
            path = _path(fields[index]); index += 1
            old, new = (None, path) if status == "A" else (path, None) if status == "D" else (path, path)
            operation = status
        else:
            raise ValueError("scope endpoint operation is unsupported")
        result.append({"operation": operation, "old_path": old, "new_path": new})
    return sorted(result, key=canonical_bytes)


def _mode(root: Path, oid: str, path: str) -> str:
    rows = _git(root, "ls-tree", oid, "--", path).splitlines()
    if len(rows) != 1 or rows[0].split("\t", 1)[-1] != path:
        raise ValueError("scope path missing or ambiguous in original endpoint")
    mode = rows[0].split()[0]
    if mode not in {"100644", "100755"}:
        raise ValueError("scope endpoint must be a regular Git blob")
    return mode


def _comment(task: dict[str, Any], locator: dict[str, Any]) -> dict[str, Any]:
    repo, number, comment_id = task["repository"], task["issue_number"], locator["comment_id"]
    value = _github(repo, f"repos/{repo}/issues/comments/{comment_id}")
    if (not isinstance(value, dict) or type(value.get("id")) is not int or value["id"] != comment_id
            or value.get("html_url") != f"{task['issue_url']}#issuecomment-{comment_id}"
            or value.get("issue_url") != f"https://api.github.com/repos/{repo}/issues/{number}"
            or not isinstance(value.get("body"), str)
            or "sha256:" + hashlib.sha256(value["body"].encode()).hexdigest() != locator["body_sha256"]
            or value.get("created_at") != value.get("updated_at")):
        raise ValueError("existing scope comment identity/digest/immutable state differs")
    try:
        datetime.fromisoformat(value["created_at"].replace("Z", "+00:00"))
    except (KeyError, ValueError, TypeError, AttributeError) as exc:
        raise ValueError("scope comment server creation time is invalid") from exc
    author = (value.get("user") or {}).get("login")
    if not isinstance(author, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*", author):
        raise ValueError("scope comment server author is invalid")
    permission = _github(repo, f"repos/{repo}/collaborators/{author}/permission")
    if not isinstance(permission, dict) or permission.get("permission") != "admin":
        raise ValueError("scope comment current administrator authority unavailable")
    return value


def _freeze(comment: dict[str, Any], task: dict[str, Any]) -> tuple[str, str]:
    body = comment["body"]
    if not body.startswith("<!-- oasis7-pm-evidence -->\n"):
        raise ValueError("original scope freeze marker is missing")
    expected = {"Task UID": task["task_uid"], "Evidence Phase": "draft_candidate_freeze", "Role": "tpm",
        "Source Worktree": task["canonical_worktree"], "Source Branch": task["task_branch"],
        "Comparison Ref": "origin/" + task["default_branch"]}
    values = {}
    for key in (*expected, "Source Head", "Comparison OID"):
        matches = re.findall(rf"^{re.escape(key)}: ([^\n]+)$", body, re.M)
        if len(matches) != 1 or (key in expected and matches[0] != expected[key]):
            raise ValueError("original freeze Task/worktree/branch/OID identity differs")
        values[key] = matches[0]
    for key in ("Source Head", "Comparison OID"):
        if not re.fullmatch(r"[0-9a-f]{40,64}", values[key]):
            raise ValueError("original freeze OID is not immutable")
    return values["Comparison OID"], values["Source Head"]


_PLAN_FIELDS = ("step_id", "acceptance_refs", "dependencies", "verification_command", "verification_evidence",
                "write_scope", "out_of_scope", "required_role_slices")
_LEGACY_ENVELOPES = {
    "## Plan-Gap Evidence — PHASE 2 GREEN implementation release": "fixed-clt-native-and-bootstrap-green",
    "## Plan-Gap Evidence: PHASE 1 RED authorization": "red-2",
    "## Plan-Gap Evidence — required signer documentation object metadata sync": "signer-doc-corpus-object-sync",
}


def _scope_clause(body: str, task_uid: str) -> tuple[str, str]:
    heading = body.splitlines()[0] if body else ""
    if heading not in {"## Plan-Gap Evidence", *_LEGACY_ENVELOPES}:
        raise ValueError("scope comment formal heading is unsupported")
    if set(re.findall(r"task_[0-9a-f]{32}", body)) != {task_uid}:
        raise ValueError("scope comment Task identity is absent/conflicting")
    matches = list(re.finditer(r"^([a-z][a-z0-9_]*):[ \t]*(.*)$", body, re.M))
    values = {}
    for index, match in enumerate(matches):
        key = match.group(1)
        if key in values:
            raise ValueError("scope comment formal fields are duplicated")
        end = matches[index + 1].start() if index + 1 < len(matches) else len(body)
        values[key] = (match.group(2) + body[match.end():end]).strip()
    if any(not values.get(key) for key in _PLAN_FIELDS):
        raise ValueError("scope comment mandatory Plan-Gap fields are missing")
    if heading == "## Plan-Gap Evidence":
        if not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", values["step_id"]):
            raise ValueError("scope Plan-Gap step ID invalid")
    elif values["step_id"] != _LEGACY_ENVELOPES[heading]:
        raise ValueError("scope legacy heading/step ID differs")
    return heading, values["write_scope"]


def _trusted_corpus(root: Path, base: str):
    source = subprocess.check_output(["git", "-C", str(root), "show", base + ":scripts/document_corpus.py"])
    # Only immutable B0 code is executed; no candidate import fallback.
    module_name = "primary_scope_corpus_" + base
    module = type(sys)(module_name)
    module.__file__ = str(root / "scripts/document_corpus.py") + "@" + base
    sys.modules[module_name] = module
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def _scope_permissions(root: Path, base: str, heading: str, clause: str,
                       task: dict[str, Any], corpus: Any) -> tuple[set[str], list[tuple[str, str]], set[str]]:
    exact, families, derived = set(), [], set()
    if heading == "## Plan-Gap Evidence":
        for item in clause.split(","):
            exact.add(_path(item.strip()))
        return exact, families, derived
    red = re.fullmatch(r"Ops ONLY ([^ ]+) plus unique UID scratch RED report/log\. Runtime architecture/contract report remains scratch-only for now\.", clause)
    if red and heading == "## Plan-Gap Evidence: PHASE 1 RED authorization":
        exact.add(_path(red[1])); return exact, families, derived
    green = re.fullmatch(r"Runtime owns ([^ ]+)/src/([A-Za-z0-9_]+)\* and (src/bin/[A-Za-z0-9_-]+\.rs) plus required Cargo\.toml/lib\.rs hooks/native cfg\(test\) tests\. Ops owns scripts/local-signer/\{([A-Za-z0-9_.,-]+)\}, existing ([A-Za-z0-9_-]+\.py) fixture updates/new focusedtests \(not immutableRED\), two ([A-Za-z0-9_-]+) design/runbook docs\. Agree separate gate packaging/build output with closed candidate manifest preserved unless explicitly documented provenance extension\. No overlap/revert others\.", clause)
    if green and heading == "## Plan-Gap Evidence — PHASE 2 GREEN implementation release":
        crate, family, binary, basenames, fixture, stem = green.groups()
        crate = _path(crate)
        _mode(root, base, crate + "/Cargo.toml")
        exact.update({crate + "/Cargo.toml", crate + "/src/lib.rs", crate + "/" + binary})
        families.append((crate + "/src/", family))
        exact.update(_path("scripts/local-signer/" + item) for item in basenames.split(","))
        exact.add(_path("scripts/local-signer/tests/" + fixture))
        view = corpus.GitCorpusView(root, base)
        pairs = [path for path in view.list_doc_paths() if path.endswith("/" + stem + ".design.md")]
        if len(pairs) != 1:
            raise ValueError("original trusted document pair is ambiguous")
        design = pairs[0]; runbook = design.removesuffix(".design.md") + ".runbook.md"
        for path in (design, runbook):
            if _mode(root, base, path) != "100644" or corpus.expected_object(view, path)["object_kind"] not in {"design", "runbook"}:
                raise ValueError("original trusted document pair is invalid")
        if design not in task.get("source_refs", []):
            raise ValueError("canonical source references substitute the original document pair")
        exact.update((design, runbook)); return exact, families, derived
    sync = re.fullmatch(r"[a-z_]+ may run canonical inventory sync ONLY for ([^ ]+\.design\.md) and \.runbook\.md; resultingtwo objectrecord shards in doc/\.governance/document-corpus/objects/ only plusUIDscratch\. No signer source/docs edits in this correction; no metadataoutsideexacttwoobjects\. Task source scope adds these necessaryderivedobjectrecords, not newpolicy/source-of-truth changes\.", clause)
    if sync and heading == "## Plan-Gap Evidence — required signer documentation object metadata sync":
        design = _path(sync[1]); runbook = design.removesuffix(".design.md") + ".runbook.md"
        if design not in task.get("source_refs", []):
            raise ValueError("scope derived sources differ from canonical references")
        derived.update((design, runbook)); return exact, families, derived
    raise ValueError("scope write_scope complete clause is unsupported")


def validate_scope_proof_shape(proof: Any) -> None:
    if isinstance(proof, dict) and proof.get("schema") == LOOP_PROOF_SCHEMA:
        if (set(proof) != {"schema", "loop_binding", "primary_package", "proof_digest"}
                or not isinstance(proof["loop_binding"], dict)
                or strict_primary_package(proof["primary_package"]) is None
                or proof["proof_digest"] != digest({k: v for k, v in proof.items() if k != "proof_digest"})):
            raise ValueError("completion loop proof fields/digest invalid")
        return
    required = {"schema", "selector", "base_oid", "head_oid", "endpoints", "source_refs", "proof_digest"}
    if not isinstance(proof, dict) or set(proof) != required or proof.get("schema") != PROOF_SCHEMA:
        raise ValueError("completion scope proof fields/schema invalid")
    validate_scope_selector(proof["selector"], proof["selector"].get("task_uid", ""))
    if any(not isinstance(proof[key], str) or not re.fullmatch(r"[0-9a-f]{40}", proof[key])
           for key in ("base_oid", "head_oid")):
        raise ValueError("completion scope proof OIDs invalid")
    seen = set()
    for endpoint in proof["endpoints"] if isinstance(proof["endpoints"], list) else []:
        if not isinstance(endpoint, dict) or set(endpoint) != {"operation", "old_path", "new_path"}:
            raise ValueError("completion scope proof endpoint fields invalid")
        operation = endpoint["operation"]
        if operation not in {"A", "D", "M", "R"}:
            raise ValueError("completion scope proof endpoint operation invalid")
        old, new = endpoint["old_path"], endpoint["new_path"]
        if ((operation == "A" and (old is not None or not isinstance(new, str)))
                or (operation == "D" and (new is not None or not isinstance(old, str)))
                or (operation in {"M", "R"} and (not isinstance(old, str) or not isinstance(new, str)))
                or (operation == "M" and old != new)):
            raise ValueError("completion scope proof endpoint identity invalid")
        for path in (old, new):
            if path is not None:
                _path(path)
        identity = (operation, old, new)
        if identity in seen:
            raise ValueError("completion scope proof endpoint duplicate")
        seen.add(identity)
    if (not isinstance(proof["endpoints"], list) or not proof["endpoints"]
            or not isinstance(proof["source_refs"], list)
            or any(not isinstance(item, str) for item in proof["source_refs"])
            or proof["proof_digest"] != digest({k: v for k, v in proof.items() if k != "proof_digest"})):
        raise ValueError("completion scope proof digest/types invalid")


@contextmanager
def _loop_helpers():
    """Load the authority's exact sibling closure, including under Python -I."""
    names = ("loop_recovery", "loop_gate", "loop", "loop_policy", "loop_contracts", "loop_terminal")
    absent = object()
    previous = {name: sys.modules.get(name, absent) for name in names}
    loaded = {}
    try:
        for name in names:
            sys.modules.pop(name, None)
        for name in ("loop_recovery", "loop_gate", "loop"):
            path = Path(__file__).parent / (name + ".py")
            if path.is_symlink() or not path.is_file() or path.resolve().parent != Path(__file__).parent.resolve():
                raise ValueError("trusted loop sibling missing or unsafe: " + name)
            spec = importlib.util.spec_from_file_location(name, path)
            if spec is None or spec.loader is None:
                raise ValueError("trusted loop sibling loader unavailable: " + name)
            module = importlib.util.module_from_spec(spec)
            sys.modules[name] = module
            spec.loader.exec_module(module)
            loaded[name] = module
        yield loaded
    finally:
        for name, value in previous.items():
            if value is absent:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = value


def loop_scope_proof(root: Path, task: dict[str, Any], base: str | None = None, head: str | None = None,
                     package: str | None = None, *, current_authority: bool = False) -> dict[str, Any]:
    with _loop_helpers() as helpers:
        return _loop_scope_proof(helpers, root, task, base, head, package, current_authority=current_authority)


def _loop_scope_proof(helpers: dict[str, Any], root: Path, task: dict[str, Any], base: str | None,
                      head: str | None, package: str | None, *, current_authority: bool) -> dict[str, Any]:
    admission, live_binding = helpers["loop_gate"].admission, helpers["loop_gate"].live_binding
    if task.get("loop_binding") is None:
        raise ValueError("completion loop binding is absent")
    if current_authority:
        _trusted_module = helpers["loop"]._trusted_module
        for field in ("task_uid", "owner_role", "bootstrap_epoch"):
            if field not in task["loop_binding"] or field not in task or task["loop_binding"][field] != task[field]:
                raise ValueError("completion loop/task identity mismatch: " + field)
        binding = live_binding(task)
        if binding != task["loop_binding"]:
            raise ValueError("completion live loop binding differs")
        tool_root = os.environ.get("OASIS7_LOOP_TOOL_ROOT")
        if not tool_root:
            raise ValueError("completion loop authority requires explicit trusted tool root")
        subprocess.run(["git", "-C", str(root), "fetch", "--no-tags", "origin", "main:refs/remotes/origin/main"],
                       check=True, capture_output=True)
        policy = _trusted_module(Path(tool_root), root, binding, "loop_policy")
        blockers = policy.validate_binding(binding)["blockers"] + policy.validate_tool_root(Path(tool_root), root, binding)["blockers"]
        contracts = _trusted_module(Path(tool_root), root, binding, "loop_contracts")
        blockers += contracts.validate_contracts(Path(tool_root), root, binding, purpose="in_flight")["blockers"]
        if blockers:
            raise ValueError("completion current loop authority rejected: " + "; ".join(blockers))
    elif base is None or head is None or admission(root, task, _oid(root, base), _oid(root, head)).get("status") != "passed":
        raise ValueError("completion requires independently passed canonical loop admission")
    proof = {"schema": LOOP_PROOF_SCHEMA, "loop_binding": task["loop_binding"], "primary_package": strict_primary_package(package)}
    proof["proof_digest"] = digest(proof)
    return proof


def existing_scope_proof(root: Path, task: dict[str, Any], selector: Any, base: str, head: str,
                         *, issue_body: str | None = None) -> dict[str, Any]:
    selector = validate_scope_selector(selector, task["task_uid"])
    freeze = _comment(task, selector["freeze"])
    original_base, original_head = _freeze(freeze, task)
    base, head = _oid(root, base), _oid(root, head)
    for old, new in ((original_base, base), (original_head, head)):
        _git(root, "merge-base", "--is-ancestor", old, new)
    for left, right in ((original_base, original_head), (base, head)):
        if len(_git(root, "merge-base", "--all", left, right).splitlines()) != 1:
            raise ValueError("scope comparison requires one unique merge base")
    if issue_body is None:
        issue = _github(task["repository"], f"repos/{task['repository']}/issues/{task['issue_number']}")
        if issue.get("number") != task["issue_number"] or issue.get("html_url") != task["issue_url"]:
            raise ValueError("scope live canonical Issue identity differs")
        issue_body = issue.get("body", "")
    if (not isinstance(issue_body, str)
            or re.findall(r"^task_uid: (task_[0-9a-f]{32})$", issue_body, re.M) != [task["task_uid"]]
            or re.findall(r"^- worktree_hint: `([^`]+)`$", issue_body, re.M) != [task["canonical_worktree"]]):
        raise ValueError("scope current live Task UID/worktree identity differs")
    refs = re.findall(r"^Source refs:\n((?:- `[^`]+`\n)+)", issue_body, re.M)
    live_refs = re.findall(r"^- `([^`]+)`$", refs[0], re.M) if len(refs) == 1 else []
    if live_refs != task.get("source_refs", []):
        raise ValueError("scope current live/cache source references differ")
    corpus = _trusted_corpus(root, original_base)
    exact, families, derived, exclusions = set(), [], set(), set()
    for locator in selector["authorizations"]:
        comment = _comment(task, locator)
        if datetime.fromisoformat(comment["created_at"].replace("Z", "+00:00")) > datetime.fromisoformat(freeze["created_at"].replace("Z", "+00:00")):
            raise ValueError("scope authorization postdates original freeze")
        heading, clause = _scope_clause(comment["body"], task["task_uid"])
        exclusion = re.search(r"^out_of_scope:[ \t]*(.*?)(?=^[a-z][a-z0-9_]*:|\Z)", comment["body"], re.M | re.S).group(1).strip()
        legacy_exclusions = {
            "## Plan-Gap Evidence — PHASE 2 GREEN implementation release": "host install/accounts/jobs/mounts/keys/signing, workflow/helper/CI changes, credentials, third_party edits, unrelated runtime.",
            "## Plan-Gap Evidence: PHASE 1 RED authorization": "all production/docs/Cargo changes during RED, all host/root/credential/key/signing changes, mount/jobs/workflow changes, old test weakening.",
            "## Plan-Gap Evidence — required signer documentation object metadata sync": "semanticentrycreation/overrides/reviewexpiry/helper/policy/CI changes; unrelatedcorpusobjects; inventorybulkregeneration; cache/journal/credentials/hosteffects/rootinstall/keys/signing.",
        }
        if heading in legacy_exclusions:
            if exclusion != legacy_exclusions[heading]:
                raise ValueError("scope legacy exclusion clause is unsupported")
        elif exclusion != "all other paths":
            exclusions.update(_path(path.strip()) for path in exclusion.split(","))
        paths, modules, sources = _scope_permissions(root, original_base, heading, clause, task, corpus)
        exact.update(paths); families.extend(modules); derived.update(sources)
    original = _endpoints(root, original_base, original_head)
    current = _endpoints(root, base, head)
    if not original or any(item not in original for item in current):
        raise ValueError("current scope endpoint expands the original frozen endpoint set")
    for endpoint in original:
        for revision, key in ((original_base, "old_path"), (original_head, "new_path")):
            path = endpoint[key]
            if path is None:
                continue
            if path in exclusions:
                raise ValueError("original frozen endpoint is explicitly out_of_scope: " + path)
            _mode(root, revision, path)
            admitted = path in exact or any(path == prefix + name + ".rs" or
                (path.startswith(prefix + name + "/") and path.endswith(".rs")) for prefix, name in families)
            if path.startswith("doc/.governance/document-corpus/objects/"):
                view = corpus.GitCorpusView(root, revision)
                record = corpus._load_wrapped(view, path, "object", corpus.OBJECT_FIELDS, set())
                source = record.get("path")
                if source not in derived or corpus.record_path("object", source) != path:
                    raise ValueError("scope object source/key outside explicit sync authority")
                corpus.ensure_hash_input(view, source)
                if record != corpus.expected_object(view, source):
                    raise ValueError("scope object complete derivation differs")
                admitted = source in exact
            if not admitted:
                raise ValueError("original frozen endpoint outside formal write_scope: " + path)
    # The current endpoints must preserve regular blobs and correct derivation too.
    for endpoint in current:
        for revision, key in ((base, "old_path"), (head, "new_path")):
            path = endpoint[key]
            if path is not None:
                _mode(root, revision, path)
                if path.startswith("doc/.governance/document-corpus/objects/"):
                    view = corpus.GitCorpusView(root, revision)
                    record = corpus._load_wrapped(view, path, "object", corpus.OBJECT_FIELDS, set())
                    source = record.get("path")
                    corpus.ensure_hash_input(view, source)
                    if source not in derived or corpus.record_path("object", source) != path or record != corpus.expected_object(view, source):
                        raise ValueError("current scope object source/key/derivation differs")
    proof = {"schema": PROOF_SCHEMA, "selector": selector, "base_oid": original_base, "head_oid": original_head,
             "endpoints": original, "source_refs": list(task.get("source_refs", []))}
    proof["proof_digest"] = digest(proof)
    return proof
