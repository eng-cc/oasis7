#!/usr/bin/env python3
"""Shared, import-pure reader and writer for document corpus v3.

The module only reads the explicitly supplied CorpusView.  It never discovers a
repository, changes import paths, imports a checkout module, or writes on import.
"""

from __future__ import annotations

import errno
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import tempfile
import unicodedata
from dataclasses import dataclass, field
from typing import Any, Protocol, Sequence


CORPUS_ROOT = "doc/.governance/document-corpus-inventory.json"
EVIDENCE_ROOT = "doc/testing/evidence/inventory.json"
REGISTRY_PATH = "doc/.governance/top-level-directory-registry.json"
OBJECTS_ROOT = "doc/.governance/document-corpus/objects"
SEMANTIC_ENTRIES_ROOT = "doc/.governance/document-corpus/semantic/entries"
SEMANTIC_BUNDLES_ROOT = "doc/.governance/document-corpus/semantic/bundles"
EVIDENCE_ENTRIES_ROOT = "doc/.governance/document-corpus/evidence/entries"
EVIDENCE_GROUPS_ROOT = "doc/.governance/document-corpus/evidence/groups"
LEGACY_SEMANTIC_ROOT = "doc/.governance/document-semantic-review-overrides.json"
PRODUCT_MODULES = (
    "agents-world-simulation",
    "player-entry-distribution",
    "world-infrastructure",
    "world-rules-core-gameplay",
)
OBJECT_FIELDS = {
    "path", "content_sha256", "registry_type", "structural_owner", "authority_entry",
    "authority_layer", "semantic_decision_owner", "object_kind", "lifecycle_candidate",
    "inventory_disposition", "routing_batch", "routing_note",
}
SEMANTIC_FIELDS = {
    "path", "content_sha256", "disposition", "decision_owner", "current_authority", "note",
}
SEMANTIC_OPTIONAL = {"retirement_gate"}
SEMANTIC_BUNDLE_FIELDS = {
    "id", "paths", "content_set_sha256", "disposition", "decision_owner", "current_authority", "note",
}
EVIDENCE_FIELDS = {
    "path", "lifecycle", "semantic_role", "retention_owner", "domain_owner",
    "required_followup_roles", "authority", "backlink", "disposition", "rationale", "residual_risk",
}
EVIDENCE_OPTIONAL = {
    "evidence_window", "observation_window", "atomic_group", "group_sha256",
    "content_sha256", "claim_boundary", "semantic_absorption", "reference_repair",
    "owner_approval", "validation",
}
SCHEMAS = {
    "object": "oasis7.document-corpus-object/v1",
    "semantic-entry": "oasis7.document-semantic-entry/v1",
    "semantic-bundle": "oasis7.document-semantic-bundle/v1",
    "evidence-entry": "oasis7.document-evidence-entry/v1",
    "evidence-group": "oasis7.document-evidence-group/v1",
}
SHA_RE = re.compile(r"^[0-9a-f]{64}$")
BUNDLE_RE = re.compile(r"^[a-z0-9-]{1,96}$")
SEMANTIC_DISPOSITIONS = {
    "retain_current_authority", "retain_active_template", "retain_repeatable_procedure",
    "retain_historical_evidence_linked", "controlled_exception_empty_pool",
    "retain_current_design_supplement", "retain_comparison_target", "retain_supporting_artifact",
    "retain_current_procedure", "retain_dated_window_evidence", "retain_dated_window_evidence_asset",
    "retain_historical_governance_decision", "retain_current_target_contract",
    "retain_outstanding_fulfillment_obligation", "retain_historical_campaign_material",
    "retain_explanatory_overview", "retain_unpublished_external_draft", "retain_historical_visual_asset",
    "retain_current_companion_authority", "retain_non_authoritative_ideation", "retain_mixed_contract_snapshot",
    "retain_historical_decision_background", "retain_historical_benchmark", "retain_mixed_runbook_observation",
    "retain_legacy_rehearsal_provenance", "retain_current_controlled_runbook", "retain_active_template_or_example",
    "retain_executable_test_plan", "retain_current_governance_control", "retain_current_channel_material",
    "retain_unpublished_external_support_bundle", "retain_current_design_companion",
    "retain_mixed_implementation_record", "retain_orphan_draft_design", "retain_historical_validation_index",
    "retain_current_interface_authority",
}
SEMANTIC_OWNERS = {
    "qa_engineer", "gameplay_designer", "game_visual_interaction_designer", "repository_health_engineer",
    "viewer_engineer", "blockchain_ops_engineer", "runtime_engineer", "liveops_community",
    "producer_system_designer", "agent_engineer", "wasm_platform_engineer",
}
EVIDENCE_LIFECYCLES = {
    "CURRENT_NAVIGATION", "CURRENT_OPERATOR_INPUT", "WINDOW_OBSERVATION", "HISTORICAL_PROVENANCE",
    "AMBIGUOUS_LIFECYCLE", "ARCHIVED_PROVENANCE", "SUPPORTING_ARTIFACT", "TEMPLATE_NOT_EVIDENCE",
}
EVIDENCE_ROLES = SEMANTIC_OWNERS
EVIDENCE_DISPOSITIONS = {"retain", "needs_domain_decision", "delete_candidate"}
EVIDENCE_SUFFIXES = {".md", ".txt", ".json", ".jsonl", ".csv", ".tsv", ".png", ".jpg", ".jpeg", ".gif", ".webp"}
MAX_METADATA_BYTES = 8 * 1024 * 1024
MAX_JSON_DEPTH = 32
MAX_RECORDS = 100_000
TOOL_VERSION = "3.0.0"


class CorpusError(ValueError):
    def __init__(self, diagnostic: "Diagnostic") -> None:
        super().__init__(diagnostic.detail)
        self.diagnostic = diagnostic


@dataclass(frozen=True)
class Diagnostic:
    code: str
    source_path: str | None = None
    record_path: str | None = None
    detail: str = ""
    repair_command: str = ""

    def as_dict(self) -> dict[str, Any]:
        return {
            "code": self.code,
            "source_path": self.source_path,
            "record_path": self.record_path,
            "detail": self.detail,
            "repair_command": self.repair_command,
        }


@dataclass(frozen=True)
class RecordLocation:
    kind: str
    source_path: str
    record_path: str
    member_paths: tuple[str, ...] = ()
    record_id: str | None = None


@dataclass
class CorpusModel:
    corpus_entry: dict[str, Any]
    evidence_entrypoint: dict[str, Any]
    objects_by_path: dict[str, dict[str, Any]] = field(default_factory=dict)
    semantic_entries_by_path: dict[str, dict[str, Any]] = field(default_factory=dict)
    semantic_bundles_by_id: dict[str, dict[str, Any]] = field(default_factory=dict)
    evidence_entries_by_path: dict[str, dict[str, Any]] = field(default_factory=dict)
    evidence_groups_by_id: dict[str, dict[str, Any]] = field(default_factory=dict)
    path_to_bundle: dict[str, str] = field(default_factory=dict)
    path_to_group: dict[str, str] = field(default_factory=dict)
    metadata_paths: set[str] = field(default_factory=set)
    diagnostics: list[Diagnostic] = field(default_factory=list)
    view_id: str | None = None


class CorpusView(Protocol):
    """Immutable Git snapshot or explicitly selected working tree."""

    snapshot_id: str | None
    snapshot_kind: str
    tree_id: str | None
    commit_id: str | None

    def list_doc_paths(self) -> Sequence[str]: ...
    def read_bytes(self, path: str) -> bytes: ...
    def file_mode(self, path: str) -> str: ...


