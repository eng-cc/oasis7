import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("starter", Path(__file__).resolve().parents[1] / "industrial-starter-evidence.py")
starter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(starter)


class StarterEvidenceTests(unittest.TestCase):
    def test_registration_uses_acknowledged_bound_agent_and_rejects_errors(self):
        payload = {"responses": [{"type": "authoritative_recovery_ack", "ack": {
            "status": "session_registered", "player_id": "p", "session_pubkey": "key",
            "agent_id": "actual-bound-agent",
        }}]}
        self.assertEqual(starter.verify_recovery_ack(payload, "p", "key"), "actual-bound-agent")
        for field, value in [("status", "reconnect_synced"), ("player_id", "other"),
                             ("session_pubkey", "other"), ("agent_id", None)]:
            changed = copy.deepcopy(payload)
            changed["responses"][0]["ack"][field] = value
            with self.assertRaises(ValueError):
                starter.verify_recovery_ack(changed, "p", "key")
        payload["responses"].append({"type": "authoritative_recovery_error", "error": {"code": "denied"}})
        with self.assertRaises(ValueError):
            starter.verify_recovery_ack(payload, "p", "key")

    def test_empty_world_bootstrap_diagnostic_preserves_runtime_blocker(self):
        diagnostic = starter.bootstrap_failure({
            "blocker_kind": "runtime_snapshot_empty_entities",
            "blocker_detail": "world has no agents/locations",
            "available_actions": [{"action_id": "request_snapshot"}],
        }, "build_factory_smelter_mk1")
        self.assertEqual(diagnostic["status"], "failed")
        self.assertEqual(diagnostic["blocker_kind"], "runtime_snapshot_empty_entities")
        self.assertEqual(diagnostic["available_action_ids"], ["request_snapshot"])
        self.assertFalse(diagnostic["canonical_production_verified"])

    def fixture(self):
        origin = dict(consensus_action_id=2, committed_height=8, action_payload_hash="a" * 64,
                      action_root="b" * 64, consensus_submitter_player_id="node-transport",
                      submission=dict(verified_player_id="p1", public_key="c" * 64, auth_nonce=1,
                                      requester_agent_id="a1", factory_id=starter.FACTORY, recipe_id=starter.RECIPE))
        milestone = dict(committed_recipe_origin=copy.deepcopy(origin), profile_id=starter.PROFILE, profile_revision=1, factory_id=starter.FACTORY,
                         recipe_id=starter.RECIPE, output_ledger="site:s1", settlement_job_id=7, settled_at=9)
        prior = {"factories": {starter.FACTORY: dict(site_id="s1", builder_agent_id="a1", output_ledger="site:s1")},
                 "factory_site_authorities": {"s1": {"owner_agent_id": "a1"}},
                 "industry_progress": {}, "material_ledgers": {"site:s1": {}}}
        current = copy.deepcopy(prior)
        current["industry_progress"]["starter_industrial_milestone"] = milestone
        current["material_ledgers"]["site:s1"]["iron_ingot"] = 2
        current["recipe_completion_receipts"] = {"7": dict(committed_recipe_origin=copy.deepcopy(origin), job_id=7, factory_id=starter.FACTORY,
            recipe_id=starter.RECIPE, accepted_batches=1, requester_agent_id="a1", output_ledger="site:s1",
            produce=[dict(kind="iron_ingot", amount=2)])}
        feasibility = dict(profile_id=starter.PROFILE, profile_revision=1, evidence_class="durable-milestone-backed")
        def payload(state):
            return {"latest_snapshot": {"runtime_snapshot": {"state": state}},
                    "player_gameplay": {"starter_industrial_feasibility": copy.deepcopy(feasibility)}}
        def ack(action):
            return {"responses": [{"type": "gameplay_action_ack", "ack": dict(action_id=action,
                target_agent_id="a1", player_id="p1", runtime_action_id=2, consensus_action_payload_hash="a" * 64)}]}
        return [ack("build_factory_smelter_mk1"), payload(prior), ack("schedule_recipe_smelter_iron_ingot"), payload(current), payload(copy.deepcopy(current)), {"responses": [{"type": "authoritative_recovery_ack", "ack": {"status": "session_registered", "player_id": "p1", "agent_id": "a1", "session_pubkey": "c" * 64}}]}]

    def test_matching_positive_owner_settlement_and_reconnect(self):
        self.assertTrue(all(starter.validate(*self.fixture())["checks"].values()))

    def test_rejected_actions_cannot_be_completed_by_progress(self):
        for index, check in [(0, "canonical_build_accepted"), (2, "canonical_recipe_accepted")]:
            records = self.fixture()
            records[index]["responses"] = [{"type": "gameplay_action_error"}]
            self.assertFalse(starter.validate(*records)["checks"][check])

    def test_queue_or_inventory_without_receipt_is_not_settlement(self):
        records = self.fixture()
        starter.state(records[3])["recipe_completion_receipts"] = {}
        self.assertFalse(starter.validate(*records)["checks"]["canonical_matching_settlement"])

    def test_foreign_owner_or_wrong_profile_is_rejected(self):
        for mutate, check in [
            (lambda s: s["factory_site_authorities"]["s1"].update(owner_agent_id="other"), "canonical_owner_output"),
            (lambda s: s["industry_progress"]["starter_industrial_milestone"].update(profile_revision=2), "canonical_profile_revision"),
            (lambda s: s["material_ledgers"]["site:s1"].update(iron_ingot=0), "canonical_positive_iron_credit"),
        ]:
            records = self.fixture()
            mutate(starter.state(records[3]))
            self.assertFalse(starter.validate(*records)["checks"][check])

    def test_missing_runtime_baseline_and_reconnect_divergence_fail(self):
        records = self.fixture()
        records[1] = {"player_gameplay": {"progress_percent": 100}}
        self.assertFalse(starter.validate(*records)["checks"]["canonical_positive_iron_credit"])
        records = self.fixture()
        starter.state(records[4])["industry_progress"]["starter_industrial_milestone"]["settlement_job_id"] = 8
        self.assertFalse(starter.validate(*records)["checks"]["canonical_reconnect_same_milestone"])

    def test_boolean_batches_amounts_and_foreign_ack_cannot_fake_production(self):
        for field in ["accepted_batches", "amount"]:
            records = self.fixture()
            receipt = starter.state(records[3])["recipe_completion_receipts"]["7"]
            if field == "amount":
                receipt["produce"][0][field] = True
            else:
                receipt[field] = True
            self.assertFalse(starter.validate(*records)["checks"]["canonical_matching_settlement"])
        records = self.fixture()
        records[2]["responses"][0]["ack"]["target_agent_id"] = "other"
        self.assertFalse(starter.validate(*records)["checks"]["canonical_ack_owner_binding"])

    def test_reconnect_owner_and_boolean_or_float_identities_fail(self):
        records = self.fixture()
        starter.state(records[4])["factory_site_authorities"]["s1"]["owner_agent_id"] = "other"
        self.assertFalse(starter.validate(*records)["checks"]["canonical_reconnect_owner_output"])
        for invalid in [True, 1.0]:
            for index in [3, 4]:
                records = self.fixture()
                records[index]["player_gameplay"]["starter_industrial_feasibility"]["profile_revision"] = invalid
                starter.state(records[index])["industry_progress"]["starter_industrial_milestone"]["profile_revision"] = invalid
                check = "canonical_profile_revision" if index == 3 else "canonical_reconnect_profile"
                self.assertFalse(starter.validate(*records)["checks"][check])
        for invalid in [True, 7.0]:
            records = self.fixture()
            records[2]["responses"][0]["ack"]["runtime_action_id"] = invalid
            self.assertFalse(starter.validate(*records)["checks"]["canonical_matching_settlement"])
        for invalid in [True, 2.0, -1]:
            records = self.fixture()
            starter.state(records[4])["material_ledgers"]["site:s1"]["iron_ingot"] = invalid
            self.assertFalse(starter.validate(*records)["checks"]["canonical_reconnect_output_preserved"])

    def test_reconnect_receipt_requires_type_preserving_identity(self):
        for field, invalid in [("accepted_batches", True), ("accepted_batches", 1.0), ("amount", 2.0)]:
            records = self.fixture()
            receipt = starter.state(records[4])["recipe_completion_receipts"]["7"]
            if field == "amount":
                receipt["produce"][0][field] = invalid
            else:
                receipt[field] = invalid
            self.assertFalse(starter.validate(*records)["checks"]["canonical_reconnect_owner_output"])

    def test_exact_submission_origin_rejects_missing_or_tampered_provenance(self):
        for field, value in [("consensus_action_id", True), ("committed_height", 8.0),
                             ("action_payload_hash", "d" * 64), ("action_root", "bad")]:
            records = self.fixture()
            receipt = starter.state(records[3])["recipe_completion_receipts"]["7"]
            receipt["committed_recipe_origin"][field] = value
            self.assertFalse(all(starter.validate(*records)["checks"].values()))
        records = self.fixture()
        starter.state(records[3])["recipe_completion_receipts"]["7"].pop("committed_recipe_origin")
        self.assertFalse(starter.validate(*records)["checks"]["canonical_exact_submission_origin"])
        records = self.fixture()
        records[2]["responses"][0]["ack"]["consensus_action_payload_hash"] = "f" * 64
        self.assertFalse(starter.validate(*records)["checks"]["canonical_exact_submission_origin"])

    def test_provenance_identities_require_nonempty_strings(self):
        for invalid in [True, 1, "", " "]:
            records = self.fixture()
            for index in [0, 2]:
                records[index]["responses"][0]["ack"].update(player_id=invalid, target_agent_id=invalid)
            records[5]["responses"][0]["ack"].update(player_id=invalid, agent_id=invalid)
            for index in [3, 4]:
                state = starter.state(records[index])
                state["factories"][starter.FACTORY]["builder_agent_id"] = invalid
                state["factory_site_authorities"]["s1"]["owner_agent_id"] = invalid
                receipt = state["recipe_completion_receipts"]["7"]
                receipt["requester_agent_id"] = invalid
                for origin in [receipt["committed_recipe_origin"], state["industry_progress"]["starter_industrial_milestone"]["committed_recipe_origin"]]:
                    origin["consensus_submitter_player_id"] = invalid
                    origin["submission"].update(verified_player_id=invalid, requester_agent_id=invalid)
            self.assertFalse(all(starter.validate(*records)["checks"].values()))

    def test_optional_hosted_nonce_matches_typed_runtime_schema(self):
        for invalid in [True, 123, "", " "]:
            records = self.fixture()
            for index in [3, 4]:
                state = starter.state(records[index])
                for origin in [state["recipe_completion_receipts"]["7"]["committed_recipe_origin"], state["industry_progress"]["starter_industrial_milestone"]["committed_recipe_origin"]]:
                    origin["submission"]["hosted_registration_nonce"] = invalid
            self.assertFalse(starter.validate(*records)["checks"]["canonical_exact_submission_origin"])


if __name__ == "__main__":
    unittest.main()
