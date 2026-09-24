#!/usr/bin/env python3
"""Deterministic input closure and product-corpus contracts for CI reuse.

This module only reads Git objects and caller-supplied trusted unit contracts.
It never executes candidate code, runs checkers, or decides merge authority.
"""
from __future__ import annotations

import hashlib
import json
import posixpath
import re
import subprocess
import sys
import urllib.parse
from pathlib import Path
from typing import Any


INPUT_SCOPE_SCHEMA = "oasis7-ci-input-scope/v1"
_OID_RE = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
_DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_PRODUCT_DOC_RE = re.compile(r"doc/product/.+\.(?:prd|design)\.md\Z")
_MARKDOWN_LINK_PARSER_PATH = "scripts/product_doc_markdown.py"
_MARKDOWN_LINK_REQUIREMENTS_PATH = "scripts/doc-governance-requirements.txt"
FALLBACK_AUTHORITY_ID = "trusted-planner-unit-inventory/v1"
_UNIT_SPEC_FIELDS = {
    "unit_id", "unit_contract", "obligation_set", "command_checker_paths",
    "input_paths", "member_roots", "dependency_edges", "applicable_policy",
    "environment_contract",
}


class InputScopeError(ValueError):
    """A malformed or incomplete input-scope contract."""


def canonical_json_bytes(value: Any) -> bytes:
    try:
        return json.dumps(
            value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError, UnicodeEncodeError) as exc:
        raise InputScopeError(f"value is not canonical JSON: {exc}") from exc


def sha256_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def _digest_json(value: Any) -> str:
    return sha256_bytes(canonical_json_bytes(value))


def _string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value or value != value.strip():
        raise InputScopeError(f"{field} must be a non-empty trimmed string")
    if any(char in value for char in "\x00\r\n"):
        raise InputScopeError(f"{field} contains a control character")
    return value


def _strings(value: Any, field: str) -> list[str]:
    if not isinstance(value, list):
        raise InputScopeError(f"{field} must be a list")
    result = [_string(item, f"{field}[{index}]") for index, item in enumerate(value)]
    if len(result) != len(set(result)):
        raise InputScopeError(f"{field} contains duplicates")
    return result


def _oid(value: Any, field: str) -> str:
    if not isinstance(value, str) or not _OID_RE.fullmatch(value):
        raise InputScopeError(f"{field} must be a lowercase Git object ID")
    return value


def _digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or not _DIGEST_RE.fullmatch(value):
        raise InputScopeError(f"{field} must be a prefixed SHA-256 digest")
    return value


def _repo_path(value: Any, field: str) -> str:
    path = _string(value, field)
    if path.startswith("/") or "\\" in path:
        raise InputScopeError(f"{field} must be a repository-relative POSIX path")
    parts = path.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise InputScopeError(f"{field} is not a normalized repository path")
    return path


