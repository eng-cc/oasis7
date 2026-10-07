"""Exact native readiness transport and the read-only v2 delivery barrier.

This records the human gate's accepted bytes. It never authorizes a merge or
replays mutable historical policy. Legacy terminal v1 does not call this module.
"""
from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

NATIVE = "<!-- oasis7-native-readiness/v2 -->"
MIGRATION = "<!-- oasis7-pr-readiness-migration/v1 -->"
OID = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
SHA = re.compile(r"[0-9a-f]{64}\Z")
SOURCE_FIELDS = {"oid", "tree_oid", "gate_entry_sha256", "claim_entry_sha256"}
COMMENT_FIELDS = {"id", "body_b64", "body_sha256", "author", "created_at", "updated_at"}
GATE_FIELDS = {"stdout_b64", "raw_sha256", "issuer", "observed_at", "gate_epoch", "epoch_version", "epoch_preimage"}
RESULT_FIELDS = {"stdout_b64", "raw_sha256", "claim_type", "status", "exit_code", "verified_at"}
BINDING_FIELDS = {"schema", "task_uid", "repository", "issue_number", "pr_number", "pr_url", "head_oid", "claim_type", "status", "exit_code", "verified_at", "gate_raw_sha256", "gate_epoch", "gate_observed_at", "source"}
PROOF_FIELDS = {"schema", "mode", "task_uid", "repository", "issue_number", "pr_number", "pr_url", "head_oid", "pre_merge_target_oid", "pr_base_oid", "merge_commit_oid", "merged_at", "source", "claim_input_gate", "claim_result", "claim_comment", "migration_comment"}
MIGRATION_FIELDS = {"schema", "task_uid", "repository", "issue_number", "pr_number", "pr_url", "head_oid", "default_branch", "pre_merge_target_oid", "pr_base_oid", "producer_generation", "source", "claim_input_gate", "claim_result", "claim_comment"}
CLAIM_FIELDS = {"claim_type", "verify_command", "verified_at", "verification_exit_code", "status", "allowed_to_claim", "claim_message", "blocked_phrase", "success_phrase", "task_uid", "repository_fingerprint_before", "repository_fingerprint_after", "verification_epoch_stable", "verification_mode", "frozen_source_head", "frozen_source_tree", "comparison_ref", "verification_profile", "repository_head", "repository_index_sha256"}


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _pairs(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise ValueError("readiness duplicate JSON key")
        obj[key] = value
    return obj


def load(raw: bytes) -> dict:
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_pairs,
                           parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))
    except (UnicodeError, ValueError) as exc:
        raise ValueError("readiness malformed unique-key UTF-8 JSON") from exc
    if not isinstance(value, dict):
        raise ValueError("readiness JSON must be an object")
    return value


def closed(value: object, keys: set, label: str) -> dict:
    if not isinstance(value, dict) or set(value) != keys:
        raise ValueError(f"readiness {label} closed schema mismatch")
    return value


def instant(value: object) -> dt.datetime:
    if not isinstance(value, str) or not re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?(?:Z|[+-]\d\d:\d\d)", value):
        raise ValueError("readiness timestamp requires exact RFC3339 offset")
    try:
        return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("readiness timestamp is invalid") from exc


def _id(value: object) -> bool:
    return type(value) is int and value > 0


def _b64(raw: bytes) -> str:
    return base64.b64encode(raw).decode("ascii")


def decode(value: object) -> bytes:
    if not isinstance(value, str):
        raise ValueError("readiness base64 is not a string")
    try:
        raw = base64.b64decode(value, validate=True)
    except ValueError as exc:
        raise ValueError("readiness base64 is invalid") from exc
    if _b64(raw) != value:
        raise ValueError("readiness base64 is noncanonical")
    return raw


def git(root: Path, *args: str) -> bytes:
    observation=sys.modules.get('recovery_observation')
    if observation is not None and observation.active() is not None:
        return observation.git(root,*args)
    try:
        return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.PIPE)
    except (OSError, subprocess.SubprocessError) as exc:
        raise ValueError("readiness source Git objects unavailable") from exc


def _entry(root: Path, oid: str, name: str) -> bytes:
    path = "scripts/pm/" + name
    row = git(root, "ls-tree", oid, "--", path).decode().strip()
    if not re.fullmatch(r"100(?:644|755) blob [0-9a-f]{40,64}\t" + re.escape(path), row):
        raise ValueError("readiness source entry is not a regular Git file")
    return git(root, "show", f"{oid}:{path}")


def validate_source(root: Path, source: dict) -> tuple[bytes, bytes]:
    closed(source, SOURCE_FIELDS, "source")
    if any(not isinstance(source[k], str) or not OID.fullmatch(source[k]) for k in ("oid", "tree_oid")):
        raise ValueError("readiness source Git identity invalid")
    if git(root, "rev-parse", source["oid"] + "^{tree}").decode().strip() != source["tree_oid"]:
        raise ValueError("readiness source tree mismatch")
    entries = tuple(_entry(root, source["oid"], name) for name in ("pr-lifecycle-gate.py", "claim-ready.sh"))
    for key, raw in zip(("gate_entry_sha256", "claim_entry_sha256"), entries):
        if source[key] != digest(raw):
            raise ValueError("readiness source entry digest mismatch")
    return entries


def source_identity(tool_root: Path) -> dict:
    """Pin the already selected executed tool root, not a caller-selected ref."""
    source = {"oid": git(tool_root, "rev-parse", "HEAD").decode().strip(),
              "tree_oid": git(tool_root, "rev-parse", "HEAD^{tree}").decode().strip()}
    for name, key in (("pr-lifecycle-gate.py", "gate_entry_sha256"), ("claim-ready.sh", "claim_entry_sha256")):
        raw = _entry(tool_root, source["oid"], name)
        if (tool_root / "scripts/pm" / name).read_bytes() != raw:
            raise ValueError("readiness executed tool differs from selected immutable Git source")
        source[key] = digest(raw)
    return source