class GitCorpusView:
    def __init__(self, repo_root: str | Path, revision: str) -> None:
        self.repo_root = Path(repo_root).resolve()
        if not revision or revision.startswith("-") or not re.fullmatch(r"[0-9a-fA-F]{7,64}", revision):
            raise CorpusError(Diagnostic("revision-invalid", detail=f"invalid immutable revision: {revision!r}"))
        try:
            object_id = self._git(["rev-parse", "--verify", f"{revision}^{{object}}"], text=True).strip()
            object_kind = self._git(["cat-file", "-t", object_id], text=True).strip()
        except CorpusError as exc:
            raise CorpusError(Diagnostic("revision-invalid", detail=f"revision does not identify a Git commit or tree: {revision!r}")) from exc
        if object_kind not in {"commit", "tree"}:
            raise CorpusError(Diagnostic("revision-invalid", detail=f"revision identifies {object_kind}, expected commit or tree"))
        self.snapshot_id = object_id
        self.snapshot_kind = object_kind
        self.commit_id = object_id if object_kind == "commit" else None
        self.tree_id = (self._git(["rev-parse", "--verify", f"{object_id}^{{tree}}"], text=True).strip()
                        if object_kind == "commit" else object_id)
        self._files: dict[str, tuple[str, str]] = {}
        self._reads: dict[str, bytes] = {}
        raw = self._git(["ls-tree", "-r", "-z", "--full-tree", self.tree_id])
        for item in raw.split(b"\0"):
            if not item:
                continue
            meta, name = item.split(b"\t", 1)
            mode, kind, oid = meta.decode("ascii").split()
            try:
                path = name.decode("utf-8", "strict")
            except UnicodeDecodeError as exc:
                raise CorpusError(Diagnostic("unsafe-path", detail=f"Git tree contains a non-UTF-8 path: {name!r}")) from exc
            self._files[path] = (mode, oid)

    def _git(self, args: list[str], *, text: bool = False) -> Any:
        result = subprocess.run(["git", "-C", str(self.repo_root), *args], check=False, capture_output=True, text=text)
        if result.returncode:
            output = result.stderr.strip() if text else result.stderr.decode("utf-8", "replace").strip()
            raise CorpusError(Diagnostic("git-view-failed", detail=output))
        return result.stdout

    def list_doc_paths(self) -> Sequence[str]:
        return tuple(sorted(path for path in self._files if path.startswith("doc/")))

    def read_bytes(self, path: str) -> bytes:
        if path in self._reads:
            return self._reads[path]
        validate_repo_path(path)
        if path not in self._files or self._files[path][0] not in {"100644", "100755"}:
            raise CorpusError(Diagnostic("source-unavailable", source_path=path, detail="path is not a regular blob in this revision"))
        data = self._git(["cat-file", "blob", self._files[path][1]])
        self._reads[path] = data
        return data

    def file_mode(self, path: str) -> str:
        validate_repo_path(path)
        return self._files.get(path, ("000000", ""))[0]


class WorktreeCorpusView:
    def __init__(self, repo_root: str | Path) -> None:
        self.repo_root = Path(repo_root).resolve()
        self.snapshot_id = None
        self.snapshot_kind = "worktree"
        self.tree_id = None
        self.commit_id = None
        self._paths: tuple[str, ...] | None = None
        self._reads: dict[str, bytes] = {}

    def list_doc_paths(self) -> Sequence[str]:
        if self._paths is not None:
            return self._paths
        found: list[str] = []
        doc = self.repo_root / "doc"
        if not doc.exists():
            self._paths = ()
            return self._paths
        for current, dirs, files in os.walk(doc, topdown=True, followlinks=False):
            parent = Path(current)
            keep: list[str] = []
            for name in dirs:
                item = parent / name
                if item.is_symlink():
                    found.append(item.relative_to(self.repo_root).as_posix())
                else:
                    keep.append(name)
            dirs[:] = keep
            for name in files:
                found.append((parent / name).relative_to(self.repo_root).as_posix())
        self._paths = tuple(sorted(found))
        return self._paths

    def _path(self, path: str) -> Path:
        validate_repo_path(path)
        target = self.repo_root
        for part in path.split("/"):
            target = target / part
            try:
                if target.is_symlink():
                    raise CorpusError(Diagnostic("unsafe-path", source_path=path, detail="symlink path component"))
            except OSError as exc:
                raise CorpusError(Diagnostic("source-unavailable", source_path=path, detail=str(exc))) from exc
        if not target.resolve(strict=False).is_relative_to(self.repo_root):
            raise CorpusError(Diagnostic("unsafe-path", source_path=path, detail="path escapes repository root"))
        return target

    def read_bytes(self, path: str) -> bytes:
        if path in self._reads:
            return self._reads[path]
        target = self._path(path)
        try:
            if not target.is_file():
                raise CorpusError(Diagnostic("source-unavailable", source_path=path, detail="not a regular file"))
            result = target.read_bytes()
            self._reads[path] = result
            return result
        except OSError as exc:
            raise CorpusError(Diagnostic("source-unavailable", source_path=path, detail=str(exc))) from exc

    def verify_unchanged(self) -> list[Diagnostic]:
        current = WorktreeCorpusView(self.repo_root)
        errors: list[Diagnostic] = []
        if tuple(current.list_doc_paths()) != self.list_doc_paths():
            errors.append(Diagnostic("concurrent-modification", detail="doc file set changed during check"))
        for path, earlier in self._reads.items():
            try:
                later = current.read_bytes(path)
            except CorpusError:
                errors.append(Diagnostic("concurrent-modification", source_path=path, detail="read source disappeared during check"))
                continue
            if sha256(later) != sha256(earlier):
                errors.append(Diagnostic("concurrent-modification", source_path=path, detail="read source changed during check"))
        return errors

    def file_mode(self, path: str) -> str:
        target = self._path(path)
        try:
            mode = target.lstat().st_mode
        except OSError:
            return "000000"
        if stat.S_ISLNK(mode):
            return "120000"
        if stat.S_ISREG(mode):
            return "100755" if mode & stat.S_IXUSR else "100644"
        if stat.S_ISDIR(mode):
            return "040000"
        return "000000"


class OverlayCorpusView:
    """In-memory file overlay used to prove an immutable migration round trip."""

    def __init__(self, base: CorpusView, writes: dict[str, bytes | None]) -> None:
        self.base = base
        self.writes = writes
        self.snapshot_id = base.snapshot_id
        self.snapshot_kind = base.snapshot_kind
        self.tree_id = base.tree_id
        self.commit_id = base.commit_id

    def list_doc_paths(self) -> Sequence[str]:
        paths = set(self.base.list_doc_paths())
        for path, value in self.writes.items():
            if not path.startswith("doc/"):
                continue
            if value is None:
                paths.discard(path)
            else:
                paths.add(path)
        return tuple(sorted(paths))

    def read_bytes(self, path: str) -> bytes:
        if path in self.writes:
            value = self.writes[path]
            if value is None:
                raise CorpusError(Diagnostic("source-unavailable", source_path=path, detail="deleted in overlay"))
            return value
        return self.base.read_bytes(path)

    def file_mode(self, path: str) -> str:
        if path in self.writes:
            return "000000" if self.writes[path] is None else "100644"
        return self.base.file_mode(path)


def snapshot_info(view: CorpusView) -> dict[str, str | None]:
    """Report the selected snapshot without implying that a tree has a commit."""
    return {"snapshot_id": view.snapshot_id, "snapshot_kind": view.snapshot_kind,
            "tree_id": view.tree_id, "commit_id": view.commit_id}


def validate_repo_path(path: str) -> str:
    if not isinstance(path, str) or not path or "\\" in path or "\x00" in path:
        raise CorpusError(Diagnostic("unsafe-path", source_path=path if isinstance(path, str) else None, detail="expected non-empty POSIX path"))
    try:
        path.encode("utf-8", "strict")
    except UnicodeEncodeError as exc:
        raise CorpusError(Diagnostic("unsafe-path", source_path=path, detail="path is not UTF-8")) from exc
    pure = PurePosixPath(path)
    if pure.is_absolute() or re.match(r"^[A-Za-z]:", path) or any(part in {"", ".", ".."} for part in path.split("/")):
        raise CorpusError(Diagnostic("unsafe-path", source_path=path, detail="absolute, drive, empty, dot, or parent segment"))
    if any(ord(char) < 32 or ord(char) == 127 for char in path):
        raise CorpusError(Diagnostic("unsafe-path", source_path=path, detail="control character in path"))
    return path


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def ensure_hash_input(view: CorpusView, source_path: str) -> None:
    """Reject a staged/filtered byte stream that differs from reviewed bytes."""
    ensure_hash_inputs(view, [source_path])


