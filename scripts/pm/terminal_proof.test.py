#!/usr/bin/env python3
"""Producer-shape checks for the canonical terminal receipt chain."""
import hashlib
import copy
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from terminal_proof import validate_receipt_chain
import terminal_proof


UID = "task_" + "a" * 32
REPOSITORY = "fixture/repo"
ISSUE_NUMBER = 11
PR_NUMBER = 12
PR_URL = f"https://github.com/{REPOSITORY}/pull/{PR_NUMBER}"


def entry(record):
    raw = json.dumps(record, sort_keys=True, separators=(",", ":")).encode()
    return {"bytes": raw, "digest": hashlib.sha256(raw).hexdigest(), "record": record}


def receipts(mode="ancestry"):
    merge = entry({
        "receipt_type": "oasis7_pr_merge", "issuer": "github_live_query", "evidence_mode": "production",
        "repository": REPOSITORY, "default_branch": "main", "pr_number": PR_NUMBER, "pr_url": PR_URL,
        "state": "MERGED", "merged_at": "2026-09-10T00:00:00Z", "head_oid": "a" * 40,
        "base_ref": "main", "observed_at": "2026-09-10T00:00:00Z",
    })
    main = {
        "receipt_type": "oasis7_main_sync", "issuer": "post-merge-main-sync", "task_uid": UID,
        "repository": REPOSITORY, "default_branch": "main", "merge_receipt_sha256": merge["digest"],
        "main_commit": "c" * 40, "remote_main_commit": "c" * 40,
        "integration_mode": mode, "observed_at": "2026-09-10T00:01:00Z",
    }
    result = {"merge": merge}
    if mode == "patch_equivalence":
        patch = entry({
            "receipt_type": "oasis7_patch_equivalence", "schema_version": 2,
            "issuer": "oasis7_patch_equivalence_helper", "branch_tip": "a" * 40,
            "main_commit": "d" * 40, "main_parent": "e" * 40, "patch_id": "f" * 40,
            "projected_tree_oid": "1" * 40, "main_tree_oid": "1" * 40,
        })
        main.update({
            "patch_equivalence_receipt_sha256": patch["digest"], "patch_id": "f" * 40,
            "projected_tree_oid": "1" * 40, "main_tree_oid": "1" * 40,
            "integration_commit": "d" * 40, "integration_parent": "e" * 40,
        })
        result["patch_equivalence"] = patch
    result["main_sync"] = entry(main)
    terminal = entry({
        "receipt_type": "oasis7_terminal_cleanup", "issuer": "post-merge-cleanup", "task_uid": UID,
        "repository": REPOSITORY, "issue_number": ISSUE_NUMBER, "pr_number": PR_NUMBER,
        "worktree": "/fixture/worktree", "branch": "task/fixture",
        "merge_receipt_sha256": merge["digest"], "main_sync_receipt_sha256": result["main_sync"]["digest"],
        "observed_at": "2026-09-10T00:02:00Z",
    })
    operations = {
        effect: {"effect": effect,
                 "operation_id": hashlib.sha256(f"{UID}:post_merge_done:{effect}".encode()).hexdigest(),
                 "committed": True}
        for effect in ("project_update", "evidence_comment", "issue_close")
    }
    result.update({
        "terminal": terminal,
        "ledger": entry({"schema": "oasis7_finalizer_ledger_v1", "task_uid": UID, "operations": operations}),
        "tombstone": entry({
            "schema": "oasis7_terminal_tombstone_v1", "task_uid": UID, "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER, "pr_number": PR_NUMBER, "workflow_phase": "post_merge_done",
            "terminal_receipt_sha256": terminal["digest"], "canonical_worktree": "/fixture/worktree",
            "task_branch": "task/fixture", "checkout_recreation_forbidden": True,
        }),
    })
    return result


PR = {
    "base": {"ref": "main"}, "head": {"sha": "a" * 40},
}