def gate_capture(raw: bytes, source: dict, repository: str, pr_number: int,
                 head_oid: str, verified_at: str, *, root: Path | None = None) -> dict:
    gate = load(raw)
    if (gate.get("evidence_mode") not in (None, "production") or gate.get("requires_live_gate") is True
            or gate.get("candidate_ready") is True or gate.get("ready_for_merge") is not True
            or gate.get("status") != "ready" or gate.get("blockers") != []):
        raise ValueError("readiness gate is not production ready")
    receipt = gate.get("readiness_receipt")
    if not isinstance(receipt, dict):
        raise ValueError("readiness gate receipt unavailable")
    if (receipt.get("receipt_type"), receipt.get("issuer"), receipt.get("repository"),
            receipt.get("pr_number"), receipt.get("head_oid")) != (
            "oasis7_pr_lifecycle_ready", "oasis7_pr_lifecycle_gate/v1", repository, pr_number, head_oid):
        raise ValueError("readiness gate identity mismatch")
    if type(receipt.get("pr_number")) is not int or type(gate.get("pr_number")) is not int or gate.get("pr_number") != pr_number:
        raise ValueError("readiness gate PR type mismatch")
    age = (instant(verified_at) - instant(receipt.get("observed_at"))).total_seconds()
    if not -30 <= age <= 600:
        raise ValueError("readiness gate was stale at native claim")
    preimage = {"repository": repository, "pr_number": pr_number, "head_oid": head_oid,
                "blockers": [], "policy": gate.get("policy_discovery"), "hold": gate.get("merge_hold")}
    # Legacy serializers use ensure_ascii=True and preserve nested sha256: text.
    if "integration_ci" in receipt:
        if root is not None and b"epoch_input['integration_ci']" not in validate_source(root, source)[0]:
            raise ValueError("readiness source does not support integration epoch")
        integration = receipt["integration_ci"]
        preimage["integration_ci"] = integration
        if isinstance(integration, dict) and integration.get("keyed_target_applicability_epoch") is not None:
            preimage["keyed_target_applicability_epoch"] = integration["keyed_target_applicability_epoch"]
    epoch = digest(json.dumps(preimage, sort_keys=True, separators=(",", ":")).encode())
    if receipt.get("gate_epoch") != epoch:
        raise ValueError("readiness source-version gate epoch mismatch")
    return {"stdout_b64": _b64(raw), "raw_sha256": digest(raw), "issuer": receipt["issuer"],
            "observed_at": receipt["observed_at"], "gate_epoch": epoch,
            "epoch_version": "legacy_v1", "epoch_preimage": preimage}


def query(repository: str, endpoint: str):
    observation=sys.modules.get('recovery_observation')
    if observation is not None and observation.active() is not None:
        return observation.api(endpoint)
    try:
        raw = subprocess.check_output(["gh", "api", endpoint], stderr=subprocess.PIPE, timeout=180)
        return json.loads(raw, object_pairs_hook=_pairs)
    except (OSError, subprocess.SubprocessError, ValueError) as exc:
        raise ValueError("readiness authenticated server readback unavailable") from exc


def permission(repository: str, author: str, *, admin: bool = False) -> None:
    if not isinstance(author, str) or not re.fullmatch(r"[A-Za-z0-9-]+", author):
        raise ValueError("readiness publisher login invalid")
    response = query(repository, f"repos/{repository}/collaborators/{author}/permission")
    allowed = {"admin"} if admin else {"admin", "maintain", "write"}
    if not isinstance(response, dict) or response.get("permission") not in allowed:
        raise ValueError("readiness publisher current permission insufficient")


def read_comments(repository: str, issue: int) -> list:
    try:
        raw = subprocess.check_output(["gh", "api", "--paginate", "--slurp",
                f"repos/{repository}/issues/{issue}/comments"], stderr=subprocess.PIPE, timeout=180)
        pages = json.loads(raw, object_pairs_hook=_pairs)
    except (OSError, subprocess.SubprocessError, ValueError) as exc:
        raise ValueError("readiness complete comment discovery unavailable") from exc
    if not isinstance(pages, list) or any(not isinstance(page, list) for page in pages):
        raise ValueError("readiness comment pagination malformed")
    return [comment for page in pages for comment in page]


def comment_capture(comment: dict) -> dict:
    body = comment.get("body")
    author = (comment.get("user") or {}).get("login")
    if not _id(comment.get("id")) or not isinstance(body, str) or not isinstance(author, str) or not author:
        raise ValueError("readiness server comment malformed")
    created, updated = comment.get("created_at"), comment.get("updated_at")
    instant(created); instant(updated)
    if any(not value.endswith(("Z", "+00:00")) for value in (created, updated)):
        raise ValueError("readiness server comment timestamps require UTC")
    if created != updated:
        raise ValueError("readiness required comment was edited")
    return {"id": comment["id"], "body_b64": _b64(body.encode()), "body_sha256": digest(body.encode()),
            "author": author, "created_at": created, "updated_at": updated}


