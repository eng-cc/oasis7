#!/usr/bin/env python3
import json
from pathlib import Path
import subprocess
import unittest

PM = Path(__file__).parent
ROOT = PM.parent.parent
SELECTOR = PM / "review-role-selector.py"
PREPARE = ROOT / "scripts/prepare-task-pr.sh"


class PrepareTaskPrReviewRiskTests(unittest.TestCase):
    def select(self, change_class, paths, ok=True):
        result = subprocess.run([str(SELECTOR), "--change-class", change_class,
                                 "--changed-path-list", paths, "--json"],
                                text=True, capture_output=True)
        self.assertEqual(0 if ok else 2, result.returncode, result.stderr)
        return json.loads(result.stdout) if ok else result.stderr

    def test_mechanical_docs_bypass_unrelated_role_fanout(self):
        roles = self.select("mechanical-doc", "README.md;doc/engineering/index.md")["roles"]
        self.assertEqual(["repository_health_engineer", "qa_engineer"], roles)
        for role in ("producer_system_designer", "liveops_community", "blockchain_ops_engineer"):
            self.assertNotIn(role, roles)

    def test_unknown_mixed_and_non_doc_override_fail_closed(self):
        for kind in ("unknown", "mixed"):
            self.assertIn("manual", self.select(kind, "README.md", ok=False).lower())
        self.assertIn("documentation-only", self.select(
            "mechanical-doc", "README.md;scripts/pm/task-closeout.sh", ok=False).lower())

    def test_prepare_task_pr_consumes_selector(self):
        source = PREPARE.read_text(encoding="utf-8")
        self.assertIn('--changed-path-list "$LOCAL_REQUIRED_CHANGED_PATHS"', source)
        self.assertIn('REQUIRED_REVIEW_ROLES="$(python3 -c', source)

    def test_prepare_task_pr_forwards_repeatable_manual_roles_to_selector_in_order(self):
        source = PREPARE.read_text(encoding="utf-8")
        self.assertIn('--review-manual-role <role>', source)
        self.assertIn('REVIEW_MANUAL_ROLES=()', source)
        self.assertIn('--review-manual-role) REVIEW_MANUAL_ROLES+=("${2:-}"); shift 2 ;;', source)
        self.assertIn(
            '    for role in "${REVIEW_MANUAL_ROLES[@]}"; do\n'
            '      ROLE_SELECTOR_ARGS+=(--manual-role "$role")\n'
            '    done',
            source,
        )

    def test_prepare_task_pr_guards_empty_manual_roles_for_bash_32(self):
        source = PREPARE.read_text(encoding="utf-8")
        self.assertIn(
            '  if [[ "${#REVIEW_MANUAL_ROLES[@]}" -gt 0 ]]; then\n'
            '    for role in "${REVIEW_MANUAL_ROLES[@]}"; do\n'
            '      ROLE_SELECTOR_ARGS+=(--manual-role "$role")\n'
            '    done\n'
            '  fi',
            source,
            "empty arrays must not expand under Bash 3.2 set -u",
        )

    def test_patch_equivalence_helper_is_directly_invocable(self):
        helper = ROOT / "scripts/pm/patch-equivalence-receipt.sh"
        self.assertTrue(helper.stat().st_mode & 0o111, "receipt helper should support its documented direct invocation")

    def test_prepare_task_pr_requires_review_packet_comparison_oid(self):
        source = PREPARE.read_text(encoding="utf-8")
        self.assertTrue('"Comparison OID": comparison_oid' in source,
                        "prepare helper does not bind packet comparison OID")
        self.assertTrue('"Comparison OID",' in source,
                        "prepare helper does not validate packet comparison OID")

    def test_prepare_task_pr_quotes_the_terminal_finalizer_command(self):
        source = PREPARE.read_text(encoding="utf-8")
        self.assertIn('CLEANUP_CMD_1="$(render_cmd', source)
        self.assertIn('"$CANONICAL_REPO_ROOT/scripts/pm/finalize-task.sh"', source)
        self.assertIn('--repo-root "$CANONICAL_REPO_ROOT"', source)


if __name__ == "__main__":
    unittest.main()
