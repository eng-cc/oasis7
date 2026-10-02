#!/usr/bin/env python3
"""Black-box regressions for document corpus v3 coverage and local writes."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from typing import Any
from unittest.mock import patch


ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = ROOT / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
import document_corpus as dc

CLI = ROOT / "scripts/document-corpus-inventory.py"
CORPUS_CHECK = ROOT / "scripts/document-corpus-inventory-check.py"
EVIDENCE_CHECK = ROOT / "scripts/doc-evidence-inventory-check.py"
CORPUS_ROOT = "doc/.governance/document-corpus-inventory.json"
EVIDENCE_ROOT = "doc/testing/evidence/inventory.json"
LEGACY_SEMANTIC_ROOT = "doc/.governance/document-semantic-review-overrides.json"
OBJECTS_ROOT = "doc/.governance/document-corpus/objects"
SEMANTIC_ROOT = "doc/.governance/document-corpus/semantic/entries"
EVIDENCE_ENTRIES_ROOT = "doc/.governance/document-corpus/evidence/entries"
EVIDENCE_GROUPS_ROOT = "doc/.governance/document-corpus/evidence/groups"
PRODUCT_MODULES = (
    "agents-world-simulation",
    "player-entry-distribution",
    "world-infrastructure",
    "world-rules-core-gameplay",
)
LEGACY_BASE = "49a5a207e549f34b97fe265ab094160e7b402ae3"
LEGACY_INVENTORY = ROOT / "scripts/fixtures/document-corpus-v3/legacy/inventory.json"
_FROZEN_SOURCE_BYTES: dict[str, bytes] | None = None
_FIXTURE_ROOT: Path | None = None


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def record_path(kind: str, source: str) -> str:
    key = sha(source.encode("utf-8"))
    roots = {
        "object": OBJECTS_ROOT,
        "semantic": SEMANTIC_ROOT,
        "evidence": EVIDENCE_ENTRIES_ROOT,
    }
    return f"{roots[kind]}/{key[:2]}/{key}.json"


def write(root: Path, path: str, data: bytes | str) -> None:
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data.encode("utf-8") if isinstance(data, str) else data)


def append_utf8_lf(path: Path, text: str) -> None:
    """Append explicit UTF-8/LF bytes without platform text-mode translation."""
    if "\r" in text:
        raise ValueError("fixture suffix must use LF bytes only")
    path.write_bytes(path.read_bytes() + text.encode("utf-8"))


def canonical(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def checked_run(argv: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, text=True, encoding="utf-8", errors="replace", capture_output=True, check=False)


def git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = checked_run(["git", "-C", str(root), *args])
    if check and result.returncode:
        raise AssertionError(f"git {' '.join(args)} failed ({result.returncode}):\n{result.stdout}{result.stderr}")
    return result


def cli(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return checked_run([sys.executable, str(CLI), "--repo-root", str(root), *args])


def checker(root: Path, *, evidence: bool = False, revision: str | None = None) -> subprocess.CompletedProcess[str]:
    script = EVIDENCE_CHECK if evidence else CORPUS_CHECK
    argv = [sys.executable, str(script), "--repo-root", str(root)]
    if revision:
        argv += ["--revision", revision]
    return checked_run(argv)


def repository_payload_snapshot(root: Path) -> dict[str, tuple[bytes, int]]:
    """Capture all non-Git, non-report files so readers cannot hide writes."""
    snapshot: dict[str, tuple[bytes, int]] = {}
    for path in root.rglob("*"):
        relative = path.relative_to(root)
        if not relative.parts or relative.parts[0] in {".git", ".cache"} or not path.is_file():
            continue
        snapshot[relative.as_posix()] = (path.read_bytes(), stat.S_IMODE(path.stat().st_mode))
    return snapshot


def diagnostics(result: subprocess.CompletedProcess[str]) -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []
    for line in (result.stdout + "\n" + result.stderr).splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict) and isinstance(value.get("code"), str):
            found.append(value)
        elif isinstance(value, list):
            found.extend(item for item in value if isinstance(item, dict) and isinstance(item.get("code"), str))
            continue
        if isinstance(value, dict) and isinstance(value.get("code"), str):
            continue
        # Current checker CLIs print stable codes and labeled locations as text;
        # management commands emit the full Diagnostic JSON object.
        match = re.match(r"^([a-z][a-z0-9-]*)(?:[: ]+)(.*)$", line)
        if match and not line.startswith(("document-corpus-inventory-check:", "doc-evidence-inventory-check:")):
            rest = match.group(2)
            source_match = re.search(r"source=([^:\\s]+)", rest)
            record_match = re.search(r"record=([^:\\s]+)", rest)
            detail = rest
            if source_match:
                detail = detail.replace(source_match.group(0), "", 1)
            if record_match:
                detail = detail.replace(record_match.group(0), "", 1)
            detail = detail.lstrip(": ")
            detail, separator, repair = detail.partition("; repair: ")
            found.append({
                "code": match.group(1),
                "source_path": source_match.group(1) if source_match else None,
                "record_path": record_match.group(1) if record_match else None,
                "detail": detail,
                "repair_command": repair if separator else "",
            })
    return found


def expect_code(result: subprocess.CompletedProcess[str], code: str, exit_code: int | None = None) -> dict[str, Any]:
    output = result.stdout + result.stderr
    if exit_code is not None:
        assert result.returncode == exit_code, f"expected exit {exit_code}, got {result.returncode}:\n{output}"
    else:
        assert result.returncode != 0, f"expected {code}, command succeeded:\n{output}"
    matches = [item for item in diagnostics(result) if item.get("code") == code]
    assert matches, f"missing diagnostic {code}:\n{output}"
    required = {"code", "source_path", "record_path", "detail", "repair_command"}
    assert required <= set(matches[0]), f"diagnostic {code} omitted contract fields: {matches[0]!r}"
    return matches[0]


def start_git(root: Path) -> None:
    git(root, "init", "--quiet")
    git(root, "config", "user.name", "Corpus Fixture")
    git(root, "config", "user.email", "corpus-fixture@example.invalid")
    # Keep canonical metadata fixtures stable regardless of the host's global
    # autocrlf setting; individual portability scenarios opt in explicitly.
    git(root, "config", "core.autocrlf", "false")


def commit_all(root: Path, message: str) -> str:
    git(root, "add", "-A")
    git(root, "commit", "--quiet", "-m", message)
    return git(root, "rev-parse", "HEAD").stdout.strip()


def make_fixture() -> Path:
    """Build a small direct-doc fixture plus only the frozen evidence subtree.

    The evidence policy intentionally pins all 128 migrated rows, so a checker
    fixture cannot reduce that cohort.  We copy those source blobs from the
    immutable baseline commit, not the rest of ``doc/``.
    """
    global _FROZEN_SOURCE_BYTES, _FIXTURE_ROOT
    if _FIXTURE_ROOT is not None:
        root = _FIXTURE_ROOT
        reset_fixture(root)
        valid = checker(root)
        assert valid.returncode == 0, f"shared v3 fixture did not reset cleanly:\n{valid.stdout}{valid.stderr}"
        return root
    root = Path(tempfile.mkdtemp(prefix="oasis7-corpus-v3-"))
    write(root, "doc/README.md", "# Fixture docs\n")
    write(root, "doc/engineering/README.md", "# Engineering fixture\n")
    write(root, "doc/engineering/doc-governance/README.md", "# Governance fixture\n")
    write(root, "doc/product/README.md", "# Product fixture\n")
    write(root, "doc/testing/README.md", "# Testing fixture\n")
    for module in PRODUCT_MODULES:
        write(root, f"doc/product/{module}/README.md", f"# {module}\n")

    baseline = json.loads(LEGACY_INVENTORY.read_text(encoding="utf-8"))
    assert baseline["version"] == 1 and len(baseline["entries"]) == 128
    if _FROZEN_SOURCE_BYTES is None:
        _FROZEN_SOURCE_BYTES = {}
        for entry in baseline["entries"]:
            path = entry["path"]
            result = subprocess.run(
                ["git", "show", f"{LEGACY_BASE}:{path}"],
                cwd=ROOT,
                capture_output=True,
                check=False,
            )
            assert result.returncode == 0, f"frozen baseline source unavailable: {path}: {result.stderr.decode('utf-8', 'replace')}"
            data = result.stdout
            if "content_sha256" in entry:
                assert sha(data) == entry["content_sha256"], f"baseline source hash mismatch: {path}"
            _FROZEN_SOURCE_BYTES[path] = data

    # The specialized policy requires these exact old sources and classifications.
    grouped: dict[str, list[dict[str, Any]]] = {}
    for old in baseline["entries"]:
        path = old["path"]
        write(root, path, _FROZEN_SOURCE_BYTES[path])
        record = dict(old)
        group_id = record.pop("atomic_group", None)
        record.pop("group_sha256", None)
        if group_id:
            grouped.setdefault(group_id, []).append(record)
        else:
            write_wrapped(root, record_path("evidence", path), "oasis7.document-evidence-entry/v1", record)
    for group_id, members in grouped.items():
        expected_hashes = {
            item["group_sha256"]
            for item in baseline["entries"]
            if item.get("atomic_group") == group_id
        }
        assert len(expected_hashes) == 1
        write_wrapped(root, f"{EVIDENCE_GROUPS_ROOT}/{group_id}.json", "oasis7.document-evidence-group/v1", {
            "id": group_id,
            "group_sha256": next(iter(expected_hashes)),
            "entries": sorted(members, key=lambda item: item["path"]),
        })

    # A small out-of-cohort row exercises independent additions throughout the suite.
    fixture_evidence = "doc/testing/evidence/fixture-2026-09-30.md"
    write(root, fixture_evidence, "Synthetic evidence record source.\n")

    # All frozen evidence authorities are real regular files in the test view.
    # Stubs suffice: checker policy validates references and ownership routing,
    # not those authorities' product semantics.
    authority_paths = {
        entry["authority"]
        for entry in baseline["entries"]
        if isinstance(entry.get("authority"), str) and entry["authority"].startswith("doc/")
    }
    authority_paths.update({"doc/README.md", "doc/engineering/doc-governance/README.md"})
    write(root, "testing-manual.md", "# Testing manual fixture\n")
    for path in sorted(authority_paths):
        if path.startswith("doc/testing/evidence/") or path == "doc/README.md":
            continue
        if not (root / path).exists():
            write(root, path, f"# Authority stub for {path}\n")

    # The fixed corpus object router requires an explicit top-level registry.
    top_dirs = {
        path.split("/", 2)[1]
        for path in authority_paths
        if path.startswith("doc/") and len(path.split("/")) > 2 and path.split("/", 2)[1] != ".governance"
    }
    top_dirs.update({"engineering", "product", "testing"})
    registry_entries: list[dict[str, str]] = []
    for name in sorted(top_dirs):
        entry_path = f"doc/{name}/README.md"
        if not (root / entry_path).exists():
            write(root, entry_path, f"# {name} fixture\n")
        if name == "product":
            directory_type, owner = "product_overlay", "producer_system_designer"
        elif name == "testing":
            directory_type, owner = "evidence_domain", "qa_engineer"
        else:
            directory_type, owner = "professional_domain", "repository_health_engineer"
        registry_entries.append({"name": name, "type": directory_type, "owner": owner, "entry": entry_path})
    registry = {"version": 1, "directories": registry_entries}
    write(root, "doc/.governance/top-level-directory-registry.json", canonical(registry))
    write(root, CORPUS_ROOT, canonical({
        "version": 3,
        "scope": "all doc files through direct objects, delegated evidence objects, and controls",
        "decision_boundary": "routing is conservative heuristic input only",
        "product_modules": list(PRODUCT_MODULES),
        "objects_root": OBJECTS_ROOT,
        "semantic_entries_root": "doc/.governance/document-corpus/semantic/entries",
        "semantic_bundles_root": "doc/.governance/document-corpus/semantic/bundles",
        "delegates": [{"kind": "testing-evidence", "path": EVIDENCE_ROOT, "expected_version": 2}],
    }))
    write(root, EVIDENCE_ROOT, canonical({
        "version": 2,
        "scope": "doc/testing/evidence/** excluding inventory.json",
        "entries_root": EVIDENCE_ENTRIES_ROOT,
        "groups_root": EVIDENCE_GROUPS_ROOT,
        "legacy_baseline_id": "document-evidence-v1-at-v3-migration",
    }))
    source_path = fixture_evidence
    source_bytes = (root / source_path).read_bytes()
    evidence_record = {
        "path": source_path,
        "lifecycle": "HISTORICAL_PROVENANCE",
        "semantic_role": "qa_fixture_provenance",
        "retention_owner": "qa_engineer",
        "domain_owner": "qa_engineer",
        "required_followup_roles": ["qa_engineer"],
        "authority": "doc/testing/README.md",
        "backlink": "doc/testing/README.md",
        "disposition": "retain",
        "rationale": "Synthetic corpus-checker fixture; not runtime or release evidence.",
        "residual_risk": "The fixture has no production evidentiary value.",
        "content_sha256": sha(source_bytes),
    }
    write(root, record_path("evidence", source_path), canonical({
        "schema": "oasis7.document-evidence-entry/v1",
        "record": evidence_record,
    }))
    for semantic_source in ("doc/product/README.md", "doc/engineering/README.md"):
        semantic = {
            "path": semantic_source,
            "content_sha256": sha((root / semantic_source).read_bytes()),
            "disposition": "retain_current_authority",
            "decision_owner": "producer_system_designer",
            "current_authority": "doc/product/README.md",
            "note": f"Synthetic review seed for {semantic_source}.",
        }
        write_wrapped(root, record_path("semantic", semantic_source), "oasis7.document-semantic-entry/v1", semantic)
    start_git(root)

    generated = cli(root, "sync", "--all", "--apply")
    assert generated.returncode == 0, f"fixture sync --all failed ({generated.returncode}):\n{generated.stdout}{generated.stderr}"
    commit_all(root, "synthetic v3 baseline")
    git(root, "branch", "-M", "base")
    valid = checker(root)
    assert valid.returncode == 0, f"synthetic v3 fixture is invalid:\n{valid.stdout}{valid.stderr}"
    evidence_valid = checker(root, evidence=True)
    assert evidence_valid.returncode == 0, f"synthetic evidence fixture is invalid:\n{evidence_valid.stdout}{evidence_valid.stderr}"
    _FIXTURE_ROOT = root
    return root


def reset_fixture(root: Path) -> None:
    """Restore the one evidence-only fixture in place between scenarios."""
    git(root, "merge", "--abort", check=False)
    git(root, "reset", "--hard", "base", check=False)
    git(root, "switch", "--force", "base", check=False)
    git(root, "clean", "-fdx")
    branches = git(root, "branch", "--format=%(refname:short)").stdout.splitlines()
    for branch in branches:
        if branch != "base":
            git(root, "branch", "-D", branch, check=False)


def remove_fixture(root: Path) -> None:
    if root == _FIXTURE_ROOT:
        git(root, "config", "core.autocrlf", "false", check=False)
        reset_fixture(root)
    else:
        shutil.rmtree(root, ignore_errors=True)


def destroy_fixture() -> None:
    global _FIXTURE_ROOT
    if _FIXTURE_ROOT is not None:
        shutil.rmtree(_FIXTURE_ROOT, ignore_errors=True)
        _FIXTURE_ROOT = None


def object_bytes(root: Path, source: str) -> bytes:
    return (root / record_path("object", source)).read_bytes()


def read_record(root: Path, path: str) -> dict[str, Any]:
    return json.loads((root / path).read_text(encoding="utf-8"))


def write_wrapped(root: Path, path: str, schema: str, record: dict[str, Any]) -> None:
    write(root, path, canonical({"schema": schema, "record": record}))


def test_t01_sync_updates_one_object_only() -> None:
    root = make_fixture()
    try:
        git(root, "config", "core.autocrlf", "true")
        source = "doc/product/agents-world-simulation/README.md"
        before_roots = {path: (root / path).read_bytes() for path in (CORPUS_ROOT, EVIDENCE_ROOT)}
        object_file = record_path("object", source)
        before = (root / object_file).read_bytes()
        source_file = root / source
        append_utf8_lf(source_file, "\nChanged one ordinary source.\n")
        preexisting = set(git(root, "status", "--porcelain").stdout.splitlines())
        result = cli(root, "sync", "--path", source, "--apply")
        assert result.returncode == 0, result.stdout + result.stderr
        assert (root / object_file).read_bytes() != before, "sync did not refresh the selected object"
        after_roots = {path: (root / path).read_bytes() for path in (CORPUS_ROOT, EVIDENCE_ROOT)}
        assert after_roots == before_roots, "ordinary sync modified a static root"
        changed = set(git(root, "status", "--porcelain").stdout.splitlines()) - preexisting
        assert changed == {f" M {object_file}"}, f"sync changed unrelated tracked paths: {changed!r}"
        assert checker(root).returncode == 0
    finally:
        remove_fixture(root)


def test_t02_repeated_sync_propose_export_are_deterministic() -> None:
    root = make_fixture()
    try:
        source = "doc/product/player-entry-distribution/README.md"
        shard = root / record_path("object", source)
        before_bytes = shard.read_bytes()
        before_mtime = shard.stat().st_mtime_ns
        before_index = git(root, "write-tree").stdout.strip()
        first_sync = cli(root, "sync", "--path", source, "--apply")
        time.sleep(0.01)
        second_sync = cli(root, "sync", "--path", source, "--apply")
        assert first_sync.returncode == second_sync.returncode == 0, first_sync.stdout + first_sync.stderr + second_sync.stdout + second_sync.stderr
        assert (root / record_path("object", source)).read_bytes() == before_bytes
        assert (root / record_path("object", source)).stat().st_mtime_ns == before_mtime, "no-op sync refreshed mtime"
        assert git(root, "write-tree").stdout.strip() == before_index, "no-op sync touched the Git index"
        proposal1 = ".cache/doc-governance/proposal-1.json"
        proposal2 = ".cache/doc-governance/proposal-2.json"
        evidence_source = "doc/testing/evidence/fixture-2026-09-30.md"
        first_proposal = cli(root, "propose", "--kind", "evidence", "--path", evidence_source, "--output", proposal1)
        second_proposal = cli(root, "propose", "--kind", "evidence", "--path", evidence_source, "--output", proposal2)
        assert first_proposal.returncode == second_proposal.returncode == 0, first_proposal.stdout + first_proposal.stderr + second_proposal.stdout + second_proposal.stderr
        assert (root / proposal1).read_bytes() == (root / proposal2).read_bytes(), "same proposal inputs produced different bytes"
        report1 = ".cache/doc-governance/export-1.json"
        report2 = ".cache/doc-governance/export-2.json"
        first_export = cli(root, "export", "--output", report1)
        second_export = cli(root, "export", "--output", report2)
        assert first_export.returncode == second_export.returncode == 0, first_export.stdout + first_export.stderr + second_export.stdout + second_export.stderr
        assert (root / report1).read_bytes() == (root / report2).read_bytes(), "same export input produced different report bytes"
        assert git(root, "write-tree").stdout.strip() == before_index, "read-only reports changed the Git index"
    finally:
        remove_fixture(root)


def test_t03_new_source_without_object_is_reported() -> None:
    root = make_fixture()
    try:
        write(root, "doc/product/world-infrastructure/new-page.md", "unregistered source\n")
        expect_code(checker(root), "new-path-drift", 1)
    finally:
        remove_fixture(root)


def test_t04_delete_requires_explicit_object_only_when_no_review_record() -> None:
    root = make_fixture()
    try:
        source = "doc/product/world-rules-core-gameplay/README.md"
        source_file = root / source
        review = {
            "path": source,
            "content_sha256": sha(source_file.read_bytes()),
            "disposition": "retain_current_authority",
            "decision_owner": "gameplay_designer",
            "current_authority": "doc/README.md",
            "note": "Synthetic review record to prove ordinary delete cannot retire review."
        }
        write_wrapped(root, record_path("semantic", source), "oasis7.document-semantic-entry/v1", review)
        source_file.unlink()
        expect_code(checker(root), "missing-path-drift", 1)
        object_before = object_bytes(root, source)
        result = cli(root, "sync", "--deleted-path", source, "--apply")
        expect_code(result, "review-record-remains", 1)
        assert object_bytes(root, source) == object_before, "blocked deletion removed the machine object"
    finally:
        remove_fixture(root)


def test_t05_strict_keys_and_duplicate_members_fail_closed() -> None:
    root = make_fixture()
    try:
        original = (root / CORPUS_ROOT).read_bytes()
        (root / CORPUS_ROOT).write_bytes(original.replace(b'"version": 3', b'"version": 3,\n  "version": 3', 1))
        expect_code(checker(root), "duplicate-json-key", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        source = "doc/product/agents-world-simulation/README.md"
        existing = record_path("object", source)
        record = read_record(root, existing)["record"]
        wrong_path = f"{OBJECTS_ROOT}/ff/{'f' * 64}.json"
        write_wrapped(root, wrong_path, "oasis7.document-corpus-object/v1", record)
        expect_code(checker(root), "wrong-record-key", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        group_id = "duplicate-members"
        member = read_record(root, record_path("evidence", source))["record"]
        group = {"id": group_id, "group_sha256": "0" * 64, "entries": [member, member]}
        write_wrapped(root, f"{EVIDENCE_GROUPS_ROOT}/{group_id}.json", "oasis7.document-evidence-group/v1", group)
        expect_code(checker(root), "record-members-invalid", 1)
    finally:
        remove_fixture(root)


def test_t06_metadata_cannot_become_a_recursive_object_source() -> None:
    for source in (CORPUS_ROOT, EVIDENCE_ROOT):
        root = make_fixture()
        try:
            fake = {
                "path": source,
                "content_sha256": sha((root / source).read_bytes()),
                "registry_type": "governance_control",
                "structural_owner": "repository_health_engineer",
                "authority_entry": "doc/engineering/doc-governance/README.md",
                "authority_layer": "controlled_exception",
                "semantic_decision_owner": "repository_health_engineer",
                "object_kind": "supporting_artifact",
                "lifecycle_candidate": "review_required",
                "inventory_disposition": "retain_for_owner_review",
                "routing_batch": ".governance-corpus-review",
                "routing_note": "fixture recursive metadata attack",
            }
            write_wrapped(root, record_path("object", source), "oasis7.document-corpus-object/v1", fake)
            result = checker(root)
            assert result.returncode == 1, result.stdout + result.stderr
            assert {item.get("code") for item in diagnostics(result)} & {"new-path-drift", "missing-path-drift"}, result.stdout + result.stderr
        finally:
            remove_fixture(root)

    root = make_fixture()
    try:
        source = record_path("object", "doc/product/agents-world-simulation/README.md")
        fake = read_record(root, source)["record"]
        fake["path"] = source
        fake["content_sha256"] = sha((root / source).read_bytes())
        write_wrapped(root, record_path("object", source), "oasis7.document-corpus-object/v1", fake)
        result = checker(root)
        expect_code(result, "missing-path-drift", 1)
    finally:
        remove_fixture(root)


def test_t07_unknown_store_file_and_symlink_fail_closed() -> None:
    root = make_fixture()
    try:
        write(root, f"{OBJECTS_ROOT}/unknown.txt", "not metadata\n")
        expect_code(checker(root), "unknown-metadata-file", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        link = root / f"{OBJECTS_ROOT}/external-link"
        link.parent.mkdir(parents=True, exist_ok=True)
        outside = root.parent / f"{root.name}-outside"
        outside.write_text("outside\n", encoding="utf-8")
        link.symlink_to(outside)
        try:
            expect_code(checker(root), "unsafe-path", 1)
        finally:
            outside.unlink(missing_ok=True)
    finally:
        remove_fixture(root)


def test_t08_nested_inventory_json_is_an_evidence_source_by_path() -> None:
    root = make_fixture()
    try:
        source = "doc/testing/evidence/nested/inventory.json"
        data = b'{"synthetic": true}\n'
        write(root, source, data)
        old = read_record(root, record_path("evidence", "doc/testing/evidence/fixture-2026-09-30.md"))["record"]
        record = dict(old)
        record["path"] = source
        record["semantic_role"] = "nested_inventory_fixture"
        record["content_sha256"] = sha(data)
        write_wrapped(root, record_path("evidence", source), "oasis7.document-evidence-entry/v1", record)
        assert checker(root).returncode == 0, (checker(root).stdout + checker(root).stderr)
    finally:
        remove_fixture(root)


def test_t09_registry_and_product_boundary_are_not_inferred_from_shards() -> None:
    root = make_fixture()
    try:
        registry_path = root / "doc/.governance/top-level-directory-registry.json"
        registry = json.loads(registry_path.read_text(encoding="utf-8"))
        next(item for item in registry["directories"] if item["name"] == "product")["owner"] = "qa_engineer"
        registry_path.write_bytes(canonical(registry))
        expect_code(checker(root), "registry-routing-drift", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        ordinary = "doc/product/agents-world-simulation/README.md"
        obj = read_record(root, record_path("object", ordinary))["record"]
        obj["path"] = source
        obj["content_sha256"] = sha((root / source).read_bytes())
        write_wrapped(root, record_path("object", source), "oasis7.document-corpus-object/v1", obj)
        expect_code(checker(root), "missing-path-drift", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        corpus = json.loads((root / CORPUS_ROOT).read_text(encoding="utf-8"))
        corpus["delegates"][0]["path"] = "doc/testing/evidence"
        (root / CORPUS_ROOT).write_bytes(canonical(corpus))
        expect_code(checker(root), "corpus-entrypoint-contract", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        source = "doc/product/unapproved-fifth-module/README.md"
        write(root, source, "fifth module\n")
        generated = cli(root, "sync", "--path", source, "--apply")
        assert generated.returncode == 0, generated.stdout + generated.stderr
        expect_code(checker(root), "product-four-module-boundary", 1)
    finally:
        remove_fixture(root)


def record_path_for_evidence(source: str) -> str:
    key = sha(source.encode("utf-8"))
    return f"doc/.governance/document-corpus/evidence/entries/{key[:2]}/{key}.json"


def build_branch(
    root: Path,
    base: str,
    name: str,
    *,
    source_updates: dict[str, bytes] | None = None,
    additions: dict[str, bytes] | None = None,
    evidence_additions: dict[str, bytes] | None = None,
) -> None:
    git(root, "switch", "-c", name, base)
    changed_objects: list[str] = []
    for source, data in (source_updates or {}).items():
        write(root, source, data)
        changed_objects.append(source)
    for source, data in (additions or {}).items():
        write(root, source, data)
        changed_objects.append(source)
    if changed_objects:
        argv = ["sync"]
        for source in changed_objects:
            argv.extend(("--path", source))
        argv.append("--apply")
        result = cli(root, *argv)
        assert result.returncode == 0, f"{name} object sync failed:\n{result.stdout}{result.stderr}"
    for source, data in (evidence_additions or {}).items():
        write(root, source, data)
        template_path = "doc/testing/evidence/fixture-2026-09-30.md"
        key = sha(template_path.encode("utf-8"))
        template = read_record(root, f"{EVIDENCE_ENTRIES_ROOT}/{key[:2]}/{key}.json")["record"]
        record = dict(template)
        record["path"] = source
        record["semantic_role"] = "independent_branch_evidence"
        record["content_sha256"] = sha(data)
        write_wrapped(root, record_path_for_evidence(source), "oasis7.document-evidence-entry/v1", record)
    commit_all(root, f"{name} independent corpus change")


def merge_both_orders(root: Path, base: str, left: str, right: str) -> str:
    trees: list[str] = []
    for label, order in (("left-right", (left, right)), ("right-left", (right, left))):
        git(root, "switch", "-c", f"merge-{label}", base)
        for branch in order:
            merged = git(root, "merge", "--no-ff", "--no-edit", branch, check=False)
            assert merged.returncode == 0, f"merge {label} failed:\n{merged.stdout}{merged.stderr}"
        corpus_valid = checker(root)
        evidence_valid = checked_run([sys.executable, str(EVIDENCE_CHECK), "--repo-root", str(root)])
        assert corpus_valid.returncode == 0, corpus_valid.stdout + corpus_valid.stderr
        assert evidence_valid.returncode == 0, evidence_valid.stdout + evidence_valid.stderr
        trees.append(git(root, "rev-parse", "HEAD^{tree}").stdout.strip())
        git(root, "switch", "base")
    assert trees[0] == trees[1], f"merge order changed logical tree: {trees}"
    return trees[0]


def test_t18_different_sources_in_one_module_merge_cleanly_both_orders() -> None:
    root = make_fixture()
    try:
        base = git(root, "rev-parse", "HEAD").stdout.strip()
        source_a = "doc/product/agents-world-simulation/README.md"
        source_b = "doc/product/agents-world-simulation/sibling.md"
        build_branch(root, base, "change-a", source_updates={source_a: b"# agents changed on A\n"})
        build_branch(root, base, "change-b", additions={source_b: b"independent sibling source\n"})
        merge_both_orders(root, base, "change-a", "change-b")
    finally:
        remove_fixture(root)


def test_t19_adjacent_new_paths_do_not_share_a_sorted_parent_list() -> None:
    root = make_fixture()
    try:
        base = git(root, "rev-parse", "HEAD").stdout.strip()
        left = "doc/product/world-infrastructure/a-new.md"
        right = "doc/product/world-infrastructure/b-new.md"
        build_branch(root, base, "add-a", additions={left: b"A\n"})
        build_branch(root, base, "add-b", additions={right: b"B\n"})
        merge_both_orders(root, base, "add-a", "add-b")
        for merge_branch in ("merge-left-right", "merge-right-left"):
            assert git(root, "show", f"{merge_branch}:{left}").stdout == "A\n"
            assert git(root, "show", f"{merge_branch}:{right}").stdout == "B\n"
    finally:
        remove_fixture(root)


def test_t20_ordinary_object_and_independent_evidence_merge_both_orders() -> None:
    root = make_fixture()
    try:
        base = git(root, "rev-parse", "HEAD").stdout.strip()
        direct = "doc/product/world-rules-core-gameplay/README.md"
        evidence = "doc/testing/evidence/independent-branch-2026-09-30.txt"
        build_branch(root, base, "ordinary-doc", source_updates={direct: b"# rule source changed\n"})
        build_branch(root, base, "evidence-update", evidence_additions={evidence: b"independent evidence branch\n"})
        merge_both_orders(root, base, "ordinary-doc", "evidence-update")
        for merge_branch in ("merge-left-right", "merge-right-left"):
            assert git(root, "show", f"{merge_branch}:{direct}").stdout == "# rule source changed\n"
            assert git(root, "show", f"{merge_branch}:{evidence}").stdout == "independent evidence branch\n"
    finally:
        remove_fixture(root)


def test_t21_auto_merged_same_source_still_conflicts_on_its_object() -> None:
    root = make_fixture()
    try:
        source = "doc/product/agents-world-simulation/README.md"
        seed = b"# agents\n\nleft=base\n\nright=base\n"
        git(root, "switch", "-c", "same-source-seed", "base")
        write(root, source, seed)
        refresh = cli(root, "sync", "--path", source, "--apply")
        assert refresh.returncode == 0, refresh.stdout + refresh.stderr
        base = commit_all(root, "seed disjoint merge regions")
        branch_a = seed.replace(b"left=base", b"left=A")
        branch_b = seed.replace(b"right=base", b"right=B")
        build_branch(root, base, "same-source-a", source_updates={source: branch_a})
        build_branch(root, base, "same-source-b", source_updates={source: branch_b})
        git(root, "switch", "-c", "same-source-merge", base)
        merged = git(root, "merge", "--no-ff", "--no-edit", "same-source-a", check=False)
        assert merged.returncode == 0, merged.stdout + merged.stderr
        merged = git(root, "merge", "--no-ff", "--no-edit", "same-source-b", check=False)
        assert merged.returncode != 0, "same-source sidecars merged silently"
        unmerged = set(git(root, "diff", "--name-only", "--diff-filter=U").stdout.splitlines())
        assert record_path("object", source) in unmerged, f"merge did not preserve object conflict: {unmerged!r}"
        git(root, "merge", "--abort")
    finally:
        remove_fixture(root)


def test_t22_readers_and_export_do_not_change_source_or_index() -> None:
    root = make_fixture()
    try:
        tree_before = git(root, "write-tree").stdout.strip()
        tracked_before = git(root, "status", "--porcelain", "--untracked-files=no").stdout
        index_path = Path(git(root, "rev-parse", "--git-path", "index").stdout.strip())
        if not index_path.is_absolute():
            index_path = root / index_path
        index_before = index_path.read_bytes()
        payload_before = repository_payload_snapshot(root)
        source = "doc/product/agents-world-simulation/README.md"
        for result in (checker(root), checker(root, evidence=True)):
            assert result.returncode == 0, result.stdout + result.stderr
            worktree_report = json.loads(result.stdout.splitlines()[-1])
            assert worktree_report == {
                "mode": "worktree",
                "snapshot_id": None,
                "snapshot_kind": "worktree",
                "tree_id": None,
                "commit_id": None,
            }, worktree_report
        revision_commit = git(root, "rev-parse", "HEAD").stdout.strip()
        revision_tree = git(root, "rev-parse", f"{revision_commit}^{{tree}}").stdout.strip()
        source_path = root / source
        committed_source = source_path.read_bytes()
        source_path.write_bytes(committed_source + b"worktree-only drift\n")
        try:
            for revision, expected_kind, expected_commit in (
                (revision_commit, "commit", revision_commit),
                (revision_tree, "tree", None),
            ):
                committed = checker(root, revision=revision)
                assert committed.returncode == 0, f"immutable revision {revision} failed:\n{committed.stdout}{committed.stderr}"
                expected_checker_report = {
                    "mode": "revision",
                    "snapshot_id": revision,
                    "snapshot_kind": expected_kind,
                    "tree_id": revision_tree,
                    "commit_id": expected_commit,
                }
                assert json.loads(committed.stdout.splitlines()[-1]) == expected_checker_report, committed.stdout
                evidence = checker(root, evidence=True, revision=revision)
                assert evidence.returncode == 0, f"evidence checker rejected immutable revision {revision}:\n{evidence.stdout}{evidence.stderr}"
                assert json.loads(evidence.stdout.splitlines()[-1]) == expected_checker_report, evidence.stdout
                expected_snapshot = {
                    "snapshot": revision,
                    "snapshot_id": revision,
                    "snapshot_kind": expected_kind,
                    "tree_id": revision_tree,
                    "commit_id": expected_commit,
                }
                located_revision = cli(root, "locate", "--kind", "object", "--path", source, "--revision", revision)
                assert located_revision.returncode == 0, located_revision.stdout + located_revision.stderr
                locate_report = json.loads(located_revision.stdout)
                assert all(locate_report.get(key) == value for key, value in expected_snapshot.items()), locate_report
                exported_revision = cli(root, "export", "--revision", revision)
                assert exported_revision.returncode == 0, exported_revision.stdout + exported_revision.stderr
                export_report = json.loads(exported_revision.stdout)
                assert all(export_report.get(key) == value for key, value in expected_snapshot.items()), export_report
            expect_code(checker(root), "object-content-drift", 1)
        finally:
            source_path.write_bytes(committed_source)
        located = cli(root, "locate", "--kind", "object", "--path", source)
        assert located.returncode == 0, located.stdout + located.stderr
        exported = cli(root, "export", "--output", ".cache/doc-governance/export.json")
        assert exported.returncode == 0, exported.stdout + exported.stderr
        assert index_path.read_bytes() == index_before, "read/check/export changed the Git index bytes"
        assert repository_payload_snapshot(root) == payload_before, "read/check/export changed source or corpus record bytes/modes"
        assert git(root, "write-tree").stdout.strip() == tree_before
        assert git(root, "status", "--porcelain", "--untracked-files=no").stdout == tracked_before
        assert (root / source).read_bytes() == b"# agents-world-simulation\n"
    finally:
        remove_fixture(root)


def test_t23_replace_fault_never_reports_success_and_scoped_retry_recovers() -> None:
    root = make_fixture()
    injector = Path(tempfile.mkdtemp(prefix="oasis7-corpus-fault-")).resolve()
    try:
        git(root, "config", "core.autocrlf", "true")
        sources = [
            "doc/product/agents-world-simulation/README.md",
            "doc/product/player-entry-distribution/README.md",
        ]
        for index, source in enumerate(sources):
            target = root / source
            append_utf8_lf(target, f"fault-stage-{index}\n")
        old_objects = {source: object_bytes(root, source) for source in sources}
        hook = '''import importlib, os, sys\nsys.path.insert(0, os.environ["OASIS7_CORPUS_MODULE_DIR"])\nmodule = importlib.import_module("document_corpus")\noriginal = module._safe_write\ncount = 0\ndef fail_second(repo_root, path, data):\n    global count\n    count += 1\n    if count == 2:\n        raise OSError("injected second replace failure")\n    return original(repo_root, path, data)\nmodule._safe_write = fail_second\n'''
        write(injector, "sitecustomize.py", hook)
        env = os.environ.copy()
        env["PYTHONPATH"] = str(injector) + os.pathsep + env.get("PYTHONPATH", "")
        env["OASIS7_CORPUS_MODULE_DIR"] = str(ROOT / "scripts")
        argv = [sys.executable, str(CLI), "--repo-root", str(root), "sync"]
        for source in sources:
            argv.extend(("--path", source))
        argv.append("--apply")
        failed = subprocess.run(argv, text=True, capture_output=True, check=False, env=env)
        assert failed.returncode != 0, "fault-injected multi-file sync claimed full success"
        assert "injected second replace failure" in failed.stdout + failed.stderr
        assert any(object_bytes(root, source) != old_objects[source] for source in sources)
        repaired = cli(root, "sync", "--path", sources[0], "--path", sources[1], "--apply")
        assert repaired.returncode == 0, repaired.stdout + repaired.stderr
        valid = checker(root)
        assert valid.returncode == 0, valid.stdout + valid.stderr
    finally:
        remove_fixture(root)
        shutil.rmtree(injector, ignore_errors=True)


def test_safe_write_falls_back_without_fchmod_and_cleans_failed_temp() -> None:
    with tempfile.TemporaryDirectory(prefix="oasis7-safe-write-fallback-") as temp:
        root = Path(temp)
        success_path = "nested/success.bin"
        payload = b"portable write fallback\x00\xff"
        with patch.object(dc.os, "fchmod", None, create=True):
            dc._safe_write(root, success_path, payload)
        success_file = root / success_path
        assert success_file.read_bytes() == payload
        assert dc.WorktreeCorpusView(root).file_mode(success_path) == "100644"

        failed_path = "nested/failed.bin"
        failed_file = root / failed_path
        temp_parent = failed_file.parent
        with patch.object(dc.os, "fchmod", None, create=True), patch.object(
            dc.os, "chmod", side_effect=PermissionError("injected chmod failure")
        ):
            try:
                dc._safe_write(root, failed_path, b"must not be installed")
            except dc.CorpusError as exc:
                assert exc.diagnostic.code == "write-failed", exc.diagnostic.as_dict()
            else:
                raise AssertionError("forced chmod failure was reported as a successful write")
        assert not failed_file.exists(), "failed chmod left a target file"
        assert not list(temp_parent.glob(f".{failed_file.name}.*.tmp")), "failed chmod left a temporary file"


def test_t24_lock_is_worktree_local_and_contention_returns_three() -> None:
    root = make_fixture()
    other = root.parent / f"{root.name}-other-worktree"
    holder: subprocess.Popen[str] | None = None
    try:
        source = "doc/product/agents-world-simulation/README.md"
        lock_path = git(root, "rev-parse", "--git-path", "document-corpus-v3.lock").stdout.strip()
        if not Path(lock_path).is_absolute():
            lock_path = str(root / lock_path)
        lock_code = '''import os, sys, time\np=sys.argv[1]\nf=open(p,"a+b")\nif os.name == "nt":\n import msvcrt; f.seek(0); msvcrt.locking(f.fileno(), msvcrt.LK_NBLCK, 1)\nelse:\n import fcntl; fcntl.flock(f.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)\nprint("LOCKED", flush=True)\ntime.sleep(5)\n'''
        holder = subprocess.Popen([sys.executable, "-c", lock_code, lock_path], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        assert holder.stdout is not None
        assert holder.stdout.readline().strip() == "LOCKED", "lock helper did not acquire the test lock"
        expect_code(cli(root, "sync", "--path", source, "--apply"), "lock-busy", 3)

        git(root, "worktree", "add", "--quiet", "--detach", str(other), "base")
        other_source = other / source
        append_utf8_lf(other_source, "other worktree\n")
        independent = cli(other, "sync", "--path", source, "--apply")
        assert independent.returncode == 0, f"separate worktree was blocked:\n{independent.stdout}{independent.stderr}"
    finally:
        if holder is not None:
            holder.terminate()
            holder.wait(timeout=10)
        if other.exists():
            git(root, "worktree", "remove", "--force", str(other), check=False)
        remove_fixture(root)


def test_t25_space_unicode_crlf_and_case_alias_safety() -> None:
    root = make_fixture()
    try:
        # This case asserts raw CRLF preservation when Git has no EOL clean filter.
        git(root, "config", "core.autocrlf", "false")
        spaced = "doc/product/agents-world-simulation/notes for 雪.md"
        raw = b"# raw bytes\r\nsecond line\r\n"
        write(root, spaced, raw)
        synced = cli(root, "sync", "--path", spaced, "--apply")
        assert synced.returncode == 0, synced.stdout + synced.stderr
        record = read_record(root, record_path("object", spaced))["record"]
        assert record["content_sha256"] == sha(raw), "CRLF source hash was normalized"

        aliases = [
            "doc/product/agents-world-simulation/CaseAlias.md",
            "doc/product/agents-world-simulation/casealias.md",
        ]
        blob_oids: list[str] = []
        for payload in (b"upper spelling\n", b"lower spelling\n"):
            hashed = subprocess.run(["git", "-C", str(root), "hash-object", "-w", "--stdin"], input=payload, capture_output=True, check=False)
            assert hashed.returncode == 0, hashed.stderr.decode("utf-8", "replace")
            blob_oids.append(hashed.stdout.decode("ascii").strip())
        git(root, "config", "core.ignorecase", "false")
        git(root, "switch", "-c", "case-alias-revision")
        for path, oid in zip(aliases, blob_oids):
            result = git(root, "update-index", "--add", "--cacheinfo", f"100644,{oid},{path}", check=False)
            assert result.returncode == 0, f"could not create logical alias tree entry {path}: {result.stderr}"
        tree = git(root, "write-tree", check=False)
        assert tree.returncode == 0, tree.stderr
        parent = git(root, "rev-parse", "HEAD").stdout.strip()
        commit = git(root, "commit-tree", tree.stdout.strip(), "-p", parent, "-m", "case alias tree")
        expect_code(checker(root, revision=commit.stdout.strip()), "path-alias-collision", 1)
    finally:
        remove_fixture(root)


def test_autocrlf_true_rejects_crlf_that_git_would_normalize() -> None:
    root = make_fixture()
    try:
        git(root, "config", "core.autocrlf", "true")
        source = "doc/product/agents-world-simulation/README.md"
        object_before = object_bytes(root, source)
        write(root, source, b"# Windows checkout bytes\r\n")
        mismatch = cli(root, "sync", "--path", source, "--apply")
        diagnostic = expect_code(mismatch, "hash-input-mismatch", 1)
        assert diagnostic["source_path"] == source, diagnostic
        assert "Git clean filters would change" in diagnostic["detail"], diagnostic
        assert object_bytes(root, source) == object_before, "autocrlf mismatch changed the object"
    finally:
        remove_fixture(root)


def current_model_as_legacy_tables(root: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    objects = [
        read_record(root, path.relative_to(root).as_posix())["record"]
        for path in sorted((root / OBJECTS_ROOT).rglob("*.json"))
    ]
    semantic_entries = [
        read_record(root, path.relative_to(root).as_posix())["record"]
        for path in sorted((root / SEMANTIC_ROOT).rglob("*.json"))
    ] if (root / SEMANTIC_ROOT).exists() else []
    semantic_bundles = [
        read_record(root, path.relative_to(root).as_posix())["record"]
        for path in sorted((root / "doc/.governance/document-corpus/semantic/bundles").glob("*.json"))
    ]
    evidence_entries = [
        read_record(root, path.relative_to(root).as_posix())["record"]
        for path in sorted((root / EVIDENCE_ENTRIES_ROOT).rglob("*.json"))
    ]
    for path in sorted((root / EVIDENCE_GROUPS_ROOT).glob("*.json")):
        group = read_record(root, path.relative_to(root).as_posix())["record"]
        for member in group["entries"]:
            expanded = dict(member)
            expanded["atomic_group"] = group["id"]
            expanded["group_sha256"] = group["group_sha256"]
            evidence_entries.append(expanded)
    return (
        {"version": 2, "objects": sorted(objects, key=lambda item: item["path"])},
        {"version": 1, "entries": sorted(semantic_entries, key=lambda item: item["path"]), "bundles": sorted(semantic_bundles, key=lambda item: item["id"])},
        {"version": 1, "entries": sorted(evidence_entries, key=lambda item: item["path"])},
    )


def write_legacy_tables(root: Path) -> None:
    corpus, semantic, evidence = current_model_as_legacy_tables(root)
    shutil.rmtree(root / "doc/.governance/document-corpus", ignore_errors=True)
    (root / CORPUS_ROOT).unlink(missing_ok=True)
    write(root, CORPUS_ROOT, canonical(corpus))
    write(root, LEGACY_SEMANTIC_ROOT, canonical(semantic))
    write(root, EVIDENCE_ROOT, canonical(evidence))


def set_semantic_note(root: Path, source: str, note: str) -> None:
    path = record_path("semantic", source)
    record = read_record(root, path)["record"]
    record["note"] = note
    write_wrapped(root, path, "oasis7.document-semantic-entry/v1", record)


def set_legacy_semantic_field(root: Path, source: str, field: str, value: Any) -> None:
    legacy = json.loads((root / LEGACY_SEMANTIC_ROOT).read_text(encoding="utf-8"))
    entry = next(row for row in legacy["entries"] if row["path"] == source)
    entry[field] = value
    write(root, LEGACY_SEMANTIC_ROOT, canonical(legacy))


def set_legacy_evidence_field(root: Path, source: str, field: str, value: Any) -> None:
    legacy = json.loads((root / EVIDENCE_ROOT).read_text(encoding="utf-8"))
    entry = next(row for row in legacy["entries"] if row["path"] == source)
    entry[field] = value
    write(root, EVIDENCE_ROOT, canonical(legacy))


def set_v3_group_field(root: Path, group_id: str, source: str, field: str, value: Any) -> None:
    path = f"{EVIDENCE_GROUPS_ROOT}/{group_id}.json"
    group = read_record(root, path)["record"]
    entry = next(row for row in group["entries"] if row["path"] == source)
    entry[field] = value
    write_wrapped(root, path, "oasis7.document-evidence-group/v1", group)


def legacy_three_way_history(root: Path, *, head_note: str, target_source: str, target_note: str) -> tuple[str, str, str]:
    """Create small old B/H snapshots and a v3 target N from one shared fixture."""
    v3_base = git(root, "rev-parse", "base").stdout.strip()
    git(root, "switch", "-c", "legacy-format-base", v3_base)
    write_legacy_tables(root)
    base = commit_all(root, "synthetic legacy base")

    changed_source = "doc/product/README.md"
    git(root, "switch", "-c", "legacy-pr", base)
    set_legacy_semantic_field(root, changed_source, "note", head_note)
    legacy_head = commit_all(root, "synthetic legacy review change")

    git(root, "switch", "-c", "target-main", v3_base)
    set_semantic_note(root, target_source, target_note)
    target = commit_all(root, "synthetic v3 target review change")
    return base, legacy_head, target


def assert_legacy_export_matches_baseline(root: Path, output_dir: str) -> None:
    source_paths = {
        "corpus": CORPUS_ROOT,
        "semantic": LEGACY_SEMANTIC_ROOT,
        "evidence": EVIDENCE_ROOT,
    }
    output_names = {
        "corpus": "document-corpus-inventory.json",
        "semantic": "document-semantic-review-overrides.json",
        "evidence": "inventory.json",
    }
    old = {
        kind: json.loads(git(ROOT, "show", f"{LEGACY_BASE}:{path}").stdout)
        for kind, path in source_paths.items()
    }
    raw = {
        kind: (root / output_dir / name).read_bytes()
        for kind, name in output_names.items()
    }
    current = {kind: json.loads(data) for kind, data in raw.items()}

    # The legacy writer's serialization contract is ordered fields with its old
    # two-space JSON dump. Record arrays are compared by stable identity because
    # the v3 normalizer sorts them; record-field order and all field values stay
    # checked below.
    for kind in ("corpus", "semantic"):
        serialized = (json.dumps(current[kind], ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        assert raw[kind] == serialized, f"{kind} export did not use the legacy ordered-field/two-space serializer"
        assert list(current[kind]) == list(old[kind]), f"{kind} root fields were reordered"

    assert raw["evidence"] == git(ROOT, "show", f"{LEGACY_BASE}:{EVIDENCE_ROOT}").stdout.encode("utf-8"), "legacy evidence table bytes changed"

    old_corpus = old["corpus"]
    current_corpus = current["corpus"]
    for field in ("version", "scope", "decision_boundary", "product_modules"):
        assert current_corpus[field] == old_corpus[field], f"legacy corpus field changed: {field}"
    assert len(current_corpus["controls"]) == len(old_corpus["controls"])
    for index, (actual, expected) in enumerate(zip(current_corpus["controls"], old_corpus["controls"])):
        assert list(actual) == list(expected), f"legacy control {index} field order changed"
        if index == 1:
            assert actual["path"] == LEGACY_SEMANTIC_ROOT
            assert actual["content_sha256"] == sha(raw["semantic"])
        elif index == 2:
            assert actual["path"] == EVIDENCE_ROOT
            assert actual["content_sha256"] == sha(raw["evidence"])
        else:
            assert actual == expected, f"legacy control {index} changed"
    assert len(current_corpus["delegates"]) == len(old_corpus["delegates"])
    for actual, expected in zip(current_corpus["delegates"], old_corpus["delegates"]):
        assert list(actual) == list(expected), "legacy delegate field order changed"
        for field, value in expected.items():
            if field == "sha256":
                assert actual[field] == sha(raw["evidence"])
            elif field == "object_count":
                assert actual[field] == len(current["evidence"]["entries"])
            else:
                assert actual[field] == value, f"legacy delegate field changed: {field}"

    old_objects = {row["path"]: row for row in old_corpus["objects"]}
    current_objects = {row["path"]: row for row in current_corpus["objects"]}
    assert current_objects.keys() == old_objects.keys(), "legacy object path set changed"
    for path, expected in old_objects.items():
        actual = current_objects[path]
        assert list(actual) == list(expected), f"legacy object field order changed for {path}"
        expected_values = dict(expected)
        if path == "doc/engineering/workflow/source-of-truth.md":
            expected_values["content_sha256"] = sha((root / path).read_bytes())
        assert actual == expected_values, f"legacy object values changed beyond the known stale source hash: {path}"

    old_semantic = old["semantic"]
    current_semantic = current["semantic"]
    for field in old_semantic:
        if field in {"entries", "bundles"}:
            continue
        assert current_semantic[field] == old_semantic[field], f"legacy semantic metadata changed: {field}"
    for rows_key, identity in (("entries", "path"), ("bundles", "id")):
        before_rows = {row[identity]: row for row in old_semantic[rows_key]}
        after_rows = {row[identity]: row for row in current_semantic[rows_key]}
        assert after_rows.keys() == before_rows.keys(), f"legacy semantic {rows_key} identity set changed"
        for key, expected in before_rows.items():
            actual = after_rows[key]
            assert list(actual) == list(expected), f"legacy semantic field order changed for {key}"
            assert actual == expected, f"legacy semantic values changed for {key}"


def test_t26_t27_full_migration_roundtrip_and_repeated_preconditions() -> None:
    # This test checks out the immutable production baseline into a linked
    # worktree. Pin checkout and every child Git invocation before worktree add
    # so a developer's global core.autocrlf setting cannot rewrite the fixture.
    with patch.dict(os.environ, {
        "GIT_CONFIG_COUNT": "1",
        "GIT_CONFIG_KEY_0": "core.autocrlf",
        "GIT_CONFIG_VALUE_0": "false",
    }):
        assert git(ROOT, "config", "--get", "core.autocrlf").stdout.strip() == "false"
        _run_t26_t27_full_migration_roundtrip_and_repeated_preconditions()


def _run_t26_t27_full_migration_roundtrip_and_repeated_preconditions() -> None:
    container = Path(tempfile.mkdtemp(prefix="oasis7-corpus-migration-")).resolve()
    target = container / "repo"
    try:
        git(ROOT, "worktree", "add", "--quiet", "--detach", str(target), LEGACY_BASE)
        report1 = ".cache/doc-governance/migration-plan-1.json"
        report2 = ".cache/doc-governance/migration-plan-2.json"
        first = cli(target, "migrate-v2", "--source-revision", LEGACY_BASE, "--report", report1)
        second = cli(target, "migrate-v2", "--source-revision", LEGACY_BASE, "--report", report2)
        assert first.returncode == second.returncode == 0, first.stdout + first.stderr + second.stdout + second.stderr
        planned1 = json.loads((target / report1).read_text(encoding="utf-8"))
        planned2 = json.loads((target / report2).read_text(encoding="utf-8"))
        assert planned1["roundtrip"].startswith("passed:"), planned1
        assert planned1["legacy_counts"] == {"objects": 511, "semantic_entries": 111, "semantic_bundles": 3, "evidence_entries": 128}
        assert planned1["write_set"] == planned2["write_set"], "same source migration dry-runs changed the planned writes"
        assert planned1["preconditions"] == planned2["preconditions"], "same source migration dry-runs changed their preconditions"
        assert git(target, "status", "--porcelain", "--untracked-files=no").stdout == "", "migration dry-run changed tracked files"

        old_corpus = json.loads(git(target, "show", f"{LEGACY_BASE}:{CORPUS_ROOT}").stdout)
        sample_source = old_corpus["objects"][0]["path"]
        conflict_path = record_path("object", sample_source)
        write(target, conflict_path, b"external shard edit\n")
        conflict = cli(target, "migrate-v2", "--source-revision", LEGACY_BASE, "--report", ".cache/doc-governance/migration-conflict.json")
        expect_code(conflict, "migration-target-drift", 1)
        assert (target / conflict_path).read_bytes() == b"external shard edit\n", "migration overwrote a conflicting preexisting shard"
        (target / conflict_path).unlink()

        applied = cli(target, "migrate-v2", "--source-revision", LEGACY_BASE, "--report", ".cache/doc-governance/migration-applied.json", "--apply")
        assert applied.returncode == 0, applied.stdout + applied.stderr
        corpus_valid = checker(target)
        evidence_valid = checker(target, evidence=True)
        assert corpus_valid.returncode == 0, corpus_valid.stdout + corpus_valid.stderr
        assert evidence_valid.returncode == 0, evidence_valid.stdout + evidence_valid.stderr

        exported_dir = ".cache/doc-governance/legacy-export"
        exported = cli(target, "export-legacy", "--output-dir", exported_dir)
        assert exported.returncode == 0, exported.stdout + exported.stderr
        assert_legacy_export_matches_baseline(target, exported_dir)

        before_repeat = {
            path.relative_to(target).as_posix(): path.read_bytes()
            for path in (target / "doc").rglob("*")
            if path.is_file()
        }
        repeated = cli(target, "migrate-v2", "--source-revision", LEGACY_BASE, "--report", ".cache/doc-governance/migration-repeat.json", "--apply")
        expect_code(repeated, "legacy-target-drift", 1)
        after_repeat = {
            path.relative_to(target).as_posix(): path.read_bytes()
            for path in (target / "doc").rglob("*")
            if path.is_file()
        }
        assert after_repeat == before_repeat, "rejected repeated migration modified migrated document files"
    finally:
        result = git(ROOT, "worktree", "remove", "--force", str(target), check=False)
        shutil.rmtree(container, ignore_errors=True)
        if result.returncode:
            raise AssertionError(f"failed to remove migration worktree: {result.stderr}")


def test_t28_three_way_import_preserves_independent_main_review() -> None:
    root = make_fixture()
    try:
        base, legacy_head, target = legacy_three_way_history(
            root,
            head_note="old PR review update",
            target_source="doc/engineering/README.md",
            target_note="independent main review update",
        )
        output = ".cache/doc-governance/legacy-delta.json"
        result = cli(root, "import-legacy-delta", "--base", base, "--legacy-head", legacy_head, "--target", target, "--output", output)
        assert result.returncode == 0, result.stdout + result.stderr
        proposal = json.loads((root / output).read_text(encoding="utf-8"))
        mutations = proposal["mutations"]
        assert len(mutations) == 1, f"independent target review was included in legacy proposal: {mutations!r}"
        assert mutations[0]["record_path"] == record_path("semantic", "doc/product/README.md")
        assert mutations[0]["record"]["path"] == "doc/product/README.md"
        assert mutations[0]["record"]["note"] == "old PR review update"
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        v3_base = git(root, "rev-parse", "base").stdout.strip()
        groups = [
            json.loads(path.read_text(encoding="utf-8"))["record"]
            for path in sorted((root / EVIDENCE_GROUPS_ROOT).glob("*.json"))
        ]
        assert len(groups) >= 2, "fixture needs two independent atomic evidence groups"
        legacy_group, main_group = groups[:2]
        legacy_member = legacy_group["entries"][0]["path"]
        main_member = main_group["entries"][0]["path"]

        git(root, "switch", "-c", "legacy-format-group-base", v3_base)
        write_legacy_tables(root)
        base = commit_all(root, "synthetic two-group legacy base")
        git(root, "switch", "-c", "legacy-pr-one-group", base)
        set_legacy_evidence_field(root, legacy_member, "rationale", "legacy PR updated group one")
        legacy_head = commit_all(root, "synthetic legacy group review change")
        git(root, "switch", "-c", "target-main", v3_base)
        set_v3_group_field(root, main_group["id"], main_member, "rationale", "main independently updated group two")
        target = commit_all(root, "synthetic main independent group review change")

        proposal_path = ".cache/doc-governance/independent-groups.json"
        result = cli(root, "import-legacy-delta", "--base", base, "--legacy-head", legacy_head, "--target", target, "--output", proposal_path)
        assert result.returncode == 0, result.stdout + result.stderr
        proposal = json.loads((root / proposal_path).read_text(encoding="utf-8"))
        mutations = proposal["mutations"]
        assert len(mutations) == 1, f"independent main group leaked into legacy proposal: {mutations!r}"
        assert mutations[0]["record_path"] == f"{EVIDENCE_GROUPS_ROOT}/{legacy_group['id']}.json"
        assert mutations[0]["record"]["id"] == legacy_group["id"]
        assert mutations[0]["record"]["entries"][0]["rationale"] == "legacy PR updated group one"
        main_after = json.loads(git(root, "show", f"{target}:{EVIDENCE_GROUPS_ROOT}/{main_group['id']}.json").stdout)["record"]
        assert main_after["entries"][0]["rationale"] == "main independently updated group two"
    finally:
        remove_fixture(root)


def test_t29_three_way_same_review_conflict_and_source_mismatch() -> None:
    root = make_fixture()
    try:
        base, legacy_head, target = legacy_three_way_history(
            root,
            head_note="old PR changed same review",
            target_source="doc/product/README.md",
            target_note="main changed same review",
        )
        conflict = cli(root, "import-legacy-delta", "--base", base, "--legacy-head", legacy_head, "--target", target, "--output", ".cache/doc-governance/conflict.json")
        expect_code(conflict, "legacy-delta-conflict", 1)
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        v3_base = git(root, "rev-parse", "base").stdout.strip()
        frozen = json.loads(LEGACY_INVENTORY.read_text(encoding="utf-8"))
        group_id = next(row["atomic_group"] for row in frozen["entries"] if row.get("atomic_group"))
        members = [row["path"] for row in frozen["entries"] if row.get("atomic_group") == group_id]
        assert len(members) > 1
        git(root, "switch", "-c", "legacy-format-base", v3_base)
        write_legacy_tables(root)
        base = commit_all(root, "synthetic grouped legacy base")
        git(root, "switch", "-c", "legacy-pr-group", base)
        set_legacy_evidence_field(root, members[0], "rationale", "legacy PR changed one member of the group")
        legacy_head = commit_all(root, "synthetic old grouped evidence change")
        git(root, "switch", "-c", "target-main", v3_base)
        set_v3_group_field(root, group_id, members[1], "rationale", "main changed a different group member")
        target = commit_all(root, "synthetic main grouped evidence change")
        result = cli(root, "import-legacy-delta", "--base", base, "--legacy-head", legacy_head, "--target", target, "--output", ".cache/doc-governance/group-conflict.json")
        failure = expect_code(result, "legacy-delta-conflict", 1)
        assert f"evidence-atomic-unit:group:{group_id}" in failure["detail"], failure
    finally:
        remove_fixture(root)

    root = make_fixture()
    try:
        source = "doc/product/agents-world-simulation/README.md"
        source_file = root / source
        source_file.write_bytes(b"# staged CRLF\r\n")
        git(root, "add", source)
        source_file.write_bytes(b"# staged CRLF\nworking tree differs\n")
        before = object_bytes(root, source)
        mismatch = cli(root, "sync", "--path", source, "--apply")
        expect_code(mismatch, "hash-input-mismatch", 1)
        assert "staged blob bytes differ" in mismatch.stdout + mismatch.stderr
        assert object_bytes(root, source) == before, "staged/worktree mismatch changed its object"
    finally:
        remove_fixture(root)

    root = make_fixture()
    marker = root.parent / f"{root.name}-clean-filter-ran"
    try:
        source = "doc/product/agents-world-simulation/filtered.md"
        write(root, ".gitattributes", f"{source} filter=qa-test-clean\n")
        git(root, "add", ".gitattributes")
        filter_command = f'"{sys.executable}" -c "from pathlib import Path; Path(\'clean-filter-ran\').write_text(\'ran\')"'
        git(root, "config", "filter.qa-test-clean.clean", filter_command)
        write(root, source, b"raw filter input\n")
        filtered = cli(root, "sync", "--path", source, "--apply")
        expect_code(filtered, "hash-input-mismatch", 1)
        assert not marker.exists(), "corpus management executed the configured Git clean filter"
    finally:
        marker.unlink(missing_ok=True)
        remove_fixture(root)

    root = make_fixture()
    try:
        v3_base = git(root, "rev-parse", "base").stdout.strip()
        git(root, "switch", "-c", "legacy-format-base", v3_base)
        write_legacy_tables(root)
        base = commit_all(root, "synthetic source-mismatch legacy base")
        git(root, "switch", "-c", "legacy-pr-source-mismatch", base)
        set_legacy_semantic_field(root, "doc/product/README.md", "note", "old review with invalid source hash")
        set_legacy_semantic_field(root, "doc/product/README.md", "content_sha256", "0" * 64)
        legacy_head = commit_all(root, "synthetic mismatched legacy source hash")
        result = cli(root, "import-legacy-delta", "--base", base, "--legacy-head", legacy_head, "--target", v3_base, "--output", ".cache/doc-governance/source-mismatch.json")
        expect_code(result, "legacy-delta-source-drift", 1)
    finally:
        remove_fixture(root)


def test_t30_legacy_tables_and_print_generated_are_explicitly_retired() -> None:
    root = make_fixture()
    try:
        rollback = cli(root, "export-legacy", "--output-dir", ".cache/doc-governance/legacy-export")
        expect_code(rollback, "legacy-export-unrepresentable", 1)

        legacy = {
            "version": 2,
            "scope": "all doc files through direct objects, delegated evidence objects, and controls",
            "objects": [],
            "controls": [],
            "delegates": [],
        }
        write(root, CORPUS_ROOT, canonical(legacy))
        expect_any = checker(root)
        assert expect_any.returncode == 1, expect_any.stdout + expect_any.stderr
        codes = {item.get("code") for item in diagnostics(expect_any)}
        assert codes & {"legacy-format-disabled", "schema-version", "corpus-entrypoint-contract"}, expect_any.stdout + expect_any.stderr
        old_flag = checked_run([sys.executable, str(CORPUS_CHECK), "--repo-root", str(root), "--print-generated"])
        assert old_flag.returncode == 2, f"retired --print-generated should be an argument error:\n{old_flag.stdout}{old_flag.stderr}"
        assert "export" in old_flag.stdout + old_flag.stderr
        unbounded_rollback = cli(root, "rollback", "--apply")
        assert unbounded_rollback.returncode == 2, f"unbounded rollback command unexpectedly accepted:\n{unbounded_rollback.stdout}{unbounded_rollback.stderr}"
    finally:
        remove_fixture(root)


def main() -> None:
    tests = [
        test_t01_sync_updates_one_object_only,
        test_t02_repeated_sync_propose_export_are_deterministic,
        test_t03_new_source_without_object_is_reported,
        test_t04_delete_requires_explicit_object_only_when_no_review_record,
        test_t05_strict_keys_and_duplicate_members_fail_closed,
        test_t06_metadata_cannot_become_a_recursive_object_source,
        test_t07_unknown_store_file_and_symlink_fail_closed,
        test_t08_nested_inventory_json_is_an_evidence_source_by_path,
        test_t09_registry_and_product_boundary_are_not_inferred_from_shards,
        test_t18_different_sources_in_one_module_merge_cleanly_both_orders,
        test_t19_adjacent_new_paths_do_not_share_a_sorted_parent_list,
        test_t20_ordinary_object_and_independent_evidence_merge_both_orders,
        test_t21_auto_merged_same_source_still_conflicts_on_its_object,
        test_t22_readers_and_export_do_not_change_source_or_index,
        test_t23_replace_fault_never_reports_success_and_scoped_retry_recovers,
        test_safe_write_falls_back_without_fchmod_and_cleans_failed_temp,
        test_t24_lock_is_worktree_local_and_contention_returns_three,
        test_t25_space_unicode_crlf_and_case_alias_safety,
        test_autocrlf_true_rejects_crlf_that_git_would_normalize,
        test_t26_t27_full_migration_roundtrip_and_repeated_preconditions,
        test_t28_three_way_import_preserves_independent_main_review,
        test_t29_three_way_same_review_conflict_and_source_mismatch,
        test_t30_legacy_tables_and_print_generated_are_explicitly_retired,
    ]
    try:
        for test in tests:
            test()
        print(f"document-corpus-inventory-check.test: OK ({len(tests)} scenarios)")
    finally:
        destroy_fixture()


if __name__ == "__main__":
    main()
