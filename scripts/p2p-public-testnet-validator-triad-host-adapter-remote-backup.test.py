#!/usr/bin/env python3
"""Contract tests for the governed triad remote-backup callback phases.

The callback fixture emits the same transaction/role/host/root-bound receipt
shape required by production validation.  It never claims that a real host
was reached.  The executor and receipt validators remain real; only live
observation is patched to keep this test secret-free and offline.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
ADAPTER_PATH = ROOT / "scripts" / "p2p-public-testnet-validator-triad-host-adapter.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


EXECUTOR = load_module(EXECUTOR_PATH, "triad_remote_backup_executor")
ADAPTER = load_module(ADAPTER_PATH, "triad_remote_backup_adapter")
TRANSACTION_ID = "remote-backup-contract-003"
PLAN_DIGEST = "d" * 64
RUNTIME_SHA256 = "a" * 64
RESET_SURFACES = list(EXECUTOR.RESET_SURFACES)
LISTENERS = {
    "storage-205": ["6632", "6832"],
    "sequencer-204": ["6631", "6831"],
}


def digest_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reset_digest() -> str:
    return hashlib.sha256(
        json.dumps(RESET_SURFACES, ensure_ascii=True, separators=(",", ":")).encode()
    ).hexdigest()


def live_observation(roots: dict[str, Path]) -> dict[str, object]:
    return {
        "observation_state": "live",
        "nodes": {
            role: {
                "role": role,
                "root": str(root),
                "active": True,
                "running": True,
                "service_state": "running",
                "independently_observed": True,
            }
            for role, root in roots.items()
        },
    }


def write_evidence(root: Path) -> dict[str, object]:
    identities = []
    for role in EXECUTOR.MUTATION_ORDER:
        path = root / f"{role}.identity.json"
        path.write_text("{}\n", encoding="utf-8")
        identities.append(
            {
                "path": str(path),
                "sha256": digest_file(path),
                "role": role,
                "node_id": f"node-{role}",
                "peer_id": f"peer-{role}",
            }
        )
    proof_path = root / "sequencer-rebuild-proof.json"
    proof_path.write_text("{}\n", encoding="utf-8")
    return {
        "identity_receipts": identities,
        "sequencer_rebuild_proof": {
            "path": str(proof_path),
            "sha256": digest_file(proof_path),
            "role": "sequencer-204",
            "node_id": "node-sequencer-204",
            "peer_id": "peer-sequencer-204",
        },
        "sequencer_rebuild_proof_path": str(proof_path),
    }


def make_transaction(root: Path) -> dict[str, object]:
    roots = {role: root / role for role in EXECUTOR.MUTATION_ORDER}
    for node_root in roots.values():
        node_root.mkdir()
        (node_root / "state.txt").write_text("fixture\n", encoding="utf-8")
    evidence = write_evidence(root)
    observations = {
        f"before-{phase}": live_observation(roots)
        for phase in (
            "staggered-storage-backup",
            "staggered-storage",
            "staggered-sequencer-backup",
            "staggered-sequencer",
        )
    }
    return {
        "schema_version": EXECUTOR.SCHEMA,
        "execution_mode": EXECUTOR.TRIAD_STAGGERED_EXECUTION_MODE,
        "transaction_id": TRANSACTION_ID,
        "plan_digest": PLAN_DIGEST,
        "phase": "staggered_prepared",
        "rollback": {
            "strategy": "staggered-target-only-cleanup",
            "required_on_gate_failure": True,
            "target_only_cleanup": True,
            "restore_deleted_chain_state": False,
            "restore_only_forensic_snapshot": True,
        },
        "mutation_order": list(EXECUTOR.MUTATION_ORDER),
        "startup_order": list(EXECUTOR.MUTATION_ORDER),
        "nodes": {
            role: {"role": role, "transport": "direct_ssh", "root": str(node_root)}
            for role, node_root in roots.items()
        },
        "package": {"runtime_sha256": RUNTIME_SHA256, "commit": "c" * 40, "run_id": "fixture-run"},
        "capacity": {
            role: {
                "required_bytes": 1024,
                "required_inodes": 128,
                "free_bytes": 4096,
                "free_inodes": 256,
                "same_filesystem": True,
                "verified": True,
                "inventory": {"total_bytes": 1, "entry_count": 1},
            }
            for role in EXECUTOR.MUTATION_ORDER
        },
        "capacity_apply": {role: {"verified": True} for role in EXECUTOR.MUTATION_ORDER},
        "staggered_preflight_receipt": {"fixture": True},
        "staggered_live_observations": observations,
        "staggered_completed_roles": [],
        "staged": {},
        "backup": {},
        "proof": {
            "identity_receipts": evidence["identity_receipts"],
            "sequencer_rebuild_proof": evidence["sequencer_rebuild_proof"],
            "sequencer_rebuild_proof_path": evidence["sequencer_rebuild_proof_path"],
        },
    }


def write_callback_fixture(path: Path, observations_path: Path, case: str) -> None:
    """Write a subprocess callback that emits valid or deliberately invalid receipt evidence."""
    reset_surfaces_literal = repr(RESET_SURFACES)
    expected_hosts_literal = repr(
        {role: EXECUTOR.HUMAN_DIRECT_SSH_CANONICAL[role]["host"] for role in EXECUTOR.MUTATION_ORDER}
    )
    expected_roots_literal = repr(
        {role: EXECUTOR.PRODUCTION_STACK_ROOT for role in EXECUTOR.MUTATION_ORDER}
    )
    listeners_literal = repr(LISTENERS)
    path.write_text(
        f"""#!{os.sys.executable}
