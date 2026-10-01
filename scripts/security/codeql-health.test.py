#!/usr/bin/env python3
import datetime as dt
import importlib.util
from pathlib import Path
import unittest
import subprocess
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("health", Path(__file__).with_name("codeql-health.py"))
health = importlib.util.module_from_spec(spec)
spec.loader.exec_module(health)
SHA = "a" * 40
NOW = dt.datetime(2026, 10, 1, tzinfo=dt.timezone.utc)


def fixture():
    return {"identity": {"sha": SHA, "ref": "refs/heads/main"}, "analyses": [], "jobs": [], "alerts": []}


def add_analysis(data, unit="rust-repo", profile="default", **extra):
    row = {"category": f"oasis7/{unit}/{profile}", "commit_sha": SHA, "ref": "refs/heads/main",
           "tool": {"name": "CodeQL", "version": "fixture"}, "created_at": "2026-09-30T23:00:00Z", "id": 1}
    row.update(extra)
    data["analyses"].append(row)
    return row


class HealthTests(unittest.TestCase):
    def rust(self, data):
        return next(u for u in health.report(data, NOW)["units"] if u["unit"] == "rust-repo" and u["profile"] == "default")

    def test_unknown_api_errors_are_not_no_findings(self):
        data = fixture()
        data.update(errors=["403"], analyses=None, alerts=None)
        result = health.report(data, NOW)
        self.assertEqual(result["status"], "unknown")
        self.assertTrue(all(u["finding_status"] == "unknown" for u in result["units"]))

    def test_upload_execution_findings_independent(self):
        data = fixture()
        add_analysis(data)
        row = self.rust(data)
        self.assertEqual((row["execution_status"], row["upload_status"], row["finding_status"]), ("unknown", "accepted", "unknown"))
        data["jobs"] = [{"name": "CodeQL / rust-repo / default", "head_sha": SHA, "ref": "refs/heads/main", "status": "completed", "conclusion": "failure",
                         "steps": [{"name": "CodeQL extraction and queries", "conclusion": "failure"}]}]
        self.assertEqual(self.rust(data)["execution_status"], "failure")
        self.assertEqual(self.rust(data)["upload_status"], "accepted")

    def test_complete_fixture_zero_is_explicit(self):
        data = fixture()
        add_analysis(data)
        data["alert_instances_complete"] = True
        self.assertEqual(self.rust(data)["finding_status"], "no_open_findings")
        data["alerts"] = [{"instances": [{"category": "oasis7/rust-repo/default", "commit_sha": SHA, "ref": "refs/heads/main"}]}]
        self.assertEqual(self.rust(data)["finding_status"], "open")

    def test_exact_identity_and_profile(self):
        for change in ({"commit_sha": "b" * 40}, {"ref": "refs/heads/other"}, {"tool": {"name": "Fake"}}, {"category": "oasis7/rust-repo/extended"}):
            data = fixture()
            add_analysis(data, **change)
            self.assertEqual(self.rust(data)["upload_status"], "unknown")

    def test_age_future_and_coverage_gap(self):
        for date in ("2026-09-28T00:00:00Z", "2026-10-02T00:00:00Z", "bad"):
            data = fixture()
            add_analysis(data, created_at=date)
            self.assertFalse(self.rust(data)["fresh"])
        data = fixture()
        add_analysis(data)
        self.assertEqual(self.rust(data)["coverage_status"], "unknown")
        self.assertIsNone(self.rust(data)["missing_manifests"])

    def test_cancelled_queued_timeout_not_complete(self):
        for state, conclusion, expected in (("queued", None, "queued"), ("completed", "cancelled", "cancelled"), ("completed", "timed_out", "timed_out")):
            data = fixture()
            data["jobs"] = [{"name": "CodeQL / rust-repo / default", "head_sha": SHA, "ref": "refs/heads/main", "status": state, "conclusion": conclusion}]
            row = self.rust(data)
            self.assertEqual(row["execution_status"], expected)
            self.assertEqual(row["upload_status"], "unknown")

    def test_invalid_identity_no_health(self):
        data = fixture()
        data["identity"]["sha"] = "short"
        self.assertEqual(health.report(data, NOW)["status"], "unknown")

    def test_upload_failure_does_not_relabel_execution(self):
        data = fixture()
        data["jobs"] = [{"name": "CodeQL / rust-repo / default", "head_sha": SHA, "ref": "refs/heads/main", "status": "completed", "conclusion": "failure",
                         "steps": [{"name": "CodeQL extraction and queries", "conclusion": "success"}, {"name": "CodeQL SARIF upload", "conclusion": "failure"}]}]
        self.assertEqual(self.rust(data)["execution_status"], "success")
        self.assertEqual(self.rust(data)["upload_status"], "failed")

    def test_read_pages_fail_closed_on_schema_permission_and_budget(self):
        for output, code in (("{}", 0), ("permission denied", 1), ("broken", 0)):
            with patch("subprocess.run", return_value=subprocess.CompletedProcess([], code, output, "")):
                with self.assertRaises((RuntimeError, health.json.JSONDecodeError)):
                    health.read_pages("fixture/repo", "code-scanning/analyses", limit=1)
        with patch("subprocess.run", return_value=subprocess.CompletedProcess([], 0, health.json.dumps([{}] * 100), "")):
            with self.assertRaisesRegex(RuntimeError, "page budget exceeded"):
                health.read_pages("fixture/repo", "code-scanning/analyses", limit=1)

    def test_live_api_unavailable_is_unknown_without_writes(self):
        with patch("subprocess.run", return_value=subprocess.CompletedProcess([], 1, "", "403")) as command:
            data = health.live("fixture/repo", "refs/heads/main", SHA)
        self.assertEqual(health.report(data, NOW)["status"], "unknown")
        self.assertTrue(all(call.args[0][:2] == ["gh", "api"] and "--method" not in call.args[0] for call in command.call_args_list))


if __name__ == "__main__":
    unittest.main()