def ensure_hash_inputs(view: CorpusView, source_paths: Sequence[str]) -> None:
    """Batch source/filter/index checks for a planned set of worktree inputs."""
    if not isinstance(view, WorktreeCorpusView):
        return
    paths = sorted(set(validate_repo_path(path) for path in source_paths))
    if not paths:
        return
    root = str(view.repo_root)
    staged = subprocess.run(["git", "-C", root, "diff", "--cached", "--name-only", "--no-renames", "-z"], capture_output=True, check=False)
    attr_input = b"\0".join(path.encode("utf-8") for path in paths) + b"\0"
    attrs = subprocess.run(["git", "-C", root, "check-attr", "-z", "--stdin", "filter", "text", "eol", "working-tree-encoding"], input=attr_input, capture_output=True, check=False)
    autocrlf = subprocess.run(["git", "-C", root, "config", "--get", "core.autocrlf"], capture_output=True, text=True, check=False)
    if staged.returncode or attrs.returncode or autocrlf.returncode not in {0, 1}:
        detail_bytes = staged.stderr or attrs.stderr or (autocrlf.stderr or "").encode()
        detail = detail_bytes.decode("utf-8", "replace").strip()
        raise CorpusError(Diagnostic("hash-input-mismatch", detail=f"cannot inspect Git staged/filter state: {detail}"))
    staged_paths = {name for name in staged.stdout.split(b"\0") if name}
    attr_values: dict[tuple[str, str], str] = {}
    chunks = attrs.stdout.split(b"\0")
    for i in range(0, len(chunks) - 2, 3):
        if chunks[i]:
            attr_values[(chunks[i].decode("utf-8"), chunks[i + 1].decode("ascii"))] = chunks[i + 2].decode("utf-8")
    autocrlf_enabled = autocrlf.returncode == 0 and autocrlf.stdout.strip().lower() in {"true", "input"}
    for source_path in paths:
        data = view.read_bytes(source_path)
        filter_value = attr_values.get((source_path, "filter"), "unspecified")
        if filter_value not in {"unspecified", "unset"}:
            raise CorpusError(Diagnostic("hash-input-mismatch", source_path=source_path, detail="custom Git clean filters are not executed by corpus management"))
        effective = any(attr_values.get((source_path, name), "unspecified") not in {"unspecified", "unset"} for name in ("text", "eol", "working-tree-encoding"))
        if effective or autocrlf_enabled:
            filtered = subprocess.run(["git", "-C", root, "hash-object", f"--path={source_path}", "--stdin"], input=data, capture_output=True, check=False)
            raw = subprocess.run(["git", "-C", root, "hash-object", "--no-filters", "--stdin"], input=data, capture_output=True, check=False)
            if filtered.returncode or raw.returncode:
                detail = (filtered.stderr or raw.stderr).decode("utf-8", "replace").strip()
                raise CorpusError(Diagnostic("hash-input-mismatch", source_path=source_path, detail=f"cannot compare Git clean-filter input: {detail}"))
            if filtered.stdout.strip() != raw.stdout.strip():
                raise CorpusError(Diagnostic("hash-input-mismatch", source_path=source_path, detail="Git clean filters would change the bytes whose SHA-256 is recorded"))
        if source_path.encode("utf-8") in staged_paths:
            index_blob = subprocess.run(["git", "-C", root, "cat-file", "blob", f":{source_path}"], capture_output=True, check=False)
            if index_blob.returncode or index_blob.stdout != data:
                raise CorpusError(Diagnostic("hash-input-mismatch", source_path=source_path, detail="staged blob bytes differ from the source bytes used by this operation"))


def path_key(path: str) -> str:
    return sha256(validate_repo_path(path).encode("utf-8"))


def record_path(kind: str, source_path: str) -> str:
    key = path_key(source_path)
    if kind in {"object", "semantic", "semantic-entry"}:
        root = OBJECTS_ROOT if kind == "object" else SEMANTIC_ENTRIES_ROOT
        return f"{root}/{key[:2]}/{key}.json"
    if kind in {"evidence", "evidence-entry"}:
        return f"{EVIDENCE_ENTRIES_ROOT}/{key[:2]}/{key}.json"
    if kind == "semantic-bundle":
        raise ValueError("bundle record paths require a bundle id")
    if kind == "evidence-group":
        raise ValueError("group record paths require a group id")
    raise ValueError(f"unknown record kind: {kind}")


def bundle_record_path(bundle_id: str) -> str:
    if not isinstance(bundle_id, str) or not BUNDLE_RE.fullmatch(bundle_id):
        raise CorpusError(Diagnostic("unsafe-record-id", detail=f"invalid bundle id {bundle_id!r}"))
    return f"{SEMANTIC_BUNDLES_ROOT}/{bundle_id}.json"


def group_record_path(group_id: str) -> str:
    if not isinstance(group_id, str) or not BUNDLE_RE.fullmatch(group_id):
        raise CorpusError(Diagnostic("unsafe-record-id", detail=f"invalid group id {group_id!r}"))
    return f"{EVIDENCE_GROUPS_ROOT}/{group_id}.json"


def _object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CorpusError(Diagnostic("duplicate-json-key", detail=f"duplicate JSON key {key!r}"))
        result[key] = value
    return result


def strict_json(data: bytes, path: str) -> Any:
    if len(data) > MAX_METADATA_BYTES:
        raise CorpusError(Diagnostic("metadata-size-limit", record_path=path, detail=f"metadata exceeds {MAX_METADATA_BYTES} bytes"))
    try:
        value = json.loads(
            data.decode("utf-8", "strict"),
            object_pairs_hook=_object_pairs,
            parse_constant=lambda item: (_ for _ in ()).throw(CorpusError(Diagnostic("json-nonfinite-number", record_path=path, detail=item))),
        )
    except CorpusError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CorpusError(Diagnostic("json-invalid", record_path=path, detail=str(exc))) from exc
    stack: list[tuple[Any, int]] = [(value, 1)]
    count = 0
    while stack:
        item, depth = stack.pop()
        count += 1
        if depth > MAX_JSON_DEPTH:
            raise CorpusError(Diagnostic("json-depth-limit", record_path=path, detail=f"maximum depth is {MAX_JSON_DEPTH}"))
        if count > MAX_RECORDS * 64:
            raise CorpusError(Diagnostic("json-object-limit", record_path=path, detail="metadata object budget exceeded"))
        if isinstance(item, dict):
            stack.extend((child, depth + 1) for child in item.values())
        elif isinstance(item, list):
            stack.extend((child, depth + 1) for child in item)
    return value


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2, separators=(",", ": ")) + "\n").encode("utf-8")


def _exact_keys(value: Any, required: set[str], optional: set[str], path: str) -> None:
    if not isinstance(value, dict):
        raise CorpusError(Diagnostic("schema-record", record_path=path, detail="record must be an object"))
    keys = set(value)
    if required - keys or keys - required - optional:
        raise CorpusError(Diagnostic("schema-record", record_path=path, detail=f"missing={sorted(required - keys)} unknown={sorted(keys - required - optional)}"))


def _load_wrapped(view: CorpusView, path: str, kind: str, required: set[str], optional: set[str] = frozenset()) -> dict[str, Any]:
    raw = view.read_bytes(path)
    data = strict_json(raw, path)
    if canonical_json(data) != raw:
        raise CorpusError(Diagnostic("json-noncanonical", record_path=path, detail="metadata must be canonical UTF-8 JSON with LF and one final newline"))
    if not isinstance(data, dict) or set(data) != {"schema", "record"} or data.get("schema") != SCHEMAS[kind]:
        raise CorpusError(Diagnostic("schema-version", record_path=path, detail=f"expected {SCHEMAS[kind]} wrapper"))
    _exact_keys(data["record"], required, set(optional), path)
    return data["record"]


def _metadata_paths(paths: Sequence[str]) -> set[str]:
    roots = (
        OBJECTS_ROOT + "/", SEMANTIC_ENTRIES_ROOT + "/", SEMANTIC_BUNDLES_ROOT + "/",
        EVIDENCE_ENTRIES_ROOT + "/", EVIDENCE_GROUPS_ROOT + "/",
    )
    return {path for path in paths if path in {CORPUS_ROOT, EVIDENCE_ROOT} or path.startswith(roots)}