def validate_comment(repository: str, issue: int, capture: dict, comments: list,
                     body: bytes, marker: str, *, admin: bool = False,
                     check_permission: bool = False) -> dict:
    closed(capture, COMMENT_FIELDS, "comment")
    if decode(capture["body_b64"]) != body or capture["body_sha256"] != digest(body):
        raise ValueError("readiness comment raw bytes mismatch")
    marked = [c for c in comments if marker in str(c.get("body") or "")]
    if marker == "<!-- oasis7-pm-claim-verification -->":
        marked = [c for c in marked if "\nClaim Type: ready_for_merge\n" in str(c.get("body") or "")]
    exact = [c for c in marked if c.get("body") == body.decode() and c.get("id") == capture["id"]]
    # Legacy and native claims are repeatable; uniqueness belongs to this body.
    # Migration keeps its separate Task-global create-once marker contract.
    if len(exact) != 1 or (marker == MIGRATION and len(marked) != 1):
        raise ValueError("readiness native comment is missing, ambiguous or inconsistent")
    if len([c for c in marked if c.get("body") == body.decode()]) != 1:
        raise ValueError("readiness exact native binding publication is duplicated")
    remote = query(repository, f"repos/{repository}/issues/comments/{capture['id']}")
    if not isinstance(remote, dict) or comment_capture(remote) != capture or comment_capture(exact[0]) != capture:
        raise ValueError("readiness comment ID readback differs")
    if remote.get("html_url") != f"https://github.com/{repository}/issues/{issue}#issuecomment-{capture['id']}":
        raise ValueError("readiness comment canonical Issue URL mismatch")
    if check_permission:
        permission(repository, capture["author"], admin=admin)
    return remote


def receipt_root(root: Path, uid: str) -> Path:
    if not re.fullmatch(r"task_[0-9a-f]{32}", uid):
        raise ValueError("readiness task UID invalid")
    command=[sys.executable, str(Path(__file__).with_name("canonical-receipt-root.py")),
            "--default-worktree", str(root), "--task-uid", uid, "--json"]
    observation=sys.modules.get('recovery_observation')
    if observation is not None and observation.active() is not None:
        raw=observation.capture(command)
    else:
        raw = subprocess.check_output(command, stderr=subprocess.PIPE)
    return Path(json.loads(raw)["receipt_root"])


def native_artifact_root(root: Path, uid: str, binding: dict) -> Path:
    closed(binding, BINDING_FIELDS, "native binding")
    return receipt_root(root, uid) / "native-readiness" / digest(canonical(binding))


