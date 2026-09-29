#!/usr/bin/env python3
"""Synthetic regressions for scoped GitHub process-exception selection."""
from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("workflow-process-exception.py")
SPEC = importlib.util.spec_from_file_location("workflow_process_exception", SCRIPT)
assert SPEC and SPEC.loader
EXCEPTIONS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXCEPTIONS)

REPO = "eng-cc/oasis7"
ISSUE = 701
TASK_UID = "task_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
HEAD = "a" * 40
ACTION = "current_pr_validation"
SCOPE = "required-gate"
ADMIN = "synthetic-admin"
NONADMIN = "synthetic-contributor"


def comment(comment_id: int, record: dict, *, login: str = ADMIN,
            created_at: str | None = None, issue_url: str | None = None) -> dict:
    body = EXCEPTIONS.MARKER + "\n" + json.dumps(
        record, ensure_ascii=True, sort_keys=True, separators=(",", ":"),
    )
    return {
        "id": comment_id,
        "issue_url": issue_url or f"https://api.github.com/repos/{REPO}/issues/{ISSUE}",
        "created_at": created_at or f"2026-09-29T00:00:{comment_id % 60:02d}Z",
        "updated_at": created_at or f"2026-09-29T00:00:{comment_id % 60:02d}Z",
        "user": {"login": login},
        "body": body,
    }


def record(*, decision: str = "waive_process_check", task_uid: str = TASK_UID,
           action: str = ACTION, head_oid: str = HEAD, scope: str = SCOPE,
           replacement: dict | None = None, target: int | None = None,
           reason: str = "synthetic test recovery evidence") -> dict:
    value = {
        "schema": EXCEPTIONS.SCHEMA,
        "task_uid": task_uid,
        "action": action,
        "head_oid": head_oid,
        "scope": scope,
        "decision": decision,
        "reason": reason,
        "authorized_decision": "synthetic authorized test decision",
        "replacement_evidence": (replacement or {"check_run_id": 31, "run_id": 41, "run_attempt": 2})
        if decision == "waive_process_check" else None,
    }
    if target is not None:
        value["target_comment_id"] = target
    return value


def resolve(comments: list[dict], *, admin= None, replacement_valid: bool = True,
            process_check_waivable: bool = True) -> dict:
    admin = admin or {ADMIN: True, NONADMIN: False}
    return EXCEPTIONS.resolve_process_exception(
        comments,
        repository=REPO,
        issue_number=ISSUE,
        task_uid=TASK_UID,
        action=ACTION,
        head_oid=HEAD,
        scope=SCOPE,
        live_admin_by_login=lambda login: admin.get(login, False),
        replacement_validator=lambda _replacement, _record: replacement_valid,
        process_check_waivable=process_check_waivable,
    )


class ProcessExceptionHistoryTest(unittest.TestCase):
    def test_no_exception_preserves_ordinary_retry(self) -> None:
        result = resolve([])
        self.assertEqual(result["status"], "none")
        self.assertFalse(result["applicable"])

    def test_malformed_old_record_does_not_poison_corrected_valid_record(self) -> None:
        malformed = comment(1, record())
        malformed["body"] = EXCEPTIONS.MARKER + "\n{"  # incomplete historical payload
        valid = comment(2, record())
        result = resolve([malformed, valid])
        self.assertEqual(result["status"], "applied")
        self.assertEqual(result["comment_id"], 2)
        self.assertEqual(result["ignored_records"], 1)

    def test_unrelated_malformed_scope_does_not_poison_current_scope(self) -> None:
        unrelated = comment(1, record(scope="other-action"))
        unrelated["body"] = EXCEPTIONS.MARKER + "\nnot-json"
        valid = comment(2, record())
        result = resolve([unrelated, valid])
        self.assertEqual(result["status"], "applied")
        self.assertEqual(result["comment_id"], 2)

    def test_pretty_json_is_accepted_but_duplicate_keys_are_ignored_as_malformed(self) -> None:
        pretty = comment(1, record())
        payload = record()
        pretty["body"] = EXCEPTIONS.MARKER + "\n" + json.dumps(payload, indent=2, sort_keys=True)
        result = resolve([pretty])
        self.assertEqual(result["status"], "applied")
        duplicate = comment(2, record())
        duplicate["body"] = EXCEPTIONS.MARKER + '\n{"schema":"x","schema":"y"}'
        valid = comment(3, record())
        result = resolve([duplicate, valid])
        self.assertEqual(result["status"], "applied")
        self.assertEqual(result["comment_id"], 3)
        self.assertEqual(result["ignored_records"], 1)

    def test_edited_exception_comment_is_not_current_authority(self) -> None:
        edited = comment(1, record())
        edited["updated_at"] = "2026-09-29T00:01:00Z"
        self.assertEqual(resolve([edited])["status"], "none")

    def test_newer_nonadmin_forgery_cannot_deny_valid_admin_record(self) -> None:
        admin_record = comment(1, record(), created_at="2026-09-29T00:00:01Z")
        forged = comment(2, record(reason="fake later waiver"), login=NONADMIN,
                         created_at="2026-09-29T00:00:02Z")
        result = resolve([admin_record, forged])
        self.assertEqual(result["status"], "applied")
        self.assertEqual(result["comment_id"], 1)

    def test_newer_matching_admin_permission_uncertainty_fails_closed(self) -> None:
        older = comment(1, record(), created_at="2026-09-29T00:00:01Z")
        newer = comment(2, record(), created_at="2026-09-29T00:00:02Z")
        result = resolve([older, newer], admin={ADMIN: True, NONADMIN: None})
        self.assertEqual(result["status"], "applied")
        uncertain = comment(3, record(), login="permission-read-failure",
                            created_at="2026-09-29T00:00:03Z")
        result = resolve([older, uncertain], admin={ADMIN: True, "permission-read-failure": None})
        self.assertEqual(result["status"], "rejected")

    def test_mismatched_head_or_task_does_not_apply(self) -> None:
        wrong = comment(1, record(head_oid="b" * 40))
        self.assertEqual(resolve([wrong])["status"], "none")
        wrong_task = comment(2, record(task_uid="task_" + "b" * 32))
        self.assertEqual(resolve([wrong_task])["status"], "none")

    def test_wrong_issue_comment_is_ignored(self) -> None:
        row = comment(1, record(), issue_url=f"https://api.github.com/repos/{REPO}/issues/{ISSUE + 1}")
        self.assertEqual(resolve([row])["status"], "none")

    def test_process_check_waiver_needs_allowed_scope_and_valid_replacement(self) -> None:
        valid = comment(1, record(replacement={"check_run_id": 31, "run_id": 41, "run_attempt": 2}))
        self.assertEqual(resolve([valid], replacement_valid=False)["status"], "rejected")
        self.assertEqual(resolve([valid], process_check_waivable=False)["status"], "rejected")
        self.assertEqual(resolve([valid])["status"], "applied")

    def test_revoke_only_affects_earlier_exact_scope_record(self) -> None:
        original = comment(1, record())
        revoked = comment(2, record(decision="revoke", target=1))
        result = resolve([original, revoked])
        self.assertEqual(result["status"], "revoked")
        self.assertFalse(result["applicable"])

    def test_supersession_cannot_target_later_or_missing_record(self) -> None:
        original = comment(1, record())
        invalid = comment(2, record(decision="supersede", target=3))
        result = resolve([original, invalid])
        self.assertEqual(result["status"], "rejected")


if __name__ == "__main__":
    unittest.main()