def _load_static(view: CorpusView, path: str, *, kind: str) -> dict[str, Any]:
    raw = view.read_bytes(path)
    value = strict_json(raw, path)
    if canonical_json(value) != raw:
        raise CorpusError(Diagnostic("json-noncanonical", record_path=path, detail="static metadata must use canonical UTF-8 JSON with LF and one final newline"))
    if not isinstance(value, dict):
        raise CorpusError(Diagnostic("schema-record", record_path=path, detail="static root must be an object"))
    if kind == "corpus":
        expected = {
            "version": 3,
            "scope": "all doc files through direct objects, delegated evidence objects, and controls",
            "decision_boundary": "routing is conservative heuristic input only",
            "product_modules": list(PRODUCT_MODULES),
            "objects_root": OBJECTS_ROOT,
            "semantic_entries_root": SEMANTIC_ENTRIES_ROOT,
            "semantic_bundles_root": SEMANTIC_BUNDLES_ROOT,
            "delegates": [{"kind": "testing-evidence", "path": EVIDENCE_ROOT, "expected_version": 2}],
        }
        code = "corpus-entrypoint-contract"
    else:
        expected = {
            "version": 2,
            "scope": "doc/testing/evidence/** excluding inventory.json",
            "entries_root": EVIDENCE_ENTRIES_ROOT,
            "groups_root": EVIDENCE_GROUPS_ROOT,
            "legacy_baseline_id": "document-evidence-v1-at-v3-migration",
        }
        code = "evidence-entrypoint-contract"
    if value != expected:
        raise CorpusError(Diagnostic(code, record_path=path, detail="static entrypoint differs from the fixed v3 contract"))
    return value


def load_corpus(view: CorpusView) -> CorpusModel:
    """Strictly load metadata and construct indexes from this one immutable view."""
    paths = tuple(view.list_doc_paths())
    if LEGACY_SEMANTIC_ROOT in paths:
        raise CorpusError(Diagnostic("legacy-format-disabled", record_path=LEGACY_SEMANTIC_ROOT, detail="legacy whole-table semantic overlay is not an active v3 input"))
    if len(paths) > MAX_RECORDS:
        raise CorpusError(Diagnostic("record-budget", detail=f"doc contains more than {MAX_RECORDS} paths"))
    folded: dict[str, str] = {}
    normalized: dict[str, str] = {}
    for path in paths:
        validate_repo_path(path)
        mode = view.file_mode(path)
        if mode not in {"100644", "100755"}:
            raise CorpusError(Diagnostic("unsafe-file-mode", source_path=path, detail=f"expected regular file, got mode {mode}"))
        fold = path.casefold()
        norm = unicodedata.normalize("NFC", path)
        if fold in folded and folded[fold] != path or norm in normalized and normalized[norm] != path:
            raise CorpusError(Diagnostic("path-alias-collision", source_path=path, detail=f"aliases {folded.get(fold) or normalized.get(norm)}"))
        folded[fold] = path
        normalized[norm] = path
    metadata_paths = _metadata_paths(paths)
    for path in paths:
        if path.startswith("doc/.governance/document-corpus/") and path not in metadata_paths:
            raise CorpusError(Diagnostic("unknown-metadata-file", record_path=path, detail="unknown file inside corpus metadata root"))
    model = CorpusModel(
        corpus_entry=_load_static(view, CORPUS_ROOT, kind="corpus"),
        evidence_entrypoint=_load_static(view, EVIDENCE_ROOT, kind="evidence"),
        metadata_paths=metadata_paths,
        view_id=view.snapshot_id,
    )
    expected_prefixes = {
        OBJECTS_ROOT + "/": ("object", OBJECT_FIELDS, set()),
        SEMANTIC_ENTRIES_ROOT + "/": ("semantic-entry", SEMANTIC_FIELDS, SEMANTIC_OPTIONAL),
        SEMANTIC_BUNDLES_ROOT + "/": ("semantic-bundle", SEMANTIC_BUNDLE_FIELDS, set()),
        EVIDENCE_ENTRIES_ROOT + "/": ("evidence-entry", EVIDENCE_FIELDS, EVIDENCE_OPTIONAL),
        EVIDENCE_GROUPS_ROOT + "/": ("evidence-group", {"id", "group_sha256", "entries"}, set()),
    }
    listed = set(paths)
    for path in sorted(metadata_paths - {CORPUS_ROOT, EVIDENCE_ROOT}):
        match = next(((prefix, spec) for prefix, spec in expected_prefixes.items() if path.startswith(prefix)), None)
        if match is None:
            raise CorpusError(Diagnostic("unknown-metadata-file", record_path=path, detail="metadata path is outside known roots"))
        prefix, (kind, required, optional) = match
        name = path[len(prefix):]
        if not name.endswith(".json") or "/" in name and kind in {"semantic-bundle", "evidence-group"}:
            raise CorpusError(Diagnostic("unknown-metadata-file", record_path=path, detail="unexpected metadata filename"))
        if kind in {"object", "semantic-entry", "evidence-entry"}:
            parts = name.split("/")
            if len(parts) != 2 or not re.fullmatch(r"[0-9a-f]{2}", parts[0]) or not re.fullmatch(r"[0-9a-f]{64}\.json", parts[1]):
                raise CorpusError(Diagnostic("unknown-metadata-file", record_path=path, detail="shard filename is not <hh>/<path-key>.json"))
        record = _load_wrapped(view, path, kind, required, optional)
        if kind == "object":
            source = validate_repo_path(record["path"])
            if path != record_path("object", source):
                raise CorpusError(Diagnostic("wrong-record-key", source_path=source, record_path=path, detail="object shard path does not match source path"))
            model.objects_by_path[source] = record
        elif kind == "semantic-entry":
            source = validate_repo_path(record["path"])
            if path != record_path("semantic", source):
                raise CorpusError(Diagnostic("wrong-record-key", source_path=source, record_path=path, detail="semantic shard path does not match source path"))
            model.semantic_entries_by_path[source] = record
        elif kind == "semantic-bundle":
            if path != bundle_record_path(record.get("id", "")):
                raise CorpusError(Diagnostic("wrong-record-key", record_path=path, detail="bundle filename does not match id"))
            model.semantic_bundles_by_id[record["id"]] = record
        elif kind == "evidence-entry":
            source = validate_repo_path(record["path"])
            if path != record_path("evidence", source):
                raise CorpusError(Diagnostic("wrong-record-key", source_path=source, record_path=path, detail="evidence shard path does not match source path"))
            model.evidence_entries_by_path[source] = record
        else:
            record_id = record.get("id")
            if path != group_record_path(record_id or ""):
                raise CorpusError(Diagnostic("wrong-record-key", record_path=path, detail="group filename does not match id"))
            model.evidence_groups_by_id[record_id] = record
    if len(model.objects_by_path) + len(model.semantic_entries_by_path) + len(model.semantic_bundles_by_id) + len(model.evidence_entries_by_path) + len(model.evidence_groups_by_id) > MAX_RECORDS:
        raise CorpusError(Diagnostic("record-budget", detail=f"metadata exceeds {MAX_RECORDS} records"))
    for collection, reverse, id_field in (
        (model.semantic_bundles_by_id, model.path_to_bundle, "paths"),
        (model.evidence_groups_by_id, model.path_to_group, "entries"),
    ):
        for record_id, record in collection.items():
            members = record[id_field]
            if id_field == "entries" and isinstance(members, list):
                members = [entry.get("path") if isinstance(entry, dict) else None for entry in members]
            if not isinstance(members, list) or not members or any(not isinstance(p, str) for p in members):
                raise CorpusError(Diagnostic("schema-record", detail=f"{record_id}: members must be nonempty string list"))
            if members != sorted(members) or len(members) != len(set(members)):
                raise CorpusError(Diagnostic("record-members-invalid", detail=f"{record_id}: members must be sorted and unique"))
            for source in members:
                validate_repo_path(source)
                if source in reverse:
                    raise CorpusError(Diagnostic("duplicate-path", source_path=source, detail="member appears in more than one group"))
                reverse[source] = record_id
    overlap = set(model.semantic_entries_by_path) & set(model.path_to_bundle)
    if overlap:
        raise CorpusError(Diagnostic("semantic-membership-overlap", source_path=sorted(overlap)[0], detail="path is both entry and bundle member"))
    overlap = set(model.evidence_entries_by_path) & set(model.path_to_group)
    if overlap:
        raise CorpusError(Diagnostic("evidence-membership-overlap", source_path=sorted(overlap)[0], detail="path is both entry and group member"))
    return model