def _git(repo_root: str, *args: str, binary: bool = False) -> bytes | str:
    try:
        result = subprocess.run(
            ["git", "-C", repo_root, *args], check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        detail = getattr(exc, "stderr", b"")
        if isinstance(detail, bytes):
            detail = detail.decode("utf-8", errors="replace")
        raise InputScopeError(f"git {' '.join(args[:2])} failed: {detail.strip()}") from exc
    return result.stdout if binary else result.stdout.decode("ascii").strip()


def git_tree_entries(repo_root: str, commit_oid: str) -> tuple[str, str, dict[str, dict[str, str]]]:
    """Return commit/tree IDs and every tracked path without following symlinks."""
    commit_oid = _oid(commit_oid, "commit_oid")
    commit = _git(repo_root, "rev-parse", "--verify", "--end-of-options", f"{commit_oid}^{{commit}}")
    tree_oid = _git(repo_root, "rev-parse", "--verify", "--end-of-options", f"{commit}^{{tree}}")
    raw = _git(repo_root, "ls-tree", "-r", "-t", "-z", "--full-tree", tree_oid, binary=True)
    entries: dict[str, dict[str, str]] = {}
    for record in raw.split(b"\x00"):
        if not record:
            continue
        try:
            metadata, raw_path = record.split(b"\t", 1)
            mode, kind, oid = metadata.decode("ascii").split(" ")
            path = raw_path.decode("utf-8", errors="strict")
        except (ValueError, UnicodeDecodeError) as exc:
            raise InputScopeError("Git tree contains an unsupported path or entry") from exc
        path = _repo_path(path, "Git tree path")
        _oid(oid, f"Git tree entry {path} OID")
        if kind not in {"blob", "tree", "commit"} or path in entries:
            raise InputScopeError("Git tree contains an invalid or duplicate entry")
        entries[path] = {"mode": mode, "type": kind, "oid": oid}
    return commit, tree_oid, entries


def _git_blob_hashes(
    repo_root: str, object_ids: set[str], *, include_contents: bool | set[str] = False,
) -> tuple[dict[str, str], dict[str, bytes]]:
    """Hash many blobs through one bounded-memory cat-file batch session."""
    hashes: dict[str, str] = {}
    contents: dict[str, bytes] = {}
    content_ids = object_ids if include_contents is True else set(include_contents or ())
    if not object_ids:
        return hashes, contents
    try:
        process = subprocess.Popen(
            ["git", "-C", repo_root, "cat-file", "--batch"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
    except OSError as exc:
        raise InputScopeError(f"cannot start git cat-file batch: {exc}") from exc
    try:
        assert process.stdin is not None and process.stdout is not None
        for object_id in sorted(object_ids):
            process.stdin.write(object_id.encode("ascii") + b"\n")
            process.stdin.flush()
            header = process.stdout.readline()
            try:
                returned_oid, kind, size_text = header.decode("ascii").strip().split(" ")
                size = int(size_text)
            except (ValueError, UnicodeDecodeError) as exc:
                raise InputScopeError("git cat-file returned a malformed blob header") from exc
            if returned_oid != object_id or kind != "blob" or size < 0:
                raise InputScopeError("git cat-file returned an unexpected object")
            remaining = size
            hasher = hashlib.sha256()
            buffer = bytearray() if object_id in content_ids else None
            while remaining:
                chunk = process.stdout.read(min(1024 * 1024, remaining))
                if not chunk:
                    raise InputScopeError("git cat-file returned truncated blob content")
                hasher.update(chunk)
                if buffer is not None:
                    buffer.extend(chunk)
                remaining -= len(chunk)
            if process.stdout.read(1) != b"\n":
                raise InputScopeError("git cat-file blob framing is invalid")
            hashes[object_id] = "sha256:" + hasher.hexdigest()
            if buffer is not None:
                contents[object_id] = bytes(buffer)
        process.stdin.close()
        stderr = process.stderr.read() if process.stderr else b""
        return_code = process.wait()
        if return_code:
            raise InputScopeError(
                "git cat-file batch failed: " + stderr.decode("utf-8", errors="replace").strip()
            )
    except Exception:
        process.kill()
        process.wait()
        raise
    finally:
        if process.stdout:
            process.stdout.close()
        if process.stderr:
            process.stderr.close()
    return hashes, contents


def _entry_fingerprint(entry: dict[str, str], blob_hashes: dict[str, str]) -> dict[str, str]:
    value = {"mode": entry["mode"], "type": entry["type"]}
    if entry["type"] == "blob":
        try:
            value["content_sha256"] = blob_hashes[entry["oid"]]
        except KeyError as exc:
            raise InputScopeError("input blob digest was not collected") from exc
    elif entry["type"] == "commit":
        value["gitlink_oid"] = entry["oid"]
    else:
        raise InputScopeError("unit inputs must identify files or submodules, not directories")
    return value


def _validate_unit_spec(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != _UNIT_SPEC_FIELDS:
        raise InputScopeError("unit contract fields are incomplete or unsupported")
    unit_id = _string(value["unit_id"], "unit_id")
    obligations = sorted(_strings(value["obligation_set"], "obligation_set"))
    commands = sorted({_repo_path(path, "command_checker_paths[]")
                       for path in _strings(value["command_checker_paths"], "command_checker_paths")})
    if not commands:
        raise InputScopeError("each validation unit must bind its command/checker files")
    paths = sorted({_repo_path(path, "input_paths[]") for path in _strings(value["input_paths"], "input_paths")})
    roots = sorted({_repo_path(path, "member_roots[]") for path in _strings(value["member_roots"], "member_roots")})
    edges = value["dependency_edges"]
    if not isinstance(edges, list):
        raise InputScopeError("dependency_edges must be a list")
    normalized_edges: list[list[str]] = []
    for index, edge in enumerate(edges):
        if not isinstance(edge, list) or len(edge) != 2:
            raise InputScopeError(f"dependency_edges[{index}] must have two endpoints")
        normalized_edges.append([
            _string(edge[0], f"dependency_edges[{index}][0]"),
            _string(edge[1], f"dependency_edges[{index}][1]"),
        ])
    normalized_edges = [list(edge) for edge in sorted({tuple(edge) for edge in normalized_edges})]
    contract = value["unit_contract"]
    policy = value["applicable_policy"]
    environment = value["environment_contract"]
    canonical_json_bytes(contract)
    canonical_json_bytes(policy)
    canonical_json_bytes(environment)
    return {
        "unit_id": unit_id,
        "unit_contract": contract,
        "obligation_set": obligations,
        "command_checker_paths": commands,
        "input_paths": paths,
        "member_roots": roots,
        "dependency_edges": normalized_edges,
        "applicable_policy": policy,
        "environment_contract": environment,
    }


def _unit_input_fingerprint(
    entries: dict[str, dict[str, str]], spec: dict[str, Any],
    blob_hashes: dict[str, str], blob_contents: dict[str, bytes],
) -> str:
    selected: list[dict[str, Any]] = []
    for path in spec["input_paths"]:
        entry = entries.get(path)
        selected.append({
            "path": path,
            "state": "present" if entry else "missing",
            **(_entry_fingerprint(entry, blob_hashes) if entry else {}),
        })
    memberships: list[dict[str, Any]] = []
    for root in spec["member_roots"]:
        root_entry = entries.get(root)
        if root_entry and root_entry["type"] != "tree":
            raise InputScopeError(f"member root {root} is not a directory")
        members = []
        prefix = root + "/"
        for path in sorted(entries):
            entry = entries[path]
            if path.startswith(prefix) and entry["type"] != "tree":
                members.append({
                    "path": path,
                    "state": "present",
                    **_entry_fingerprint(entry, blob_hashes),
                })
        memberships.append({
            "root": root,
            "exists": bool(root_entry),
            "members": members,
        })
    selected_paths = set(spec["input_paths"])
    selected_paths.update(
        path for path in entries if any(path.startswith(root + "/") for root in spec["member_roots"])
    )
    command_inputs = []
    for path in spec["command_checker_paths"]:
        entry = entries.get(path)
        if not entry or entry["type"] == "tree" or entry["mode"] == "120000":
            raise InputScopeError(f"command/checker path is missing or indirect: {path}")
        selected_paths.add(path)
        command_inputs.append({"path": path, **_entry_fingerprint(entry, blob_hashes)})
    _validate_symlink_closure(entries, selected_paths, blob_contents)
    payload = {
        "schema": INPUT_SCOPE_SCHEMA,
        "unit_id": spec["unit_id"],
        "unit_contract": spec["unit_contract"],
        "obligation_set": spec["obligation_set"],
        "command_checker_inputs": command_inputs,
        "selected_inputs": selected,
        "membership_and_dependency_edges": {
            "roots": memberships,
            "edges": spec["dependency_edges"],
        },
        "applicable_policy": spec["applicable_policy"],
        "environment_contract": spec["environment_contract"],
    }
    return _digest_json(payload)


def _resolve_symlink_path(path: str, target: bytes) -> str:
    try:
        text = target.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise InputScopeError(f"symlink target is not UTF-8: {path}") from exc
    if not text or text.startswith("/") or "\\" in text or "\x00" in text:
        raise InputScopeError(f"symlink target escapes repository scope: {path}")
    resolved = posixpath.normpath(posixpath.join(posixpath.dirname(path), text))
    if resolved == ".." or resolved.startswith("../"):
        raise InputScopeError(f"symlink target escapes repository scope: {path}")
    return _repo_path(resolved, "symlink target")


def _validate_symlink_closure(
    entries: dict[str, dict[str, str]], selected_paths: set[str], blob_contents: dict[str, bytes],
) -> None:
    """Require every selected symlink's resolved file/directory contents in scope."""
    pending = [path for path in sorted(selected_paths)
               if (entry := entries.get(path)) and entry["mode"] == "120000"]
    visited: set[str] = set()
    while pending:
        path = pending.pop()
        if path in visited:
            raise InputScopeError("symlink cycle in selected inputs")
        visited.add(path)
        entry = entries[path]
        # Symlink blobs are in the exact selected input set and were hashed in
        # the same tree. Resolve from the object payload, never through the OS.
        target_bytes = blob_contents.get(entry["oid"])
        if target_bytes is None:
            raise InputScopeError("selected symlink target bytes are unavailable")
        target_path = _resolve_symlink_path(path, target_bytes)
        target_entry = entries.get(target_path)
        if target_entry is None:
            raise InputScopeError(f"selected symlink target is missing: {path}")
        if target_entry["type"] == "tree":
            descendants = {
                member for member, item in entries.items()
                if member.startswith(target_path + "/") and item["type"] != "tree"
            }
            if not descendants.issubset(selected_paths):
                raise InputScopeError(f"selected symlink directory closure is incomplete: {path}")
            pending.extend(member for member in descendants
                           if entries[member]["mode"] == "120000" and member not in visited)
        elif target_path not in selected_paths:
            raise InputScopeError(f"selected symlink target is outside the unit input set: {path}")
        elif target_entry["mode"] == "120000" and target_path not in visited:
            pending.append(target_path)


def build_input_scope_snapshot(
    repo_root: str,
    commit_oid: str,
    unit_specs: list[dict[str, Any]],
    product_corpus: dict[str, Any],
    *,
    closure_status: str = "complete",
    closure_reason: str | None = None,
    fallback_unit_ids: list[str] | None = None,
    fallback_scope_complete: bool = False,
) -> dict[str, Any]:
    """Build a target snapshot; unknown closure can only return full fallback work."""
    commit, tree_oid, entries = git_tree_entries(repo_root, commit_oid)
    specs = [_validate_unit_spec(spec) for spec in unit_specs]
    unit_ids = [spec["unit_id"] for spec in specs]
    if len(unit_ids) != len(set(unit_ids)):
        raise InputScopeError("unit IDs must be unique")
    unit_id_set = set(unit_ids)
    edges = sorted({tuple(edge) for spec in specs for edge in spec["dependency_edges"]})
    if closure_status == "complete":
        if any(source not in unit_id_set or dependency not in unit_id_set
               for source, dependency in edges):
            raise InputScopeError("dependency graph references an unknown unit")
    elif closure_status == "unknown":
        edges = []
    else:
        raise InputScopeError("closure_status must be complete or unknown")
    corpus = _normalize_product_corpus(product_corpus)
    if not set(corpus["unit_ids"]).issubset(unit_ids):
        raise InputScopeError("product-corpus obligation refers to an unknown unit")

    needed_blob_ids: set[str] = set()
    for spec in specs:
        for path in spec["command_checker_paths"]:
            entry = entries.get(path)
            if not entry or entry["type"] == "tree":
                raise InputScopeError(f"command/checker path is missing: {path}")
            if entry["type"] == "blob":
                needed_blob_ids.add(entry["oid"])
        for path in spec["input_paths"]:
            entry = entries.get(path)
            if entry and entry["type"] == "blob":
                needed_blob_ids.add(entry["oid"])
        for root in spec["member_roots"]:
            prefix = root + "/"
            needed_blob_ids.update(
                entry["oid"] for path, entry in entries.items()
                if path.startswith(prefix) and entry["type"] == "blob"
            )
    # Collect symlink target strings only for paths already inside each unit's
    # declared input or membership closure.
    symlink_ids = {
        entry["oid"] for spec in specs
        for path, entry in entries.items()
        if entry["mode"] == "120000"
        and (path in spec["input_paths"]
             or any(path.startswith(root + "/") for root in spec["member_roots"]))
    }
    blob_hashes, blob_contents = _git_blob_hashes(
        repo_root, needed_blob_ids, include_contents=symlink_ids,
    )

    if closure_status == "complete":
        if closure_reason is not None:
            raise InputScopeError("complete closure cannot carry a reason")
        if fallback_unit_ids is not None or fallback_scope_complete:
            raise InputScopeError("complete closure cannot carry a fallback claim")
        required_units = sorted(unit_ids)
        fingerprints = {
            spec["unit_id"]: _unit_input_fingerprint(entries, spec, blob_hashes, blob_contents)
            for spec in specs
        }
        fallback_complete = True
        fallback_contract = None
        normalized_reason = None
    elif closure_status == "unknown":
        normalized_reason = _string(closure_reason, "closure_reason")
        if type(fallback_scope_complete) is not bool or not fallback_scope_complete:
            raise InputScopeError("unknown closure requires trusted planner fallback coverage")
        fallback = sorted(set(_strings(fallback_unit_ids or [], "fallback_unit_ids")))
        planner_units = sorted(unit_ids)
        if fallback != planner_units:
            raise InputScopeError("fallback unit IDs must exactly match the complete planner unit inventory")
        required_units = sorted(set(planner_units) | set(corpus["unit_ids"]))
        fingerprints = {}
        fallback_complete = True
        fallback_contract = planner_fallback_contract(planner_units, commit, tree_oid)
    else:
        raise InputScopeError("closure_status must be complete or unknown")

    snapshot = {
        "schema": INPUT_SCOPE_SCHEMA,
        "target_oid": commit,
        "target_tree_oid": tree_oid,
        "closure_status": {"status": closure_status, "reason": normalized_reason},
        "required_test_units": required_units,
        "input_fingerprints": fingerprints,
        "dependency_edges": [list(edge) for edge in edges],
        "fallback_complete": fallback_complete,
        "fallback_contract": fallback_contract,
        "product_corpus": corpus,
    }
    validate_input_scope_snapshot(snapshot)
    return snapshot


def _normalize_product_corpus(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != {"status", "unit_ids", "obligations", "errors", "digest"}:
        raise InputScopeError("product-corpus contract fields are incomplete or unsupported")
    status = _string(value["status"], "product_corpus.status")
    if status not in {"complete", "blocked"}:
        raise InputScopeError("product-corpus status is unsupported")
    unit_ids = sorted(_strings(value["unit_ids"], "product_corpus.unit_ids"))
    errors = sorted(_strings(value["errors"], "product_corpus.errors"))
    obligations = value["obligations"]
    if not isinstance(obligations, list):
        raise InputScopeError("product-corpus obligations must be a list")
    normalized_obligations: list[dict[str, str]] = []
    obligation_ids: set[str] = set()
    for index, item in enumerate(obligations):
        if not isinstance(item, dict) or set(item) != {"obligation_id", "unit_id"}:
            raise InputScopeError(f"product-corpus obligation {index} is malformed")
        obligation_id = _string(item["obligation_id"], f"product_corpus.obligations[{index}].obligation_id")
        unit_id = _string(item["unit_id"], f"product_corpus.obligations[{index}].unit_id")
        if obligation_id in obligation_ids:
            raise InputScopeError("product-corpus obligation IDs must be unique")
        obligation_ids.add(obligation_id)
        normalized_obligations.append({"obligation_id": obligation_id, "unit_id": unit_id})
    normalized_obligations.sort(key=lambda item: (item["obligation_id"], item["unit_id"]))
    if {item["unit_id"] for item in normalized_obligations} != set(unit_ids):
        raise InputScopeError("every product-corpus unit must have an obligation")
    preimage = {
        "status": status,
        "unit_ids": unit_ids,
        "obligations": normalized_obligations,
        "errors": errors,
    }
    digest = _digest(value["digest"], "product_corpus.digest")
    if digest != _digest_json(preimage):
        raise InputScopeError("product-corpus digest mismatch")
    return {**preimage, "digest": digest}


def product_corpus_descriptor(
    unit_ids: list[str], obligations: list[dict[str, str]], errors: list[str] | None = None,
) -> dict[str, Any]:
    """Create a canonical descriptor for the complete product-doc obligation set."""
    if not isinstance(obligations, list):
        raise InputScopeError("product-corpus obligations must be a list")
    normalized_obligations: list[dict[str, str]] = []
    for index, item in enumerate(obligations):
        if not isinstance(item, dict) or set(item) != {"obligation_id", "unit_id"}:
            raise InputScopeError(f"product-corpus obligation {index} is malformed")
        normalized_obligations.append({
            "obligation_id": _string(item["obligation_id"], "obligation_id"),
            "unit_id": _string(item["unit_id"], "unit_id"),
        })
    preimage = {
        "status": "blocked" if errors else "complete",
        "unit_ids": sorted(_strings(unit_ids, "product_corpus.unit_ids")),
        "obligations": sorted(
            normalized_obligations,
            key=lambda item: (item["obligation_id"], item["unit_id"]),
        ),
        "errors": sorted(_strings(errors or [], "product_corpus.errors")),
    }
    if {item["unit_id"] for item in preimage["obligations"]} != set(preimage["unit_ids"]):
        raise InputScopeError("every product-corpus unit must have an obligation")
    return {**preimage, "digest": _digest_json(preimage)}


def planner_fallback_contract(
    unit_ids: list[str], target_oid: str, target_tree_oid: str,
) -> dict[str, Any]:
    """Bind fallback coverage to the trusted planner's complete unit inventory."""
    normalized_ids = sorted(_strings(unit_ids, "fallback_contract.unit_ids"))
    preimage = {
        "authority": FALLBACK_AUTHORITY_ID,
        "target_oid": _oid(target_oid, "fallback_contract.target_oid"),
        "target_tree_oid": _oid(target_tree_oid, "fallback_contract.target_tree_oid"),
        "unit_ids": normalized_ids,
    }
    return {**preimage, "digest": _digest_json(preimage)}


def _markdown_link_destinations(text: str) -> list[str]:
    """Read actual CommonMark link nodes through the repository's canonical parser."""
    scripts_dir = str(Path(__file__).resolve().parents[1])
    if scripts_dir not in sys.path:
        sys.path.insert(0, scripts_dir)
    try:
        from product_doc_markdown import parse_markdown_links
    except (ImportError, RuntimeError) as exc:
        raise InputScopeError(f"canonical product Markdown parser unavailable: {exc}") from exc
    return [link.target for link in parse_markdown_links(text)]


def _product_links(source: str, text: str) -> list[tuple[str, str]]:
    links: set[tuple[str, str]] = set()
    for destination in _markdown_link_destinations(text):
        if not destination:
            continue
        parsed = urllib.parse.urlsplit(destination)
        if parsed.scheme or parsed.netloc:
            continue
        raw_path = urllib.parse.unquote(parsed.path)
        if raw_path.startswith("/"):
            # Absolute paths escape the repository-relative link contract.
            continue
        elif raw_path:
            target = posixpath.normpath(posixpath.join(posixpath.dirname(source), raw_path))
        else:
            target = source
        if target == ".." or target.startswith("../"):
            continue
        links.add((target, urllib.parse.unquote(parsed.fragment)))
    return sorted(links)


def build_product_corpus_unit_specs(
    repo_root: str,
    commit_oid: str,
    *,
    command_checker_paths: list[str],
    applicable_policy: Any,
    environment_contract: Any,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Enumerate current product docs and all local repository-link relationships."""
    _, _, entries = git_tree_entries(repo_root, commit_oid)
    product_checker_paths = sorted({
        *(_repo_path(path, "command_checker_paths[]")
          for path in _strings(command_checker_paths, "command_checker_paths")),
        _MARKDOWN_LINK_PARSER_PATH,
        _MARKDOWN_LINK_REQUIREMENTS_PATH,
    })
    product_paths = sorted(
        path for path, entry in entries.items()
        if entry["type"] == "blob" and _PRODUCT_DOC_RE.fullmatch(path)
    )
    errors = [
        "product_document_symlink:" + path
        for path in product_paths if entries[path]["mode"] == "120000"
    ]
    documents = [path for path in product_paths if entries[path]["mode"] != "120000"]
    link_rows: list[tuple[str, str, str]] = []
    document_oids = {entries[path]["oid"] for path in documents}
    _, document_contents = _git_blob_hashes(
        repo_root, document_oids, include_contents=document_oids,
    )
    for source in documents:
        entry = entries[source]
        raw = document_contents[entry["oid"]]
        try:
            content = raw.decode("utf-8", errors="strict")
        except UnicodeDecodeError:
            errors.append("product_document_not_utf8:" + source)
            continue
        for target, anchor in _product_links(source, content):
            link_rows.append((source, target, anchor))
            target_entry = entries.get(target)
            if target_entry is None or target_entry["type"] in {"tree", "commit"}:
                errors.append(f"product_link_target_missing:{source}:{target}")
            elif target_entry["mode"] == "120000":
                errors.append(f"product_link_target_symlink:{source}:{target}")

    specs: list[dict[str, Any]] = []
    obligations: list[dict[str, str]] = []
    membership_id = "product-corpus:membership"
    membership_contract = {"kind": "product-corpus-membership/v1", "documents": documents}
    specs.append({
        "unit_id": membership_id,
        "unit_contract": membership_contract,
        "obligation_set": [membership_id],
        "command_checker_paths": product_checker_paths,
        "input_paths": [],
        "member_roots": [],
        "dependency_edges": [],
        "applicable_policy": applicable_policy,
        "environment_contract": environment_contract,
    })
    obligations.append({"obligation_id": membership_id, "unit_id": membership_id})
    unit_ids = [membership_id]
    for path in documents:
        unit_id = "product-document::" + path
        specs.append({
            "unit_id": unit_id,
            "unit_contract": {"kind": "product-document/v1", "path": path},
            "obligation_set": ["product-document:" + path],
            "command_checker_paths": product_checker_paths,
            "input_paths": [path],
            "member_roots": [],
            "dependency_edges": [],
            "applicable_policy": applicable_policy,
            "environment_contract": environment_contract,
        })
        unit_ids.append(unit_id)
        obligations.append({"obligation_id": "product-document:" + path, "unit_id": unit_id})
    for source, target, anchor in sorted(set(link_rows)):
        unit_id = f"product-link::{source}->{target}#{anchor}"
        obligation_id = f"product-link:{source}->{target}#{anchor}"
        specs.append({
            "unit_id": unit_id,
            "unit_contract": {
                "kind": "product-link/v1", "source": source,
                "target": target, "anchor": anchor,
            },
            "obligation_set": [obligation_id],
            "command_checker_paths": product_checker_paths,
            "input_paths": [source, target] if source != target else [source],
            "member_roots": [],
            "dependency_edges": [],
            "applicable_policy": applicable_policy,
            "environment_contract": environment_contract,
        })
        unit_ids.append(unit_id)
        obligations.append({"obligation_id": obligation_id, "unit_id": unit_id})
    return specs, product_corpus_descriptor(unit_ids, obligations, errors)


def validate_input_scope_snapshot(value: Any) -> dict[str, Any]:
    """Validate the closed schema consumed by the disabled applicability evaluator."""
    fields = {
        "schema", "target_oid", "target_tree_oid", "closure_status",
        "required_test_units", "input_fingerprints", "dependency_edges", "fallback_complete",
        "fallback_contract", "product_corpus",
    }
    if not isinstance(value, dict) or set(value) != fields:
        raise InputScopeError("input-scope snapshot fields are incomplete or unsupported")
    if value["schema"] != INPUT_SCOPE_SCHEMA:
        raise InputScopeError("input-scope schema is unsupported")
    _oid(value["target_oid"], "input_scope.target_oid")
    _oid(value["target_tree_oid"], "input_scope.target_tree_oid")
    closure = value["closure_status"]
    if not isinstance(closure, dict) or set(closure) != {"status", "reason"}:
        raise InputScopeError("input-scope closure status is malformed")
    status = closure["status"]
    if status == "complete":
        if closure["reason"] is not None:
            raise InputScopeError("complete closure cannot carry a reason")
    elif status == "unknown":
        _string(closure["reason"], "input_scope.closure_status.reason")
    else:
        raise InputScopeError("input-scope closure status is unsupported")
    units = sorted(_strings(value["required_test_units"], "input_scope.required_test_units"))
    raw_edges = value["dependency_edges"]
    if not isinstance(raw_edges, list):
        raise InputScopeError("input-scope dependency edges must be a list")
    edges: list[tuple[str, str]] = []
    for index, edge in enumerate(raw_edges):
        if not isinstance(edge, list) or len(edge) != 2:
            raise InputScopeError(f"input-scope dependency edge {index} is malformed")
        pair = (
            _string(edge[0], f"input_scope.dependency_edges[{index}][0]"),
            _string(edge[1], f"input_scope.dependency_edges[{index}][1]"),
        )
        if pair[0] not in units or pair[1] not in units:
            raise InputScopeError("input-scope dependency edge references an unknown unit")
        edges.append(pair)
    if edges != sorted(set(edges)):
        raise InputScopeError("input-scope dependency edges must be sorted and unique")
    fingerprints = value["input_fingerprints"]
    if not isinstance(fingerprints, dict) or any(not isinstance(key, str) for key in fingerprints):
        raise InputScopeError("input-scope fingerprints must be an object")
    if status == "complete":
        if set(fingerprints) != set(units):
            raise InputScopeError("complete input closure must fingerprint every required unit")
        for unit, digest in fingerprints.items():
            _digest(digest, f"input_scope.input_fingerprints.{unit}")
    elif fingerprints:
        raise InputScopeError("unknown closure cannot publish partial fingerprints")
    corpus = _normalize_product_corpus(value["product_corpus"])
    if corpus["status"] != "complete" or corpus["errors"]:
        raise InputScopeError("product-corpus membership or relationship closure is incomplete")
    if not set(corpus["unit_ids"]).issubset(units):
        raise InputScopeError("required test units omit product-corpus obligations")
    if not corpus["unit_ids"]:
        raise InputScopeError("product-corpus membership obligation is missing")
    fallback_complete = value["fallback_complete"]
    if type(fallback_complete) is not bool or not fallback_complete:
        raise InputScopeError("input scope has no complete conservative fallback")
    if status == "complete":
        if value["fallback_contract"] is not None:
            raise InputScopeError("complete input closure cannot carry a fallback contract")
    else:
        contract = value["fallback_contract"]
        if not isinstance(contract, dict) or set(contract) != {
            "authority", "target_oid", "target_tree_oid", "unit_ids", "digest",
        }:
            raise InputScopeError("unknown closure lacks the planner fallback authority contract")
        try:
            expected_contract = planner_fallback_contract(
                contract["unit_ids"], value["target_oid"], value["target_tree_oid"],
            )
        except InputScopeError as exc:
            raise InputScopeError("planner fallback authority contract is malformed") from exc
        if contract != expected_contract:
            raise InputScopeError("planner fallback authority contract digest or authority is invalid")
        if set(units) != set(expected_contract["unit_ids"]) | set(corpus["unit_ids"]):
            raise InputScopeError("unknown closure fallback units disagree with the complete planner inventory")
    return {
        **value,
        "required_test_units": units,
        "input_fingerprints": dict(fingerprints),
        "dependency_edges": [list(edge) for edge in edges],
        "fallback_contract": value["fallback_contract"],
        "product_corpus": corpus,
    }


def select_affected_units(prior: Any, target: Any) -> dict[str, Any]:
    """Compare two complete closures or widen every obligation on uncertainty."""
    try:
        old = validate_input_scope_snapshot(prior)
        new = validate_input_scope_snapshot(target)
    except InputScopeError as exc:
        return {"status": "blocked", "required_test_units": [], "reused_units": [],
                "retired_units": [], "blockers": ["INPUT_SCOPE_INVALID:" + str(exc)]}
    old_units = set(old["required_test_units"])
    new_units = set(new["required_test_units"])
    if (old["closure_status"]["status"] != "complete"
            or new["closure_status"]["status"] != "complete"):
        return {"status": "widened", "required_test_units": sorted(new_units),
                "reused_units": [], "retired_units": sorted(old_units - new_units),
                "blockers": []}
    changed_nodes = {
        unit for unit in new_units
        if unit not in old_units
        or old["input_fingerprints"][unit] != new["input_fingerprints"][unit]
    }
    old_edges = {tuple(edge) for edge in old["dependency_edges"]}
    new_edges = {tuple(edge) for edge in new["dependency_edges"]}
    changed_edges = old_edges ^ new_edges
    for edge in changed_edges:
        changed_nodes.update(edge)
    affected = changed_nodes & new_units
    changed = True
    while changed:
        changed = False
        for consumer, dependency in old_edges | new_edges:
            if dependency in changed_nodes and consumer not in changed_nodes:
                changed_nodes.add(consumer)
                if consumer in new_units:
                    affected.add(consumer)
                changed = True
    required = sorted(affected)
    return {"status": "complete", "required_test_units": required,
            "reused_units": sorted(new_units - set(required)),
            "retired_units": sorted(old_units - new_units), "blockers": []}


def aggregate_product_corpus_results(scope: Any, results: Any) -> dict[str, Any]:
    """Require one current input-bound result for every product-corpus obligation unit."""
    try:
        snapshot = validate_input_scope_snapshot(scope)
    except InputScopeError as exc:
        return {"status": "blocked", "required_units": [],
                "blockers": ["PRODUCT_CORPUS_SCOPE_INVALID:" + str(exc)]}
    if not isinstance(results, list) or any(not isinstance(item, dict) for item in results):
        return {"status": "blocked", "required_units": [],
                "blockers": ["PRODUCT_CORPUS_RESULTS_INVALID"]}
    expected_ids = set(snapshot["product_corpus"]["unit_ids"])
    obligations: dict[str, list[str]] = {unit: [] for unit in expected_ids}
    for row in snapshot["product_corpus"]["obligations"]:
        obligations[row["unit_id"]].append(row["obligation_id"])
    matching: dict[str, list[dict[str, Any]]] = {unit: [] for unit in expected_ids}
    for item in results:
        unit_id = item.get("unit_id")
        if isinstance(unit_id, str) and unit_id in expected_ids:
            matching[unit_id].append(item)
    required: list[str] = []
    blockers: list[str] = []
    for unit in sorted(expected_ids):
        rows = matching[unit]
        if not rows:
            required.append(unit)
            continue
        if len(rows) != 1:
            blockers.append("PRODUCT_CORPUS_UNIT_AMBIGUOUS:" + unit)
            continue
        row = rows[0]
        try:
            digest = _digest(row.get("input_digest"), "product result input_digest")
            row_obligations = sorted(_strings(row.get("obligation_ids"), "product result obligation_ids"))
        except InputScopeError:
            blockers.append("PRODUCT_CORPUS_LEGACY_OR_MALFORMED_RESULT:" + unit)
            continue
        if row_obligations != sorted(obligations[unit]):
            blockers.append("PRODUCT_CORPUS_OBLIGATION_MISMATCH:" + unit)
            continue
        if snapshot["closure_status"]["status"] != "complete" or digest != snapshot["input_fingerprints"].get(unit):
            required.append(unit)
            continue
        result_status = row.get("status")
        if not isinstance(result_status, str) or result_status not in {"passed", "reused"}:
            blockers.append("PRODUCT_CORPUS_RESULT_NOT_ACCEPTED:" + unit)
    status = "blocked" if blockers else "revalidate" if required else "complete"
    return {"status": status, "required_units": sorted(required), "blockers": sorted(blockers)}
