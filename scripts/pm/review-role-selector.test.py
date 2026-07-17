#!/usr/bin/env python3
import json
from pathlib import Path
import subprocess
import unittest

SCRIPT = Path(__file__).with_name("review-role-selector.py")

class ReviewRoleSelectorTests(unittest.TestCase):
    def select(self, *args, ok=True):
        self.assertTrue(SCRIPT.is_file(), "missing deterministic review-role-selector.py production helper")
        result = subprocess.run([str(SCRIPT), *args, "--json"], text=True, capture_output=True)
        self.assertEqual(0 if ok else 2, result.returncode, result.stderr)
        return json.loads(result.stdout) if ok else result.stderr

    def test_mechanical_and_workflow_docs_use_health_and_qa(self):
        for kind in ("mechanical-doc", "workflow-doc"):
            with self.subTest(kind=kind):
                self.assertEqual(
                    ["repository_health_engineer", "qa_engineer"],
                    self.select("--change-class", kind)["roles"],
                )

    def test_domain_semantic_docs_add_domain_and_conditionally_qa(self):
        base = ["repository_health_engineer", "runtime_engineer"]
        self.assertEqual(base, self.select("--change-class", "domain-semantic-doc", "--domain-role", "runtime_engineer")["roles"])
        self.assertEqual(base + ["qa_engineer"], self.select(
            "--change-class", "domain-semantic-doc", "--domain-role", "runtime_engineer", "--verification-affected"
        )["roles"])

    def test_external_messaging_adds_liveops_and_only_adds_qa_for_verification(self):
        base = ["repository_health_engineer", "liveops_community"]
        self.assertEqual(base, self.select("--change-class", "external-messaging")["roles"])
        self.assertEqual(base + ["qa_engineer"], self.select(
            "--change-class", "external-messaging", "--verification-affected"
        )["roles"])

    def test_unknown_or_mixed_fails_closed_for_manual_selection(self):
        for kind in ("unknown", "mixed"):
            self.assertIn("manual", self.select("--change-class", kind, ok=False).lower())

if __name__ == "__main__": unittest.main()