def _registry(view: CorpusView) -> dict[str, dict[str, str]]:
    data = strict_json(view.read_bytes(REGISTRY_PATH), REGISTRY_PATH)
    if not isinstance(data, dict) or not isinstance(data.get("directories"), list):
        raise CorpusError(Diagnostic("registry-invalid", record_path=REGISTRY_PATH, detail="expected directories list"))
    entries: dict[str, dict[str, str]] = {}
    for item in data["directories"]:
        if not isinstance(item, dict) or not isinstance(item.get("name"), str) or item["name"] in entries:
            raise CorpusError(Diagnostic("registry-invalid", record_path=REGISTRY_PATH, detail="invalid or duplicate directory entry"))
        entries[item["name"]] = item
    return entries


def object_kind(path: str) -> str:
    name = PurePosixPath(path).name
    for suffix, kind in ((".prd.md", "prd"), (".design.md", "design"), (".manual.md", "manual"), (".runbook.md", "runbook"), (".migration.md", "migration")):
        if name.endswith(suffix):
            return kind
    if name == "README.md":
        return "landing"
    if name == "prd.index.md":
        return "prd_index"
    return "markdown_record" if name.endswith(".md") else "supporting_artifact"


def lifecycle(path: str, kind: str) -> str:
    name = PurePosixPath(path).name
    if any(value in name for value in ("archive", "legacy", "historical", "-2026-", "-2025-", "-2024-")):
        return "historical_candidate"
    if path.startswith("doc/product/") or kind in {"prd", "design", "manual", "runbook", "landing", "prd_index"}:
        return "active_candidate"
    return "review_required"


def _authority_layer(registry_type: str) -> str:
    if registry_type == "product_overlay":
        return "product_overlay"
    if registry_type == "evidence_domain":
        return "evidence_domain"
    if registry_type in {"retired_archive", "ephemeral_evidence_pool"}:
        return "controlled_exception"
    return "professional_domain"


def expected_object(view: CorpusView, source_path: str) -> dict[str, Any]:
    validate_repo_path(source_path)
    entries = _registry(view)
    parts = source_path.split("/")
    top = parts[1] if len(parts) > 2 else ""
    if top == "":
        contract = {"registry_type": "doc_root", "structural_owner": "repository_health_engineer", "authority_entry": "doc/README.md", "authority_layer": "professional_domain"}
    elif top == ".governance":
        contract = {"registry_type": "governance_control", "structural_owner": "repository_health_engineer", "authority_entry": "doc/engineering/doc-governance/README.md", "authority_layer": "controlled_exception"}
    else:
        entry = entries.get(top)
        if entry is None:
            raise CorpusError(Diagnostic("registry-routing-drift", source_path=source_path, record_path=REGISTRY_PATH, detail=f"unregistered top-level directory {top}"))
        contract = {"registry_type": entry["type"], "structural_owner": entry["owner"], "authority_entry": entry["entry"], "authority_layer": _authority_layer(entry["type"])}
    kind = object_kind(source_path)
    return {
        "path": source_path,
        "content_sha256": sha256(view.read_bytes(source_path)),
        **contract,
        "semantic_decision_owner": contract["structural_owner"],
        "object_kind": kind,
        "lifecycle_candidate": lifecycle(source_path, kind),
        "inventory_disposition": "retain_for_owner_review",
        "routing_batch": f"{top or 'doc-root'}-corpus-review",
        "routing_note": "heuristic routing only; no lifecycle, authority, migration, or deletion decision is authorized by this inventory",
    }


def _semantic_digest(view: CorpusView, paths: Sequence[str]) -> str:
    material = b"".join(path.encode("utf-8") + b"\0" + sha256(view.read_bytes(path)).encode("ascii") + b"\n" for path in sorted(paths))
    return sha256(material)


def _evidence_digest(records: Sequence[dict[str, Any]]) -> str:
    material = "".join(f"{record['content_sha256']}  {record['path']}\n" for record in sorted(records, key=lambda x: x["path"]))
    return sha256(material.encode("utf-8"))


def _is_evidence_source(path: str) -> bool:
    return path.startswith("doc/testing/evidence/") and path != EVIDENCE_ROOT


def validate_corpus(view: CorpusView, model: CorpusModel | None = None) -> list[Diagnostic]:
    model = model or load_corpus(view)
    errors: list[Diagnostic] = list(model.diagnostics)
    paths = set(view.list_doc_paths())
    metadata = model.metadata_paths
    evidence_sources = {p for p in paths if _is_evidence_source(p) and p not in metadata}
    for path in sorted(evidence_sources):
        if PurePosixPath(path).suffix.lower() not in EVIDENCE_SUFFIXES:
            errors.append(Diagnostic("evidence-source-type", source_path=path, detail="unsupported evidence source type; executable and arbitrary binary files are not evidence assets"))
    direct_sources = paths - metadata - evidence_sources
    object_paths = set(model.objects_by_path)
    if len(object_paths) != len(model.objects_by_path):
        errors.append(Diagnostic("duplicate-path", record_path=OBJECTS_ROOT, detail="duplicate object path"))
    for path in sorted(direct_sources - object_paths):
        errors.append(Diagnostic("new-path-drift", source_path=path, detail="source has no object shard", repair_command=f"python3 scripts/document-corpus-inventory.py sync --path {path} --apply"))
    for path in sorted(object_paths - direct_sources):
        errors.append(Diagnostic("missing-path-drift", source_path=path, record_path=record_path("object", path), detail="object has no direct source"))
    for path in sorted(direct_sources & object_paths):
        try:
            expected = expected_object(view, path)
        except CorpusError as exc:
            errors.append(exc.diagnostic)
            continue
        if model.objects_by_path[path] != expected:
            expected_owner = expected.get("structural_owner")
            code = "registry-routing-drift" if model.objects_by_path[path].get("structural_owner") != expected_owner else "object-content-drift"
            errors.append(Diagnostic(code, source_path=path, record_path=record_path("object", path), detail="object record differs from source and routing rules"))
    sem_members = set(model.semantic_entries_by_path) | set(model.path_to_bundle)
    for path in sorted(sem_members):
        if path not in direct_sources | evidence_sources:
            errors.append(Diagnostic("semantic-content-drift", source_path=path, detail="review points to missing or metadata source"))
            continue
        source_hash = sha256(view.read_bytes(path))
        record = model.semantic_entries_by_path.get(path)
        if record:
            if record["content_sha256"] != source_hash:
                errors.append(Diagnostic("semantic-content-drift", source_path=path, record_path=record_path("semantic", path), detail="source changed since semantic review"))
            if record["disposition"] not in SEMANTIC_DISPOSITIONS:
                errors.append(Diagnostic("semantic-disposition", source_path=path, record_path=record_path("semantic", path), detail="unrecognized reviewed disposition"))
            if record["decision_owner"] not in SEMANTIC_OWNERS:
                errors.append(Diagnostic("semantic-owner", source_path=path, record_path=record_path("semantic", path), detail="unrecognized decision owner"))
            if record["disposition"] == "retain_outstanding_fulfillment_obligation" and not isinstance(record.get("retirement_gate"), str):
                errors.append(Diagnostic("semantic-obligation-gate", source_path=path, record_path=record_path("semantic", path), detail="outstanding obligation requires retirement_gate"))
            if view.file_mode(record["current_authority"]) not in {"100644", "100755"}:
                errors.append(Diagnostic("semantic-authority", source_path=path, record_path=record_path("semantic", path), detail="current authority is missing"))
    for bundle_id, bundle in model.semantic_bundles_by_id.items():
        for path in bundle["paths"]:
            if path not in direct_sources | evidence_sources:
                errors.append(Diagnostic("semantic-content-drift", source_path=path, record_path=bundle_record_path(bundle_id), detail="bundle member is missing or metadata"))
        if all(p in paths for p in bundle["paths"]):
            expected_digest = _semantic_digest(view, bundle["paths"])
            if bundle["content_set_sha256"] != expected_digest:
                errors.append(Diagnostic("semantic-bundle-drift", record_path=bundle_record_path(bundle_id), detail="bundle source digest or members changed"))
        if bundle["disposition"] not in SEMANTIC_DISPOSITIONS or bundle["decision_owner"] not in SEMANTIC_OWNERS:
            errors.append(Diagnostic("semantic-bundle-drift", record_path=bundle_record_path(bundle_id), detail="bundle disposition or decision owner invalid"))
        if view.file_mode(bundle["current_authority"]) not in {"100644", "100755"}:
            errors.append(Diagnostic("semantic-bundle-drift", record_path=bundle_record_path(bundle_id), detail="bundle current authority missing"))
    for path in sorted(set(model.evidence_entries_by_path) | set(model.path_to_group)):
        if path not in evidence_sources:
            errors.append(Diagnostic("evidence-path-drift", source_path=path, detail="evidence record does not map to an evidence source"))
    if set(model.evidence_entries_by_path) | set(model.path_to_group) != evidence_sources:
        for path in sorted(evidence_sources - (set(model.evidence_entries_by_path) | set(model.path_to_group))):
            errors.append(Diagnostic("evidence-path-drift", source_path=path, detail="evidence source has no evidence record"))
    # Four-module boundary is fixed, not inferred from corpus input.
    actual_product = sorted({
        relative.split("/", 1)[0]
        for path in paths
        if path.startswith("doc/product/")
        for relative in [path.removeprefix("doc/product/")]
        if "/" in relative
    })
    if actual_product != list(PRODUCT_MODULES):
        errors.append(Diagnostic("product-four-module-boundary", detail=f"expected {list(PRODUCT_MODULES)}, got {actual_product}"))
    return errors