class TypedProjectProofRed(unittest.TestCase):
    def item(self):
        nodes = [{'__typename': 'ProjectV2ItemFieldSingleSelectValue',
            'name': value, 'field': {'name': name}} for name, value in
            [('Status', 'Done'), ('PM Status', 'done'), ('Workflow Phase', 'done')]]
        nodes += [{'__typename': 'ProjectV2ItemFieldTextValue', 'text': UID,
            'field': {'name': 'Task UID'}}, {'__typename': 'ProjectV2ItemFieldRepositoryValue',
            'field': {'name': 'Repository'}, 'repository': {'nameWithOwner': REPOSITORY}}]
        return {'id': 'I', 'project': {'id': 'P', 'number': 1, 'owner': {'login': 'fixture'}},
            'content': {'number': ISSUE_NUMBER,
                'url': f'https://github.com/{REPOSITORY}/issues/{ISSUE_NUMBER}'},
            'fieldValues': {'pageInfo': {'hasNextPage': False}, 'nodes': nodes}}

    def test_actual_terminal_parser_retains_typed_repository_metadata(self):
        values = terminal_proof._project_values(self.item())
        self.assertEqual(values['fields']['Repository'], REPOSITORY)
        self.assertEqual(values['fields']['Status'], 'Done')
        self.assertEqual(values['fields']['Task UID'], UID)

    def test_all_uncertain_union_shapes_and_duplicate_names_fail_closed(self):
        valid = self.item(); repository = valid['fieldValues']['nodes'][-1]
        cases = [{}, None, {'field': {'name': 'Repository'}},
            {**repository, '__typename': 'Unknown'}, {**repository, 'repository': None},
            {**repository, 'repository': {'nameWithOwner': 'other/repo'}},
            {**repository, 'field': {'name': 'Status'}},
            {'__typename': 'ProjectV2ItemFieldTextValue', 'field': {'name': 'Other'}, 'text': True},
            {'__typename': 'ProjectV2ItemFieldSingleSelectValue', 'field': {'name': 'Other'}, 'name': None}]
        for candidate in cases:
            item = copy.deepcopy(valid); item['fieldValues']['nodes'][-1] = candidate
            with self.subTest(candidate=candidate), self.assertRaises(ValueError):
                terminal_proof._project_values(item)
        item = copy.deepcopy(valid); item['fieldValues']['nodes'].append(repository)
        with self.assertRaises(ValueError): terminal_proof._project_values(item)
        for page in [{}, {'hasNextPage': True}, {'hasNextPage': 'false'}]:
            item = copy.deepcopy(valid); item['fieldValues']['pageInfo'] = page
            with self.subTest(page=page), self.assertRaises(ValueError):
                terminal_proof._project_values(item)


class MainSyncProducerProof(unittest.TestCase):
    def check(self, chain):
        return validate_receipt_chain(
            chain, repository=REPOSITORY, task_uid=UID, issue_number=ISSUE_NUMBER,
            pr_number=PR_NUMBER, pr_url=PR_URL, pr=PR,
        )

    def test_real_ancestry_producer_shape_passes(self):
        self.check(receipts())

    def test_ancestry_requires_synchronized_main_commits(self):
        for field in ("main_commit", "remote_main_commit"):
            chain = receipts()
            chain["main_sync"]["record"].pop(field)
            with self.subTest(field=field):
                with self.assertRaisesRegex(ValueError, "synchronized producer commits"):
                    self.check(chain)
        chain = receipts()
        chain["main_sync"]["record"]["remote_main_commit"] = "d" * 40
        with self.assertRaisesRegex(ValueError, "local/remote commits disagree"):
            self.check(chain)

    def test_merge_receipt_requires_observed_producer_readback(self):
        chain = receipts()
        chain["merge"]["record"].pop("observed_at")
        with self.assertRaisesRegex(ValueError, "lacks merged version"):
            self.check(chain)

    def test_real_patch_equivalence_producer_shape_passes(self):
        self.check(receipts("patch_equivalence"))

    def test_patch_equivalence_requires_actual_bound_receipt(self):
        for field in ("patch_equivalence_receipt_sha256", "patch_id", "projected_tree_oid",
                      "main_tree_oid", "integration_commit", "integration_parent"):
            chain = receipts("patch_equivalence")
            chain["main_sync"]["record"].pop(field)
            with self.subTest(field=field):
                with self.assertRaises(ValueError):
                    self.check(chain)
        chain = receipts("patch_equivalence")
        chain["main_sync"]["record"]["patch_equivalence_receipt_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "digest disagrees"):
            self.check(chain)

        chain = receipts("patch_equivalence")
        chain["patch_equivalence"]["record"]["main_commit"] = "9" * 40
        with self.assertRaisesRegex(ValueError, "provenance mismatch"):
            self.check(chain)


if __name__ == "__main__":
    unittest.main()
