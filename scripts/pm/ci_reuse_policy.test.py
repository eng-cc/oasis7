"""Trusted W policy resolution remains disabled and rejects caller substitutes."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

HERE = Path(__file__).parent
SPEC = importlib.util.spec_from_file_location("ci_reuse_policy", HERE / "ci_reuse_policy.py")
policy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(policy)


class TrustedPolicyTests(unittest.TestCase):
    def test_policy_default_is_disabled_and_has_stable_c3_identity(self):
        value = policy.default_effective_policy()
        self.assertEqual([], value["enabled_capabilities"])
        self.assertEqual([], value["approved_executor_contract_digests"])
        self.assertEqual(15368, value["check_app_id"])
        self.assertRegex(policy.effective_policy_identity(value)["digest"], r"^sha256:[0-9a-f]{64}$")
        with self.assertRaisesRegex(ValueError, "W-owned disabled default"):
            policy.effective_policy_identity({**value, "enabled_capabilities": ["input-scope-reuse/v1"]})

    def test_resolver_binds_helper_config_ref_and_default_branch_workflow_sha(self):
        helper = (HERE / "ci_reuse_policy.py").read_bytes()
        config = (HERE.parents[1] / "scripts/ci-required-scope.v2.json").read_bytes()
        workflow_sha = "a" * 40
        context = policy.resolve_trusted_policy_context(
            repository="owner/repo",
            default_branch="main",
            workflow_ref="owner/repo/.github/workflows/rust.yml@refs/heads/main",
            workflow_sha=workflow_sha,
            default_branch_sha=workflow_sha,
            policy_source=helper,
            local_policy_source=helper,
            planner_config=config,
        )
        self.assertEqual(workflow_sha, context["planner_inventory_authority"]["planner_authority_oid"])
        self.assertEqual(
            "sha256:" + __import__("hashlib").sha256(config).hexdigest(),
            context["planner_inventory_authority"]["planner_config_sha256"],
        )
        self.assertEqual([], context["effective_policy"]["enabled_capabilities"])

        invalid = (
            {"workflow_ref": "owner/repo/.github/workflows/rust.yml@refs/heads/feature"},
            {"default_branch_sha": "b" * 40},
            {"policy_source": helper + b"# changed"},
            {"planner_config": b'{"schema":"unknown"}'},
        )
        for changes in invalid:
            kwargs = {
                "repository": "owner/repo",
                "default_branch": "main",
                "workflow_ref": "owner/repo/.github/workflows/rust.yml@refs/heads/main",
                "workflow_sha": workflow_sha,
                "default_branch_sha": workflow_sha,
                "policy_source": helper,
                "local_policy_source": helper,
                "planner_config": config,
            }
            kwargs.update(changes)
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                policy.resolve_trusted_policy_context(**kwargs)

    def test_cli_reads_policy_and_planner_config_from_exact_w(self):
        source = (HERE / "ci_reuse_policy.py").read_bytes()
        config = (HERE.parents[1] / "scripts/ci-required-scope.v2.json").read_bytes()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "scripts/pm").mkdir(parents=True)
            (root / "scripts").mkdir(exist_ok=True)
            (root / policy.POLICY_PATH).write_bytes(source)
            (root / policy.PLANNER_CONFIG_PATH).write_bytes(config)
            subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "Policy Test"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "policy@example.invalid"], check=True)
            subprocess.run(["git", "-C", str(root), "add", "scripts"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "-qm", "trusted W fixture"], check=True)
            workflow_sha = subprocess.check_output(
                ["git", "-C", str(root), "rev-parse", "HEAD"], text=True,
            ).strip()
            output = root / "trusted-context.json"
            result = policy.main([
                "trusted-context", "--root", str(root), "--repository", "owner/repo",
                "--default-branch", "main",
                "--workflow-ref", "owner/repo/.github/workflows/rust.yml@refs/heads/main",
                "--workflow-sha", workflow_sha, "--default-branch-sha", workflow_sha,
                "--output", str(output),
            ])
            self.assertEqual(0, result)
            context = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(workflow_sha, context["workflow_sha"])
            self.assertEqual([], context["effective_policy"]["enabled_capabilities"])


if __name__ == "__main__":
    unittest.main()