def _evidence_record_from_group_member(path: str, group: dict[str, Any]) -> dict[str, Any]:
    item = next((e for e in group["entries"] if isinstance(e, dict) and e.get("path") == path), None)
    if item is not None:
        record = dict(item)
        record["atomic_group"] = group["id"]
        record["group_sha256"] = group["group_sha256"]
        return record
    return {"path": path}


def validate_evidence(view: CorpusView, model: CorpusModel | None = None) -> list[Diagnostic]:
    """Validate generic evidence schema and group membership; specialized policy is separate."""
    model = model or load_corpus(view)
    errors: list[Diagnostic] = []
    actual = {p for p in view.list_doc_paths() if _is_evidence_source(p)}
    represented = set(model.evidence_entries_by_path) | set(model.path_to_group)
    for path in sorted(actual - represented):
        errors.append(Diagnostic("path-coverage", source_path=path, record_path=EVIDENCE_ROOT, detail="evidence source lacks an entry or group"))
    if represented != actual:
        for path in sorted(represented - actual):
            errors.append(Diagnostic("missing-path-drift", source_path=path, detail="evidence entry has no source"))
    for path, entry in model.evidence_entries_by_path.items():
        if "atomic_group" in entry or "group_sha256" in entry:
            errors.append(Diagnostic("schema-record", source_path=path, record_path=record_path("evidence", path), detail="atomic group fields belong only in an evidence group wrapper"))
        if entry.get("lifecycle") == "WINDOW_OBSERVATION":
            errors.append(Diagnostic("evidence-group-drift", source_path=path, record_path=record_path("evidence", path), detail="window observations must belong to an atomic group"))
        errors.extend(_validate_evidence_record(view, path, entry, record_path("evidence", path)))
    for group_id, group in model.evidence_groups_by_id.items():
        if not isinstance(group.get("group_sha256"), str) or not SHA_RE.fullmatch(group["group_sha256"]):
            errors.append(Diagnostic("evidence-group-drift", record_path=group_record_path(group_id), detail="invalid group sha256"))
        for entry in group.get("entries", []):
            if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
                errors.append(Diagnostic("schema-record", record_path=group_record_path(group_id), detail="group entries must be full evidence records"))
                continue
            path = entry["path"]
            if "atomic_group" in entry or "group_sha256" in entry:
                errors.append(Diagnostic("schema-record", source_path=path, record_path=group_record_path(group_id), detail="group member payload omits repeated group fields"))
            errors.extend(_validate_evidence_record(view, path, entry, group_record_path(group_id), group_id=group_id))
            if entry.get("atomic_group", group_id) != group_id:
                errors.append(Diagnostic("evidence-group-drift", source_path=path, record_path=group_record_path(group_id), detail="group identity mismatch"))
        entries = group.get("entries", [])
        if all(isinstance(entry, dict) and isinstance(entry.get("path"), str) and isinstance(entry.get("content_sha256"), str) for entry in entries):
            if _evidence_digest(entries) != group.get("group_sha256"):
                errors.append(Diagnostic("evidence-group-drift", record_path=group_record_path(group_id), detail="group member digest differs"))
    return errors


def _validate_evidence_record(view: CorpusView, path: str, entry: dict[str, Any], rec_path: str, *, group_id: str | None = None) -> list[Diagnostic]:
    errors: list[Diagnostic] = []
    missing = EVIDENCE_FIELDS - set(entry)
    unknown = set(entry) - EVIDENCE_FIELDS - EVIDENCE_OPTIONAL
    if missing or unknown:
        errors.append(Diagnostic("schema-record", source_path=path, record_path=rec_path, detail=f"missing={sorted(missing)} unknown={sorted(unknown)}"))
        return errors
    for ref in ("authority", "backlink"):
        target = entry.get(ref)
        if not isinstance(target, str) or view.file_mode(target) not in {"100644", "100755"}:
            errors.append(Diagnostic(f"missing-{ref}", source_path=path, record_path=rec_path, detail=f"missing {ref}: {target}"))
    for key in EVIDENCE_FIELDS - {"path", "required_followup_roles"}:
        if not isinstance(entry.get(key), str) or not entry[key].strip():
            errors.append(Diagnostic("empty-field", source_path=path, record_path=rec_path, detail=f"empty {key}"))
    if entry.get("disposition") == "delete_candidate":
        for gate in ("semantic_absorption", "reference_repair", "owner_approval", "validation"):
            if not entry.get(gate):
                errors.append(Diagnostic(f"delete-gate-{gate}", source_path=path, record_path=rec_path, detail=f"delete_candidate requires {gate}"))
    for hash_field in ("content_sha256", "group_sha256"):
        if hash_field in entry and (not isinstance(entry[hash_field], str) or not SHA_RE.fullmatch(entry[hash_field])):
            errors.append(Diagnostic("evidence-group-drift", source_path=path, record_path=rec_path, detail=f"invalid {hash_field}"))
    if not isinstance(entry.get("lifecycle"), str) or entry["lifecycle"] not in EVIDENCE_LIFECYCLES:
        errors.append(Diagnostic("lifecycle", source_path=path, record_path=rec_path, detail="unrecognized lifecycle"))
    if not isinstance(entry.get("disposition"), str) or entry["disposition"] not in EVIDENCE_DISPOSITIONS:
        errors.append(Diagnostic("disposition", source_path=path, record_path=rec_path, detail="unrecognized disposition"))
    for key in ("retention_owner", "domain_owner"):
        if not isinstance(entry.get(key), str) or entry[key] not in EVIDENCE_ROLES:
            errors.append(Diagnostic(f"owner-{key}", source_path=path, record_path=rec_path, detail="unrecognized owner"))
    roles = entry.get("required_followup_roles")
    if not isinstance(roles, list) or not roles or any(not isinstance(role, str) or role not in EVIDENCE_ROLES for role in roles):
        errors.append(Diagnostic("followup-roles", source_path=path, record_path=rec_path, detail="followup roles must be a nonempty list of known roles"))
    for key in ("evidence_window", "observation_window", "claim_boundary", "semantic_absorption", "reference_repair", "owner_approval", "validation"):
        if key in entry and (not isinstance(entry[key], str) or not entry[key].strip()):
            errors.append(Diagnostic("empty-field", source_path=path, record_path=rec_path, detail=f"empty {key}"))
    if "atomic_group" in entry and (not isinstance(entry["atomic_group"], str) or not BUNDLE_RE.fullmatch(entry["atomic_group"])):
        errors.append(Diagnostic("evidence-group-drift", source_path=path, record_path=rec_path, detail="invalid atomic_group"))
    if entry.get("lifecycle") == "WINDOW_OBSERVATION":
        required = ("observation_window", "content_sha256", "claim_boundary")
        if not all(key in entry for key in required):
            errors.append(Diagnostic("evidence-group-drift", source_path=path, record_path=rec_path, detail="window observation requires window, source hash, and claim boundary"))
        if group_id is not None and entry.get("observation_window") != group_id:
            errors.append(Diagnostic("evidence-group-drift", source_path=path, record_path=rec_path, detail="observation_window must match its atomic group"))
    if "content_sha256" in entry and path in view.list_doc_paths() and entry["content_sha256"] != sha256(view.read_bytes(path)):
        errors.append(Diagnostic("evidence-content-drift", source_path=path, record_path=rec_path, detail="source changed since evidence snapshot"))
    return errors


