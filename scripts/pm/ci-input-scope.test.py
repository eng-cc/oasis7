#!/usr/bin/env python3
"""Focused tests for deterministic CI input closure and corpus completeness."""
import subprocess
import tempfile
import unittest
from pathlib import Path

import ci_input_scope as inputs


COMMAND_PATH = "scripts/check-product.py"


class InputScopeContractTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.git("init", "-q")
        self.git("config", "user.email", "ci-input-scope@example.invalid")
        self.git("config", "user.name", "CI input scope tests")
        self.write("pkg/a/a.rs", "fn a() {}\n")
        self.write("pkg/b/b.rs", "fn b() {}\n")
        self.write(COMMAND_PATH, "def check(): return True\n")
        scripts_root = Path(__file__).resolve().parents[1]
        self.write(
            "scripts/product_doc_markdown.py",
            (scripts_root / "product_doc_markdown.py").read_text(encoding="utf-8"),
        )
        self.write(
            "scripts/doc-governance-requirements.txt",
            (scripts_root / "doc-governance-requirements.txt").read_text(encoding="utf-8"),
        )
        self.write(
            "doc/product/core/alpha.prd.md",
            "# Alpha\n\nSee [beta](beta.design.md#scope).\n",
        )
        self.write("doc/product/core/beta.design.md", "# Scope\n\nBeta design.\n")
        self.base = self.commit("base")

    def tearDown(self):
        self.temporary.cleanup()

    def git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.root), *args], check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        ).stdout.strip()

    def write(self, path, text):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text, encoding="utf-8")

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")

    def trusted_inventory_issuer(self, commit, unit_specs, product_corpus):
        normalized_commit, tree_oid, _entries = inputs.git_tree_entries(str(self.root), commit)
        return {
            "schema": inputs.TRUSTED_PLANNER_INVENTORY_SCHEMA,
            "authority": {
                "schema": inputs.PLANNER_INVENTORY_AUTHORITY_SCHEMA,
                "repository": "eng-cc/oasis7",
                "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
                "planner_authority_oid": self.base,
                "planner_config_sha256": "sha256:" + "a" * 64,
            },
            "producer": {
                "run_id": 10, "run_attempt": 1,
                "check_app_id": 30, "check_run_id": 40,
            },
            "target_oid": normalized_commit,
            "target_tree_oid": tree_oid,
            "unit_ids": sorted(spec["unit_id"] for spec in unit_specs),
            "inventory_digest": inputs.planner_inventory_digest(
                unit_specs, product_corpus, normalized_commit, tree_oid,
            ),
        }

    @staticmethod
    def trusted_inventory_readback(issuer, *, artifact_id=20):
        return {
            **issuer,
            "producer": {**issuer["producer"], "artifact_id": artifact_id},
        }

    def select(self, prior, target, *, trusted_prior_inventory=None,
               trusted_target_inventory=None):
        return inputs.select_affected_units(
            prior, target,
            trusted_prior_inventory=(trusted_prior_inventory or self.trusted_inventory_readback(
                prior["planner_inventory_issuer"],
            )),
            trusted_target_inventory=(trusted_target_inventory or self.trusted_inventory_readback(
                target["planner_inventory_issuer"],
            )),
        )

    @staticmethod
    def unit(unit_id, path, member_root, *, edges=None):
        return {
            "unit_id": unit_id,
            "unit_contract": {"kind": "test-unit/v1", "id": unit_id},
            "obligation_set": ["obligation:" + unit_id],
            "command_checker_paths": [COMMAND_PATH],
            "input_paths": [path],
            "member_roots": [member_root],
            "dependency_edges": edges or [],
            "applicable_policy": {"policy": "fixture-v1"},
            "environment_contract": {"toolchain": "fixture-v1"},
        }

    def scope(self, commit, *, unit_specs=None, closure_status="complete",
              closure_reason=None, fallback_unit_ids=None, fallback_scope_complete=False):
        unit_specs = unit_specs or [
            self.unit("unit-a", "pkg/a/a.rs", "pkg/a"),
            self.unit("unit-b", "pkg/b/b.rs", "pkg/b"),
        ]
        corpus_specs, corpus = inputs.build_product_corpus_unit_specs(
            str(self.root), commit,
            command_checker_paths=[COMMAND_PATH],
            applicable_policy={"policy": "product-v1"},
            environment_contract={"python": "fixture-v1"},
        )
        if closure_status == "unknown" and fallback_unit_ids is None:
            fallback_unit_ids = [spec["unit_id"] for spec in [*unit_specs, *corpus_specs]]
            fallback_scope_complete = True
        all_specs = [*unit_specs, *corpus_specs]
        return inputs.build_input_scope_snapshot(
            str(self.root), commit, all_specs, corpus,
            planner_inventory_issuer=self.trusted_inventory_issuer(commit, all_specs, corpus),
            closure_status=closure_status,
            closure_reason=closure_reason,
            fallback_unit_ids=fallback_unit_ids,
            fallback_scope_complete=fallback_scope_complete,
        )

    def test_unit_fingerprints_are_stable_and_include_checker_bytes(self):
        first = self.scope(self.base)
        second = self.scope(self.base)
        self.assertEqual(first, second)

        self.write(COMMAND_PATH, "def check(): return False\n")
        changed_commit = self.commit("change checker")
        changed = self.scope(changed_commit)
        self.assertNotEqual(
            first["input_fingerprints"]["unit-a"],
            changed["input_fingerprints"]["unit-a"],
        )

    def test_changed_input_selects_only_the_affected_unit(self):
        self.write("pkg/a/a.rs", "fn a() { let changed = true; }\n")
        changed_commit = self.commit("change a")
        result = self.select(self.scope(self.base), self.scope(changed_commit))

        self.assertEqual("complete", result["status"])
        self.assertEqual(["unit-a"], result["required_test_units"])
        self.assertIn("unit-b", result["reused_units"])

    def test_added_and_deleted_paths_change_membership_and_selected_units(self):
        unit_specs = [
            self.unit("unit-a", "pkg/a/optional.rs", "pkg/a"),
            self.unit("unit-b", "pkg/b/b.rs", "pkg/b"),
        ]
        before = self.scope(self.base, unit_specs=unit_specs)
        self.write("pkg/a/optional.rs", "pub const OPTIONAL: bool = true;\n")
        added_commit = self.commit("add scoped input")
        added = self.scope(added_commit, unit_specs=unit_specs)
        self.assertEqual(
            ["unit-a"],
            self.select(before, added)["required_test_units"],
        )

        (self.root / "pkg/a/optional.rs").unlink()
        deleted_commit = self.commit("delete scoped input")
        deleted = self.scope(deleted_commit, unit_specs=unit_specs)
        self.assertEqual(
            ["unit-a"],
            self.select(added, deleted)["required_test_units"],
        )

    def test_dependency_change_widens_to_consumers_over_edge_union(self):
        unit_specs = [
            self.unit("unit-a", "pkg/a/a.rs", "pkg/a"),
            self.unit("unit-b", "pkg/b/b.rs", "pkg/b", edges=[["unit-b", "unit-a"]]),
        ]
        before = self.scope(self.base, unit_specs=unit_specs)
        self.write("pkg/a/a.rs", "fn a() { let changed = true; }\n")
        changed_commit = self.commit("change dependency")
        after = self.scope(changed_commit, unit_specs=unit_specs)

        result = self.select(before, after)
        self.assertEqual(["unit-a", "unit-b"], result["required_test_units"])

    def test_mode_change_invalidates_the_selected_unit(self):
        before = self.scope(self.base)
        (self.root / "pkg/a/a.rs").chmod(0o755)
        changed_commit = self.commit("change input mode")
        after = self.scope(changed_commit)

        self.assertEqual(
            ["unit-a"],
            self.select(before, after)["required_test_units"],
        )

    def test_symlink_target_is_hashed_and_must_be_inside_declared_closure(self):
        link = self.root / "pkg/a/link.rs"
        link.symlink_to("a.rs")
        link_commit = self.commit("add selected symlink")
        scoped_link = self.unit("unit-a", "pkg/a/link.rs", "pkg/a")
        before = self.scope(link_commit, unit_specs=[
            scoped_link, self.unit("unit-b", "pkg/b/b.rs", "pkg/b"),
        ])

        self.write("pkg/a/a.rs", "fn a() { let changed = true; }\n")
        changed_commit = self.commit("change symlink target content")
        after = self.scope(changed_commit, unit_specs=[
            scoped_link, self.unit("unit-b", "pkg/b/b.rs", "pkg/b"),
        ])
        self.assertEqual(
            ["unit-a"],
            self.select(before, after)["required_test_units"],
        )

        narrow = dict(scoped_link)
        narrow["member_roots"] = []
        with self.assertRaises(inputs.InputScopeError):
            self.scope(changed_commit, unit_specs=[
                narrow, self.unit("unit-b", "pkg/b/b.rs", "pkg/b"),
            ])

    def test_unknown_closure_widens_to_complete_fallback_without_reuse(self):
        prior = self.scope(self.base)
        target = self.scope(
            self.base,
            closure_status="unknown",
            closure_reason="dependency resolver unavailable",
        )
        result = self.select(prior, target)

        self.assertEqual("widened", result["status"])
        self.assertEqual(target["required_test_units"], result["required_test_units"])
        self.assertEqual([], result["reused_units"])
        self.assertEqual(
            "oasis7-ci-fallback-unit-manifest/v1",
            target["fallback_contract"]["schema"],
        )
        self.assertEqual(
            target["required_test_units"],
            sorted(set(target["fallback_contract"]["unit_ids"]) | set(target["product_corpus"]["unit_ids"])),
        )
        incomplete = dict(target)
        incomplete["required_test_units"] = [
            unit for unit in target["required_test_units"] if unit != "unit-b"
        ]
        with self.assertRaises(inputs.InputScopeError):
            inputs.validate_input_scope_snapshot(
                incomplete, trusted_planner_inventory=self.trusted_inventory_readback(
                    target["planner_inventory_issuer"],
                ),
            )

    def test_unknown_closure_without_complete_fallback_is_rejected(self):
        with self.assertRaises(inputs.InputScopeError):
            self.scope(
                self.base,
                closure_status="unknown",
                closure_reason="dependency resolver unavailable",
                fallback_unit_ids=["unit-a"],
                fallback_scope_complete=False,
            )

    def test_unknown_closure_rejects_incomplete_fallback_set_even_when_claimed_complete(self):
        with self.assertRaises(inputs.InputScopeError):
            self.scope(
                self.base,
                closure_status="unknown",
                closure_reason="dependency resolver unavailable",
                fallback_unit_ids=["unit-a"],
                fallback_scope_complete=True,
            )

    def test_product_corpus_enumerates_documents_membership_and_cross_document_links(self):
        snapshot = self.scope(self.base)
        corpus = snapshot["product_corpus"]
        obligations = {row["obligation_id"] for row in corpus["obligations"]}

        self.assertIn("product-corpus:membership", obligations)
        self.assertIn("product-document:doc/product/core/alpha.prd.md", obligations)
        self.assertIn(
            "product-link:doc/product/core/alpha.prd.md->doc/product/core/beta.design.md#scope",
            obligations,
        )
        self.assertEqual(set(corpus["unit_ids"]), set(snapshot["required_test_units"]) & set(corpus["unit_ids"]))

    def test_product_corpus_tracks_non_product_repository_link_target_contents(self):
        self.write("doc/engineering/authority.md", "# Rule\n\nCurrent authority.\n")
        self.write(
            "doc/product/core/alpha.prd.md",
            "# Alpha\n\nSee [authority](../../engineering/authority.md#rule).\n",
        )
        linked_commit = self.commit("link to repository authority")
        before = self.scope(linked_commit)
        link_unit = (
            "product-link::doc/product/core/alpha.prd.md"
            "->doc/engineering/authority.md#rule"
        )
        self.assertIn(link_unit, before["input_fingerprints"])

        self.write("doc/engineering/authority.md", "# Rule\n\nChanged authority.\n")
        changed_commit = self.commit("change linked authority")
        after = self.scope(changed_commit)
        result = self.select(before, after)

        self.assertIn(link_unit, result["required_test_units"])

    def test_product_corpus_fingerprints_canonical_markdown_parser(self):
        before = self.scope(self.base)
        parser_path = self.root / "scripts/product_doc_markdown.py"
        parser_path.write_text(
            parser_path.read_text(encoding="utf-8") + "\n# changed parser contract\n",
            encoding="utf-8",
        )
        changed_commit = self.commit("change product Markdown parser")
        after = self.scope(changed_commit)

        document_unit = "product-document::doc/product/core/alpha.prd.md"
        self.assertNotEqual(
            before["input_fingerprints"][document_unit],
            after["input_fingerprints"][document_unit],
        )

    def test_product_corpus_tracks_commonmark_reference_link_target_contents(self):
        self.write("doc/engineering/authority.md", "# Scope\n\nCurrent authority.\n")
        self.write(
            "doc/product/core/alpha.prd.md",
            "# Alpha\n\nSee [authority][rule].\n\n"
            "[rule]: ../../engineering/authority.md#scope\n",
        )
        linked_commit = self.commit("add CommonMark reference link")
        before = self.scope(linked_commit)
        link_unit = (
            "product-link::doc/product/core/alpha.prd.md"
            "->doc/engineering/authority.md#scope"
        )
        self.assertIn(link_unit, before["input_fingerprints"])

        self.write("doc/engineering/authority.md", "# Changed\n\nChanged authority.\n")
        changed_commit = self.commit("change reference-linked authority")
        after = self.scope(changed_commit)

        self.assertIn(
            link_unit,
            self.select(before, after)["required_test_units"],
        )

    def test_product_corpus_blocks_missing_non_product_repository_link_targets(self):
        self.write(
            "doc/product/core/alpha.prd.md",
            "# Alpha\n\nSee [authority](../../engineering/missing.md#rule).\n",
        )
        changed_commit = self.commit("add missing repository link")
        _specs, corpus = inputs.build_product_corpus_unit_specs(
            str(self.root), changed_commit,
            command_checker_paths=[COMMAND_PATH],
            applicable_policy={"policy": "product-v1"},
            environment_contract={"python": "fixture-v1"},
        )

        self.assertEqual("blocked", corpus["status"])
        self.assertTrue(any("product_link_target_missing" in error for error in corpus["errors"]))

    def test_product_corpus_addition_and_deletion_update_membership_obligations(self):
        before = self.scope(self.base)
        self.write("doc/product/core/gamma.prd.md", "# Gamma\n")
        added_commit = self.commit("add product document")
        added = self.scope(added_commit)
        added_result = self.select(before, added)
        self.assertIn("product-corpus:membership", added_result["required_test_units"])
        self.assertIn("product-document::doc/product/core/gamma.prd.md", added_result["required_test_units"])

        (self.root / "doc/product/core/gamma.prd.md").unlink()
        deleted_commit = self.commit("delete product document")
        deleted = self.scope(deleted_commit)
        deleted_result = self.select(added, deleted)
        self.assertIn("product-corpus:membership", deleted_result["required_test_units"])
        self.assertIn("product-document::doc/product/core/gamma.prd.md", deleted_result["retired_units"])

    def test_product_corpus_aggregation_requires_exact_fingerprints_and_obligations(self):
        scope = self.scope(self.base)
        corpus = scope["product_corpus"]
        by_unit = {unit: [] for unit in corpus["unit_ids"]}
        for obligation in corpus["obligations"]:
            by_unit[obligation["unit_id"]].append(obligation["obligation_id"])
        passed = [
            {
                "unit_id": unit,
                "obligation_ids": sorted(by_unit[unit]),
                "input_digest": scope["input_fingerprints"][unit],
                "status": "passed",
            }
            for unit in sorted(by_unit)
        ]
        self.assertEqual("complete", inputs.aggregate_product_corpus_results(
            scope, passed, trusted_planner_inventory=self.trusted_inventory_readback(
                scope["planner_inventory_issuer"],
            ),
        )["status"])

        missing = inputs.aggregate_product_corpus_results(
            scope, passed[:-1], trusted_planner_inventory=self.trusted_inventory_readback(
                scope["planner_inventory_issuer"],
            ),
        )
        self.assertEqual("revalidate", missing["status"])
        self.assertTrue(missing["required_units"])

        legacy = [dict(item) for item in passed]
        legacy[0].pop("input_digest")
        self.assertEqual("blocked", inputs.aggregate_product_corpus_results(
            scope, legacy, trusted_planner_inventory=self.trusted_inventory_readback(
                scope["planner_inventory_issuer"],
            ),
        )["status"])

        incomplete = [dict(item) for item in passed]
        incomplete[0]["obligation_ids"] = []
        self.assertEqual("blocked", inputs.aggregate_product_corpus_results(
            scope, incomplete, trusted_planner_inventory=self.trusted_inventory_readback(
                scope["planner_inventory_issuer"],
            ),
        )["status"])

    def test_missing_product_link_target_prevents_complete_corpus_snapshot(self):
        self.write(
            "doc/product/core/alpha.prd.md",
            "# Alpha\n\nSee [missing](missing.prd.md#scope).\n",
        )
        changed_commit = self.commit("break product link")
        product_specs, corpus = inputs.build_product_corpus_unit_specs(
            str(self.root), changed_commit,
            command_checker_paths=[COMMAND_PATH],
            applicable_policy={"policy": "product-v1"},
            environment_contract={"python": "fixture-v1"},
        )
        self.assertEqual("blocked", corpus["status"])
        self.assertTrue(corpus["errors"])
        with self.assertRaises(inputs.InputScopeError):
            inputs.build_input_scope_snapshot(
                str(self.root), changed_commit, product_specs, corpus,
                planner_inventory_issuer=self.trusted_inventory_issuer(
                    changed_commit, product_specs, corpus,
                ),
            )

    def test_snapshot_cannot_omit_corpus_units_or_ignore_unknown_fields(self):
        scope = self.scope(self.base)
        missing_corpus_unit = dict(scope)
        missing_corpus_unit["required_test_units"] = ["unit-a", "unit-b"]
        missing_corpus_unit["input_fingerprints"] = {
            "unit-a": scope["input_fingerprints"]["unit-a"],
            "unit-b": scope["input_fingerprints"]["unit-b"],
        }
        with self.assertRaises(inputs.InputScopeError):
            inputs.validate_input_scope_snapshot(
                missing_corpus_unit, trusted_planner_inventory=self.trusted_inventory_readback(
                    scope["planner_inventory_issuer"],
                ),
            )

        unknown_field = {**scope, "ignored": True}
        with self.assertRaises(inputs.InputScopeError):
            inputs.validate_input_scope_snapshot(
                unknown_field, trusted_planner_inventory=self.trusted_inventory_readback(
                    scope["planner_inventory_issuer"],
                ),
            )

    def test_snapshot_self_assertion_or_untrusted_subset_cannot_establish_inventory(self):
        full = self.scope(self.base)
        with self.assertRaisesRegex(inputs.InputScopeError, "trusted planner inventory"):
            inputs.validate_input_scope_snapshot(full)

        subset = dict(full)
        subset_units = [unit for unit in full["required_test_units"] if unit != "unit-b"]
        subset["required_test_units"] = subset_units
        subset["input_fingerprints"] = {
            unit: full["input_fingerprints"][unit] for unit in subset_units
        }
        with self.assertRaisesRegex(inputs.InputScopeError, "units disagree"):
            inputs.validate_input_scope_snapshot(
                subset, trusted_planner_inventory=self.trusted_inventory_readback(
                    full["planner_inventory_issuer"],
                ),
            )

        forged_issuer = dict(full["planner_inventory_issuer"])
        forged_issuer["unit_ids"] = subset_units
        forged_issuer["inventory_digest"] = "sha256:" + "0" * 64
        subset["planner_inventory_issuer"] = forged_issuer
        with self.assertRaisesRegex(inputs.InputScopeError, "not bound"):
            inputs.validate_input_scope_snapshot(
                subset, trusted_planner_inventory=self.trusted_inventory_readback(
                    full["planner_inventory_issuer"],
                ),
            )

    def test_inventory_binding_rejects_mismatched_issuer_or_attempt_locator(self):
        scope = self.scope(self.base)
        bad_locator = self.trusted_inventory_readback(scope["planner_inventory_issuer"])
        bad_locator["producer"] = {**bad_locator["producer"], "run_attempt": 2}
        with self.assertRaisesRegex(inputs.InputScopeError, "not bound"):
            inputs.validate_input_scope_snapshot(
                scope, trusted_planner_inventory=bad_locator,
            )

        malformed_authority = self.trusted_inventory_readback(
            scope["planner_inventory_issuer"],
        )
        malformed_authority["authority"] = {
            **malformed_authority["authority"],
            "workflow_ref": "refs/heads/main",
        }
        with self.assertRaisesRegex(inputs.InputScopeError, "default-branch workflow"):
            inputs.validate_input_scope_snapshot(
                scope, trusted_planner_inventory=malformed_authority,
            )

    def local_target_observation(self, commit, unit_specs, product_corpus, *,
                                 assessed_target_oid=None, source_head_oid=None,
                                 source_scope_oid=None):
        commit, tree_oid, _entries = inputs.git_tree_entries(str(self.root), commit)
        planner_authority_oid = self.base
        config_digest = "sha256:" + "9" * 64
        invocation = {
            "schema": inputs.LOCAL_PLANNER_INVOCATION_SCHEMA,
            "planner_authority_oid": planner_authority_oid,
            "planner_config_sha256": config_digest,
            "event_name": "workflow_dispatch",
            "run_mode": "integration_revalidation",
            "base_ref": self.base,
            "head_ref": source_head_oid or self.base,
            "task_uid": "task_12345678901234567890123456789012",
            "scope_base_oid": source_scope_oid or self.base,
            "impact_projection_sha256": "sha256:" + "8" * 64,
            "changed_paths": ["pkg/a/a.rs"],
            "planner_output_sha256": "sha256:" + "7" * 64,
        }
        invocation["digest"] = inputs.local_planner_invocation_digest(invocation)
        return inputs.build_target_observation(
            authority={
                "schema": inputs.PLANNER_INVENTORY_AUTHORITY_SCHEMA,
                "repository": "eng-cc/oasis7",
                "workflow_ref": "eng-cc/oasis7/.github/workflows/rust.yml@refs/heads/main",
                "planner_authority_oid": planner_authority_oid,
                "planner_config_sha256": config_digest,
            },
            planner_invocation=invocation,
            repository="eng-cc/oasis7",
            task_uid=invocation["task_uid"],
            pr_number=7,
            source_head_oid=source_head_oid or self.base,
            source_scope_oid=source_scope_oid or self.base,
            assessed_target_oid=assessed_target_oid or commit,
            input_scope_commit_oid=commit,
            input_scope_tree_oid=tree_oid,
            effective_policy_identity={
                "schema": inputs.EFFECTIVE_POLICY_IDENTITY_SCHEMA,
                "digest": "sha256:" + "6" * 64,
            },
            unit_specs=unit_specs,
            product_corpus=product_corpus,
        )

    def test_local_target_observation_is_distinct_from_execution_inventory(self):
        specs = [self.unit("unit-a", "pkg/a/a.rs", "pkg/a")]
        corpus_specs, corpus = inputs.build_product_corpus_unit_specs(
            str(self.root), self.base,
            command_checker_paths=[COMMAND_PATH],
            applicable_policy={"policy": "product-v1"},
            environment_contract={"python": "fixture-v1"},
        )
        specs.extend(corpus_specs)
        observation = self.local_target_observation(self.base, specs, corpus)
        scope = inputs.build_input_scope_snapshot(
            str(self.root), self.base, specs, corpus,
            target_observation=observation,
        )

        self.assertEqual(inputs.TARGET_INPUT_SCOPE_SCHEMA, scope["schema"])
        self.assertEqual(inputs.TARGET_OBSERVATION_SCHEMA, observation["schema"])
        self.assertEqual(observation, scope["target_observation"])
        self.assertNotIn("producer", observation)
        self.assertNotIn("artifact_id", observation)
        self.assertNotIn("run_id", observation)
        self.assertNotIn("status", observation)
        validated = inputs.validate_input_scope_snapshot(
            scope, trusted_target_observation=observation,
        )
        self.assertEqual(observation["assessed_target_oid"],
                         validated["target_observation"]["assessed_target_oid"])

    def test_local_target_observation_rejects_wrong_q_w_and_inventory(self):
        specs = [self.unit("unit-a", "pkg/a/a.rs", "pkg/a")]
        corpus_specs, corpus = inputs.build_product_corpus_unit_specs(
            str(self.root), self.base,
            command_checker_paths=[COMMAND_PATH],
            applicable_policy={"policy": "product-v1"},
            environment_contract={"python": "fixture-v1"},
        )
        specs.extend(corpus_specs)
        observation = self.local_target_observation(self.base, specs, corpus)
        scope = inputs.build_input_scope_snapshot(
            str(self.root), self.base, specs, corpus, target_observation=observation,
        )

        wrong_q = {**observation, "assessed_target_oid": "b" * 40}
        with self.assertRaisesRegex(inputs.InputScopeError, "not bound"):
            inputs.validate_input_scope_snapshot(
                scope, trusted_target_observation=wrong_q,
            )
        wrong_w = {**observation, "authority": {
            **observation["authority"], "planner_authority_oid": "c" * 40,
        }}
        with self.assertRaisesRegex(inputs.InputScopeError, "trusted W|not bound"):
            inputs.validate_input_scope_snapshot(
                scope, trusted_target_observation=wrong_w,
            )
        wrong_inventory = {**observation, "inventory_digest": "sha256:" + "0" * 64}
        with self.assertRaisesRegex(inputs.InputScopeError, "not bound"):
            inputs.validate_input_scope_snapshot(
                scope, trusted_target_observation=wrong_inventory,
            )

    def test_local_target_observation_rejects_success_claims_and_bad_invocation_digest(self):
        specs = [self.unit("unit-a", "pkg/a/a.rs", "pkg/a")]
        corpus_specs, corpus = inputs.build_product_corpus_unit_specs(
            str(self.root), self.base,
            command_checker_paths=[COMMAND_PATH],
            applicable_policy={"policy": "product-v1"},
            environment_contract={"python": "fixture-v1"},
        )
        specs.extend(corpus_specs)
        observation = self.local_target_observation(self.base, specs, corpus)

        with self.assertRaisesRegex(inputs.InputScopeError, "fields are incomplete or unsupported"):
            inputs.validate_target_observation_binding(
                {**observation, "test_status": "passed"}, observation,
            )
        for fake_remote_locator in (
            {"artifact_id": 99},
            {"producer": {"run_id": 10, "run_attempt": 1, "check_run_id": 20}},
        ):
            with self.subTest(fake_remote_locator=fake_remote_locator), self.assertRaisesRegex(
                inputs.InputScopeError, "fields are incomplete or unsupported",
            ):
                inputs.validate_target_observation_binding(
                    {**observation, **fake_remote_locator}, observation,
                )
        bad_invocation = {**observation, "planner_invocation": {
            **observation["planner_invocation"], "digest": "sha256:" + "0" * 64,
        }}
        with self.assertRaisesRegex(inputs.InputScopeError, "invocation digest"):
            inputs.validate_target_observation_binding(bad_invocation, observation)

    def test_source_execution_inventory_still_requires_artifact_readback(self):
        scope = self.scope(self.base)
        binding = self.trusted_inventory_readback(scope["planner_inventory_issuer"])
        del binding["producer"]["artifact_id"]
        with self.assertRaisesRegex(inputs.InputScopeError, "producer locator is incomplete"):
            inputs.validate_input_scope_snapshot(
                scope, trusted_planner_inventory=binding,
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