import hashlib
import json
import os
import sys
from pathlib import Path

args = sys.argv
transaction_path = Path(args[args.index('--transaction') + 1])
phase = args[args.index('--phase') + 1]
transaction = json.loads(transaction_path.read_text(encoding='utf-8'))
target = {{
    'staggered-storage-backup': 'storage-205',
    'staggered-sequencer-backup': 'sequencer-204',
    'staggered-storage': 'storage-205',
    'staggered-sequencer': 'sequencer-204',
}}.get(phase)
peer = 'sequencer-204' if target == 'storage-205' else 'storage-205'
roots = transaction['nodes']
reset_surfaces = {reset_surfaces_literal}
reset_digest = hashlib.sha256(json.dumps(reset_surfaces, ensure_ascii=True, separators=(',', ':')).encode()).hexdigest()
expected_hosts = {expected_hosts_literal}
expected_roots = {expected_roots_literal}

def live_node(role):
    return {{
        'role': role,
        'root': roots[role]['root'],
        'active': True,
        'running': True,
        'service_state': 'running',
        'independently_observed': True,
        'healthz_ok': True,
        'nrestarts': 0,
        'oom_panic_segfault': False,
        'listeners': {listeners_literal}[role],
        'runtime_sha256': transaction['package']['runtime_sha256'],
        'full_chain_status_called': False,
    }}

def remote_backup(role):
    txid = transaction['transaction_id']
    backup_root = f"{{expected_roots[role]}}/backups/{{txid}}"
    manifest_sha = 'b' * 64
    entry = {{
        'schema_version': 'oasis7.validator_pair_rebuild_remote_backup_receipt.v1',
        'role': role,
        'transaction_id': txid,
        'remote_target': True,
        'credential_transport': 'fd-only-v1',
        'remote_host': expected_hosts[role],
        'remote_root': expected_roots[role],
        'backup_root': backup_root,
        'manifest': f"{{backup_root}}/manifest.json",
        'manifest_sha256': manifest_sha,
        'reset_surface_manifest_sha256': manifest_sha,
        'reset_surfaces': reset_surfaces,
        'capacity': {{
            'verified': True,
            'same_filesystem': True,
            'available_bytes': 4096,
            'free_bytes': 4096,
            'required_bytes': transaction['capacity'][role]['required_bytes'],
            'free_inodes': 256,
            'required_inodes': transaction['capacity'][role]['required_inodes'],
        }},
        'backup_non_seed': {{
            'forensic_only': True,
            'seed_eligible': False,
            'restore_deleted_chain_state': False,
        }},
    }}
    if {case!r} == 'missing':
        entry.pop('remote_target')
    elif {case!r} == 'stale':
        entry['transaction_id'] = 'stale-transaction'
    return entry