def locate(model: CorpusModel, source_path: str) -> list[RecordLocation]:
    """Return exact endpoints and complete membership for one source in the model."""
    validate_repo_path(source_path)
    found: list[RecordLocation] = []
    if source_path in model.objects_by_path or source_path in model.metadata_paths:
        endpoint = record_path("object", source_path) if source_path in model.objects_by_path else source_path
        found.append(RecordLocation("object" if source_path in model.objects_by_path else "static-control", source_path, endpoint, (source_path,)))
    if source_path in model.semantic_entries_by_path:
        found.append(RecordLocation("semantic-entry", source_path, record_path("semantic", source_path), (source_path,)))
    if source_path in model.path_to_bundle:
        bundle_id = model.path_to_bundle[source_path]
        members = tuple(model.semantic_bundles_by_id[bundle_id]["paths"])
        found.append(RecordLocation("semantic-bundle", source_path, bundle_record_path(bundle_id), members, bundle_id))
    if source_path in model.evidence_entries_by_path:
        found.append(RecordLocation("evidence-entry", source_path, record_path("evidence", source_path), (source_path,)))
    if source_path in model.path_to_group:
        group_id = model.path_to_group[source_path]
        members = tuple(e["path"] for e in model.evidence_groups_by_id[group_id]["entries"])
        found.append(RecordLocation("evidence-group", source_path, group_record_path(group_id), members, group_id))
    return sorted(found, key=lambda x: (x.kind, x.record_path))


def normalize_legacy(raw_corpus: dict[str, Any], raw_semantic: dict[str, Any], raw_evidence: dict[str, Any]) -> dict[str, Any]:
    """Normalize old whole tables without dropping fields or rewriting values."""
    if raw_corpus.get("version") != 2 or raw_evidence.get("version") != 1 or raw_semantic.get("version") != 1:
        raise CorpusError(Diagnostic("legacy-format-disabled", detail="legacy inputs must be corpus v2, semantic v1 and evidence v1"))
    objects = raw_corpus.get("objects")
    semantic_entries = raw_semantic.get("entries")
    bundles = raw_semantic.get("bundles")
    evidence_entries = raw_evidence.get("entries")
    if not all(isinstance(items, list) for items in (objects, semantic_entries, bundles, evidence_entries)):
        raise CorpusError(Diagnostic("legacy-schema", detail="legacy arrays missing"))
    return {
        "objects": sorted(objects, key=lambda item: item["path"]),
        "semantic_entries": sorted(semantic_entries, key=lambda item: item["path"]),
        "semantic_bundles": sorted(bundles, key=lambda item: item["id"]),
        "evidence_entries": sorted(evidence_entries, key=lambda item: item["path"]),
    }


def normalize_current(model: CorpusModel) -> dict[str, Any]:
    evidence = [dict(item) for item in model.evidence_entries_by_path.values()]
    for group in model.evidence_groups_by_id.values():
        for item in group["entries"]:
            expanded = dict(item)
            expanded["atomic_group"] = group["id"]
            expanded["group_sha256"] = group["group_sha256"]
            evidence.append(expanded)
    return {
        "objects": sorted((dict(v) for v in model.objects_by_path.values()), key=lambda item: item["path"]),
        "semantic_entries": sorted((dict(v) for v in model.semantic_entries_by_path.values()), key=lambda item: item["path"]),
        "semantic_bundles": sorted((dict(v) for v in model.semantic_bundles_by_id.values()), key=lambda item: item["id"]),
        "evidence_entries": sorted(evidence, key=lambda item: item["path"]),
    }


def _load_record_for_proposal(kind: str, record: dict[str, Any], path: str) -> None:
    if kind == "semantic-entry":
        _exact_keys(record, SEMANTIC_FIELDS, SEMANTIC_OPTIONAL, path)
    elif kind == "semantic-bundle":
        _exact_keys(record, SEMANTIC_BUNDLE_FIELDS, set(), path)
    elif kind == "evidence-entry":
        _exact_keys(record, EVIDENCE_FIELDS, EVIDENCE_OPTIONAL - {"atomic_group", "group_sha256"}, path)
    elif kind == "evidence-group":
        _exact_keys(record, {"id", "group_sha256", "entries"}, set(), path)
    else:
        raise CorpusError(Diagnostic("proposal-schema", record_path=path, detail="record kind cannot be reviewed by this tool"))


def _evidence_group_records(entries: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    singles: list[dict[str, Any]] = []
    grouped: dict[str, list[dict[str, Any]]] = {}
    for original in entries:
        item = dict(original)
        group_id = item.pop("atomic_group", None)
        group_hash = item.pop("group_sha256", None)
        if group_id is None:
            singles.append(item)
        else:
            grouped.setdefault(group_id, []).append(item)
            if group_hash:
                item["_legacy_group_sha256"] = group_hash
    groups: list[dict[str, Any]] = []
    for group_id, members in sorted(grouped.items()):
        members.sort(key=lambda item: item["path"])
        group_hashes = {member.pop("_legacy_group_sha256", None) for member in members}
        if len(group_hashes) > 1:
            raise CorpusError(Diagnostic("evidence-group-drift", detail=f"legacy group {group_id} has inconsistent hashes"))
        group_hash = next(iter(group_hashes))
        if not group_hash:
            group_hash = _evidence_digest(members)
        groups.append({"id": group_id, "group_sha256": group_hash, "entries": members})
    return singles, groups


def _wrapped(kind: str, record: dict[str, Any]) -> bytes:
    return canonical_json({"schema": SCHEMAS[kind], "record": record})


def v3_files_from_legacy(raw_corpus: dict[str, Any], raw_semantic: dict[str, Any], raw_evidence: dict[str, Any]) -> dict[str, bytes]:
    normalized = normalize_legacy(raw_corpus, raw_semantic, raw_evidence)
    files: dict[str, bytes] = {
        CORPUS_ROOT: canonical_json({
            "version": 3,
            "scope": "all doc files through direct objects, delegated evidence objects, and controls",
            "decision_boundary": "routing is conservative heuristic input only",
            "product_modules": list(PRODUCT_MODULES),
            "objects_root": OBJECTS_ROOT,
            "semantic_entries_root": SEMANTIC_ENTRIES_ROOT,
            "semantic_bundles_root": SEMANTIC_BUNDLES_ROOT,
            "delegates": [{"kind": "testing-evidence", "path": EVIDENCE_ROOT, "expected_version": 2}],
        }),
        EVIDENCE_ROOT: canonical_json({
            "version": 2,
            "scope": "doc/testing/evidence/** excluding inventory.json",
            "entries_root": EVIDENCE_ENTRIES_ROOT,
            "groups_root": EVIDENCE_GROUPS_ROOT,
            "legacy_baseline_id": "document-evidence-v1-at-v3-migration",
        }),
    }
    for record in normalized["objects"]:
        files[record_path("object", record["path"])] = _wrapped("object", record)
    for record in normalized["semantic_entries"]:
        files[record_path("semantic", record["path"])] = _wrapped("semantic-entry", record)
    for record in normalized["semantic_bundles"]:
        files[bundle_record_path(record["id"])] = _wrapped("semantic-bundle", record)
    evidence_singles, groups = _evidence_group_records(normalized["evidence_entries"])
    for record in evidence_singles:
        files[record_path("evidence", record["path"])] = _wrapped("evidence-entry", record)
    for group in groups:
        files[group_record_path(group["id"])] = _wrapped("evidence-group", group)
    return files


def _safe_write(repo_root: Path, path: str, data: bytes | None) -> None:
    validate_repo_path(path)
    # Check every existing component before creating parents. This catches a
    # symlinked metadata root as well as a symlink at the final file path.
    view = WorktreeCorpusView(repo_root)
    view._path(path)
    target = repo_root / path
    try:
        target.parent.mkdir(parents=True, exist_ok=True)
        view._path(path)
        if data is None:
            target.unlink(missing_ok=True)
            return
        if target.is_file() and target.read_bytes() == data:
            return
        fd, tmp_name = tempfile.mkstemp(prefix=f".{target.name}.", suffix=".tmp", dir=target.parent)
        with os.fdopen(fd, "wb") as handle:
            # Windows does not expose fchmod; keep the same non-executable
            # mode contract by applying it to the temporary path there.
            fchmod = getattr(os, "fchmod", None)
            if fchmod is None:
                os.chmod(tmp_name, 0o644)
            else:
                fchmod(handle.fileno(), 0o644)
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp_name, target)
    except BaseException as exc:
        try:
            if "tmp_name" in locals():
                os.unlink(tmp_name)
        except OSError:
            pass
        if isinstance(exc, OSError):
            raise CorpusError(Diagnostic("write-failed", record_path=path, detail=str(exc))) from exc
        raise


