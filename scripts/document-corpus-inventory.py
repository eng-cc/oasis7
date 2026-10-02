#!/usr/bin/env python3
"""Explicit local management commands for document corpus v3."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import sys
from typing import Any

import document_corpus as dc
import document_evidence_policy as evidence_policy


EXIT_DATA = 1
EXIT_INPUT = 2
EXIT_STALE = 3


def _hash(data: bytes | None) -> str | None:
    return hashlib.sha256(data).hexdigest() if data is not None else None


def _emit(value: Any) -> None:
    print(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2))


def _diagnostic(code: str, detail: str, *, source: str | None = None, record: str | None = None, repair: str = "") -> dict[str, Any]:
    return dc.Diagnostic(code, source, record, detail, repair).as_dict()


def _view(args: argparse.Namespace) -> dc.CorpusView:
    if getattr(args, "revision", None):
        if getattr(args, "worktree", False):
            raise dc.CorpusError(dc.Diagnostic("input-conflict", detail="--revision and --worktree are mutually exclusive"))
        return dc.GitCorpusView(args.repo_root, args.revision)
    return dc.WorktreeCorpusView(args.repo_root)


def _read_raw(view: dc.CorpusView, path: str) -> dict[str, Any]:
    value = dc.strict_json(view.read_bytes(path), path)
    if not isinstance(value, dict):
        raise dc.CorpusError(dc.Diagnostic("schema-record", record_path=path, detail="expected JSON object"))
    return value


def _write_report(repo_root: Path, path_text: str, value: Any | bytes) -> str:
    path = Path(path_text)
    if path.is_absolute():
        try:
            relative = path.relative_to(repo_root)
        except ValueError as exc:
            raise dc.CorpusError(dc.Diagnostic("report-path", detail="report must be inside repository .cache/doc-governance")) from exc
    else:
        relative = path
    rel = relative.as_posix()
    if not rel.startswith(".cache/doc-governance/"):
        raise dc.CorpusError(dc.Diagnostic("report-path", detail="report output must be under .cache/doc-governance/"))
    dc.validate_repo_path(rel)
    cursor = repo_root
    for component in rel.split("/")[:-1]:
        cursor = cursor / component
        if cursor.is_symlink() or (cursor.exists() and not cursor.is_dir()):
            raise dc.CorpusError(dc.Diagnostic("report-path", detail=f"unsafe report directory: {cursor}"))
    target = repo_root / relative
    if target.is_symlink() or (target.exists() and not target.is_file()):
        raise dc.CorpusError(dc.Diagnostic("report-path", detail="report target must be absent or a regular file"))
    tracked = subprocess.run(["git", "-C", str(repo_root), "ls-files", "--error-unmatch", "--", rel], capture_output=True, text=True, check=False)
    if tracked.returncode == 0:
        raise dc.CorpusError(dc.Diagnostic("report-path", detail="report output must not overwrite a tracked file"))
    if tracked.returncode != 1:
        raise dc.CorpusError(dc.Diagnostic("report-path", detail=tracked.stderr.strip() or "cannot verify report is untracked"))
    target.parent.mkdir(parents=True, exist_ok=True)
    data = value if isinstance(value, bytes) else dc.canonical_json(value)
    dc._safe_write(repo_root, rel, data)
    return rel


def _read_file_json(path: Path) -> dict[str, Any]:
    try:
        value = dc.strict_json(path.read_bytes(), path.as_posix())
    except OSError as exc:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail=f"legacy template unavailable: {exc}")) from exc
    if not isinstance(value, dict):
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail="legacy template is not an object"))
    return value


def _ordered_record(record: dict[str, Any], fields: tuple[str, ...]) -> dict[str, Any]:
    required = set(fields) - {"retirement_gate"}
    if not required <= set(record) or set(record) - set(fields):
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail=f"record fields cannot be represented: missing={sorted(required - set(record))} extra={sorted(set(record) - set(fields))}"))
    return {key: record[key] for key in fields if key in record}


def _preserve_legacy_row_order(records: list[dict[str, Any]], template: list[dict[str, Any]], identity: str) -> list[dict[str, Any]]:
    by_identity = {record[identity]: record for record in records}
    ordered_ids = [record[identity] for record in template if record.get(identity) in by_identity]
    ordered_ids.extend(sorted(set(by_identity) - set(ordered_ids)))
    return [by_identity[key] for key in ordered_ids]


def _legacy_json(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def _record_field_diff(before: dict[str, Any] | None, after: dict[str, Any] | None) -> list[dict[str, Any]]:
    changes = []
    for field in sorted(set(before or {}) | set(after or {})):
        old_present = before is not None and field in before
        new_present = after is not None and field in after
        old_value = before.get(field) if old_present else None
        new_value = after.get(field) if new_present else None
        if old_present != new_present or old_value != new_value:
            changes.append({"field": field, "before_present": old_present, "before": old_value,
                            "after_present": new_present, "after": new_value})
    return changes


def _wrap(kind: str, record: dict[str, Any]) -> bytes:
    return dc.canonical_json({"schema": dc.SCHEMAS[kind], "record": record})


def command_locate(args: argparse.Namespace) -> int:
    view = _view(args)
    model = dc.load_corpus(view)
    locations = dc.locate(model, args.path)
    if args.kind == "object":
        locations = [item for item in locations if item.kind in {"object", "static-control"}]
    elif args.kind == "semantic":
        locations = [item for item in locations if item.kind.startswith("semantic-")]
    elif args.kind == "evidence":
        locations = [item for item in locations if item.kind.startswith("evidence-")]
    _emit({**dc.snapshot_info(view), "snapshot": view.snapshot_id, "source_path": args.path,
           "locations": [item.__dict__ for item in locations]})
    return 0


def command_sync(args: argparse.Namespace) -> int:
    view = _view(args)
    if not isinstance(view, dc.WorktreeCorpusView):
        raise dc.CorpusError(dc.Diagnostic("write-requires-worktree", detail="sync cannot write an immutable revision"))
    model = dc.load_corpus(view)
    paths = list(args.path or [])
    deleted = list(args.deleted_path or [])
    if args.all:
        if paths or deleted:
            raise dc.CorpusError(dc.Diagnostic("input-conflict", detail="--all cannot be combined with explicit paths"))
        known = set(view.list_doc_paths())
        paths = sorted(known - model.metadata_paths - {p for p in known if p.startswith("doc/testing/evidence/")})
        deleted = sorted(set(model.objects_by_path) - set(paths))
    plan = dc.plan_sync(view, paths, deleted)
    changes = [{"path": path, "operation": "delete" if value is None else "write", "sha256": _hash(value)} for path, value in sorted(plan.writes.items())]
    if args.apply:
        result = dc.apply_plan(plan)
        _emit({"operation": "sync", "applied": True, "changed": list(result.changed_paths), "unchanged": list(result.unchanged_paths)})
    else:
        _emit({"operation": "sync", "applied": False, "planned": changes})
    return 0


def _preconditions(view: dc.CorpusView, sources: list[str], record_paths: list[str]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    known = set(view.list_doc_paths())
    dc.ensure_hash_inputs(view, [path for path in sorted(set(sources)) if path in known])
    source_items = [{"path": path, "sha256": _hash(view.read_bytes(path)) if path in known else None} for path in sorted(set(sources))]
    record_items = [{"path": path, "sha256": _hash(view.read_bytes(path)) if path in known else None} for path in sorted(set(record_paths))]
    return source_items, record_items


def _proposal_for(args: argparse.Namespace, view: dc.CorpusView, model: dc.CorpusModel) -> dict[str, Any]:
    kind = args.kind
    supplied_members = list(args.member or [])
    if args.path:
        if supplied_members:
            raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="--member is valid only for a new bundle or group"))
        locations = dc.locate(model, args.path)
        if kind == "semantic":
            grouped = [item for item in locations if item.kind == "semantic-bundle"]
            if grouped:
                raise dc.CorpusError(dc.Diagnostic("proposal-group-required", source_path=args.path, record_path=grouped[0].record_path, detail=f"select --bundle {grouped[0].record_id}"))
            record_path = dc.record_path("semantic", args.path)
            current = model.semantic_entries_by_path.get(args.path)
            if current is None:
                if args.path not in view.list_doc_paths():
                    raise dc.CorpusError(dc.Diagnostic("source-unavailable", source_path=args.path, detail="new semantic review requires an existing source"))
                record = {"path": args.path, "content_sha256": dc.sha256(view.read_bytes(args.path)), "disposition": "", "decision_owner": "", "current_authority": "", "note": ""}
            else:
                record = dict(current)
                record["content_sha256"] = dc.sha256(view.read_bytes(args.path))
            source_paths = [args.path]
            wrapped_kind = "semantic-entry"
        else:
            grouped = [item for item in locations if item.kind == "evidence-group"]
            if grouped:
                raise dc.CorpusError(dc.Diagnostic("proposal-group-required", source_path=args.path, record_path=grouped[0].record_path, detail=f"select --group {grouped[0].record_id}"))
            record_path = dc.record_path("evidence", args.path)
            current = model.evidence_entries_by_path.get(args.path)
            if current is None:
                if args.path not in view.list_doc_paths():
                    raise dc.CorpusError(dc.Diagnostic("source-unavailable", source_path=args.path, detail="new evidence review requires an existing source"))
                record = {"path": args.path, "lifecycle": "", "semantic_role": "", "retention_owner": "", "domain_owner": "", "required_followup_roles": [], "authority": "", "backlink": "", "disposition": "", "rationale": "", "residual_risk": ""}
            else:
                record = dict(current)
                if "content_sha256" in record:
                    record["content_sha256"] = dc.sha256(view.read_bytes(args.path))
            source_paths = [args.path]
            wrapped_kind = "evidence-entry"
    elif args.bundle:
        if kind != "semantic":
            raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="--bundle is a semantic selector"))
        if args.bundle in model.semantic_bundles_by_id:
            if supplied_members:
                raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="--member cannot replace members of an existing bundle"))
            record = dict(model.semantic_bundles_by_id[args.bundle])
        else:
            members = sorted(set(supplied_members))
            if not members or any(path not in view.list_doc_paths() for path in members):
                raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="new bundle requires --member paths that exist in this view"))
            record = {"id": args.bundle, "paths": members, "content_set_sha256": "", "disposition": "", "decision_owner": "", "current_authority": "", "note": ""}
        record["content_set_sha256"] = dc._semantic_digest(view, record["paths"])
        record_path = dc.bundle_record_path(args.bundle)
        source_paths = list(record["paths"])
        wrapped_kind = "semantic-bundle"
    elif args.group:
        if kind != "evidence":
            raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="--group is an evidence selector"))
        if args.group in model.evidence_groups_by_id:
            if supplied_members:
                raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="--member cannot replace members of an existing group"))
            old = model.evidence_groups_by_id[args.group]
            record = {"id": old["id"], "entries": [dict(entry) for entry in old["entries"]]}
        else:
            members = sorted(set(supplied_members))
            if not members or any(path not in view.list_doc_paths() for path in members):
                raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="new group requires --member paths that exist in this view"))
            record = {"id": args.group, "entries": []}
            for path in members:
                if path in model.path_to_group:
                    raise dc.CorpusError(dc.Diagnostic("proposal-group-membership", source_path=path, detail="member is already in an atomic group"))
                current = model.evidence_entries_by_path.get(path)
                if current:
                    entry = dict(current)
                else:
                    entry = {"path": path, "lifecycle": "", "semantic_role": "", "retention_owner": "", "domain_owner": "", "required_followup_roles": [], "authority": "", "backlink": "", "disposition": "", "rationale": "", "residual_risk": ""}
                entry["content_sha256"] = dc.sha256(view.read_bytes(path))
                record["entries"].append(entry)
            record["group_sha256"] = ""
        source_paths = [entry["path"] for entry in record["entries"]]
        for entry in record["entries"]:
            entry["content_sha256"] = dc.sha256(view.read_bytes(entry["path"]))
        record["group_sha256"] = dc._evidence_digest(record["entries"])
        record_path = dc.group_record_path(args.group)
        wrapped_kind = "evidence-group"
    else:
        raise dc.CorpusError(dc.Diagnostic("proposal-selector", detail="select exactly one --path, --bundle, or --group"))
    source_checks, record_checks = _preconditions(view, source_paths, [record_path])
    return {
        "schema": "oasis7.document-corpus-review-proposal/v1",
        "kind": kind,
        "source_preconditions": source_checks,
        "record_preconditions": record_checks,
        "mutations": [{"record_path": record_path, "operation": "upsert", "record": record}],
    }


def command_propose(args: argparse.Namespace) -> int:
    view = _view(args)
    model = dc.load_corpus(view)
    proposal = _proposal_for(args, view, model)
    path = _write_report(Path(args.repo_root).resolve(), args.output, proposal)
    _emit({"proposal": path, "schema": proposal["schema"], "mutations": len(proposal["mutations"])})
    return 0


def command_apply_review(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve()
    view = dc.WorktreeCorpusView(repo_root)
    proposal_path = Path(args.proposal)
    if not proposal_path.is_absolute():
        proposal_path = repo_root / proposal_path
    try:
        proposal_rel = proposal_path.absolute().relative_to(repo_root).as_posix()
        proposal_resolved = proposal_path.resolve(strict=True)
    except (OSError, ValueError) as exc:
        raise dc.CorpusError(dc.Diagnostic("proposal-path", detail="proposal must be a regular file under repository .cache/doc-governance")) from exc
    if not proposal_resolved.is_relative_to(repo_root) or not proposal_rel.startswith(".cache/doc-governance/") or proposal_path.is_symlink() or not proposal_path.is_file():
        raise dc.CorpusError(dc.Diagnostic("proposal-path", detail="proposal must be a regular file under repository .cache/doc-governance"))
    view._path(proposal_rel)
    tracked = subprocess.run(["git", "-C", str(repo_root), "ls-files", "--error-unmatch", "--", proposal_rel], capture_output=True, text=True, check=False)
    if tracked.returncode == 0:
        raise dc.CorpusError(dc.Diagnostic("proposal-path", detail="proposal must be untracked"))
    if tracked.returncode != 1:
        raise dc.CorpusError(dc.Diagnostic("proposal-path", detail=tracked.stderr.strip() or "cannot verify proposal is untracked"))
    raw = dc.strict_json(view.read_bytes(proposal_rel), proposal_rel)
    if not isinstance(raw, dict) or set(raw) != {"schema", "kind", "source_preconditions", "record_preconditions", "mutations"} or raw.get("schema") != "oasis7.document-corpus-review-proposal/v1":
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="proposal schema or fields invalid"))
    proposal_kind = raw.get("kind")
    if proposal_kind not in {"semantic", "evidence", "legacy-delta"}:
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="kind must be semantic, evidence, or legacy-delta"))
    source_preconditions = _parse_preconditions(raw["source_preconditions"], "source_preconditions")
    record_preconditions = _parse_preconditions(raw["record_preconditions"], "record_preconditions")
    writes: dict[str, bytes | None] = {}
    derived_sources: set[str] = set()
    current_model = dc.load_corpus(view)
    allowed_roots = (dc.SEMANTIC_ENTRIES_ROOT + "/", dc.SEMANTIC_BUNDLES_ROOT + "/", dc.EVIDENCE_ENTRIES_ROOT + "/", dc.EVIDENCE_GROUPS_ROOT + "/")
    mutations = raw["mutations"]
    if not isinstance(mutations, list) or not mutations:
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="mutations must be a nonempty list"))
    for mutation in raw["mutations"]:
        if not isinstance(mutation, dict) or mutation.get("operation") not in {"upsert", "delete"}:
            raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="mutation must be an upsert or delete object"))
        operation = mutation["operation"]
        expected_mutation_fields = {"record_path", "operation", "record"} if operation == "upsert" else {"record_path", "operation"}
        if set(mutation) != expected_mutation_fields:
            raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="mutation has missing or undeclared fields"))
        path = dc.validate_repo_path(mutation.get("record_path"))
        if not path.startswith(allowed_roots):
            raise dc.CorpusError(dc.Diagnostic("proposal-scope", record_path=path, detail="review writes must target semantic/evidence shards"))
        if path in writes:
            raise dc.CorpusError(dc.Diagnostic("proposal-schema", record_path=path, detail="proposal repeats a record target"))
        kind = _kind_for_record_path(path)
        if proposal_kind == "semantic" and not kind.startswith("semantic-") or proposal_kind == "evidence" and not kind.startswith("evidence-"):
            raise dc.CorpusError(dc.Diagnostic("proposal-scope", record_path=path, detail="mutation kind differs from proposal kind"))
        if operation == "delete":
            writes[path] = None
            derived_sources.update(_sources_for_record_path(current_model, path))
        else:
            record = mutation.get("record")
            if kind not in dc.SCHEMAS:
                raise dc.CorpusError(dc.Diagnostic("proposal-schema", record_path=path, detail="invalid review record kind"))
            if not isinstance(record, dict):
                raise dc.CorpusError(dc.Diagnostic("proposal-schema", record_path=path, detail="upsert record must be an object"))
            dc._load_record_for_proposal(kind, record, path)
            if _record_path_for(kind, record) != path:
                raise dc.CorpusError(dc.Diagnostic("wrong-record-key", record_path=path, detail="record identity does not match mutation path"))
            derived_sources.update(_sources_for_record(kind, record))
            bytes_value = _wrap(kind, record)
            parsed = dc.strict_json(bytes_value, path)
            dc._load_record_for_proposal(kind, parsed["record"], path)
            writes[path] = bytes_value
    if {path for path, _ in record_preconditions} != set(writes):
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="record preconditions must cover exactly the mutation targets"))
    declared_sources = {path for path, _ in source_preconditions}
    # Existing deletes can be resolved through the current model. A fully
    # applied proposal may no longer have that record, so its source set is
    # accepted only on the all-targets-already-equal idempotent path below.
    unresolved_delete_sources = False
    if any(m["operation"] == "delete" for m in mutations):
        unresolved_delete_sources = any(not _sources_for_record_path(current_model, m["record_path"]) for m in mutations if m["operation"] == "delete")
    for path, expected in source_preconditions:
        actual = _hash(view.read_bytes(path)) if view.file_mode(path) in {"100644", "100755"} else None
        if actual != expected:
            raise dc.CorpusError(dc.Diagnostic("proposal-stale", source_path=path, detail="source precondition changed"))
    dc.ensure_hash_inputs(view, [path for path, expected in source_preconditions if expected is not None])
    current_bytes = {path: view.read_bytes(path) if view.file_mode(path) in {"100644", "100755"} else None for path in writes}
    already_applied = all(current_bytes[path] == value for path, value in writes.items())
    if declared_sources != derived_sources and not (already_applied and unresolved_delete_sources):
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail="source preconditions must cover exactly the affected source members"))
    if already_applied:
        _validate_candidate(view, writes)
        _emit({"applied": False, "already_applied": True, "changed": [], "unchanged": sorted(writes)})
        return 0
    expected_old = dict(record_preconditions)
    old_state_matches = all(_hash(current_bytes[path]) == expected_old[path] if current_bytes[path] is not None else expected_old[path] is None for path in writes)
    if not old_state_matches:
        raise dc.CorpusError(dc.Diagnostic("proposal-stale", detail="record inputs are neither the exact reviewed state nor the complete proposed state"))
    _validate_candidate(view, writes)
    plan = dc.WritePlan(repo_root, source_preconditions, record_preconditions, writes, "apply-review")
    if args.apply:
        result = dc.apply_plan(plan)
        _emit({"applied": True, "changed": list(result.changed_paths), "unchanged": list(result.unchanged_paths)})
    else:
        _emit({"applied": False, "planned": sorted(writes)})
    return 0


def _parse_preconditions(value: Any, label: str) -> list[tuple[str, str | None]]:
    if not isinstance(value, list):
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail=f"{label} must be a list"))
    parsed: list[tuple[str, str | None]] = []
    for item in value:
        if not isinstance(item, dict) or set(item) != {"path", "sha256"}:
            raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail=f"{label} items must contain path and sha256 only"))
        path = dc.validate_repo_path(item["path"])
        digest = item["sha256"]
        if digest is not None and (not isinstance(digest, str) or not dc.SHA_RE.fullmatch(digest)):
            raise dc.CorpusError(dc.Diagnostic("proposal-schema", source_path=path, detail=f"{label} sha256 must be lowercase hex or null"))
        parsed.append((path, digest))
    if parsed != sorted(parsed) or len({path for path, _ in parsed}) != len(parsed):
        raise dc.CorpusError(dc.Diagnostic("proposal-schema", detail=f"{label} must be sorted by unique path"))
    return parsed


def _kind_for_record_path(path: str) -> str:
    if path.startswith(dc.SEMANTIC_ENTRIES_ROOT + "/"):
        return "semantic-entry"
    if path.startswith(dc.SEMANTIC_BUNDLES_ROOT + "/"):
        return "semantic-bundle"
    if path.startswith(dc.EVIDENCE_ENTRIES_ROOT + "/"):
        return "evidence-entry"
    if path.startswith(dc.EVIDENCE_GROUPS_ROOT + "/"):
        return "evidence-group"
    return ""


def _record_path_for(kind: str, record: dict[str, Any]) -> str:
    if kind == "semantic-entry":
        return dc.record_path("semantic", record["path"])
    if kind == "semantic-bundle":
        return dc.bundle_record_path(record["id"])
    if kind == "evidence-entry":
        return dc.record_path("evidence", record["path"])
    if kind == "evidence-group":
        return dc.group_record_path(record["id"])
    return ""


def _sources_for_record(kind: str, record: dict[str, Any]) -> set[str]:
    if kind in {"semantic-entry", "evidence-entry"}:
        return {dc.validate_repo_path(record["path"])}
    if kind == "semantic-bundle":
        return {dc.validate_repo_path(path) for path in record["paths"]}
    if kind == "evidence-group":
        return {dc.validate_repo_path(entry["path"]) for entry in record["entries"]}
    return set()


def _sources_for_record_path(model: dc.CorpusModel, record_path: str) -> set[str]:
    for source, record in model.semantic_entries_by_path.items():
        if dc.record_path("semantic", source) == record_path:
            return {source}
    for bundle_id, record in model.semantic_bundles_by_id.items():
        if dc.bundle_record_path(bundle_id) == record_path:
            return set(record["paths"])
    for source, record in model.evidence_entries_by_path.items():
        if dc.record_path("evidence", source) == record_path:
            return {source}
    for group_id, record in model.evidence_groups_by_id.items():
        if dc.group_record_path(group_id) == record_path:
            return {entry["path"] for entry in record["entries"]}
    return set()


def _validate_candidate(view: dc.WorktreeCorpusView, writes: dict[str, bytes | None]) -> None:
    overlay = dc.OverlayCorpusView(view, writes)
    model = dc.load_corpus(overlay)
    errors = dc.validate_corpus(overlay, model) + dc.validate_evidence(overlay, model)
    baseline_path = Path(__file__).resolve().parent / "fixtures/document-corpus-v3/legacy/inventory.json"
    try:
        baseline = baseline_path.read_bytes()
    except OSError as exc:
        raise dc.CorpusError(dc.Diagnostic("frozen-baseline-invalid", detail=f"legacy evidence baseline fixture unavailable: {exc}")) from exc
    errors.extend(evidence_policy.validate_evidence_policy(overlay, model, baseline))
    if errors:
        first = errors[0]
        raise dc.CorpusError(dc.Diagnostic(first.code, first.source_path, first.record_path, first.detail, first.repair_command))
    changed = view.verify_unchanged()
    if changed:
        first = changed[0]
        raise dc.CorpusError(dc.Diagnostic("concurrent-modification", first.source_path, first.record_path, first.detail, first.repair_command))


def command_export(args: argparse.Namespace) -> int:
    view = _view(args)
    model = dc.load_corpus(view)
    data = dc.normalize_current(model)
    report = {**dc.snapshot_info(view), "snapshot": view.snapshot_id, **data}
    if args.output:
        path = _write_report(Path(args.repo_root).resolve(), args.output, report)
        _emit({"output": path, "sha256": _hash(dc.canonical_json(report)), "counts": {key: len(value) for key, value in data.items()}})
    else:
        _emit(report)
    return 0


def command_export_legacy(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve()
    view = _view(args)
    model = dc.load_corpus(view)
    errors = dc.validate_corpus(view, model) + dc.validate_evidence(view, model)
    try:
        baseline = (Path(__file__).resolve().parent / "fixtures/document-corpus-v3/legacy/inventory.json").read_bytes()
    except OSError as exc:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail=f"baseline fixture unavailable: {exc}")) from exc
    errors.extend(evidence_policy.validate_evidence_policy(view, model, baseline))
    if errors:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail="current model fails validation: " + "; ".join(item.code for item in errors[:12])))
    baseline_data = dc.strict_json(baseline, "legacy evidence baseline")
    normalized = dc.normalize_current(model)
    current_evidence = normalized["evidence_entries"]
    if current_evidence != sorted(baseline_data["entries"], key=lambda item: item["path"]):
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail="v1 legacy evidence cannot express the current cohort exactly"))
    fixture_root = Path(__file__).resolve().parent / "fixtures/document-corpus-v3/legacy"
    corpus_template = _read_file_json(fixture_root / "document-corpus-inventory.json")
    semantic_template = _read_file_json(fixture_root / "document-semantic-review-overrides.json")
    entry_fields = ("path", "content_sha256", "disposition", "decision_owner", "current_authority", "note", "retirement_gate")
    bundle_fields = ("id", "disposition", "decision_owner", "current_authority", "note", "content_set_sha256", "paths")
    object_fields = ("path", "content_sha256", "registry_type", "structural_owner", "authority_entry", "authority_layer", "semantic_decision_owner", "object_kind", "lifecycle_candidate", "inventory_disposition", "routing_batch", "routing_note")
    semantic_template["entries"] = _preserve_legacy_row_order(
        [_ordered_record(record, entry_fields) for record in normalized["semantic_entries"]],
        semantic_template.get("entries", []), "path")
    semantic_template["bundles"] = _preserve_legacy_row_order(
        [_ordered_record(record, bundle_fields) for record in normalized["semantic_bundles"]],
        semantic_template.get("bundles", []), "id")
    semantic_bytes = _legacy_json(semantic_template)
    evidence_bytes = (fixture_root / "inventory.json").read_bytes()
    legacy_objects = _preserve_legacy_row_order(
        [_ordered_record(record, object_fields) for record in normalized["objects"]],
        corpus_template.get("objects", []), "path")
    legacy_root = {
        "version": 2,
        "scope": "all doc files through direct objects, delegated evidence objects, and controls",
        "decision_boundary": "routing is conservative heuristic input only",
        "product_modules": list(dc.PRODUCT_MODULES),
        "controls": [
            {"path": dc.CORPUS_ROOT, "content_sha256": "self-referential-control"},
            {"path": dc.LEGACY_SEMANTIC_ROOT, "content_sha256": dc.sha256(semantic_bytes)},
            {"path": dc.EVIDENCE_ROOT, "content_sha256": dc.sha256(evidence_bytes)},
        ],
        "delegates": [{
            "path": dc.EVIDENCE_ROOT,
            "exact_scope": "doc/testing/evidence/** excluding inventory.json",
            "checker_command": "python3 scripts/doc-evidence-inventory-check.py --repo-root <repo-root>",
            "expected_version": 1,
            "sha256": dc.sha256(evidence_bytes),
            "object_count": len(current_evidence),
        }],
        "objects": legacy_objects,
    }
    outputs = {
        "document-corpus-inventory.json": _legacy_json(legacy_root),
        "document-semantic-review-overrides.json": semantic_bytes,
        "inventory.json": evidence_bytes,
    }
    try:
        roundtrip = dc.normalize_legacy(legacy_root, semantic_template, baseline_data)
    except dc.CorpusError as exc:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail=f"legacy schema round trip failed: {exc.diagnostic.code}")) from exc
    if roundtrip != normalized:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail="legacy serialization cannot preserve every current model field"))
    legacy_controls = {dc.CORPUS_ROOT, dc.LEGACY_SEMANTIC_ROOT, dc.EVIDENCE_ROOT}
    current_paths = set(view.list_doc_paths())
    delegated_sources = {path for path in current_paths if dc._is_evidence_source(path) and path != dc.EVIDENCE_ROOT}
    direct_sources = current_paths - model.metadata_paths - delegated_sources
    legacy_object_paths = {record["path"] for record in legacy_objects}
    legacy_evidence_paths = {record["path"] for record in baseline_data["entries"]}
    if direct_sources != legacy_object_paths:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail=f"legacy object coverage differs: missing={sorted(direct_sources - legacy_object_paths)[:8]} extra={sorted(legacy_object_paths - direct_sources)[:8]}"))
    if delegated_sources != legacy_evidence_paths:
        raise dc.CorpusError(dc.Diagnostic("legacy-export-unrepresentable", detail=f"legacy evidence coverage differs: missing={sorted(delegated_sources - legacy_evidence_paths)[:8]} extra={sorted(legacy_evidence_paths - delegated_sources)[:8]}"))
    directory = Path(args.output_dir)
    if directory.is_absolute():
        try:
            directory = directory.relative_to(repo_root)
        except ValueError as exc:
            raise dc.CorpusError(dc.Diagnostic("report-path", detail="legacy export must stay inside repository cache")) from exc
    prefix = directory.as_posix().rstrip("/")
    if not prefix.startswith(".cache/doc-governance/"):
        raise dc.CorpusError(dc.Diagnostic("report-path", detail="legacy export must be under .cache/doc-governance/"))
    manifest = {"schema": "oasis7.document-corpus-v3-legacy-export/v1", **dc.snapshot_info(view), "snapshot": view.snapshot_id,
                "files": [{"path": name, "sha256": _hash(content), "bytes": len(content)} for name, content in sorted(outputs.items())],
                "restore_targets": [dc.CORPUS_ROOT, dc.LEGACY_SEMANTIC_ROOT, dc.EVIDENCE_ROOT],
                "remove_before_validation": sorted(path for path in view.list_doc_paths() if path.startswith("doc/.governance/document-corpus/") or path in {dc.CORPUS_ROOT, dc.EVIDENCE_ROOT})}
    for name, content in outputs.items():
        _write_report(repo_root, f"{prefix}/{name}", content)
    manifest_path = _write_report(repo_root, f"{prefix}/manifest.json", manifest)
    _emit({"output_directory": prefix, "manifest": manifest_path, "snapshot": view.snapshot_id, "files": len(outputs)})
    return 0


def _legacy_fixture_files(source_view: dc.GitCorpusView, source_oid: str) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, bytes]]:
    raw_paths = (dc.CORPUS_ROOT, dc.LEGACY_SEMANTIC_ROOT, dc.EVIDENCE_ROOT)
    raw_bytes = {path: source_view.read_bytes(path) for path in raw_paths}
    raw = {path: dc.strict_json(data, path) for path, data in raw_bytes.items()}
    fixture_root = "scripts/fixtures/document-corpus-v3/legacy"
    fixture_files = {f"{fixture_root}/{Path(path).name}": data for path, data in raw_bytes.items()}
    manifest = {
        "schema": "oasis7.document-corpus-v3-legacy-fixture/v1",
        "source_revision": source_oid,
        "files": [{"path": Path(path).name, "sha256": _hash(raw_bytes[path]), "bytes": len(raw_bytes[path])} for path in raw_paths],
    }
    fixture_files[f"{fixture_root}/manifest.json"] = dc.canonical_json(manifest)
    return raw[dc.CORPUS_ROOT], raw[dc.LEGACY_SEMANTIC_ROOT], raw[dc.EVIDENCE_ROOT], fixture_files


def command_migrate(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve()
    started = time.perf_counter()
    source_view = dc.GitCorpusView(repo_root, args.source_revision)
    if source_view.snapshot_kind != "commit" or source_view.commit_id is None:
        raise dc.CorpusError(dc.Diagnostic("revision-invalid", detail="migrate-v2 requires an immutable commit source revision"))
    source_oid = source_view.commit_id
    legacy_corpus, legacy_semantic, legacy_evidence, fixture_files = _legacy_fixture_files(source_view, source_oid)
    try:
        legacy_normalized = dc.normalize_legacy(legacy_corpus, legacy_semantic, legacy_evidence)
        proof_files = dc.v3_files_from_legacy(legacy_corpus, legacy_semantic, legacy_evidence)
        proof_files[dc.LEGACY_SEMANTIC_ROOT] = None
        proof_view = dc.OverlayCorpusView(source_view, proof_files)
        proof_model = dc.load_corpus(proof_view)
        proof_value = dc.normalize_current(proof_model)
        if proof_value != legacy_normalized:
            raise dc.CorpusError(dc.Diagnostic("migration-roundtrip-loss", detail="v2 to v3 normalized model differs before target source delta"))
        proof_errors = dc.validate_corpus(proof_view, proof_model) + dc.validate_evidence(proof_view, proof_model)
        proof_errors.extend(evidence_policy.validate_evidence_policy(proof_view, proof_model, source_view.read_bytes(dc.EVIDENCE_ROOT)))
        unexpected_proof_errors = [item for item in proof_errors if item.code != "object-content-drift"]
        if unexpected_proof_errors:
            raise dc.CorpusError(dc.Diagnostic("migration-roundtrip-invalid", detail="converted source snapshot failed: " + "; ".join(item.code for item in unexpected_proof_errors[:12])))
        target = dc.WorktreeCorpusView(repo_root)
        new_files = dc.v3_files_from_legacy(legacy_corpus, legacy_semantic, legacy_evidence)
        current_paths = set(target.list_doc_paths())
        # Keep the legacy source bytes as a pinned fixture while moving the active tables.
        new_files.update(fixture_files)
        old_object_records = {row["path"]: row for row in legacy_corpus["objects"]}
        target_metadata = dc._metadata_paths(tuple(current_paths))
        direct_paths = current_paths - target_metadata - {p for p in current_paths if dc._is_evidence_source(p)}
        direct_paths.discard(dc.LEGACY_SEMANTIC_ROOT)
        current_objects: dict[str, dict[str, Any]] = {}
        for path in sorted(direct_paths):
            if path in current_paths:
                current_objects[path] = dc.expected_object(target, path)
        # Replace converted baseline object bytes with the target snapshot's source hashes/routing.
        for path, record in current_objects.items():
            new_files[dc.record_path("object", path)] = _wrap("object", record)
        target_object_paths = set(current_objects)
        removed = sorted(set(old_object_records) - target_object_paths)
        for path in removed:
            new_files.pop(dc.record_path("object", path), None)
        new_files[dc.CORPUS_ROOT] = new_files[dc.CORPUS_ROOT]
        new_files[dc.EVIDENCE_ROOT] = new_files[dc.EVIDENCE_ROOT]
        if dc.LEGACY_SEMANTIC_ROOT in current_paths:
            new_files[dc.LEGACY_SEMANTIC_ROOT] = None  # type: ignore[assignment]
        destinations = set(new_files)
        old_paths = current_paths
        legacy_source_bytes = {
            dc.CORPUS_ROOT: source_view.read_bytes(dc.CORPUS_ROOT),
            dc.LEGACY_SEMANTIC_ROOT: source_view.read_bytes(dc.LEGACY_SEMANTIC_ROOT),
            dc.EVIDENCE_ROOT: source_view.read_bytes(dc.EVIDENCE_ROOT),
        }
        for path, expected_bytes in legacy_source_bytes.items():
            if path not in old_paths or target.read_bytes(path) != expected_bytes:
                raise dc.CorpusError(dc.Diagnostic("legacy-target-drift", record_path=path, detail="target legacy table differs from the immutable migration source; import the legacy delta first"))
        existing_shards = {path for path in old_paths if path.startswith((dc.OBJECTS_ROOT + "/", dc.SEMANTIC_ENTRIES_ROOT + "/", dc.SEMANTIC_BUNDLES_ROOT + "/", dc.EVIDENCE_ENTRIES_ROOT + "/", dc.EVIDENCE_GROUPS_ROOT + "/"))}
        for path in existing_shards & set(new_files):
            planned_bytes = new_files[path]
            if planned_bytes is not None and target.read_bytes(path) != planned_bytes:
                raise dc.CorpusError(dc.Diagnostic("migration-target-drift", record_path=path, detail="existing v3 shard differs from the conversion plan"))
        preconditions = [(path, _hash(target.read_bytes(path)) if path in old_paths else None) for path in sorted(destinations)]
        current_evidence_sources = {p for p in current_paths if dc._is_evidence_source(p)}
        source_paths = direct_paths | current_evidence_sources
        source_preconditions = [(path, _hash(target.read_bytes(path))) for path in sorted(source_paths) if path in current_paths]
        writes: dict[str, bytes | None] = dict(new_files)
        object_destinations = {path for path in old_paths if path.startswith(dc.OBJECTS_ROOT + "/")}
        for path in object_destinations - destinations:
            preconditions.append((path, _hash(target.read_bytes(path))))
            writes[path] = None
        plan = dc.WritePlan(repo_root, source_preconditions, preconditions, writes, "migrate-v2")
        converted = dc.normalize_legacy(legacy_corpus, legacy_semantic, legacy_evidence)
        target_overlay = dc.OverlayCorpusView(target, writes)
        target_model = dc.load_corpus(target_overlay)
        target_drift = dc.validate_corpus(target_overlay, target_model) + dc.validate_evidence(target_overlay, target_model)
        target_drift.extend(evidence_policy.validate_evidence_policy(target_overlay, target_model, source_view.read_bytes(dc.EVIDENCE_ROOT)))
        target_drift_rows = []
        for item in target_drift:
            if item.code not in {"semantic-content-drift", "semantic-bundle-drift", "evidence-content-drift", "evidence-group-drift", "frozen-cohort-drift", "current-operator-input-boundary", "readme-navigation"}:
                continue
            row = item.as_dict()
            if item.source_path in target_model.semantic_entries_by_path:
                reviewed = target_model.semantic_entries_by_path[item.source_path]["content_sha256"]
                row["reviewed_sha256"] = reviewed
                row["current_sha256"] = dc.sha256(target_overlay.read_bytes(item.source_path))
            elif item.source_path in target_model.evidence_entries_by_path:
                reviewed = target_model.evidence_entries_by_path[item.source_path].get("content_sha256")
                row["reviewed_sha256"] = reviewed
                row["current_sha256"] = dc.sha256(target_overlay.read_bytes(item.source_path))
            target_drift_rows.append(row)
        legacy_stale = [item.source_path for item in proof_errors if item.code == "object-content-drift" and item.source_path]
        legacy_stale_details = []
        for path in legacy_stale:
            record = old_object_records.get(path, {})
            legacy_stale_details.append({"path": path, "legacy_content_sha256": record.get("content_sha256"), "source_snapshot_sha256": dc.sha256(source_view.read_bytes(path))})
        source_byte_count = sum(len(target.read_bytes(path)) for path in sorted(source_paths) if path in current_paths)
        try:
            import resource
            peak_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        except (ImportError, AttributeError):
            peak_rss = None
        elapsed = time.perf_counter() - started
        source_old_paths = set(old_object_records) | {row["path"] for row in legacy_evidence["entries"]}
        source_new_paths = target_object_paths | current_evidence_sources
        source_added = sorted(source_new_paths - source_old_paths)
        source_removed = sorted(source_old_paths - source_new_paths)
        source_changed = sorted(path for path in source_old_paths & source_new_paths
                                if dc.sha256(source_view.read_bytes(path)) != dc.sha256(target.read_bytes(path)))
        source_delta_details = ([{"path": path, "before_sha256": None, "after_sha256": dc.sha256(target.read_bytes(path))}
                                 for path in source_added]
                                + [{"path": path, "before_sha256": dc.sha256(source_view.read_bytes(path)), "after_sha256": None}
                                   for path in source_removed]
                                + [{"path": path, "before_sha256": dc.sha256(source_view.read_bytes(path)), "after_sha256": dc.sha256(target.read_bytes(path))}
                                   for path in source_changed])
        object_field_differences = [{"path": path, "changes": _record_field_diff(old_object_records.get(path), current_objects.get(path))}
                                    for path in sorted(set(old_object_records) | set(current_objects))
                                    if old_object_records.get(path) != current_objects.get(path)]
        try:
            import sys
            peak_rss_units = "bytes" if sys.platform == "darwin" else ("KiB" if sys.platform.startswith("linux") else "platform units")
        except ImportError:
            peak_rss_units = "platform units"
        write_bytes = sum(len(value) for value in writes.values() if value is not None)
        report = {
            "schema": "oasis7.document-corpus-v3-migration-report/v1",
            "tool_version": dc.TOOL_VERSION,
            "source_revision": source_oid,
            "source_kind": source_view.snapshot_kind,
            "source_commit": source_view.commit_id,
            "source_tree": source_view.tree_id,
            "legacy_counts": {key: len(value) for key, value in converted.items()},
            "target_counts": {"objects": len(current_objects), "semantic_entries": len(converted["semantic_entries"]), "semantic_bundles": len(converted["semantic_bundles"]), "evidence_entries": len(converted["evidence_entries"])},
            "source_delta": {"added": source_added, "removed": source_removed, "changed": source_changed, "details": source_delta_details},
            "legacy_object_record_delta": {"added": sorted(set(current_objects) - set(old_object_records)), "removed": sorted(set(old_object_records) - set(current_objects)), "changed_fields": object_field_differences},
            "roundtrip_comparison": {"equivalent": proof_value == legacy_normalized,
                                      "semantic_bundle_ids": sorted(item["id"] for item in converted["semantic_bundles"]),
                                      "evidence_group_ids": sorted(item["id"] for item in proof_model.evidence_groups_by_id.values()),
                                      "semantic_bundle_differences": [], "evidence_group_differences": [],
                                      "record_field_differences": []},
            "allowed_wrapper_differences": [
                {"old_path": dc.CORPUS_ROOT, "new_path": dc.CORPUS_ROOT, "change": "legacy global table replaced by static v3 entrypoint and sharded objects"},
                {"old_path": dc.LEGACY_SEMANTIC_ROOT, "new_path": dc.SEMANTIC_ENTRIES_ROOT, "change": "legacy global semantic table replaced by content-addressed entry and bundle shards"},
                {"old_path": dc.EVIDENCE_ROOT, "new_path": dc.EVIDENCE_ROOT, "change": "v1 delegate entrypoint converted to static v2 roots backed by frozen legacy fixture"},
            ],
            "legacy_stale_object_paths": sorted(legacy_stale),
            "legacy_stale_object_details": legacy_stale_details,
            "target_review_drift": target_drift_rows,
            "fixture_files": [{"path": path, "sha256": _hash(data)} for path, data in sorted(fixture_files.items())],
            "write_set": [{"path": path, "operation": "delete" if data is None else "write", "sha256": _hash(data)} for path, data in sorted(writes.items())],
            "preconditions": [{"path": path, "sha256": digest} for path, digest in sorted(set(preconditions + source_preconditions))],
            "roundtrip": "passed: legacy normalized model equals converted v3 model at immutable source revision",
            "measurements": {"N_doc_paths": len(current_paths), "B_source_bytes": source_byte_count,
                             "planned_write_file_count": sum(value is not None for value in writes.values()),
                             "planned_delete_file_count": sum(value is None for value in writes.values()),
                             "planned_write_bytes": write_bytes,
                             "elapsed_seconds": round(elapsed, 6), "peak_rss": peak_rss, "peak_rss_units": peak_rss_units},
            "execution": {"mode": "apply" if args.apply else "plan", "exit_code": 0},
        }
    except dc.CorpusError:
        raise
    if args.apply:
        result = dc.apply_plan(plan)
        elapsed = time.perf_counter() - started
        report["measurements"]["elapsed_seconds"] = round(elapsed, 6)
        changed = set(result.changed_paths)
        report["measurements"]["actual_write_file_count"] = sum(data is not None and path in changed for path, data in writes.items())
        report["measurements"]["actual_delete_file_count"] = sum(data is None and path in changed for path, data in writes.items())
        report["measurements"]["actual_write_bytes"] = sum(len(data) for path, data in writes.items() if data is not None and path in result.changed_paths)
        try:
            import resource
            report["measurements"]["peak_rss"] = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        except (ImportError, AttributeError):
            report["measurements"]["peak_rss"] = None
        report["execution"].update({"applied": True, "changed_count": len(result.changed_paths), "unchanged_count": len(result.unchanged_paths)})
        report_path = _write_report(repo_root, args.report, report)
        _emit({"report": report_path, "applied": True, "changed_count": len(result.changed_paths), "unchanged_count": len(result.unchanged_paths), "source_revision": source_oid})
    else:
        report["measurements"]["actual_write_file_count"] = 0
        report["measurements"]["actual_delete_file_count"] = 0
        report["measurements"]["actual_write_bytes"] = 0
        report["execution"]["applied"] = False
        report_path = _write_report(repo_root, args.report, report)
        _emit({"report": report_path, "applied": False, "planned_count": len(plan.writes), "source_revision": source_oid})
    return 0


def command_import_delta(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve()
    base_view = dc.GitCorpusView(repo_root, args.base)
    head_view = dc.GitCorpusView(repo_root, args.legacy_head)
    target_view = dc.GitCorpusView(repo_root, args.target)
    if base_view.snapshot_kind != "commit" or head_view.snapshot_kind != "commit":
        raise dc.CorpusError(dc.Diagnostic("revision-invalid", detail="import-legacy-delta base and legacy head must be immutable commits"))
    base = _legacy_model(base_view)
    head = _legacy_model(head_view)
    target = dc.normalize_current(dc.load_corpus(target_view))
    proposal, conflicts = _three_way_proposal(base, head, target, target_view)
    if conflicts:
        code = "legacy-delta-source-drift" if any(item.startswith("legacy-delta-source-drift:") for item in conflicts) else "legacy-delta-conflict"
        raise dc.CorpusError(dc.Diagnostic(code, detail="; ".join(conflicts)))
    output = _write_report(repo_root, args.output, proposal)
    _emit({"proposal": output, "mutations": len(proposal["mutations"]), "base": base_view.snapshot_id,
           "legacy_head": head_view.snapshot_id, "target": target_view.snapshot_id,
           "base_snapshot": dc.snapshot_info(base_view), "legacy_head_snapshot": dc.snapshot_info(head_view),
           "target_snapshot": dc.snapshot_info(target_view)})
    return 0


def _legacy_model(view: dc.GitCorpusView) -> dict[str, Any]:
    return dc.normalize_legacy(_read_raw(view, dc.CORPUS_ROOT), _read_raw(view, dc.LEGACY_SEMANTIC_ROOT), _read_raw(view, dc.EVIDENCE_ROOT))


def _three_way_proposal(base: dict[str, Any], head: dict[str, Any], target: dict[str, Any], view: dc.GitCorpusView) -> tuple[dict[str, Any], list[str]]:
    mutations: list[dict[str, Any]] = []
    source_paths: set[str] = set()
    record_preconditions: set[str] = set()
    conflicts: list[str] = []
    entries_to_compare = (
        ("semantic-entry", "semantic_entries", "path"),
        ("semantic-bundle", "semantic_bundles", "id"),
    )
    for kind, name, identity in entries_to_compare:
        b = {item[identity]: item for item in base[name]}
        h = {item[identity]: item for item in head[name]}
        n = {item[identity]: item for item in target[name]}
        for key in sorted(set(b) | set(h) | set(n)):
            before, proposed, current = b.get(key), h.get(key), n.get(key)
            if proposed == before:
                chosen = current
            elif current == before or proposed == current:
                chosen = proposed
            else:
                conflicts.append(f"{name}:{key}")
                continue
            if chosen == current:
                continue
            if chosen is not None:
                drift = _legacy_record_source_drift(kind, chosen, view)
                if drift:
                    conflicts.append("legacy-delta-source-drift:" + drift)
                    continue
            if kind == "semantic-entry":
                path = dc.record_path("semantic", key)
            elif kind == "semantic-bundle":
                path = dc.bundle_record_path(key)
            record_preconditions.add(path)
            source_paths.update(_sources_for_record(kind, before or proposed or current or {}))
            if proposed is not None:
                mutations.append({"record_path": path, "operation": "upsert", "record": chosen})
            else:
                mutations.append({"record_path": path, "operation": "delete"})
    base_units = _evidence_units(base)
    head_units = _evidence_units(head)
    target_units = _evidence_units(target)
    for key in sorted(set(base_units) | set(head_units) | set(target_units)):
        before, proposed, current = base_units.get(key), head_units.get(key), target_units.get(key)
        if proposed == before:
            chosen = current
        elif current == before or proposed == current:
            chosen = proposed
        else:
            conflicts.append(f"evidence-atomic-unit:{key}")
            continue
        if chosen == current:
            continue
        unit_kind = (proposed or before or current)["kind"]
        unit_record = (proposed or before or current)["record"]
        if chosen is not None:
            drift = _legacy_record_source_drift(unit_kind, chosen["record"], view)
            if drift:
                conflicts.append("legacy-delta-source-drift:" + drift)
                continue
        if unit_kind == "evidence-group":
            path = dc.group_record_path(unit_record["id"])
        else:
            path = dc.record_path("evidence", unit_record["path"])
        record_preconditions.add(path)
        source_paths.update(_sources_for_record(unit_kind, unit_record))
        if proposed is not None:
            mutations.append({"record_path": path, "operation": "upsert", "record": chosen["record"]})
        else:
            mutations.append({"record_path": path, "operation": "delete"})
    proposal = {
        "schema": "oasis7.document-corpus-review-proposal/v1",
        "kind": "legacy-delta",
        "source_preconditions": [{"path": path, "sha256": _hash(view.read_bytes(path)) if view.file_mode(path) in {"100644", "100755"} else None} for path in sorted(source_paths)],
        "record_preconditions": [{"path": path, "sha256": _hash(view.read_bytes(path)) if view.file_mode(path) in {"100644", "100755"} else None} for path in sorted(record_preconditions)],
        "mutations": sorted(mutations, key=lambda item: item["record_path"]),
    }
    return proposal, conflicts


def _evidence_units(model: dict[str, Any]) -> dict[str, dict[str, Any]]:
    singles, groups = dc._evidence_group_records(model["evidence_entries"])
    result: dict[str, dict[str, Any]] = {}
    for record in singles:
        result["entry:" + record["path"]] = {"kind": "evidence-entry", "record": record}
    for record in groups:
        result["group:" + record["id"]] = {"kind": "evidence-group", "record": record}
    return result


def _legacy_record_source_drift(kind: str, record: dict[str, Any], view: dc.GitCorpusView) -> str | None:
    if kind in {"semantic-entry", "evidence-entry"}:
        paths = [record["path"]]
    elif kind == "semantic-bundle":
        paths = list(record["paths"])
    else:
        paths = [entry["path"] for entry in record["entries"]]
    for path in paths:
        if view.file_mode(path) not in {"100644", "100755"}:
            return f"{path}: source missing from target revision"
        expected = record.get("content_sha256") if kind in {"semantic-entry", "evidence-entry"} else None
        if kind == "evidence-group":
            expected = next((entry.get("content_sha256") for entry in record["entries"] if entry.get("path") == path), None)
        if expected is not None and expected != dc.sha256(view.read_bytes(path)):
            return f"{path}: reviewed source hash differs from target revision"
    if kind == "semantic-bundle" and dc._semantic_digest(view, paths) != record.get("content_set_sha256"):
        return f"{record.get('id')}: bundle source digest differs from target revision"
    if kind == "evidence-group":
        entries = record["entries"]
        if all(isinstance(entry.get("content_sha256"), str) for entry in entries):
            if dc._evidence_digest(entries) != record.get("group_sha256"):
                return f"{record.get('id')}: evidence group digest differs from target revision"
    return None


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    sub = parser.add_subparsers(dest="command", required=True)
    locate = sub.add_parser("locate")
    locate.add_argument("--kind", choices=("object", "semantic", "evidence"))
    locate.add_argument("--path", required=True)
    _view_flags(locate)
    locate.set_defaults(run=command_locate)
    sync = sub.add_parser("sync")
    sync.add_argument("--path", action="append")
    sync.add_argument("--deleted-path", action="append")
    sync.add_argument("--all", action="store_true")
    sync.add_argument("--apply", action="store_true")
    _view_flags(sync)
    sync.set_defaults(run=command_sync)
    propose = sub.add_parser("propose")
    propose.add_argument("--kind", choices=("semantic", "evidence"), required=True)
    selector = propose.add_mutually_exclusive_group(required=True)
    selector.add_argument("--path")
    selector.add_argument("--bundle")
    selector.add_argument("--group")
    propose.add_argument("--member", action="append", default=[])
    propose.add_argument("--output", required=True)
    _view_flags(propose)
    propose.set_defaults(run=command_propose)
    apply_review = sub.add_parser("apply-review")
    apply_review.add_argument("--proposal", required=True)
    apply_review.add_argument("--apply", action="store_true")
    apply_review.set_defaults(run=command_apply_review)
    export = sub.add_parser("export")
    export.add_argument("--output")
    _view_flags(export)
    export.set_defaults(run=command_export)
    legacy_export = sub.add_parser("export-legacy")
    legacy_export.add_argument("--output-dir", required=True)
    _view_flags(legacy_export)
    legacy_export.set_defaults(run=command_export_legacy)
    migrate = sub.add_parser("migrate-v2")
    migrate.add_argument("--source-revision", required=True)
    migrate.add_argument("--report", required=True)
    migrate.add_argument("--apply", action="store_true")
    migrate.set_defaults(run=command_migrate)
    delta = sub.add_parser("import-legacy-delta")
    delta.add_argument("--base", required=True)
    delta.add_argument("--legacy-head", required=True)
    delta.add_argument("--target", required=True)
    delta.add_argument("--output", required=True)
    delta.set_defaults(run=command_import_delta)
    for child in (locate, sync, propose, apply_review, export, legacy_export, migrate, delta):
        child.add_argument("--repo-root", type=Path, default=argparse.SUPPRESS)
    return parser


def _view_flags(parser: argparse.ArgumentParser) -> None:
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--revision")
    group.add_argument("--worktree", action="store_true")


def main() -> int:
    parser = build_parser()
    try:
        args = parser.parse_args()
        return args.run(args)
    except dc.CorpusError as exc:
        print(json.dumps({"status": "error", **exc.diagnostic.as_dict()}, ensure_ascii=False), file=sys.stderr)
        return EXIT_STALE if exc.diagnostic.code in {"proposal-stale", "concurrent-modification", "lock-busy"} else EXIT_DATA
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(json.dumps(_diagnostic("input-invalid", str(exc)), ensure_ascii=False), file=sys.stderr)
        return EXIT_INPUT


if __name__ == "__main__":
    raise SystemExit(main())
