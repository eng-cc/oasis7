#!/usr/bin/env python3
"""Black-box regressions for evidence sidecars, review drift, and frozen policy."""
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
CLI = ROOT / "scripts/document-corpus-inventory.py"
EVIDENCE_CHECK = ROOT / "scripts/doc-evidence-inventory-check.py"
CORPUS_TEST = ROOT / "scripts/document-corpus-inventory-check.test.py"
GROUPS_ROOT = "doc/.governance/document-corpus/evidence/groups"
EVIDENCE_ROOT = "doc/testing/evidence/inventory.json"


_spec = importlib.util.spec_from_file_location("corpus_test_harness", CORPUS_TEST)
assert _spec is not None and _spec.loader is not None
_harness = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_harness)


def sha(data: bytes) -> str:
    return _harness.sha(data)


def checked_run(argv: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, text=True, encoding="utf-8", errors="replace", capture_output=True, check=False)


def cli(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return checked_run([sys.executable, str(CLI), "--repo-root", str(root), *args])


def checker(root: Path) -> subprocess.CompletedProcess[str]:
    return checked_run([sys.executable, str(EVIDENCE_CHECK), "--repo-root", str(root)])


def expect_code(result: subprocess.CompletedProcess[str], code: str, exit_code: int = 1) -> dict[str, Any]:
    output = result.stdout + result.stderr
    assert result.returncode == exit_code, f"expected exit {exit_code}, got {result.returncode}:\n{output}"
    matches = [item for item in _harness.diagnostics(result) if item.get("code") == code]
    assert matches, f"missing diagnostic {code}:\n{output}"
    required = {"code", "source_path", "record_path", "detail", "repair_command"}
    assert required <= set(matches[0]), f"diagnostic {code} omitted fields: {matches[0]!r}"
    return matches[0]


def expect_any_code(result: subprocess.CompletedProcess[str], codes: set[str]) -> dict[str, Any]:
    output = result.stdout + result.stderr
    assert result.returncode == 1, f"expected validation failure, got {result.returncode}:\n{output}"
    found = [item for item in _harness.diagnostics(result) if item.get("code") in codes]
    assert found, f"expected one of {sorted(codes)}:\n{output}"
    required = {"code", "source_path", "record_path", "detail", "repair_command"}
    assert required <= set(found[0]), f"diagnostic omitted fields: {found[0]!r}"
    return found[0]


def record_path(source: str) -> str:
    return _harness.record_path("evidence", source)


def evidence_record(root: Path, source: str, *, role: str = "fixture_evidence") -> dict[str, Any]:
    payload = (root / source).read_bytes()
    return {
        "path": source,
        "lifecycle": "HISTORICAL_PROVENANCE",
        "semantic_role": role,
        "retention_owner": "qa_engineer",
        "domain_owner": "qa_engineer",
        "required_followup_roles": ["qa_engineer"],
        "authority": "doc/testing/README.md",
        "backlink": "doc/testing/README.md",
        "disposition": "retain",
        "rationale": "Synthetic evidence inventory regression; not live readiness evidence.",
        "residual_risk": "Fixture content has no production evidentiary value.",
        "content_sha256": sha(payload),
    }


def write_evidence(root: Path, source: str, record: dict[str, Any]) -> None:
    _harness.write_wrapped(
        root,
        record_path(source),
        "oasis7.document-evidence-entry/v1",
        record,
    )


def semantic_record(root: Path, source: str, **overrides: Any) -> dict[str, Any]:
    defaults = {
        "path": source,
        "content_sha256": sha((root / source).read_bytes()),
        "disposition": "retain_current_authority",
        "decision_owner": "producer_system_designer",
        "current_authority": "doc/product/README.md",
        "note": "Synthetic semantic record for evidence-adjacent regression.",
    }
    defaults.update(overrides)
    return defaults


def semantic_digest(root: Path, paths: list[str]) -> str:
    payload = b"".join(
        path.encode("utf-8") + b"\0" + sha((root / path).read_bytes()).encode("ascii") + b"\n"
        for path in sorted(paths)
    )
    return sha(payload)


def group_digest(records: list[dict[str, Any]]) -> str:
    payload = "".join(f"{record['content_sha256']}  {record['path']}\n" for record in sorted(records, key=lambda item: item["path"]))
    return sha(payload.encode("utf-8"))


def test_t10_reviewed_content_change_does_not_refresh_the_review() -> None:
    root = _harness.make_fixture()
    try:
        source = "doc/product/agents-world-simulation/README.md"
        record = semantic_record(root, source)
        path = _harness.record_path("semantic", source)
        _harness.write_wrapped(root, path, "oasis7.document-semantic-entry/v1", record)
        before = (root / path).read_bytes()
        source_file = root / source
        source_file.write_text(source_file.read_text(encoding="utf-8") + "changed after review\n", encoding="utf-8")
        sync = cli(root, "sync", "--path", source, "--apply")
        assert sync.returncode == 0, sync.stdout + sync.stderr
        assert (root / path).read_bytes() == before, "ordinary object sync renewed semantic review bytes"
        expect_code(_harness.checker(root), "semantic-content-drift", 1)
    finally:
        _harness.remove_fixture(root)


def test_t11_bundle_digest_and_member_contract_are_preserved() -> None:
    root = _harness.make_fixture()
    try:
        paths = [
            "doc/product/agents-world-simulation/README.md",
            "doc/product/player-entry-distribution/README.md",
        ]
        bundle = {
            "id": "fixture-bundle",
            "paths": sorted(paths),
            "content_set_sha256": semantic_digest(root, paths),
            "disposition": "retain_current_authority",
            "decision_owner": "producer_system_designer",
            "current_authority": "doc/product/README.md",
            "note": "Synthetic bundle for content-set digest verification.",
        }
        _harness.write_wrapped(root, "doc/.governance/document-corpus/semantic/bundles/fixture-bundle.json", "oasis7.document-semantic-bundle/v1", bundle)
        valid = _harness.checker(root)
        assert valid.returncode == 0, valid.stdout + valid.stderr
        target = root / paths[0]
        target.write_text(target.read_text(encoding="utf-8") + "bundle source changed\n", encoding="utf-8")
        sync = cli(root, "sync", "--path", paths[0], "--apply")
        assert sync.returncode == 0, sync.stdout + sync.stderr
        expect_code(_harness.checker(root), "semantic-bundle-drift", 1)
    finally:
        _harness.remove_fixture(root)

    for malformed_paths in (list(reversed(paths)), [paths[0], paths[0]]):
        root = _harness.make_fixture()
        try:
            bundle["paths"] = malformed_paths
            bundle["content_set_sha256"] = semantic_digest(root, paths)
            _harness.write_wrapped(root, "doc/.governance/document-corpus/semantic/bundles/fixture-bundle.json", "oasis7.document-semantic-bundle/v1", bundle)
            expect_code(_harness.checker(root), "record-members-invalid", 1)
        finally:
            _harness.remove_fixture(root)


def test_t12_invalid_semantic_authority_owner_disposition_and_obligation_fail() -> None:
    invalid_cases = [
        ("semantic-owner", {"decision_owner": "not_a_role"}),
        ("semantic-disposition", {"disposition": "current_pass"}),
        ("semantic-authority", {"current_authority": "doc/missing-authority.md"}),
    ]
    for expected, override in invalid_cases:
        root = _harness.make_fixture()
        try:
            source = "doc/product/world-infrastructure/README.md"
            record = semantic_record(root, source, **override)
            _harness.write_wrapped(root, _harness.record_path("semantic", source), "oasis7.document-semantic-entry/v1", record)
            expect_code(_harness.checker(root), expected, 1)
        finally:
            _harness.remove_fixture(root)

    root = _harness.make_fixture()
    try:
        source = "doc/product/world-rules-core-gameplay/README.md"
        record = semantic_record(root, source, disposition="retain_outstanding_fulfillment_obligation")
        _harness.write_wrapped(root, _harness.record_path("semantic", source), "oasis7.document-semantic-entry/v1", record)
        expect_code(_harness.checker(root), "semantic-obligation-gate", 1)
    finally:
        _harness.remove_fixture(root)


def test_t13_stale_review_proposal_returns_three_without_writes() -> None:
    root = _harness.make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        proposal = ".cache/doc-governance/review-proposal.json"
        planned = cli(root, "propose", "--kind", "evidence", "--path", source, "--output", proposal)
        assert planned.returncode == 0, planned.stdout + planned.stderr
        proposal_bytes = (root / proposal).read_bytes()
        record_files = {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in (root / "doc/.governance/document-corpus").rglob("*")
            if path.is_file()
        }
        source_file = root / source
        source_file.write_text(source_file.read_text(encoding="utf-8") + "stale proposal source\n", encoding="utf-8")
        source_after = source_file.read_bytes()
        result = cli(root, "apply-review", "--proposal", proposal, "--apply")
        expect_code(result, "proposal-stale", 3)
        after_files = {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in (root / "doc/.governance/document-corpus").rglob("*")
            if path.is_file()
        }
        assert after_files == record_files, "stale apply partially wrote a record"
        assert source_file.read_bytes() == source_after
        assert (root / proposal).read_bytes() == proposal_bytes
    finally:
        _harness.remove_fixture(root)


def test_review_proposal_shape_preconditions_and_complete_upserts() -> None:
    root = _harness.make_fixture()
    try:
        semantic_source = "doc/product/README.md"
        semantic_target = _harness.record_path("semantic", semantic_source)
        original_semantic = (root / semantic_target).read_bytes()
        semantic_proposal = ".cache/doc-governance/semantic-review.json"
        planned = cli(root, "propose", "--kind", "semantic", "--path", semantic_source, "--output", semantic_proposal)
        assert planned.returncode == 0, planned.stdout + planned.stderr
        semantic_value = json.loads((root / semantic_proposal).read_text(encoding="utf-8"))
        semantic_record_value = semantic_value["mutations"][0]["record"]
        semantic_record_value["note"] = "Explicit reviewer update; no other disposition fields changed."
        _harness.write(root, semantic_proposal, _harness.canonical(semantic_value))
        applied_semantic = cli(root, "apply-review", "--proposal", semantic_proposal, "--apply")
        assert applied_semantic.returncode == 0, applied_semantic.stdout + applied_semantic.stderr
        after_semantic = json.loads((root / semantic_target).read_text(encoding="utf-8"))["record"]
        assert after_semantic["note"] == semantic_record_value["note"]
        before_record = json.loads(original_semantic)["record"]
        assert {key: value for key, value in after_semantic.items() if key != "note"} == {
            key: value for key, value in before_record.items() if key != "note"
        }, "semantic upsert changed fields beyond the explicit reviewer note"
        semantic_bytes = (root / semantic_target).read_bytes()
        replay = cli(root, "apply-review", "--proposal", semantic_proposal, "--apply")
        assert replay.returncode == 0, replay.stdout + replay.stderr
        assert json.loads(replay.stdout)["already_applied"] is True
        assert (root / semantic_target).read_bytes() == semantic_bytes, "idempotent semantic review replay rewrote the record"

        evidence_source = "doc/testing/evidence/fixture-2026-09-30.md"
        evidence_target = record_path(evidence_source)
        original_evidence = (root / evidence_target).read_bytes()
        evidence_proposal = ".cache/doc-governance/evidence-review.json"
        planned = cli(root, "propose", "--kind", "evidence", "--path", evidence_source, "--output", evidence_proposal)
        assert planned.returncode == 0, planned.stdout + planned.stderr
        evidence_value = json.loads((root / evidence_proposal).read_text(encoding="utf-8"))
        evidence_record_value = evidence_value["mutations"][0]["record"]
        evidence_record_value["rationale"] = "Explicit reviewer update; preserves every other evidence field."
        _harness.write(root, evidence_proposal, _harness.canonical(evidence_value))
        applied_evidence = cli(root, "apply-review", "--proposal", evidence_proposal, "--apply")
        assert applied_evidence.returncode == 0, applied_evidence.stdout + applied_evidence.stderr
        after_evidence = json.loads((root / evidence_target).read_text(encoding="utf-8"))["record"]
        assert after_evidence["rationale"] == evidence_record_value["rationale"]
        before_evidence_record = json.loads(original_evidence)["record"]
        assert {key: value for key, value in after_evidence.items() if key != "rationale"} == {
            key: value for key, value in before_evidence_record.items() if key != "rationale"
        }, "evidence upsert changed fields beyond the explicit reviewer rationale"
        valid_corpus = _harness.checker(root)
        valid_evidence = checker(root)
        assert valid_corpus.returncode == valid_evidence.returncode == 0, valid_corpus.stdout + valid_corpus.stderr + valid_evidence.stdout + valid_evidence.stderr
    finally:
        _harness.remove_fixture(root)

    for level, extra_field, extra_value in (
        ("top-level", "approved", True),
        ("mutation", "unexpected", "must reject"),
    ):
        root = _harness.make_fixture()
        try:
            source = "doc/product/README.md"
            proposal = ".cache/doc-governance/tampered-proposal.json"
            planned = cli(root, "propose", "--kind", "semantic", "--path", source, "--output", proposal)
            assert planned.returncode == 0, planned.stdout + planned.stderr
            value = json.loads((root / proposal).read_text(encoding="utf-8"))
            if level == "top-level":
                value[extra_field] = extra_value
            else:
                value["mutations"][0][extra_field] = extra_value
            target = _harness.record_path("semantic", source)
            original = (root / target).read_bytes()
            _harness.write(root, proposal, _harness.canonical(value))
            rejected = cli(root, "apply-review", "--proposal", proposal, "--apply")
            expect_code(rejected, "proposal-schema", 1)
            assert (root / target).read_bytes() == original, "malformed proposal wrote a semantic record"
        finally:
            _harness.remove_fixture(root)

    root = _harness.make_fixture()
    try:
        source = "doc/product/README.md"
        proposal = ".cache/doc-governance/tampered-precondition.json"
        planned = cli(root, "propose", "--kind", "semantic", "--path", source, "--output", proposal)
        assert planned.returncode == 0, planned.stdout + planned.stderr
        value = json.loads((root / proposal).read_text(encoding="utf-8"))
        value["mutations"][0]["record"]["note"] = "Different candidate with a deliberately stale record precondition."
        value["record_preconditions"][0]["sha256"] = "0" * 64
        target = _harness.record_path("semantic", source)
        original = (root / target).read_bytes()
        _harness.write(root, proposal, _harness.canonical(value))
        rejected = cli(root, "apply-review", "--proposal", proposal, "--apply")
        expect_code(rejected, "proposal-stale", 3)
        assert (root / target).read_bytes() == original, "stale record precondition wrote a semantic record"
    finally:
        _harness.remove_fixture(root)


def test_t14_group_member_drift_and_membership_changes_fail() -> None:
    root = _harness.make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        entry_path = record_path(source)
        entry = json.loads((root / entry_path).read_text(encoding="utf-8"))["record"]
        group = {"id": "fixture-window", "group_sha256": group_digest([entry]), "entries": [entry]}
        _harness.write_wrapped(root, f"{GROUPS_ROOT}/fixture-window.json", "oasis7.document-evidence-group/v1", group)
        (root / entry_path).unlink()
        assert checker(root).returncode == 0, checker(root).stdout + checker(root).stderr
        source_file = root / source
        original_source = source_file.read_bytes()
        source_file.write_text(source_file.read_text(encoding="utf-8") + "member changed\n", encoding="utf-8")
        expect_code(checker(root), "evidence-content-drift", 1)
        source_file.write_bytes(original_source)

        group["entries"][0]["content_sha256"] = "0" * 64
        _harness.write_wrapped(root, f"{GROUPS_ROOT}/fixture-window.json", "oasis7.document-evidence-group/v1", group)
        expect_code(_harness.checker(root), "evidence-group-drift", 1)
        expect_code(checker(root), "evidence-group-drift", 1)

        _harness.write_wrapped(root, entry_path, "oasis7.document-evidence-entry/v1", entry)
        expect_code(_harness.checker(root), "evidence-membership-overlap", 1)
    finally:
        _harness.remove_fixture(root)

    root = _harness.make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        entry_path = record_path(source)
        entry = json.loads((root / entry_path).read_text(encoding="utf-8"))["record"]
        group = {"id": "fixture-window", "group_sha256": group_digest([entry]), "entries": [entry, dict(entry)]}
        _harness.write_wrapped(root, f"{GROUPS_ROOT}/fixture-window.json", "oasis7.document-evidence-group/v1", group)
        expect_code(checker(root), "record-members-invalid", 1)
    finally:
        _harness.remove_fixture(root)

    root = _harness.make_fixture()
    try:
        grouped = [
            row["atomic_group"]
            for row in json.loads(_harness.LEGACY_INVENTORY.read_text(encoding="utf-8"))["entries"]
            if row.get("atomic_group")
        ]
        group_id = grouped[0]
        path = f"{GROUPS_ROOT}/{group_id}.json"
        group = json.loads((root / path).read_text(encoding="utf-8"))["record"]
        assert len(group["entries"]) > 1
        group["entries"].pop()
        group["group_sha256"] = group_digest(group["entries"])
        _harness.write_wrapped(root, path, "oasis7.document-evidence-group/v1", group)
        expect_any_code(checker(root), {"path-coverage", "frozen-cohort-drift"})
    finally:
        _harness.remove_fixture(root)


def test_t15_new_out_of_cohort_evidence_does_not_rewrite_static_roots_or_groups() -> None:
    root = _harness.make_fixture()
    try:
        static_before = (root / EVIDENCE_ROOT).read_bytes()
        groups_before = {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in (root / GROUPS_ROOT).rglob("*")
            if path.is_file()
        }
        source = "doc/testing/evidence/new-owner-review-2026-09-30.txt"
        payload = b"new, reviewable evidence; outside every frozen cohort\r\n"
        _harness.write(root, source, payload)
        write_evidence(root, source, evidence_record(root, source))
        valid = checker(root)
        assert valid.returncode == 0, valid.stdout + valid.stderr
        assert (root / EVIDENCE_ROOT).read_bytes() == static_before
        groups_after = {
            path.relative_to(root).as_posix(): path.read_bytes()
            for path in (root / GROUPS_ROOT).rglob("*")
            if path.is_file()
        }
        assert groups_after == groups_before, "new standalone evidence rewrote a frozen group"
    finally:
        _harness.remove_fixture(root)


def test_t16_t17_frozen_cohort_boundary_and_historical_readme_claims() -> None:
    root = _harness.make_fixture()
    try:
        peer = "doc/testing/evidence/public-testnet-governed-bootstrap-validator-triad-bootstrap-peers-2026-09-15.txt"
        peer_path = record_path(peer)
        wrapped = json.loads((root / peer_path).read_text(encoding="utf-8"))
        peer_original = (root / peer_path).read_bytes()
        wrapped["record"]["lifecycle"] = "HISTORICAL_PROVENANCE"
        (root / peer_path).write_bytes(_harness.canonical(wrapped))
        boundary = checker(root)
        expect_any_code(boundary, {"frozen-cohort-drift", "current-operator-input-boundary", "classification-drift"})
        (root / peer_path).write_bytes(peer_original)

        readme = root / "doc/testing/evidence/README.md"
        readme_original = readme.read_bytes()
        readme.write_bytes(readme_original + "\n当前 release evidence bundle 该从哪里开始看\n".encode("utf-8"))
        expect_any_code(checker(root), {"readme-navigation", "evidence-navigation"})
        readme.write_bytes(readme_original)
    finally:
        _harness.remove_fixture(root)


def test_legacy_authority_coverage_classification_and_missing_authority() -> None:
    root = _harness.make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        path = record_path(source)
        wrapped = json.loads((root / path).read_text(encoding="utf-8"))
        wrapped["record"]["authority"] = "doc/missing-authority.md"
        (root / path).write_bytes(_harness.canonical(wrapped))
        expect_code(checker(root), "missing-authority", 1)
    finally:
        _harness.remove_fixture(root)

    root = _harness.make_fixture()
    outside = root.parent / f"{root.name}-authority-outside"
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        path = record_path(source)
        wrapped = json.loads((root / path).read_text(encoding="utf-8"))
        outside.write_text("external authority\n", encoding="utf-8")
        link = root / "doc/testing/evidence/authority-link.md"
        link.symlink_to(outside)
        wrapped["record"]["authority"] = "doc/testing/evidence/authority-link.md"
        (root / path).write_bytes(_harness.canonical(wrapped))
        expect_any_code(checker(root), {"unsafe-path", "missing-authority"})
    finally:
        outside.unlink(missing_ok=True)
        _harness.remove_fixture(root)

    root = _harness.make_fixture()
    try:
        source = "doc/testing/evidence/fixture-2026-09-30.md"
        path = record_path(source)
        wrapped = json.loads((root / path).read_text(encoding="utf-8"))
        wrapped["record"]["lifecycle"] = "NOT_A_LIFECYCLE"
        (root / path).write_bytes(_harness.canonical(wrapped))
        expect_any_code(checker(root), {"classification-drift", "lifecycle", "schema-record"})
    finally:
        _harness.remove_fixture(root)


def main() -> None:
    tests = [
        test_t10_reviewed_content_change_does_not_refresh_the_review,
        test_t11_bundle_digest_and_member_contract_are_preserved,
        test_t12_invalid_semantic_authority_owner_disposition_and_obligation_fail,
        test_t13_stale_review_proposal_returns_three_without_writes,
        test_review_proposal_shape_preconditions_and_complete_upserts,
        test_t14_group_member_drift_and_membership_changes_fail,
        test_t15_new_out_of_cohort_evidence_does_not_rewrite_static_roots_or_groups,
        test_t16_t17_frozen_cohort_boundary_and_historical_readme_claims,
        test_legacy_authority_coverage_classification_and_missing_authority,
    ]
    try:
        for test in tests:
            test()
        print(f"doc-evidence-inventory-check.test: OK ({len(tests)} scenarios)")
    finally:
        _harness.destroy_fixture()


if __name__ == "__main__":
    main()