@dataclass
class WritePlan:
    repo_root: Path
    source_preconditions: list[tuple[str, str | None]]
    record_preconditions: list[tuple[str, str | None]]
    writes: dict[str, bytes | None]
    operation: str


@dataclass(frozen=True)
class WriteResult:
    changed_paths: tuple[str, ...]
    unchanged_paths: tuple[str, ...]


def plan_sync(view: CorpusView, paths: list[str], deleted_paths: list[str]) -> WritePlan:
    if not isinstance(view, WorktreeCorpusView):
        raise CorpusError(Diagnostic("write-requires-worktree", detail="sync only writes an explicit working tree"))
    if not paths and not deleted_paths:
        raise CorpusError(Diagnostic("sync-path-required", detail="provide --path/--deleted-path or --all"))
    if len(set(paths)) != len(paths) or len(set(deleted_paths)) != len(deleted_paths):
        raise CorpusError(Diagnostic("duplicate-path", detail="sync path repeated"))
    if set(paths) & set(deleted_paths):
        raise CorpusError(Diagnostic("sync-path-conflict", detail="path cannot be both updated and deleted"))
    model = load_corpus(view)
    all_paths = set(view.list_doc_paths())
    writes: dict[str, bytes | None] = {}
    sources: list[tuple[str, str | None]] = []
    records: list[tuple[str, str | None]] = []
    for path in sorted(paths):
        validate_repo_path(path)
        if not path.startswith("doc/") or path in model.metadata_paths or _is_evidence_source(path):
            raise CorpusError(Diagnostic("sync-scope", source_path=path, detail="sync only manages ordinary direct objects"))
        if path not in all_paths:
            raise CorpusError(Diagnostic("source-unavailable", source_path=path, detail="source does not exist; use --deleted-path only when absent"))
        target = record_path("object", path)
        expected = expected_object(view, path)
        sources.append((path, sha256(view.read_bytes(path))))
        old = view.read_bytes(target) if target in all_paths else None
        desired = _wrapped("object", expected)
        if old != desired:
            records.append((target, sha256(old) if old is not None else None))
            writes[target] = desired
    for path in sorted(deleted_paths):
        validate_repo_path(path)
        if not path.startswith("doc/") or path in model.metadata_paths or _is_evidence_source(path):
            raise CorpusError(Diagnostic("sync-scope", source_path=path, detail="sync only manages ordinary direct objects"))
        if path in all_paths:
            raise CorpusError(Diagnostic("source-still-exists", source_path=path, detail="deleted path is still present"))
        target = record_path("object", path)
        if path in model.semantic_entries_by_path or path in model.path_to_bundle or path in model.evidence_entries_by_path or path in model.path_to_group:
            raise CorpusError(Diagnostic("review-record-remains", source_path=path, detail="explicitly review linked semantic/evidence records first"))
        if target in all_paths:
            sources.append((path, None))
            records.append((target, sha256(view.read_bytes(target))))
            writes[target] = None
    ensure_hash_inputs(view, [path for path, digest in sources if digest is not None])
    return WritePlan(view.repo_root, sources, records, writes, "sync")


def _write_lock(repo_root: Path):
    git_dir = subprocess.run(["git", "-C", str(repo_root), "rev-parse", "--git-path", "document-corpus-v3.lock"], capture_output=True, text=True, check=False)
    if git_dir.returncode:
        raise CorpusError(Diagnostic("lock-path-unavailable", detail=git_dir.stderr.strip()))
    path = Path(git_dir.stdout.strip())
    if not path.is_absolute():
        path = repo_root / path
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        handle = path.open("a+b")
    except OSError as exc:
        raise CorpusError(Diagnostic("lock-path-unavailable", detail=str(exc))) from exc
    try:
        try:
            if os.name == "nt":
                import msvcrt
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as exc:
            raise CorpusError(Diagnostic("lock-busy", detail=str(exc))) from exc
        return handle
    except BaseException:
        handle.close()
        raise


def apply_plan(plan: WritePlan) -> WriteResult:
    repo_root = plan.repo_root.resolve()
    lock = _write_lock(repo_root)
    try:
        view = WorktreeCorpusView(repo_root)
        actual_sources = [(path, sha256(view.read_bytes(path)) if view.file_mode(path) in {"100644", "100755"} else None) for path, _ in plan.source_preconditions]
        if actual_sources != plan.source_preconditions:
            raise CorpusError(Diagnostic("concurrent-modification", detail="source precondition changed"))
        ensure_hash_inputs(view, [path for path, digest in plan.source_preconditions if digest is not None])
        actual_records = [(path, sha256(view.read_bytes(path)) if view.file_mode(path) in {"100644", "100755"} else None) for path, _ in plan.record_preconditions]
        if actual_records != plan.record_preconditions:
            raise CorpusError(Diagnostic("concurrent-modification", detail="record precondition changed"))
        # Preflight the complete write set before touching any file.
        for path in sorted(plan.writes):
            view._path(path)
            if view.file_mode(path) not in {"000000", "100644", "100755"}:
                raise CorpusError(Diagnostic("unsafe-path", record_path=path, detail="write target must be absent or a regular file"))
        if view.verify_unchanged():
            raise CorpusError(Diagnostic("concurrent-modification", detail="working tree changed during write preflight"))
        changed: list[str] = []
        unchanged: list[str] = []
        for path, data in sorted(plan.writes.items()):
            target = repo_root / path
            old = target.read_bytes() if target.is_file() else None
            if old == data:
                unchanged.append(path)
                continue
            _safe_write(repo_root, path, data)
            changed.append(path)
        after = WorktreeCorpusView(repo_root)
        actual_sources = [(path, sha256(after.read_bytes(path)) if after.file_mode(path) in {"100644", "100755"} else None) for path, _ in plan.source_preconditions]
        if actual_sources != plan.source_preconditions:
            raise CorpusError(Diagnostic("concurrent-modification", detail="source changed while records were being written"))
        return WriteResult(tuple(changed), tuple(unchanged))
    finally:
        if os.name == "nt":
            try:
                import msvcrt
                lock.seek(0)
                msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
            except OSError:
                pass
        else:
            import fcntl
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        lock.close()