binding = transaction['adapter_binding']
evidence = binding['evidence_bindings']
nodes = {{role: live_node(role) for role in ('storage-205', 'sequencer-204')}}
receipt = {{
    'schema_version': 'oasis7.validator_pair_rebuild_host_receipt.v2',
    'phase': phase,
    'execution_mode': transaction['execution_mode'],
    'transaction_id': transaction['transaction_id'],
    'plan_digest': transaction['plan_digest'],
    'evidence_bindings': evidence,
    'repository_executable': binding['repository_executable'],
    'captured_at': binding['phase_window_started_at'],
    'mutation_order': transaction['mutation_order'],
    'startup_order': transaction['startup_order'],
    'observer_mutation': False,
    'max_simultaneously_stopped_validators': 1,
    'identity_receipts': evidence['identity_receipts'],
    'sequencer_rebuild_proof': evidence['sequencer_rebuild_proof'],
    'package': binding['package'],
    'nodes': nodes,
}}
if phase in ('staggered-storage-backup', 'staggered-sequencer-backup'):
    nodes[target].update(remote_backup(target))
    nodes[target]['backup_verified'] = True
    receipt.update({{
        'staggered_phase': 'remote_backup',
        'backup_role': target,
        'live_peer_role': peer,
        'backup_before_stop': True,
        'target_stopped_before_reset': False,
        'reset_started_after_target_stop': False,
        'backup_receipt': nodes[target],
    }})
else:
    nodes[target]['post_delete_absence'] = {{
        'absent': True,
        'target_set': reset_surfaces,
        'target_set_sha256': reset_digest,
    }}
    receipt.update({{
        'staggered_phase': 'member_cutover',
        'target_role': target,
        'live_peer_role': peer,
        'live_peer_readback': True,
        'rebuilt_member_readiness': True,
        'target_stopped_before_reset': {{
            'active': False,
            'running': False,
            'service_state': 'stopped',
            'independently_observed': True,
        }},
        'reset_started_after_target_stop': True,
    }})