def _binding_shape(binding: dict) -> None:
    closed(binding, BINDING_FIELDS, "native binding")
    if (binding["schema"] != "oasis7-native-readiness/v2" or binding["claim_type"] != "ready_for_merge"
            or binding["status"] != "verified" or type(binding["exit_code"]) is not int or binding["exit_code"] != 0
            or not _id(binding["issue_number"]) or not _id(binding["pr_number"])
            or not isinstance(binding["task_uid"], str) or not re.fullmatch(r"task_[0-9a-f]{32}", binding["task_uid"])
            or not isinstance(binding["repository"], str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", binding["repository"])
            or binding["pr_url"] != f"https://github.com/{binding['repository']}/pull/{binding['pr_number']}"
            or not isinstance(binding["head_oid"], str) or not OID.fullmatch(binding["head_oid"])):
        raise ValueError("readiness native binding field types/values mismatch")
    for key in ("gate_raw_sha256", "gate_epoch"):
        if not isinstance(binding[key], str) or not SHA.fullmatch(binding[key]):
            raise ValueError("readiness native binding digest invalid")
    instant(binding["verified_at"]); instant(binding["gate_observed_at"])
    source = closed(binding["source"], SOURCE_FIELDS, "source")
    for key in ("oid", "tree_oid"):
        if not isinstance(source[key], str) or not OID.fullmatch(source[key]):
            raise ValueError("readiness native binding source OID invalid")
    for key in ("gate_entry_sha256", "claim_entry_sha256"):
        if not isinstance(source[key], str) or not SHA.fullmatch(source[key]):
            raise ValueError("readiness native binding source digest invalid")


def select_native_comment(comments: list[dict], identity: dict,
                          merged_at: str) -> tuple[dict, dict]:
    """Select by immutable server time/ID, never local paths or fallback.

    Selection intentionally precedes validation of the selected artifacts.
    An edited or unreadable latest relevant binding cannot expose older proof.
    """
    candidates = []
    prefix = NATIVE + "\n"
    for comment in comments:
        body = comment.get("body")
        if NATIVE not in str(body or ""):
            continue
        if not isinstance(body, str) or not body.startswith(prefix):
            raise ValueError("readiness native marker body malformed")
        binding = load(body[len(prefix):].encode())
        # A reciprocal Task Issue can contain identifiable unrelated evidence;
        # it cannot make a caller-selected binding authoritative for this Task.
        if binding.get("task_uid") not in (None, identity["task_uid"]):
            continue
        _binding_shape(binding)
        if (binding["schema"] != "oasis7-native-readiness/v2"
                or body.encode() != prefix.encode() + canonical(binding)):
            raise ValueError("readiness native binding version/body mismatch")
        for key in ("task_uid", "repository", "issue_number", "pr_number", "pr_url"):
            if canonical(binding[key]) != canonical(identity[key]):
                raise ValueError("readiness native reciprocal binding identity mismatch")
        if binding["head_oid"] != identity["head_oid"]:
            continue
        if not _id(comment.get("id")):
            raise ValueError("readiness native selection server comment ID invalid")
        created = instant(comment.get("created_at"))
        if created < instant(merged_at):
            candidates.append((created, comment["id"], comment, binding))
    if not candidates:
        raise ValueError("readiness latest reciprocal premerge native binding unavailable")
    _created, _id_value, comment, binding = max(candidates, key=lambda item: item[:2])
    return comment, binding


def _store_once(path: Path, raw: bytes) -> None:
    if path.exists():
        if path.read_bytes() != raw:
            raise ValueError("readiness immutable artifact conflicts: " + path.name)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as out:
            out.write(raw); out.flush(); os.fsync(out.fileno())
    except FileExistsError:
        if path.read_bytes() != raw:
            raise ValueError("readiness immutable artifact concurrent conflict")


def _mapping(root: Path, uid: str) -> dict:
    data = load((root / ".pm/github-project-sync/tasks.json").read_bytes())
    record = (data.get("tasks") or {}).get(uid)
    if not isinstance(record, dict):
        raise ValueError("readiness unique Task mapping unavailable")
    return record


def _identity(uid: str, record: dict, issue: dict, pr: dict) -> dict:
    repo, num, number = record.get("repository"), record.get("issue_number"), record.get("pr_number")
    if (not isinstance(repo, str) or not re.fullmatch(r"[^/\s]+/[^/\s]+", repo)
            or not _id(num) or not _id(number) or record.get("task_uid", uid) != uid
            or record.get("pr_url") != f"https://github.com/{repo}/pull/{number}"):
        raise ValueError("readiness canonical mapping identity mismatch")
    body = issue.get("body", "")
    if (issue.get("number") != num or issue.get("html_url") != f"https://github.com/{repo}/issues/{num}"
            or re.findall(r"^task_uid:\s*([^\n]+)$", body, re.M) != [uid]):
        raise ValueError("readiness reciprocal Issue identity mismatch")
    for key, value in (("pr_number", str(number)), ("pr_url", record["pr_url"])):
        if re.findall(r"^- " + key + r": `([^`]+)`$", body, re.M) != [value]:
            raise ValueError("readiness reciprocal Issue PR mismatch")
    if (pr.get("number") != number or pr.get("html_url") != record["pr_url"]
            or re.findall(r"^Task: ([^\n]+)$", pr.get("body", ""), re.M) != [uid]
            or re.findall(r"^Refs #([0-9]+)$", pr.get("body", ""), re.M) != [str(num)]):
        raise ValueError("readiness reciprocal PR identity mismatch")
    for side in ("base", "head"):
        full = ((pr.get(side) or {}).get("repo") or {}).get("full_name")
        if full is not None and full != repo:
            raise ValueError("readiness reciprocal PR repository mismatch")
    head = (pr.get("head") or {}).get("sha")
    if not isinstance(head, str) or not OID.fullmatch(head):
        raise ValueError("readiness accepted head invalid")
    return {"task_uid": uid, "repository": repo, "issue_number": num, "pr_number": number,
            "pr_url": record["pr_url"], "head_oid": head}


def publish_native(root: Path, tool_root: Path, uid: str, gate_raw: bytes, result: dict) -> dict:
    record = _mapping(root, uid)
    repo = record.get("repository")
    issue = query(repo, f"repos/{repo}/issues/{record.get('issue_number')}")
    pr = query(repo, f"repos/{repo}/pulls/{record.get('pr_number')}")
    identity = _identity(uid, record, issue, pr)
    initial_raw = json.dumps(result, ensure_ascii=False).encode()
    _result(result_capture(initial_raw), native=False, uid=uid)
    if result.get("repository_head") != identity["head_oid"]:
        raise ValueError("readiness claim result does not name accepted H")
    if str(pr.get("state")).upper() != "OPEN" or pr.get("merged") is True:
        raise ValueError("readiness native publication requires open PR")
    source = source_identity(tool_root)
    gate = gate_capture(gate_raw, source, repo, identity["pr_number"], identity["head_oid"], result["verified_at"], root=tool_root)
    binding = {"schema": "oasis7-native-readiness/v2", **identity, "claim_type": "ready_for_merge",
               "status": "verified", "exit_code": 0, "verified_at": result["verified_at"],
               "gate_raw_sha256": gate["raw_sha256"], "gate_epoch": gate["gate_epoch"],
               "gate_observed_at": gate["observed_at"], "source": source}
    body = NATIVE.encode() + b"\n" + canonical(binding)
    author = query(repo, "user").get("login")
    permission(repo, author)
    existing = [c for c in read_comments(repo, identity["issue_number"]) if c.get("body") == body.decode()]
    if len(existing) > 1:
        raise ValueError("readiness exact native binding publication is duplicated")
    if not existing:
        with tempfile.NamedTemporaryFile("wb", delete=False) as handle:
            handle.write(body); path = Path(handle.name)
        try:
            subprocess.run(["gh", "issue", "comment", str(identity["issue_number"]), "-R", repo,
                            "--body-file", str(path)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        finally:
            path.unlink(missing_ok=True)
    comments = read_comments(repo, identity["issue_number"])
    matches = [c for c in comments if c.get("body") == body.decode()]
    if len(matches) != 1:
        raise ValueError("readiness native publication has no unique server readback")
    capture = comment_capture(matches[0])
    if capture["author"] != author or instant(capture["created_at"]) < instant(result["verified_at"]):
        raise ValueError("readiness authenticated publisher/time mismatch")
    validate_comment(repo, identity["issue_number"], capture, comments, body, NATIVE, check_permission=True)
    result = {**result, "readiness_binding": binding, "readiness_comment": capture}
    raw = json.dumps(result, ensure_ascii=False).encode() + b"\n"
    dest = native_artifact_root(root, uid, binding)
    _store_once(dest / "gate.stdout", gate_raw)
    _store_once(dest / "result.stdout", raw)
    _store_once(dest / "comment.json", canonical(capture))
    return result


def result_capture(raw: bytes) -> dict:
    result = load(raw)
    return {"stdout_b64": _b64(raw), "raw_sha256": digest(raw), "claim_type": result.get("claim_type"),
            "status": result.get("status"), "exit_code": result.get("verification_exit_code"),
            "verified_at": result.get("verified_at")}


def capture_legacy(root: Path, tool_root: Path, uid: str, input_path: Path) -> dict:
    """Admin transports a legacy human claim while the reciprocal PR is open.

    The selected tool checkout must be the actually read current default T;
    caller-chosen old ancestors cannot select the first-migration generation.
    The legacy producer alone evaluates readiness; this helper only captures.
    """
    record = _mapping(root, uid)
    repo = record.get("repository")
    issue = query(repo, f"repos/{repo}/issues/{record.get('issue_number')}")
    pr = query(repo, f"repos/{repo}/pulls/{record.get('pr_number')}")
    identity = _identity(uid, record, issue, pr)
    if str(pr.get("state")).upper() != "OPEN" or pr.get("merged") is True:
        raise ValueError("readiness legacy capture requires open PR")
    repository = query(repo, f"repos/{repo}")
    branch = repository.get("default_branch")
    if not isinstance(branch, str) or branch != (pr.get("base") or {}).get("ref"):
        raise ValueError("readiness capture default branch mismatch")
    import urllib.parse
    target = query(repo, f"repos/{repo}/git/ref/heads/{urllib.parse.quote(branch, safe='')}").get("object", {}).get("sha")
    source = source_identity(tool_root)
    if (source["oid"] != target or git(tool_root, "symbolic-ref", "--short", "HEAD").decode().strip() != branch):
        raise ValueError("readiness capture tools are not the actually current trusted default T")
    if b"oasis7-native-readiness/v2" in validate_source(tool_root, source)[1]:
        raise ValueError("readiness migration cannot capture a native-v2 producer")
    base_oid = (pr.get("base") or {}).get("sha")
    if not isinstance(base_oid, str) or not OID.fullmatch(base_oid):
        raise ValueError("readiness capture immutable PR base unavailable")
    if b"oasis7-native-readiness/v2" in _entry(root, base_oid, "claim-ready.sh"):
        raise ValueError("readiness first capture PR base already supports native v2")
    author = query(repo, "user").get("login")
    permission(repo, author, admin=True)
    initial_comments = read_comments(repo, identity["issue_number"])
    if any(MIGRATION in str(c.get("body") or "") for c in initial_comments):
        raise ValueError("readiness migration transport already exists")
    raw = input_path.read_bytes()
    # Legacy readiness gate itself remains unchanged and reruns its normal
    # live checks. Read the same file immediately before and after invocation.
    if digest(input_path.read_bytes()) != digest(raw):
        raise ValueError("readiness legacy input changed before claim")
    env = dict(os.environ, PM_ROOT_DIR=str(root))
    process = subprocess.run(["bash", str(tool_root / "scripts/pm/claim-ready.sh"),
        "--claim-type", "ready_for_merge", "--verification-profile", "repository_required",
        "--task-uid", uid, "--pr-gate-json", str(input_path), "--json"],
        cwd=tool_root, env=env, capture_output=True)
    if digest(input_path.read_bytes()) != digest(raw):
        raise ValueError("readiness legacy input changed during claim")
    if process.returncode:
        raise ValueError("readiness legacy producer did not report success")
    captured_result = result_capture(process.stdout)
    result = _result(captured_result, native=False, uid=uid)
    gate = gate_capture(raw, source, repo, identity["pr_number"], identity["head_oid"], result["verified_at"], root=tool_root)
    comments = read_comments(repo, identity["issue_number"])
    body = _legacy_body(uid, result)
    matches = [c for c in comments if c.get("body") == body.decode()]
    if len(matches) != 1:
        raise ValueError("readiness legacy successful claim has no unique exact comment")
    claim_comment = comment_capture(matches[0])
    validate_comment(repo, identity["issue_number"], claim_comment, comments, body,
                     "<!-- oasis7-pm-claim-verification -->", check_permission=True)
    if instant(result["verified_at"]) > instant(claim_comment["created_at"]):
        raise ValueError("readiness legacy native claim publication order invalid")
    # Re-read all current identities and selected source immediately before
    # transport publication; a capture cannot promote drifted readiness.
    if source_identity(tool_root) != source:
        raise ValueError("readiness selected trusted source changed during capture")
    current_pr = query(repo, f"repos/{repo}/pulls/{identity['pr_number']}")
    if (current_pr != pr or str(current_pr.get("state")).upper() != "OPEN"):
        raise ValueError("readiness PR changed during legacy capture")
    if query(repo, f"repos/{repo}/git/ref/heads/{urllib.parse.quote(branch, safe='')}").get("object", {}).get("sha") != target:
        raise ValueError("readiness trusted target changed during legacy capture")
    payload = {"schema": "oasis7-pr-readiness-migration/v1", **identity,
        "default_branch": branch, "pre_merge_target_oid": target, "pr_base_oid": base_oid,
        "producer_generation": "legacy_v1", "source": source, "claim_input_gate": gate,
        "claim_result": captured_result, "claim_comment": claim_comment}
    body = MIGRATION.encode() + b"\n" + canonical(payload)
    permission(repo, author, admin=True)
    with tempfile.NamedTemporaryFile("wb", delete=False) as handle:
        handle.write(body); path = Path(handle.name)
    try:
        subprocess.run(["gh", "issue", "comment", str(identity["issue_number"]), "-R", repo,
            "--body-file", str(path)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    finally:
        path.unlink(missing_ok=True)
    comments = read_comments(repo, identity["issue_number"])
    matches = [c for c in comments if c.get("body") == body.decode()]
    if len(matches) != 1:
        raise ValueError("readiness migration publication has no unique readback")
    migration = comment_capture(matches[0])
    if migration["author"] != author or instant(migration["created_at"]) < instant(claim_comment["created_at"]):
        raise ValueError("readiness migration publisher/time mismatch")
    validate_comment(repo, identity["issue_number"], migration, comments, body, MIGRATION, admin=True, check_permission=True)
    return {"payload": payload, "migration_comment": migration}


def _result(capture: dict, *, native: bool, uid: str) -> dict:
    closed(capture, RESULT_FIELDS, "claim result")
    raw = decode(capture["stdout_b64"])
    result = load(raw)
    closed(result, CLAIM_FIELDS | ({"readiness_binding", "readiness_comment"} if native else set()), "native stdout")
    if capture != result_capture(raw) or capture["raw_sha256"] != digest(raw):
        raise ValueError("readiness claim stdout binding mismatch")
    if (capture["claim_type"], capture["status"], capture["exit_code"], result.get("allowed_to_claim"), result.get("task_uid")) != (
            "ready_for_merge", "verified", 0, True, uid) or type(capture["exit_code"]) is not int:
        raise ValueError("readiness claim is not successful native readiness")
    if (result.get("allowed_to_claim") is not True or type(result.get("verification_exit_code")) is not int
            or result.get("verification_epoch_stable") is not True):
        raise ValueError("readiness successful claim field types mismatch")
    before, after = result.get("repository_fingerprint_before"), result.get("repository_fingerprint_after")
    if not isinstance(before, str) or not SHA.fullmatch(before) or before != after:
        raise ValueError("readiness claim repository epoch was not stable")
    instant(capture["verified_at"])
    return result


def _gate(capture: dict, source: dict, identity: dict, verified: str, root: Path) -> None:
    closed(capture, GATE_FIELDS, "gate")
    expected = gate_capture(decode(capture["stdout_b64"]), source, identity["repository"],
                identity["pr_number"], identity["head_oid"], verified, root=root)
    if canonical(capture) != canonical(expected):
        raise ValueError("readiness exact gate capture mismatch")


def _legacy_body(uid: str, result: dict) -> bytes:
    return "\n".join(["<!-- oasis7-pm-claim-verification -->", f"Task UID: {uid}",
        f"Claim Type: {result['claim_type']}", f"Verified At: {result['verified_at']}",
        f"Verification Exit Code: {result['verification_exit_code']}", f"Verification Status: {result['status']}",
        f"Verify Command: {result['verify_command']}", f"Claim Message: {result['claim_message']}", ""]).encode()


def validate_readiness_proof(root: Path, uid: str, record: dict, *, live_pr: dict,
                             comments: list[dict], live_issue: dict | None = None,
                             proof: dict | None = None) -> dict:
    """Read only; no file, Task, Project, comment, ledger or tombstone writes."""
    dest = receipt_root(root, uid)
    if proof is None:
        try:
            raw = (dest / "readiness-proof.json").read_bytes()
        except OSError as exc:
            raise ValueError("required readiness-proof.json unavailable") from exc
        proof = load(raw)
    else:
        raw = canonical(proof)
    closed(proof, PROOF_FIELDS, "delivery proof")
    repo = record.get("repository")
    issue = live_issue or query(repo, f"repos/{repo}/issues/{record.get('issue_number')}")
    identity = _identity(uid, record, issue, live_pr)
    if canonical({k: proof.get(k) for k in identity}) != canonical(identity) or proof["schema"] != "oasis7-delivery-readiness/v2":
        raise ValueError("readiness proof accepted Task/PR/head mismatch")
    if (live_pr.get("merged") is not True or str(live_pr.get("state")).upper() != "CLOSED"
            or proof["merge_commit_oid"] != live_pr.get("merge_commit_sha")
            or proof["merged_at"] != live_pr.get("merged_at")):
        raise ValueError("readiness proof server merge M/time mismatch")
    for key in ("pre_merge_target_oid", "pr_base_oid", "merge_commit_oid"):
        if not isinstance(proof[key], str) or not OID.fullmatch(proof[key]):
            raise ValueError("readiness proof OID invalid")
    base_oid = (live_pr.get("base") or {}).get("sha")
    if base_oid is None or proof["pr_base_oid"] != base_oid:
        raise ValueError("readiness immutable PR base mismatch")
    if proof["pre_merge_target_oid"] != (proof.get("source") or {}).get("oid"):
        raise ValueError("readiness premerge selected tool target mismatch")
    from terminal_proof import read_live_repository, validate_live_repository
    live_repository = read_live_repository(repo, proof["merge_commit_oid"])
    validate_live_repository(live_repository, repo, proof["merge_commit_oid"],
                            default_branch=(live_pr.get("base") or {}).get("ref"))
    gate_source, claim_source = validate_source(root, proof["source"])
    native = proof["mode"] == "native_v2"
    if not native and proof["mode"] != "legacy_activation":
        raise ValueError("readiness mode unknown")
    if native != (b"oasis7-native-readiness/v2" in claim_source):
        raise ValueError("readiness source producer generation mismatch")
    result = _result(proof["claim_result"], native=native, uid=uid)
    if result.get("repository_head") != identity["head_oid"]:
        raise ValueError("readiness native result repository head is not accepted H")
    _gate(proof["claim_input_gate"], proof["source"], identity, result["verified_at"], root)
    if native:
        binding = closed(result["readiness_binding"], BINDING_FIELDS, "native binding")
        selected_comment, selected_binding = select_native_comment(comments, identity, proof["merged_at"])
        if (canonical(selected_binding) != canonical(binding)
                or selected_comment.get("id") != proof["claim_comment"].get("id")):
            raise ValueError("readiness proof differs from latest server-selected native binding")
        expected = {"schema": "oasis7-native-readiness/v2", **identity, "claim_type": "ready_for_merge",
            "status": "verified", "exit_code": 0, "verified_at": result["verified_at"],
            "gate_raw_sha256": proof["claim_input_gate"]["raw_sha256"],
            "gate_epoch": proof["claim_input_gate"]["gate_epoch"],
            "gate_observed_at": proof["claim_input_gate"]["observed_at"], "source": proof["source"]}
        if canonical(binding) != canonical(expected) or canonical(result["readiness_comment"]) != canonical(proof["claim_comment"]):
            raise ValueError("readiness native result/binding/comment mismatch")
        native_dest = native_artifact_root(root, uid, binding)
        try:
            if ((native_dest / "gate.stdout").read_bytes() != decode(proof["claim_input_gate"]["stdout_b64"])
                    or (native_dest / "result.stdout").read_bytes() != decode(proof["claim_result"]["stdout_b64"])
                    or canonical(load((native_dest / "comment.json").read_bytes())) != canonical(proof["claim_comment"])):
                raise ValueError("readiness persisted native artifact bytes changed")
        except OSError as exc:
            raise ValueError("readiness persisted native artifact unavailable") from exc
        body = NATIVE.encode() + b"\n" + canonical(binding)
        validate_comment(repo, identity["issue_number"], proof["claim_comment"], comments, body, NATIVE)
        if proof["migration_comment"] is not None:
            raise ValueError("readiness native mode contains migration evidence")
    else:
        body = _legacy_body(uid, result)
        validate_comment(repo, identity["issue_number"], proof["claim_comment"], comments, body,
                         "<!-- oasis7-pm-claim-verification -->")
        migration = closed(proof["migration_comment"], COMMENT_FIELDS, "migration comment")
        transport_body = decode(migration["body_b64"])
        prefix = MIGRATION.encode() + b"\n"
        if not transport_body.startswith(prefix):
            raise ValueError("readiness migration marker invalid")
        payload = closed(load(transport_body[len(prefix):]), MIGRATION_FIELDS, "migration payload")
        if transport_body != prefix + canonical(payload):
            raise ValueError("readiness migration body noncanonical")
        expected = {"schema": "oasis7-pr-readiness-migration/v1", **identity,
            "default_branch": (live_pr.get("base") or {}).get("ref"),
            "pre_merge_target_oid": proof["pre_merge_target_oid"], "pr_base_oid": proof["pr_base_oid"],
            "producer_generation": "legacy_v1", "source": proof["source"],
            "claim_input_gate": proof["claim_input_gate"], "claim_result": proof["claim_result"],
            "claim_comment": proof["claim_comment"]}
        if canonical(payload) != canonical(expected):
            raise ValueError("readiness migration payload/proof mismatch")
        validate_comment(repo, identity["issue_number"], migration, comments, transport_body, MIGRATION, admin=True)
        # Both pinned actual T and immutable PR base must be legacy; this exact
        # merge M must introduce compatible native producer and delivery reader.
        if b"oasis7-native-readiness/v2" in _entry(root, proof["pr_base_oid"], "claim-ready.sh"):
            raise ValueError("readiness first migration base already supports native v2")
        if (b"oasis7-native-readiness/v2" not in _entry(root, proof["merge_commit_oid"], "claim-ready.sh")
                or b"readiness_proof_sha256" not in _entry(root, proof["merge_commit_oid"], "terminal_proof.py")):
            raise ValueError("readiness migration M does not activate compatible v2 producer/consumer")
        if not instant(proof["claim_comment"]["created_at"]) <= instant(migration["created_at"]) < instant(proof["merged_at"]):
            raise ValueError("readiness migration publication order invalid")
    if not instant(result["verified_at"]) <= instant(proof["claim_comment"]["created_at"]) < instant(proof["merged_at"]):
        raise ValueError("readiness native claim publication order invalid")
    return {"record": proof, "digest": digest(raw), "bytes": raw}


def create_readiness_proof(root: Path, uid: str, *, write: bool = False) -> dict:
    """Build from exact persisted native artifacts or the unique admin transport.

    Always fully validate before create-once publication. The caller's normal
    delivery context still validates Project and merge ancestry before writes.
    """
    record = _mapping(root, uid)
    repo = record.get("repository")
    issue = query(repo, f"repos/{repo}/issues/{record.get('issue_number')}")
    pr = query(repo, f"repos/{repo}/pulls/{record.get('pr_number')}")
    identity = _identity(uid, record, issue, pr)
    comments = read_comments(repo, identity["issue_number"])
    # The same bounded postmerge identity/ancestry inputs used by delivery are
    # checked before publishing even the proof file itself. Project status may
    # still be pre-task_done here; identity and complete pagination may not.
    from terminal_proof import read_live_repository, validate_live_repository
    from loop_terminal import read_project, normalize_project_fields
    live_repository = read_live_repository(repo, pr.get("merge_commit_sha"))
    validate_live_repository(live_repository, repo, pr.get("merge_commit_sha"),
                            default_branch=(pr.get("base") or {}).get("ref"))
    project = read_project(repo, identity["issue_number"])
    if (project.get("owner") != repo.split("/", 1)[0] or type(project.get("number")) is not int or project.get("number") != 1
            or project.get("page_complete") is not True or not project.get("id")):
        raise ValueError("readiness Project identity/pagination unavailable")
    items = [item for item in project.get("items", [])
        if (item.get("project") or {}).get("id") == project["id"]
        and (item.get("content") or {}).get("number") == identity["issue_number"]
        and (item.get("content") or {}).get("url") == f"https://github.com/{repo}/issues/{identity['issue_number']}"
        and re.findall(r"^task_uid:\s*([^\n]+)$", str((item.get("content") or {}).get("body") or ""), re.M) == [uid]]
    if len(items) != 1 or items[0].get("id") != record.get("project_item_id"):
        raise ValueError("readiness unique Project Task binding unavailable")
    fields = normalize_project_fields(items[0], repo)
    if fields.get("Task UID", uid) != uid:
        raise ValueError("readiness Project Task UID differs")
    dest = receipt_root(root, uid)
    if (dest / "readiness-proof.json").exists():
        validated = validate_readiness_proof(root, uid, record, live_pr=pr, comments=comments, live_issue=issue)
        _prior_delivery_admission(root, uid, validated["record"])
        return validated
    natives = [c for c in comments if NATIVE in str(c.get("body") or "")]
    migrations = [c for c in comments if MIGRATION in str(c.get("body") or "")]
    if natives and migrations:
        raise ValueError("readiness mixed native/migration evidence")
    if natives:
        selected_comment, selected_binding = select_native_comment(comments, identity, pr.get("merged_at"))
        native_dest = native_artifact_root(root, uid, selected_binding)
        try:
            gate_raw = (native_dest / "gate.stdout").read_bytes()
            result_raw = (native_dest / "result.stdout").read_bytes()
            comment = load((native_dest / "comment.json").read_bytes())
        except OSError as exc:
            raise ValueError("readiness native exact artifacts unavailable") from exc
        result = load(result_raw)
        if (canonical(result.get("readiness_binding")) != canonical(selected_binding)
                or comment.get("id") != selected_comment.get("id")):
            raise ValueError("readiness canonical artifacts differ from server-selected native binding")
        source = result.get("readiness_binding", {}).get("source")
        proof = {"schema": "oasis7-delivery-readiness/v2", "mode": "native_v2", **identity,
            "pre_merge_target_oid": source["oid"], "pr_base_oid": (pr.get("base") or {}).get("sha"),
            "merge_commit_oid": pr.get("merge_commit_sha"), "merged_at": pr.get("merged_at"),
            "source": source, "claim_input_gate": gate_capture(gate_raw, source, repo, identity["pr_number"],
                identity["head_oid"], result["verified_at"], root=root),
            "claim_result": result_capture(result_raw), "claim_comment": comment, "migration_comment": None}
    elif len(migrations) == 1:
        body = str(migrations[0].get("body") or "")
        if not body.startswith(MIGRATION + "\n"):
            raise ValueError("readiness migration exact marker unavailable")
        payload = closed(load(body[len(MIGRATION) + 1:].encode()), MIGRATION_FIELDS, "migration payload")
        proof = {"schema": "oasis7-delivery-readiness/v2", "mode": "legacy_activation", **identity,
            **{k: payload[k] for k in ("pre_merge_target_oid", "pr_base_oid", "source", "claim_input_gate", "claim_result", "claim_comment")},
            "merge_commit_oid": pr.get("merge_commit_sha"), "merged_at": pr.get("merged_at"),
            "migration_comment": comment_capture(migrations[0])}
    else:
        raise ValueError("required readiness proof/native artifacts/unique migration unavailable")
    validated = validate_readiness_proof(root, uid, record, live_pr=pr, comments=comments, live_issue=issue, proof=proof)
    _prior_delivery_admission(root, uid, validated["record"])
    if write:
        _store_once(dest / "readiness-proof.json", validated["bytes"])
    return validated


def _prior_delivery_admission(root: Path, uid: str, readiness: dict) -> None:
    """Call the existing producer's complete prior-receipt/recovery parser."""
    path = Path(__file__).with_name("post-merge-finalize.py")
    spec = importlib.util.spec_from_file_location("readiness_prior_delivery", path)
    if spec is None or spec.loader is None:
        raise ValueError("readiness prior-delivery admission helper unavailable")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    helper.validate_prior_delivery_admission(root, uid, readiness)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True); parser.add_argument("--task-uid", required=True)
    parser.add_argument("--create", action="store_true")
    parser.add_argument("--publish-native", action="store_true"); parser.add_argument("--tool-root")
    parser.add_argument("--capture-legacy", action="store_true")
    parser.add_argument("--recover-merged-delivery",action="store_true")
    parser.add_argument("--gate-input"); parser.add_argument("--result-json")
    args = parser.parse_args()
    try:
        if args.recover_merged_delivery:
            if args.publish_native or args.capture_legacy or args.tool_root or args.gate_input or args.result_json:
                raise ValueError('recovery and native readiness publication modes are exclusive')
            import terminal_recovery
            result=(terminal_recovery.publish_recovery if args.create else terminal_recovery.collect_recovery)(Path(args.repo_root),args.task_uid)
            print(terminal_recovery.obs.canonical(result).decode())
            return 0
        if args.publish_native and args.capture_legacy:
            raise ValueError("readiness publication modes are exclusive")
        if args.capture_legacy:
            result = capture_legacy(Path(args.repo_root), Path(args.tool_root), args.task_uid, Path(args.gate_input))
            print(json.dumps(result, ensure_ascii=False))
        elif args.publish_native:
            result = publish_native(Path(args.repo_root), Path(args.tool_root), args.task_uid,
                Path(args.gate_input).read_bytes(), load(args.result_json.encode()))
            print(json.dumps(result, ensure_ascii=False))
        else:
            result = create_readiness_proof(Path(args.repo_root), args.task_uid, write=args.create)
            print(json.dumps({"status": "passed", "readiness_proof_sha256": result["digest"]}))
    except (ValueError, OSError, subprocess.SubprocessError, KeyError, TypeError) as exc:
        print(f"readiness: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