record_path = Path({str(observations_path)!r})
rows = json.loads(record_path.read_text(encoding='utf-8')) if record_path.exists() else []
rows.append({{'phase': phase, 'role': target, 'transaction': transaction}})
record_path.write_text(json.dumps(rows, sort_keys=True), encoding='utf-8')
print(json.dumps(receipt, sort_keys=True, separators=(',', ':')))
""",
        encoding="utf-8",
    )
    path.chmod(0o700)


class GovernedRemoteBackupPhaseTests(unittest.TestCase):
    def run_fixture(self, case: str) -> tuple[dict[str, object], list[dict[str, object]]]:
        with tempfile.TemporaryDirectory(prefix=f"oasis7-remote-backup-{case}-") as temp_dir:
            root = Path(temp_dir)
            transaction = make_transaction(root)
            transaction_path = root / "transaction.json"
            EXECUTOR.write_json(transaction_path, transaction)
            observations_path = root / "observations.json"
            observations_path.write_text("[]", encoding="utf-8")
            callback = root / "callback.py"
            write_callback_fixture(callback, observations_path, case)
            credential_path = root / "credential"
            credential_path.write_text("fixture-secret\n", encoding="utf-8")
            credential_fd = os.open(credential_path, os.O_RDONLY)
            direct_args = argparse.Namespace(
                adapter_credential_fd=credential_fd,
                credential_fd=None,
                adapter_storage_credential_fd=None,
                adapter_sequencer_credential_fd=None,
                credential_env=None,
            )
            try:
                with patch.object(EXECUTOR, "_record_staggered_live_reobserve", return_value={}):
                    if case == "valid":
                        EXECUTOR._continue_staggered_transaction(
                            transaction,
                            transaction_path,
                            callback,
                            direct_args,
                            fresh_direct_observation=False,
                        )
                    else:
                        with self.assertRaises(SystemExit):
                            EXECUTOR._continue_staggered_transaction(
                                transaction,
                                transaction_path,
                                callback,
                                direct_args,
                                fresh_direct_observation=False,
                            )
            finally:
                os.close(credential_fd)
            persisted = EXECUTOR.load_json(transaction_path, "fixture transaction")
            observations = json.loads(observations_path.read_text(encoding="utf-8"))
            return persisted, observations

    def test_valid_remote_backup_phase_is_durable_before_mutation(self) -> None:
        transaction, observations = self.run_fixture("valid")
        phases = [item["phase"] for item in observations]
        self.assertEqual(
            phases,
            [
                "staggered-storage-backup",
                "staggered-storage",
                "staggered-sequencer-backup",
                "staggered-sequencer",
            ],
        )
        backups = transaction.get("backup")
        self.assertIsInstance(backups, dict)
        self.assertEqual(set(backups), set(EXECUTOR.MUTATION_ORDER))
        for role in EXECUTOR.MUTATION_ORDER:
            entry = backups[role]
            self.assertIs(entry["remote_target"], True)
            self.assertEqual(entry["role"], role)
            self.assertEqual(entry["transaction_id"], TRANSACTION_ID)
            self.assertEqual(entry["remote_host"], EXECUTOR.HUMAN_DIRECT_SSH_CANONICAL[role]["host"])
            self.assertEqual(entry["remote_root"], EXECUTOR.PRODUCTION_STACK_ROOT)
            self.assertEqual(entry["reset_surface_manifest_sha256"], entry["manifest_sha256"])
            self.assertIs(entry["backup_non_seed"]["seed_eligible"], False)
            self.assertTrue(entry["capacity"]["verified"])
        self.assertEqual(transaction["phase"], "applied")

    def test_missing_or_stale_remote_receipt_fails_before_member_mutation(self) -> None:
        for case in ("missing", "stale"):
            with self.subTest(case=case):
                transaction, observations = self.run_fixture(case)
                self.assertEqual([item["phase"] for item in observations], ["staggered-storage-backup"])
                self.assertNotIn("staggered-storage", [item["phase"] for item in observations])
                self.assertEqual(transaction.get("backup"), {})
                callback = transaction.get("adapter_callback")
                self.assertIsInstance(callback, dict)
                self.assertEqual(callback.get("status"), "failed")

    def test_fixedssh_argv_keeps_canonical_host_pin_and_inherited_fd(self) -> None:
        role = "storage-205"
        expected = EXECUTOR.HUMAN_DIRECT_SSH_CANONICAL[role]
        transport = ADAPTER.FixedSSH.__new__(ADAPTER.FixedSSH)
        transport.inventory = {"nodes": {role: {"host": expected["host"]}}}
        transport.known_hosts = Path("/fixture/pinned-known-hosts")
        transport.fds = {"shared": 17}
        transport.sshpass = "/usr/bin/sshpass"
        transport.ssh = "/usr/bin/ssh"
        argv = transport._argv(role, "true")
        self.assertEqual(argv[:3], ["/usr/bin/sshpass", "-d", "17"])
        self.assertIn("StrictHostKeyChecking=yes", argv)
        self.assertIn("UserKnownHostsFile=/fixture/pinned-known-hosts", argv)
        self.assertEqual(argv[-2:], [expected["host"], "true"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
