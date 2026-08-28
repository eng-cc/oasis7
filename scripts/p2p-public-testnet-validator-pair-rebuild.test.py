#!/usr/bin/env python3
"""Contract tests for the governed validator pair rebuild executor.

These tests intentionally exercise only local temporary roots.  They are the
RED contract for task #3324 (predecessor task #3318); no live validator, SSH
target, or observer is used.
"""

from __future__ import annotations

import copy
import datetime as dt
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
import shutil
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EXECUTOR = ROOT / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"
WRAPPER = ROOT / "scripts" / "p2p-public-testnet-rebuild-validators.sh"
PROVENANCE = ROOT / "scripts" / "p2p-public-testnet-validator-pair-provenance.py"
RELEASE_BUNDLE = ROOT / "scripts" / "release-candidate-bundle.sh"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_digest(value: dict) -> str:
    body = {key: item for key, item in value.items() if key != "canonical_digest"}
    return hashlib.sha256(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


CANONICAL_RESET_SURFACES = [
    "data/execution-records",
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/storage",
    "data/runtime-root",
    "data/replication-root",
    "output/chain-runtime",
    "output/node-distfs",
]


FROZEN_HEAD_OID = subprocess.run(
    ["git", "-C", str(ROOT), "rev-parse", "--verify", "HEAD^{commit}"],
    check=True,
    text=True,
    capture_output=True,
).stdout.strip()


class ValidatorPairRebuildContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="oasis7-pair-rebuild-contract-")
        self.root = Path(self.tmp.name)
        self.external_fixture_paths: list[Path] = []
        self._hermetic_environment_restore: dict[str, str | None] = {}
        self.package = self.root / "package"
        self.package.mkdir()
        runtime = self.package / "oasis7_chain_runtime"
        runtime.write_bytes(b"runtime-261\n")
        self.runtime_sha = sha256(runtime)
        self.runtime_size = runtime.stat().st_size
        (self.package / "BUILDINFO").write_text(
            "run_id=3218\ncommit=" + "a" * 40 + "\npackage_version=0.0.0+testnet.261\n",
            encoding="utf-8",
        )
        (self.package / "SHA256SUMS").write_text(
            f"{self.runtime_sha}  oasis7_chain_runtime\n",
            encoding="utf-8",
        )
        self.governed = self.root / "governed"
        self.governed.mkdir()
        for name, content in {
            "manifest.json": '{"network_id":"oasis7-public-testnet-governed-20260606","chain_id":"oasis7-public-testnet-governed-20260606"}\n',
            "genesis.json": '{"world_id":"oasis7-public-testnet-governed-20260606","chain_id":"oasis7-public-testnet-governed-20260606"}\n',
            "registry.json": '{"validators":[{"node_id":"triad-testnet-sequencer"},{"node_id":"triad-testnet-storage"}]}\n',
            "bootstrap.txt": "/ip4/127.0.0.1/tcp/6831/p2p/peer-sequencer-204\n/ip4/127.0.0.1/tcp/6832/p2p/peer-storage-205\n",
            "world.json": '{"snapshot":true}\n',
        }.items():
            (self.governed / name).write_text(content, encoding="utf-8")
        self.provenance = self.root / "provenance.json"
        self.signing_key = self.root / "attestor-key.pem"
        self.public_key = self.root / "attestor-public.pem"
        self.signature = self.root / "attestor-signature.bin"
        subprocess.run(
            [
                "openssl",
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-out",
                str(self.signing_key),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.run(
            ["openssl", "pkey", "-in", str(self.signing_key), "-pubout", "-out", str(self.public_key)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self._write_provenance()
        self.trust_root = self.root / "provenance-trust-root.json"
        trust_root = {
            "schema_version": "oasis7.validator_pair_provenance_trust_root.v1",
            "allowlist": [
                {
                    "signer_id": "testnet-package-attestor",
                    "algorithm": "openssl-rsa-sha256",
                    "public_key_sha256": sha256(self.public_key),
                }
            ],
        }
        trust_root["root_digest"] = canonical_digest(trust_root)
        self.trust_root.write_text(json.dumps(trust_root, indent=2) + "\n", encoding="utf-8")
        self.identity_receipts = self.root / "identity-receipts.json"
        identity_paths = []
        for role in ("storage-205", "sequencer-204"):
            identity_path = self.root / f"identity-{role}.json"
            signature_path = self.root / f"identity-{role}.sig"
            self._write_attestation(
                identity_path,
                signature_path,
                "oasis7.validator_identity_receipt.v1",
                role,
                f"peer-{role}",
                node_id="triad-testnet-storage" if role == "storage-205" else "triad-testnet-sequencer",
            )
            identity_paths.append(str(identity_path))
        self.identity_receipts.write_text(json.dumps({"receipts": identity_paths}, indent=2) + "\n", encoding="utf-8")
        self.rebuild_proof = self.root / "sequencer-204-rebuild-proof.json"
        self._write_attestation(
            self.rebuild_proof,
            self.root / "sequencer-204-rebuild-proof.sig",
            "oasis7.validator_pair_rebuild_proof.v1",
            "sequencer-204",
            "peer-sequencer-204",
            "triad-testnet-sequencer",
        )
        self.impact = self.root / "consumer-impact.json"
        self.impact.write_text(
            json.dumps(
                {
                    "impact": "none",
                    "evidence_source": "local contract fixture",
                    "timestamp": "2026-08-18T00:00:00Z",
                    "validators_already_stopped": True,
                    "outage_update_channel": "n/a",
                    "recovery_update_checkpoint": "n/a",
                    "producer_wording_approval": "n/a",
                    "decision": "proceed",
                }
            ),
            encoding="utf-8",
        )
        self.nodes = {}
        for role in ("storage-205", "sequencer-204"):
            node = self.root / role
            (node / "data").mkdir(parents=True)
            (node / "config").mkdir()
            (node / "current" / "bin").mkdir(parents=True)
            (node / "current" / "bin" / "oasis7_chain_runtime").write_bytes(b"runtime-old\n")
            (node / "config" / "node.env").write_text("NODE_ID=" + role + "\n", encoding="utf-8")
            self.nodes[role] = node
        self.capacity = self.root / "capacity.json"
        self.capacity.write_text(
            json.dumps(
                {
                    role: {
                        "free_bytes": 10_000_000,
                        "free_inodes": 100_000,
                        "same_filesystem": True,
                    }
                    for role in self.nodes
                }
            ),
            encoding="utf-8",
        )
        self.quiescence_id = "fixture-quiescence-3481"
        self.quiescence_proof = self.root / "quiescence-proof.json"
        self._write_quiescence_fixture()
        self.out = self.root / "out"

    def tearDown(self) -> None:
        for path in self.external_fixture_paths:
            path.unlink(missing_ok=True)
        for name, previous in self._hermetic_environment_restore.items():
            if previous is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = previous
        self.tmp.cleanup()

    def _install_hermetic_human_direct_environment(self, fixture: dict[str, Path | str]) -> None:
        """Expose the setUp direct-SSH fixture to legacy subprocess callers.

        Older plan/apply tests invoke ``subprocess.run`` directly and do not
        receive the newer helper's explicit environment.  Their persisted
        executor audit still causes a live direct re-observation, so the
        inherited environment must point at the same loopback-only fake GitHub
        and SSH binaries.  Rejection tests continue to pass their own env via
        ``_run_live_human_direct_ssh`` and are unaffected.
        """
        values = {
            "PATH": f"{fixture['bin']}:{os.environ.get('PATH', '')}",
            "HUMAN_DIRECT_SSH_ARGV_LOG": str(fixture["ssh_args"]),
            "HUMAN_DIRECT_SSH_COMMAND_LOG": str(fixture["ssh_commands"]),
            "HUMAN_DIRECT_SSH_SEAM_LOG": str(fixture["ssh_seam"]),
            "HUMAN_DIRECT_SSH_SSHPASS_LOG": str(fixture["sshpass"]),
            "HUMAN_DIRECT_SSH_SYSTEMCTL_LOG": str(fixture["systemctl"]),
            "HUMAN_DIRECT_SSH_SERVICE_LOG": str(fixture["service"]),
            "HUMAN_DIRECT_SSH_FAKE_STATE": str(fixture["state"]),
            "HUMAN_DIRECT_SSH_SECRET": str(fixture["secret"]),
            "OASIS7_VALIDATOR_PAIR_NONCE_LEDGER": str(fixture["nonce_ledger"]),
            "HUMAN_DIRECT_SSH_GITHUB_RESPONSE": str(fixture["github_response"]),
            "HUMAN_DIRECT_SSH_GITHUB_LOG": str(fixture["github_calls"]),
        }
        for name, value in values.items():
            if name not in self._hermetic_environment_restore:
                self._hermetic_environment_restore[name] = os.environ.get(name)
            os.environ[name] = value
        if "HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE" not in self._hermetic_environment_restore:
            self._hermetic_environment_restore["HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE"] = os.environ.get(
                "HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE"
            )
        os.environ.pop("HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE", None)

    def _write_provenance(self, *, signature: bool = True, extra_governed: Path | None = None) -> None:
        governed = {
            name: {
                "path": str(path),
                "sha256": sha256(path),
                "size_bytes": path.stat().st_size,
                "entry_count": 1,
                "link_count": 0,
                "dir_count": 0,
                "file_count": 1,
            }
            for name, path in (
                ("manifest", self.governed / "manifest.json"),
                ("genesis", self.governed / "genesis.json"),
                ("registry", self.governed / "registry.json"),
                ("bootstrap", self.governed / "bootstrap.txt"),
                ("world", self.governed / "world.json"),
            )
        }
        if extra_governed is not None:
            governed["unexpected"] = {
                "path": str(extra_governed),
                "sha256": sha256(extra_governed),
                "size_bytes": extra_governed.stat().st_size,
                "entry_count": 1,
                "link_count": 0,
                "dir_count": 0,
                "file_count": 1,
            }
        payload = {
            "schema_version": "oasis7.validator_pair_rebuild_provenance.v1",
            "network_id": "oasis7-public-testnet-governed-20260606",
            "chain_id": "oasis7-public-testnet-governed-20260606",
            "package": {
                "run_id": "3218",
                "commit": "a" * 40,
                "package_version": "0.0.0+testnet.261",
                "runtime_sha256": self.runtime_sha,
                "runtime_size_bytes": self.runtime_size,
                "buildinfo_sha256": sha256(self.package / "BUILDINFO"),
                "sha256sums_sha256": sha256(self.package / "SHA256SUMS"),
            },
            "governed": governed,
            "signature": (
                {
                    "status": "verified",
                    "signer_id": "testnet-package-attestor",
                    "algorithm": "openssl-rsa-sha256",
                    "signature_ref": str(self.signature),
                    "public_key_ref": str(self.public_key),
                    "public_key_sha256": sha256(self.public_key),
                }
                if signature
                else None
            ),
        }
        canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
        payload["binding_digest"] = hashlib.sha256(canonical.encode()).hexdigest()
        self.provenance.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        if signature:
            payload_path = self.root / "provenance-signing-payload.bin"
            payload_path.write_bytes(canonical.encode())
            subprocess.run(
                ["openssl", "dgst", "-sha256", "-sign", str(self.signing_key), "-out", str(self.signature), str(payload_path)],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

    def _write_attestation(self, path: Path, signature_path: Path, schema: str, role: str, peer_id: str, node_id: str) -> None:
        value = {
            "schema_version": schema,
            "role": role,
            "peer_id": peer_id,
            "node_id": node_id,
            "signer_id": "testnet-package-attestor",
            "algorithm": "openssl-rsa-sha256",
            "public_key_ref": str(self.public_key),
            "public_key_sha256": sha256(self.public_key),
            "signature_ref": str(signature_path),
            "trust_root_digest": json.loads(self.trust_root.read_text(encoding="utf-8"))["root_digest"],
        }
        body = {key: item for key, item in value.items() if key not in {"signature_ref", "public_key_ref"}}
        payload_path = path.with_suffix(path.suffix + ".payload")
        payload_path.write_bytes(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode())
        subprocess.run(
            ["openssl", "dgst", "-sha256", "-sign", str(self.signing_key), "-out", str(signature_path), str(payload_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        payload_path.unlink()
        path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

    def _write_host_adapter(self) -> Path:
        adapter = self.root / "host-adapter.py"
        adapter.write_text(
            """#!/usr/bin/env python3
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

transaction = json.loads(Path(sys.argv[sys.argv.index('--transaction') + 1]).read_text())
phase = sys.argv[sys.argv.index('--phase') + 1]
nodes = {}
for role in transaction['mutation_order']:
    nodes[role] = {
        'role': role,
        'root': transaction['nodes'][role]['root'],
        'active': phase == 'apply',
        'running': phase == 'apply',
        'service_state': 'running' if phase == 'apply' else 'stopped',
        'independently_observed': True,
    }
    if phase == 'quiesce':
        nodes[role].update({'active': False, 'running': False})
    elif phase == 'backup':
        nodes[role].update({'backup_verified': True})
    elif phase == 'rollback':
        nodes[role].update({'rollback_verified': True})
    else:
        nodes[role].update({
            'healthz_ok': True,
            'nrestarts': 0,
            'oom_panic_segfault': False,
            'runtime_sha256': transaction['package']['runtime_sha256'],
            'runtime_size_bytes': transaction['package']['runtime_size_bytes'],
            'listeners': ['6631', '6831'] if role == 'sequencer-204' else ['6632', '6832'],
            'full_chain_status_called': False,
        })
binding = transaction['adapter_binding']
evidence = binding['evidence_bindings']
print(json.dumps({
    'schema_version': 'oasis7.validator_pair_rebuild_host_receipt.v2',
    'phase': phase,
    'operation': 'quiesce-only' if phase == 'quiesce' else None,
    'quiescence_id': transaction.get('quiescence_request', {}).get('quiescence_id') if phase == 'quiesce' else None,
    'plan_digest': transaction['plan_digest'],
    'transaction_id': transaction['transaction_id'],
    'request_digest': transaction.get('quiescence_request', {}).get('request_digest') if phase == 'quiesce' else None,
    'impact_record_sha256': transaction.get('quiescence_request', {}).get('impact_record_sha256') if phase == 'quiesce' else None,
    'captured_at': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
    'mutation_order': transaction['mutation_order'],
    'startup_order': transaction['startup_order'],
    'nodes': nodes,
    'observer_mutation': False,
    'repository_executable': transaction['adapter_binding']['repository_executable'],
    'sequencer_proof_url': transaction['proof']['sequencer_proof_url'],
    'evidence_bindings': evidence,
    'identity_receipts': evidence['identity_receipts'],
    'sequencer_rebuild_proof': evidence['sequencer_rebuild_proof'],
}))
""",
            encoding="utf-8",
        )
        adapter.chmod(0o755)
        return adapter

    def _write_failure_host_adapter(self, phase_log: Path, *failed_phases: str) -> Path:
        """Wrap the valid fixture adapter with deterministic phase failures."""
        adapter = self._write_host_adapter()
        source = adapter.read_text(encoding="utf-8")
        phase_log_literal = repr(str(phase_log))
        failed_phases_literal = repr(set(failed_phases))
        source = source.replace(
            "phase = sys.argv[sys.argv.index('--phase') + 1]\n",
            "phase = sys.argv[sys.argv.index('--phase') + 1]\n"
            f"with Path({phase_log_literal}).open('a', encoding='utf-8') as handle:\n"
            "    handle.write(phase + '\\n')\n"
            f"if phase in {failed_phases_literal}:\n"
            "    raise SystemExit(42)\n",
            1,
        )
        adapter.write_text(source, encoding="utf-8")
        adapter.chmod(0o755)
        return adapter

    def _write_quiescence_fixture(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(
            transaction_id=self.quiescence_id,
            impact_path=self.impact,
        )
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.human_direct_fixture = fixture
        self._install_hermetic_human_direct_environment(fixture)
        self.quiescence_proof = Path(fixture["out"]) / "quiescence-proof.json"
        self.assertTrue(self.quiescence_proof.exists(), result.stdout)

    def _write_quiescence_adapter(self, *, active: bool = False) -> Path:
        adapter = self.root / "quiescence-host-adapter.py"
        adapter.write_text(
            f"""#!/usr/bin/env python3
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

transaction = json.loads(Path(sys.argv[sys.argv.index('--transaction') + 1]).read_text())
nodes = {{}}
for role in transaction['mutation_order']:
    nodes[role] = {{
        'role': role,
        'root': transaction['nodes'][role]['root'],
        'active': {str(active)},
        'running': {str(active)},
        'service_state': 'active' if {str(active)} else 'stopped',
        'independently_observed': True,
    }}
print(json.dumps({{
    'schema_version': 'oasis7.validator_pair_rebuild_quiescence_receipt.v1',
    'phase': 'quiesce',
    'operation': 'quiesce-only',
    'quiescence_id': transaction['quiescence_id'],
    'transaction_id': transaction['transaction_id'],
    'request_digest': transaction['request_digest'],
    'impact_record_sha256': transaction['impact_record_sha256'],
    'captured_at': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
    'mutation_order': transaction['mutation_order'],
    'startup_order': transaction['startup_order'],
    'nodes': nodes,
    'observer_mutation': False,
}}))
""",
            encoding="utf-8",
        )
        adapter.chmod(0o755)
        return adapter

    def _base_args(self) -> list[str]:
        return [
            sys.executable,
            str(EXECUTOR),
            "plan",
            "--package-dir",
            str(self.package),
            "--provenance",
            str(self.provenance),
            "--trust-root",
            str(self.trust_root),
            "--identity-receipts",
            str(self.identity_receipts),
            "--sequencer-rebuild-proof",
            str(self.rebuild_proof),
            "--sequencer-proof-url",
            "http://127.0.0.1:6631/v1/chain/rebuild-proof",
            "--consumer-impact-record",
            str(self.impact),
            "--stopped-quiescence-proof",
            str(self.quiescence_proof),
            "--quiescence-id",
            self.quiescence_id,
            "--capacity-json",
            str(self.capacity),
            "--node",
            f"storage-205=local:{self.nodes['storage-205']}",
            "--node",
            f"sequencer-204=local:{self.nodes['sequencer-204']}",
            "--out-dir",
            str(self.out),
        ]

    def _apply_args(
        self,
        transaction_path: Path,
        adapter: Path | None = None,
        fixture: dict[str, Path | str] | None = None,
    ) -> list[str]:
        """Build an apply command with explicit hermetic direct-observation routing."""
        direct_fixture = fixture or self.human_direct_fixture
        args = [
            sys.executable,
            str(EXECUTOR),
            "apply",
            "--transaction",
            str(transaction_path),
        ]
        if adapter is not None:
            args.extend(("--host-adapter", str(adapter)))
        args.extend(
            (
                "--request",
                str(direct_fixture["request"]),
                "--known-hosts",
                str(direct_fixture["known_hosts"]),
            )
        )
        return args

    def _write_human_direct_ssh_fixture(
        self,
        *,
        transaction_id: str = "human-direct-ssh-tx-3481",
        impact_path: Path | None = None,
        state: str = "quiet",
        stale: bool = False,
        expired: bool = False,
        replayed: bool = False,
        mismatched: str | None = None,
        full_status: bool = False,
    ) -> dict[str, Path | str]:
        """Build a completely local human_direct_ssh fixture.

        The direct-SSH protocol is deliberately exercised through fake
        executables. A test which accidentally reaches a real ssh, sshpass,
        curl, or systemctl binary therefore fails closed instead of touching
        a developer or testnet host.
        """
        fixture = Path(tempfile.mkdtemp(prefix="human-direct-ssh-", dir=str(self.root)))
        bin_dir = fixture / "bin"
        bin_dir.mkdir(parents=True)
        known_hosts = fixture / "known_hosts"
        known_hosts.write_text(
            "39.104.204.172 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILBk7BgiUTvJZD/LSJLRHUw4eGFFe3eyUs3eG2ZU0xxj\n"
            "39.104.205.67 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILU4G7oGjBJ+tzaZh4nQJjGkPKM1gXQZYT3GWJLMDhZC\n",
            encoding="utf-8",
        )
        nonce_ledger = self.root.parent / f"oasis7-human-direct-nonce-{len(self.external_fixture_paths)}.jsonl"
        nonce_ledger.write_text("", encoding="utf-8")
        nonce_ledger.chmod(0o600)
        self.external_fixture_paths.append(nonce_ledger)
        authority = fixture / "stop-authority.json"
        nonce = "human-direct-ssh-nonce-3481"
        issued_at = dt.datetime.now(dt.timezone.utc).replace(microsecond=0)
        issued_at_text = issued_at.isoformat().replace("+00:00", "Z")
        expires_at_text = (issued_at + dt.timedelta(minutes=15)).isoformat().replace("+00:00", "Z")
        authority_payload = {
            "schema_version": "oasis7.human_stop_authority.v1",
            "task_uid": "task_0aa7659e0fd74e1a8e4bb27d7dc2416a",
            "transaction_id": transaction_id,
            "nonce": nonce,
            "authorized": True,
            "stopped_at": issued_at_text,
            "actor": "human-operator-fixture",
            "source": "live_github_task_readback",
            "authenticated": True,
            "live_readback": True,
            "repository": "eng-cc/oasis7",
            "issue_number": 3481,
            "action": "validator_pair_rebuild_stop",
            "targets": ["storage-205", "sequencer-204"],
        }
        authority.write_text(json.dumps(authority_payload, sort_keys=True) + "\n", encoding="utf-8")
        request = {
            "schema_version": "oasis7.human_direct_ssh_request.v1",
            "mode": "human_direct_ssh",
            "task_uid": authority_payload["task_uid"],
            "transaction_id": transaction_id,
            "request_digest": "",
            "nonce": nonce,
            "issued_at": issued_at_text,
            "expires_at": expires_at_text,
            "stop_authority_path": str(authority),
            "stop_authority_sha256": sha256(authority),
            "inventory_path": str(ROOT / "scripts" / "public-testnet-validator-pair-inventory.v1.json"),
            "inventory_sha256": sha256(ROOT / "scripts" / "public-testnet-validator-pair-inventory.v1.json"),
            "nonce_ledger_path": str(nonce_ledger),
            "known_hosts_path": str(known_hosts),
            "observer_mutation": False,
            "quiescence_id": transaction_id,
            "impact_record_path": str(impact_path) if impact_path is not None else None,
            "impact_record_sha256": sha256(impact_path) if impact_path is not None else None,
            "hosts": {
                "sequencer": {
                    "role": "sequencer",
                    "host": "root@39.104.204.172",
                    "root": "/opt/oasis7/p2p-testnet",
                    "service": "oasis7-triad-sequencer.service",
                    "host_key_fingerprint": "SHA256:7NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
                },
                "storage": {
                    "role": "storage",
                    "host": "root@39.104.205.67",
                    "root": "/opt/oasis7/p2p-testnet",
                    "service": "oasis7-triad-storage.service",
                    "host_key_fingerprint": "SHA256:1SVgiaT5JLCw8PsPpVfLE9UyWNf82IJDZsiE7LAa1gI",
                },
            },
            "readback": {
                "process_command": "ps -eo pid=,args=",
                "listener_command": "ss -ltn",
                "service_readback_command": "service-readback --read-only",
                "quiet_window_seconds": 2,
            },
        }
        if stale:
            request["issued_at"] = "2000-01-01T00:00:00Z"
            request["expires_at"] = "2000-01-01T00:01:00Z"
        if expired:
            request["expires_at"] = "2000-01-01T00:01:00Z"
        if replayed:
            request["nonce"] = "human-direct-ssh-replayed-nonce"
            request["replay_of_transaction_id"] = "human-direct-ssh-tx-3479"
        if mismatched == "transaction":
            request["transaction_id"] = "human-direct-ssh-tx-other"
        elif mismatched == "request":
            request["request_digest"] = "f" * 64
        elif mismatched == "fingerprint":
            request["hosts"]["sequencer"]["host_key_fingerprint"] = "SHA256:not-the-pinned-key"
        elif mismatched == "role":
            request["hosts"]["sequencer"]["role"] = "root"
        elif mismatched == "root":
            request["hosts"]["sequencer"]["root"] = "/tmp/caller-owned-root"
        elif mismatched == "service":
            request["hosts"]["sequencer"]["service"] = "caller-owned.service"
        elif mismatched == "command":
            request["readback"]["process_command"] = "echo caller-owned-command"
        elif mismatched == "observer":
            request["observer_mutation"] = True
        if full_status:
            request["full_status_url"] = "http://sequencer.example/v1/chain/status"
            request["full_status_http_status"] = 204
        body_value = {
            "schema_version": "oasis7.validator_pair_rebuild_live_authority.v1",
            "task_uid": authority_payload["task_uid"],
            "action": "validator_pair_rebuild",
            "transaction_id": authority_payload["transaction_id"],
            "nonce": authority_payload["nonce"],
            "inventory_sha256": request["inventory_sha256"],
            "head_oid": FROZEN_HEAD_OID,
            "issued_at": issued_at_text,
            "expires_at": expires_at_text,
        }
        body = json.dumps(body_value, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
        request["github_live"] = {
            "provider": "github",
            "repository": "eng-cc/oasis7",
            "issue_number": 3481,
            "comment_id": 5452142815,
            "actor": "human-operator-fixture",
            "body_sha256": hashlib.sha256(body.encode()).hexdigest(),
            "task_uid": request["task_uid"],
            "action": "validator_pair_rebuild",
            "transaction_id": request["transaction_id"],
            "nonce": request["nonce"],
            "inventory_sha256": request["inventory_sha256"],
            "head_oid": body_value["head_oid"],
            "issued_at": request["issued_at"],
            "expires_at": request["expires_at"],
        }
        if not request["request_digest"]:
            request["request_digest"] = hashlib.sha256(
                json.dumps(
                    {key: value for key, value in request.items() if key != "request_digest"},
                    ensure_ascii=True,
                    sort_keys=True,
                    separators=(",", ":"),
                ).encode()
            ).hexdigest()
        request_path = fixture / "request.json"
        request_path.write_text(json.dumps(request, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        out_dir = fixture / "out"
        out_dir.mkdir()
        ssh_args_log = fixture / "ssh-argv.jsonl"
        ssh_command_log = fixture / "ssh-command.log"
        ssh_seam_log = fixture / "ssh-seam.jsonl"
        sshpass_log = fixture / "sshpass-argv.jsonl"
        systemctl_log = fixture / "systemctl.log"
        service_log = fixture / "service-readback.log"
        receipt_path = out_dir / "receipt.json"
        fake_ssh = bin_dir / "ssh"
        fake_ssh.write_text(
            """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

args = sys.argv[1:]
secret = os.environ.get("HUMAN_DIRECT_SSH_SECRET", "")
argv_log = Path(os.environ["HUMAN_DIRECT_SSH_ARGV_LOG"])
command_log = Path(os.environ["HUMAN_DIRECT_SSH_COMMAND_LOG"])
seam_log = Path(os.environ["HUMAN_DIRECT_SSH_SEAM_LOG"])
with argv_log.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(args, sort_keys=True) + "\\n")
command = args[-1] if args else ""
with command_log.open("a", encoding="utf-8") as handle:
    handle.write(command + "\\n")
with seam_log.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({
        "environment_binding_present": bool(secret),
        "temporary_fd_present": bool(os.environ.get("HUMAN_DIRECT_SSH_SECRET_FD")),
    }) + "\\n")
if secret and any(secret in value for value in args):
    print("fixture rejected secret in ssh argv", file=sys.stderr)
    raise SystemExit(97)
state = os.environ.get("HUMAN_DIRECT_SSH_FAKE_STATE", "quiet")
count = len(command_log.read_text(encoding="utf-8").splitlines())
service_active = state == "active"
process_active = state in ("active", "active-process")
listener_active = state in ("active", "active-listener")
if "ps -eo" in command or "pgrep" in command:
    print("1234 oasis7_chain_runtime --active" if process_active else (f"quiet-observation-{count}" if state == "flapping" else ""))
elif "ss -ltn" in command or "lsof" in command:
    print("LISTEN 0 128 0.0.0.0:6631" if listener_active else "")
else:
    print(json.dumps({
        "schema_version": "oasis7.human_direct_ssh_readback.v1",
        "active": service_active,
        "running": service_active,
        "listeners": ["6631"] if service_active else [],
        "service_state": "active" if service_active else "stopped",
        "independently_observed": True,
    }, sort_keys=True))
""",
            encoding="utf-8",
        )
        fake_ssh.chmod(0o755)
        fake_sshpass = bin_dir / "sshpass"
        fake_sshpass.write_text(
            """#!/usr/bin/env python3
import json
import os
import shutil
import sys
from pathlib import Path

with Path(os.environ["HUMAN_DIRECT_SSH_SSHPASS_LOG"]).open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({"argv": sys.argv[1:], "secret_env": bool(os.environ.get("SSHPASS"))}) + "\\n")
if len(sys.argv) < 2 or sys.argv[1] != "-e":
    raise SystemExit(96)
os.execv(shutil.which("ssh") or "ssh", sys.argv[2:])
""",
            encoding="utf-8",
        )
        fake_sshpass.chmod(0o755)
        fake_systemctl = bin_dir / "systemctl"
        fake_systemctl.write_text(
            """#!/usr/bin/env python3
import os
import sys
from pathlib import Path

with Path(os.environ["HUMAN_DIRECT_SSH_SYSTEMCTL_LOG"]).open("a", encoding="utf-8") as handle:
    handle.write(" ".join(sys.argv[1:]) + "\\n")
raise SystemExit(97)
""",
            encoding="utf-8",
        )
        fake_systemctl.chmod(0o755)
        fake_service = bin_dir / "service-readback"
        fake_service.write_text(
            """#!/usr/bin/env python3
import json
import os
from pathlib import Path

Path(os.environ["HUMAN_DIRECT_SSH_SERVICE_LOG"]).open("a", encoding="utf-8").close()
print(json.dumps({"read_only": True, "state": os.environ.get("HUMAN_DIRECT_SSH_FAKE_STATE", "quiet")}))
""",
            encoding="utf-8",
        )
        fake_service.chmod(0o755)
        fake_curl = bin_dir / "curl"
        fake_curl.write_text(
            """#!/usr/bin/env python3
import sys
print("curl is forbidden in human_direct_ssh fixture", file=sys.stderr)
raise SystemExit(98)
""",
            encoding="utf-8",
        )
        fake_curl.chmod(0o755)
        github_response = fixture / "github-live-comment.json"
        github_response.write_text(
            json.dumps(
                {
                    "id": 5452142815,
                    "user": {"login": "human-operator-fixture"},
                    "body": body,
                    "created_at": issued_at_text,
                    "updated_at": issued_at_text,
                    "deleted": False,
                    "revoked": False,
                },
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        github_calls = fixture / "github-api-argv.jsonl"
        fake_gh = bin_dir / "gh"
        fake_gh.write_text(
            """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

with Path(os.environ["HUMAN_DIRECT_SSH_GITHUB_LOG"]).open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(sys.argv[1:], sort_keys=True) + "\\n")
if os.environ.get("HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE") == "1":
    print("fake GitHub API unavailable", file=sys.stderr)
    raise SystemExit(73)
print(Path(os.environ["HUMAN_DIRECT_SSH_GITHUB_RESPONSE"]).read_text(encoding="utf-8"), end="")
""",
            encoding="utf-8",
        )
        fake_gh.chmod(0o755)
        return {
            "fixture": fixture,
            "bin": bin_dir,
            "known_hosts": known_hosts,
            "nonce_ledger": nonce_ledger,
            "request": request_path,
            "out": out_dir,
            "ssh_args": ssh_args_log,
            "ssh_commands": ssh_command_log,
            "ssh_seam": ssh_seam_log,
            "sshpass": sshpass_log,
            "systemctl": systemctl_log,
            "service": service_log,
            "github_response": github_response,
            "github_calls": github_calls,
            "receipt": receipt_path,
            "secret": "human-direct-ssh-fixture-secret-3481",
            "state": state,
        }

    def _human_direct_ssh_env(self, fixture: dict[str, Path | str]) -> dict[str, str]:
        env = os.environ.copy()
        env["PATH"] = f"{fixture['bin']}:{env.get('PATH', '')}"
        env["HUMAN_DIRECT_SSH_ARGV_LOG"] = str(fixture["ssh_args"])
        env["HUMAN_DIRECT_SSH_COMMAND_LOG"] = str(fixture["ssh_commands"])
        env["HUMAN_DIRECT_SSH_SEAM_LOG"] = str(fixture["ssh_seam"])
        env["HUMAN_DIRECT_SSH_SSHPASS_LOG"] = str(fixture["sshpass"])
        env["HUMAN_DIRECT_SSH_SYSTEMCTL_LOG"] = str(fixture["systemctl"])
        env["HUMAN_DIRECT_SSH_SERVICE_LOG"] = str(fixture["service"])
        env["HUMAN_DIRECT_SSH_FAKE_STATE"] = str(fixture["state"])
        env["HUMAN_DIRECT_SSH_SECRET"] = str(fixture["secret"])
        env["OASIS7_VALIDATOR_PAIR_NONCE_LEDGER"] = str(fixture["nonce_ledger"])
        env["HUMAN_DIRECT_SSH_GITHUB_RESPONSE"] = str(fixture["github_response"])
        env["HUMAN_DIRECT_SSH_GITHUB_LOG"] = str(fixture["github_calls"])
        return env

    def _human_direct_ssh_args(self, fixture: dict[str, Path | str], *extra: str) -> list[str]:
        return [
            str(WRAPPER),
            "human_direct_ssh",
            "--request",
            str(fixture["request"]),
            "--known-hosts",
            str(fixture["known_hosts"]),
            "--out-dir",
            str(fixture["out"]),
            "--credential-env",
            "HUMAN_DIRECT_SSH_SECRET",
            *extra,
        ]

    def _run_human_direct_ssh(self, fixture: dict[str, Path | str], *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            self._human_direct_ssh_args(fixture, *extra),
            text=True,
            capture_output=True,
            env=self._human_direct_ssh_env(fixture),
        )

    def _write_live_github_fixture(
        self,
        *,
        transaction_id: str = "human-direct-ssh-tx-3481",
        impact_path: Path | None = None,
    ) -> dict[str, Path | str]:
        """Add a local fake-GitHub authority response to the fake-SSH fixture.

        The executor must obtain the authority from the live comment surface,
        not from the request's persisted stop-authority JSON.  The fake
        ``gh`` command is deliberately installed ahead of the real PATH and
        has no network implementation; an implementation which silently
        reaches the real GitHub CLI would therefore fail this fixture.
        """
        fixture = self._write_human_direct_ssh_fixture(
            transaction_id=transaction_id,
            impact_path=impact_path,
        )
        request_path = Path(fixture["request"])
        request = json.loads(request_path.read_text(encoding="utf-8"))
        request["readback"]["quiet_window_seconds"] = 0.1
        request["inventory"] = {
            "schema_version": "oasis7.public_testnet_validator_inventory.v1",
            "nodes": [
                {
                    "node_id": "triad-testnet-sequencer",
                    "role": "sequencer-204",
                    "host": "root@39.104.204.172",
                    "root": "/opt/oasis7/p2p-testnet",
                    "service": "oasis7-triad-sequencer.service",
                    "ports": ["6631", "6831"],
                    "host_key_fingerprint": "SHA256:39-sequencer-pin",
                },
                {
                    "node_id": "triad-testnet-storage",
                    "role": "storage-205",
                    "host": "root@39.104.205.67",
                    "root": "/opt/oasis7/p2p-testnet",
                    "service": "oasis7-triad-storage.service",
                    "ports": ["6632", "6832"],
                    "host_key_fingerprint": "SHA256:39-storage-pin",
                },
            ],
        }
        inventory_sha256 = hashlib.sha256(
            json.dumps(request["inventory"], ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        body_value = {
            "schema_version": "oasis7.validator_pair_rebuild_live_authority.v1",
            "task_uid": request["task_uid"],
            "action": "validator_pair_rebuild",
            "transaction_id": request["transaction_id"],
            "nonce": request["nonce"],
            "inventory_sha256": inventory_sha256,
            "head_oid": FROZEN_HEAD_OID,
            "issued_at": request["issued_at"],
            "expires_at": request["expires_at"],
        }
        body = json.dumps(body_value, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
        response = {
            "id": 5452142815,
            "user": {"login": "release-authority-fixture"},
            "body": body,
            "created_at": request["issued_at"],
            "updated_at": request["issued_at"],
            "deleted": False,
            "revoked": False,
        }
        request["github_live"] = {
            "provider": "github",
            "repository": "eng-cc/oasis7",
            "issue_number": 3481,
            "comment_id": response["id"],
            "actor": response["user"]["login"],
            "body_sha256": hashlib.sha256(body.encode()).hexdigest(),
            "task_uid": body_value["task_uid"],
            "action": body_value["action"],
            "transaction_id": body_value["transaction_id"],
            "nonce": body_value["nonce"],
            "inventory_sha256": inventory_sha256,
            "head_oid": body_value["head_oid"],
            "issued_at": body_value["issued_at"],
            "expires_at": body_value["expires_at"],
        }
        request["request_digest"] = hashlib.sha256(
            json.dumps(
                {key: value for key, value in request.items() if key != "request_digest"},
                ensure_ascii=True,
                sort_keys=True,
                separators=(",", ":"),
            ).encode()
        ).hexdigest()
        request_path.write_text(json.dumps(request, indent=2, sort_keys=True) + "\n", encoding="utf-8")

        response_path = Path(fixture["fixture"]) / "github-live-comment.json"
        response_path.write_text(json.dumps(response, sort_keys=True) + "\n", encoding="utf-8")
        github_calls = Path(fixture["fixture"]) / "github-api-argv.jsonl"
        fake_gh = Path(fixture["bin"]) / "gh"
        fake_gh.write_text(
            """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

with Path(os.environ["HUMAN_DIRECT_SSH_GITHUB_LOG"]).open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(sys.argv[1:], sort_keys=True) + "\\n")
if os.environ.get("HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE") == "1":
    print("fake GitHub API unavailable", file=sys.stderr)
    raise SystemExit(73)
print(Path(os.environ["HUMAN_DIRECT_SSH_GITHUB_RESPONSE"]).read_text(encoding="utf-8"), end="")
""",
            encoding="utf-8",
        )
        fake_gh.chmod(0o755)
        fixture["github_response"] = response_path
        fixture["github_calls"] = github_calls
        return fixture

    def _live_github_env(
        self,
        fixture: dict[str, Path | str],
        *,
        unavailable: bool = False,
    ) -> dict[str, str]:
        env = self._human_direct_ssh_env(fixture)
        env["HUMAN_DIRECT_SSH_GITHUB_RESPONSE"] = str(fixture["github_response"])
        env["HUMAN_DIRECT_SSH_GITHUB_LOG"] = str(fixture["github_calls"])
        if unavailable:
            env["HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE"] = "1"
        else:
            env.pop("HUMAN_DIRECT_SSH_GITHUB_UNAVAILABLE", None)
        return env

    def _run_live_human_direct_ssh(
        self,
        fixture: dict[str, Path | str],
        *,
        unavailable: bool = False,
        extra: tuple[str, ...] = (),
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            self._human_direct_ssh_args(fixture, *extra),
            text=True,
            capture_output=True,
            env=self._live_github_env(fixture, unavailable=unavailable),
        )

    def _rewrite_request(self, fixture: dict[str, Path | str], mutate: object) -> dict[str, Any]:
        request_path = Path(fixture["request"])
        request = json.loads(request_path.read_text(encoding="utf-8"))
        mutate(request)
        request["request_digest"] = hashlib.sha256(
            json.dumps(
                {key: value for key, value in request.items() if key != "request_digest"},
                ensure_ascii=True,
                sort_keys=True,
                separators=(",", ":"),
            ).encode()
        ).hexdigest()
        request_path.write_text(json.dumps(request, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return request

    def _rewrite_github_response(self, fixture: dict[str, Path | str], mutate: object) -> dict[str, Any]:
        response_path = Path(fixture["github_response"])
        response = json.loads(response_path.read_text(encoding="utf-8"))
        mutate(response)
        response_path.write_text(json.dumps(response, sort_keys=True) + "\n", encoding="utf-8")
        return response

    def _configure_exact_operator_inventory(self, fixture: dict[str, Path | str]) -> None:
        known_hosts = Path(fixture["known_hosts"])
        known_hosts.write_text(
            "39.104.204.172 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILBk7BgiUTvJZD/LSJLRHUw4eGFFe3eyUs3eG2ZU0xxj\n"
            "39.104.205.67 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILU4G7oGjBJ+tzaZh4nQJjGkPKM1gXQZYT3GWJLMDhZC\n",
            encoding="utf-8",
        )

        def mutate(request: dict[str, Any]) -> None:
            request["hosts"]["sequencer"].update(
                {
                    "host": "root@39.104.204.172",
                    "root": "/opt/oasis7/p2p-testnet",
                    "host_key_fingerprint": "SHA256:7NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU",
                }
            )
            request["hosts"]["storage"].update(
                {
                    "host": "root@39.104.205.67",
                    "root": "/opt/oasis7/p2p-testnet",
                    "host_key_fingerprint": "SHA256:1SVgiaT5JLCw8PsPpVfLE9UyWNf82IJDZsiE7LAa1gI",
                }
            )

        request = self._rewrite_request(fixture, mutate)
        body = json.loads(json.loads(Path(fixture["github_response"]).read_text(encoding="utf-8"))["body"])
        body["inventory_sha256"] = hashlib.sha256(
            json.dumps(request["inventory"], ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        response = json.loads(Path(fixture["github_response"]).read_text(encoding="utf-8"))
        response["body"] = json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":"))
        Path(fixture["github_response"]).write_text(json.dumps(response, sort_keys=True) + "\n", encoding="utf-8")
        request["github_live"]["inventory_sha256"] = body["inventory_sha256"]
        request["github_live"]["body_sha256"] = hashlib.sha256(response["body"].encode()).hexdigest()
        request_path = Path(fixture["request"])
        request["request_digest"] = hashlib.sha256(
            json.dumps(
                {key: value for key, value in request.items() if key != "request_digest"},
                ensure_ascii=True,
                sort_keys=True,
                separators=(",", ":"),
            ).encode()
        ).hexdigest()
        request_path.write_text(json.dumps(request, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def test_human_direct_ssh_accepts_only_canonical_read_only_request(self) -> None:
        fixture = self._write_human_direct_ssh_fixture()
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_rejects_arbitrary_adapter_input(self) -> None:
        fixture = self._write_human_direct_ssh_fixture()
        adapter = Path(fixture["fixture"]) / "caller-owned-adapter.py"
        adapter.write_text("#!/usr/bin/env python3\nraise SystemExit(0)\n", encoding="utf-8")
        adapter.chmod(0o755)
        result = self._run_human_direct_ssh(fixture, "--host-adapter", str(adapter))
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(adapter.*(forbidden|unsupported|not allowed)|arbitrary.*adapter)")
        self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_requires_pinned_host_key_and_strict_checking(self) -> None:
        fixture = self._write_human_direct_ssh_fixture()
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = [json.loads(line) for line in Path(fixture["ssh_args"]).read_text(encoding="utf-8").splitlines()]
        self.assertGreaterEqual(len(rows), 2)
        for args in rows:
            self.assertIn("StrictHostKeyChecking=yes", args)
            self.assertIn(f"UserKnownHostsFile={Path(fixture['known_hosts']).resolve()}", args)
            self.assertNotIn("StrictHostKeyChecking=no", args)
            self.assertNotIn("UserKnownHostsFile=/dev/null", args)

    def test_human_direct_ssh_binds_fixed_role_root_service_and_command_allowlist(self) -> None:
        fixture = self._write_human_direct_ssh_fixture()
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        commands = Path(fixture["ssh_commands"]).read_text(encoding="utf-8")
        self.assertIn("ps -eo pid=,args=", commands)
        self.assertIn("ss -ltn", commands)
        self.assertIn("sequencer", commands)
        self.assertIn("storage", commands)
        self.assertNotRegex(commands, r"(?i)(systemctl|reset|stage|start|stop|rm\s+-rf|mkdir)")

    def test_human_direct_ssh_rejects_non_allowlisted_role_root_service_or_command(self) -> None:
        for field in ("role", "root", "service", "command"):
            with self.subTest(field=field):
                fixture = self._write_human_direct_ssh_fixture(mismatched=field)
                result = self._run_human_direct_ssh(fixture)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(result.stderr, rf"(?i)({field}|allow.?list|fixed|canonical|binding)")

    def test_human_direct_ssh_rejects_observer_mutation_true(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(mismatched="observer")
        result = self._run_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(observer|mutation|read.?only|quiescen)")

    def test_human_direct_ssh_keeps_secret_out_of_argv_logs_and_receipt(self) -> None:
        fixture = self._write_human_direct_ssh_fixture()
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        secret = str(fixture["secret"])
        for path in (fixture["ssh_args"], fixture["ssh_commands"], fixture["sshpass"], fixture["receipt"]):
            if Path(path).exists():
                self.assertNotIn(secret, Path(path).read_text(encoding="utf-8"))
        self.assertNotIn(secret, result.stdout)
        self.assertNotIn(secret, result.stderr)
        seam_rows = [json.loads(line) for line in Path(fixture["ssh_seam"]).read_text(encoding="utf-8").splitlines()]
        self.assertTrue(any(row["environment_binding_present"] or row["temporary_fd_present"] for row in seam_rows))
        self.assertNotRegex(
            Path(fixture["ssh_args"]).read_text(encoding="utf-8") if Path(fixture["ssh_args"]).exists() else "",
            r"(?i)(--password|--secret|secret=|password=)",
        )

    def test_human_direct_ssh_rejects_stale_or_replayed_transaction_request_fingerprint_nonce(self) -> None:
        cases = (
            ({"stale": True}, r"(?i)(stale|expired|timestamp|nonce)"),
            ({"expired": True}, r"(?i)(expired|expiry|stale|timestamp)"),
            ({"replayed": True}, r"(?i)(replay|nonce|transaction)"),
            ({"mismatched": "transaction"}, r"(?i)(transaction|binding)"),
            ({"mismatched": "request"}, r"(?i)(request.*digest|binding|digest)"),
            ({"mismatched": "fingerprint"}, r"(?i)(fingerprint|host.?key|binding)"),
        )
        for options, pattern in cases:
            with self.subTest(options=options):
                fixture = self._write_human_direct_ssh_fixture(**options)
                result = self._run_human_direct_ssh(fixture)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(result.stderr, pattern)

    def test_human_direct_ssh_rejects_active_process_or_listener(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(state="active")
        result = self._run_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(active|listener|quiescen|running)")

    def test_human_direct_ssh_rejects_active_process_readback(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(state="active-process")
        result = self._run_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(active.*process|running|quiescen)")

    def test_human_direct_ssh_rejects_active_listener_readback(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(state="active-listener")
        result = self._run_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(active.*listener|listener|quiescen)")

    def test_human_direct_ssh_requires_stable_quiet_window(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(state="flapping")
        result = self._run_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(quiet|quiescen|stable|window|changed)")

    def test_human_direct_ssh_quiesce_never_uses_systemctl_or_mutation_commands(self) -> None:
        fixture = self._write_human_direct_ssh_fixture()
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(Path(fixture["systemctl"]).read_text(encoding="utf-8"), "")
        commands = Path(fixture["ssh_commands"]).read_text(encoding="utf-8")
        self.assertNotRegex(commands, r"(?i)(systemctl|reset|stage|start|stop|kill|rm\s+-rf|mkdir|cp\s)")

    def test_human_direct_ssh_forbids_full_204_chain_status(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(full_status=True)
        result = self._run_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(204|full.*status|/v1/chain/status|status.*forbidden)")

    def test_human_direct_ssh_rejects_caller_or_persisted_authority_without_live_github_comment(self) -> None:
        for source in ("caller", "persisted"):
            with self.subTest(source=source):
                fixture = self._write_live_github_fixture()

                def mutate(request: dict[str, Any]) -> None:
                    request.pop("github_live", None)
                    authority = {
                        "authorized": True,
                        "schema_version": "oasis7.human_stop_authority.v1",
                        "task_uid": request["task_uid"],
                        "transaction_id": request["transaction_id"],
                        "nonce": request["nonce"],
                        "stopped_at": request["issued_at"],
                    }
                    if source == "caller":
                        request["caller_authority"] = authority
                    else:
                        persisted = Path(fixture["fixture"]) / "persisted-authority.json"
                        persisted.write_text(json.dumps(authority) + "\n", encoding="utf-8")
                        request["persisted_authority_path"] = str(persisted)

                self._rewrite_request(fixture, mutate)
                result = self._run_live_human_direct_ssh(fixture)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(result.stderr, r"(?i)(live|github|authority|comment|evidence|capability)")
                self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_requires_live_github_binding_for_actor_comment_body_task_action_transaction_nonce_inventory_head_and_freshness(self) -> None:
        def body_mutator(field: str, response: dict[str, Any]) -> None:
            if field == "actor":
                response["user"]["login"] = "untrusted-actor"
            elif field == "comment_id":
                response["id"] = 5452142816
            else:
                body = json.loads(response["body"])
                if field == "body_sha256":
                    body["action"] = "caller-authored-action"
                elif field == "task_uid":
                    body[field] = "foreign-task-3481"
                elif field == "action":
                    body[field] = "reset_and_start"
                elif field == "transaction_id":
                    body[field] = "foreign-transaction-3481"
                elif field == "nonce":
                    body[field] = "foreign-nonce-3481"
                elif field == "inventory_sha256":
                    body[field] = "f" * 64
                elif field == "head_oid":
                    body[field] = "e" * 40
                elif field == "freshness":
                    body["issued_at"] = "2000-01-01T00:00:00Z"
                    body["expires_at"] = "2000-01-01T00:01:00Z"
                response["body"] = json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":"))

        for field in (
            "actor",
            "comment_id",
            "body_sha256",
            "task_uid",
            "action",
            "transaction_id",
            "nonce",
            "inventory_sha256",
            "head_oid",
            "freshness",
        ):
            with self.subTest(binding=field):
                fixture = self._write_live_github_fixture()
                self._rewrite_github_response(fixture, lambda response, field=field: body_mutator(field, response))
                if field == "freshness":
                    response = json.loads(Path(fixture["github_response"]).read_text(encoding="utf-8"))
                    body_sha256 = hashlib.sha256(response["body"].encode()).hexdigest()

                    def bind_stale_body(request: dict[str, Any]) -> None:
                        request["github_live"]["body_sha256"] = body_sha256

                    self._rewrite_request(fixture, bind_stale_body)
                result = self._run_live_human_direct_ssh(fixture)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(
                    result.stderr,
                    r"(?i)(live|github|authority|actor|comment|digest|task|action|transaction|nonce|inventory|head|fresh|expired|binding)",
                )
                self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_rejects_authority_head_not_matching_executor_frozen_head(self) -> None:
        fixture = self._write_live_github_fixture()

        def mutate_response(response: dict[str, Any]) -> None:
            body = json.loads(response["body"])
            body["head_oid"] = "e" * 40
            response["body"] = json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":"))

        response = self._rewrite_github_response(fixture, mutate_response)
        authority_head = json.loads(response["body"])["head_oid"]
        authority_body_sha256 = hashlib.sha256(response["body"].encode()).hexdigest()

        def bind_rewritten_authority(request: dict[str, Any]) -> None:
            request["github_live"]["head_oid"] = authority_head
            request["github_live"]["body_sha256"] = authority_body_sha256

        self._rewrite_request(fixture, bind_rewritten_authority)
        result = self._run_live_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(frozen|executor|head|commit|binding)")
        self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_fails_closed_for_edited_deleted_revoked_or_unavailable_github_authority(self) -> None:
        for state in ("edited", "deleted", "revoked", "api_unavailable"):
            with self.subTest(state=state):
                fixture = self._write_live_github_fixture()
                if state == "api_unavailable":
                    result = self._run_live_human_direct_ssh(fixture, unavailable=True)
                else:
                    def mutate(response: dict[str, Any], state=state) -> None:
                        response[state] = True
                        if state == "edited":
                            response["updated_at"] = "2000-01-01T00:00:01Z"

                    self._rewrite_github_response(fixture, mutate)
                    result = self._run_live_human_direct_ssh(fixture)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(result.stderr, r"(?i)(live|github|comment|edited|deleted|revoked|unavailable|api|authority)")
                self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_rejects_caller_host_known_hosts_root_and_service_overrides_with_live_authority(self) -> None:
        for field in ("host", "known_hosts", "root", "service"):
            with self.subTest(override=field):
                fixture = self._write_live_github_fixture()

                def mutate(request: dict[str, Any]) -> None:
                    if field == "host":
                        request["hosts"]["sequencer"].update(
                            {
                                "host": "root@storage.example",
                                "host_key_fingerprint": "SHA256:storage-fixture-pin",
                            }
                        )
                    elif field == "known_hosts":
                        alternate = Path(fixture["fixture"]) / "caller-known_hosts"
                        alternate.write_text(Path(fixture["known_hosts"]).read_text(encoding="utf-8"), encoding="utf-8")
                        request["known_hosts_path"] = str(alternate)
                    elif field == "root":
                        request["hosts"]["sequencer"]["root"] = "/opt/caller-owned-root"
                    else:
                        request["hosts"]["sequencer"]["service"] = "caller-owned.service"

                self._rewrite_request(fixture, mutate)
                result = self._run_live_human_direct_ssh(fixture)
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(result.stderr, rf"(?i)({field}|host|known.?hosts|root|service|allow.?list|binding|github|authority)")
                self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_accepts_only_exact_repo_inventory_for_204_and_205_opt_roots_and_pins(self) -> None:
        fixture = self._write_live_github_fixture()
        self._configure_exact_operator_inventory(fixture)
        result = self._run_live_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(Path(fixture["receipt"]).read_text(encoding="utf-8"))
        self.assertEqual(receipt["hosts"]["sequencer-204"]["host"], "root@39.104.204.172")
        self.assertEqual(receipt["hosts"]["storage-205"]["host"], "root@39.104.205.67")
        self.assertEqual(receipt["hosts"]["sequencer-204"]["root"], "/opt/oasis7/p2p-testnet")
        self.assertEqual(receipt["hosts"]["storage-205"]["root"], "/opt/oasis7/p2p-testnet")
        self.assertEqual(receipt["host_fingerprints"]["sequencer-204"], "SHA256:7NkC2GehDCcN+IWPbaxh+0JuIVGCEtKpdK69S6fHZPU")
        self.assertEqual(receipt["host_fingerprints"]["storage-205"], "SHA256:1SVgiaT5JLCw8PsPpVfLE9UyWNf82IJDZsiE7LAa1gI")

    def test_human_direct_ssh_rejects_live_inventory_digest_not_equal_to_repo_inventory(self) -> None:
        fixture = self._write_live_github_fixture()

        def mutate(response: dict[str, Any]) -> None:
            body = json.loads(response["body"])
            body["inventory_sha256"] = "a" * 64
            response["body"] = json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":"))

        self._rewrite_github_response(fixture, mutate)
        response = json.loads(Path(fixture["github_response"]).read_text(encoding="utf-8"))
        body = json.loads(response["body"])
        self._rewrite_request(
            fixture,
            lambda request: request["github_live"].update(
                {
                    "inventory_sha256": body["inventory_sha256"],
                    "body_sha256": hashlib.sha256(response["body"].encode()).hexdigest(),
                }
            ),
        )
        result = self._run_live_human_direct_ssh(fixture)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(inventory|repository|canonical|binding|github|authority)")
        self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_rewrite_of_receipt_or_proof_cannot_authorize_plan_admission(self) -> None:
        fixture = self._write_live_github_fixture(
            transaction_id=self.quiescence_id,
            impact_path=self.impact,
        )
        direct = self._run_live_human_direct_ssh(fixture)
        self.assertEqual(direct.returncode, 0, direct.stderr)
        request = json.loads(Path(fixture["request"]).read_text(encoding="utf-8"))
        fake_authority = request["github_live"]
        for name in ("receipt.json", "quiescence-adapter-receipt.json"):
            path = Path(fixture["out"]) / name
            value = json.loads(path.read_text(encoding="utf-8"))
            value["github_live"] = fake_authority
            value["canonical_digest"] = canonical_digest(value)
            path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
        proof_path = Path(fixture["out"]) / "quiescence-proof.json"
        proof = json.loads(proof_path.read_text(encoding="utf-8"))
        source_path = Path(fixture["out"]) / "quiescence-adapter-receipt.json"
        proof["github_live"] = fake_authority
        proof["source_receipt_sha256"] = sha256(source_path)
        proof["canonical_digest"] = canonical_digest(proof)
        proof_path.write_text(json.dumps(proof, sort_keys=True) + "\n", encoding="utf-8")
        self._rewrite_github_response(fixture, lambda response: response.update({"revoked": True}))
        self.quiescence_proof = proof_path
        result = subprocess.run(
            self._base_args(),
            text=True,
            capture_output=True,
            env=self._live_github_env(fixture),
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(live|github|revoked|authority|receipt|proof|admission|evidence)")
        self.assertFalse((self.out / "transaction.json").exists())

    def test_apply_repeats_live_github_authority_and_fake_ssh_before_any_mutation(self) -> None:
        fixture = self._write_live_github_fixture(
            transaction_id=self.quiescence_id,
            impact_path=self.impact,
        )
        audit = {
            "audit_only": True,
            "direct_request_path": str(fixture["request"]),
            "direct_request_sha256": sha256(Path(fixture["request"])),
            "known_hosts_path": str(fixture["known_hosts"]),
            "credential_binding": {
                "kind": "temporary-fd-or-environment",
                "environment_name": "HUMAN_DIRECT_SSH_SECRET",
            },
        }
        audit_path = self.root / "executor-routing-audit.json"
        audit["canonical_digest"] = canonical_digest(audit)
        audit_path.write_text(json.dumps(audit, sort_keys=True) + "\n", encoding="utf-8")
        self.quiescence_proof = audit_path
        planned = subprocess.run(
            self._base_args(),
            text=True,
            capture_output=True,
            env=self._live_github_env(fixture),
        )
        self.assertEqual(planned.returncode, 0, planned.stderr)
        transaction_path = self.out / "transaction.json"
        transaction = json.loads(transaction_path.read_text(encoding="utf-8"))
        request = json.loads(Path(fixture["request"]).read_text(encoding="utf-8"))
        transaction.update(
            {
                "github_live": request["github_live"],
                "live_ssh_request_path": str(fixture["request"]),
                "live_ssh_known_hosts_path": str(fixture["known_hosts"]),
            }
        )
        transaction["plan_digest"] = hashlib.sha256(
            json.dumps(
                {key: value for key, value in transaction.items() if key != "plan_digest"},
                ensure_ascii=True,
                sort_keys=True,
                separators=(",", ":"),
            ).encode()
        ).hexdigest()
        transaction_path.write_text(json.dumps(transaction, sort_keys=True) + "\n", encoding="utf-8")
        self._rewrite_github_response(fixture, lambda response: response.update({"revoked": True}))
        adapter = self._write_host_adapter()
        before = {
            role: (self.nodes[role] / "current" / "bin" / "oasis7_chain_runtime").read_bytes()
            for role in self.nodes
        }
        ssh_before = Path(fixture["ssh_commands"]).read_text(encoding="utf-8")
        result = subprocess.run(
            [
                str(WRAPPER),
                "apply",
                "--transaction",
                str(transaction_path),
                "--host-adapter",
                str(adapter),
                "--request",
                str(fixture["request"]),
                "--known-hosts",
                str(fixture["known_hosts"]),
            ],
            text=True,
            capture_output=True,
            env=self._live_github_env(fixture),
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr, r"(?i)(live|github|revoked|authority|ssh|before.*mutation|admission)")
        for role in self.nodes:
            self.assertEqual((self.nodes[role] / "current" / "bin" / "oasis7_chain_runtime").read_bytes(), before[role])
        self.assertEqual(Path(fixture["ssh_commands"]).read_text(encoding="utf-8"), ssh_before)

    def test_human_direct_ssh_rejects_pending_nonce_ledger_outside_output_and_worktree(self) -> None:
        fixture = self._write_live_github_fixture()
        with tempfile.TemporaryDirectory(prefix="oasis7-human-direct-nonce-") as ledger_dir:
            ledger = Path(ledger_dir) / "nonce-ledger.json"
            request = json.loads(Path(fixture["request"]).read_text(encoding="utf-8"))
            self._rewrite_request(fixture, lambda value: value.update({"nonce_ledger_path": str(ledger)}))
            request = json.loads(Path(fixture["request"]).read_text(encoding="utf-8"))
            ledger.write_text(
                json.dumps(
                    {
                        "schema_version": "oasis7.validator_pair_rebuild_nonce.v1",
                        "nonce": request["nonce"],
                        "task_uid": request["task_uid"],
                        "transaction_id": request["transaction_id"],
                        "request_digest": request["request_digest"],
                        "authority_sha256": request["github_live"]["body_sha256"],
                        "state": "pending",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            ledger.chmod(0o600)
            result = self._run_live_human_direct_ssh(fixture)
            self.assertNotEqual(result.returncode, 0)
            self.assertRegex(result.stderr, r"(?i)(nonce|pending|ledger|replay|authority|github)")
            self.assertFalse(Path(fixture["out"]) / "nonce-ledger.json" == ledger)
            self.assertFalse(Path(fixture["receipt"]).exists())

    def test_human_direct_ssh_rejects_replayed_nonce_from_external_ledger(self) -> None:
        fixture = self._write_live_github_fixture()
        first = self._run_live_human_direct_ssh(fixture)
        self.assertEqual(first.returncode, 0, first.stderr)
        replay_out = Path(fixture["fixture"]) / "replay-out"
        ssh_after_first = Path(fixture["ssh_commands"]).read_text(encoding="utf-8")
        replay = self._run_live_human_direct_ssh(fixture, extra=("--out-dir", str(replay_out)))
        self.assertNotEqual(replay.returncode, 0)
        self.assertRegex(replay.stderr, r"(?i)(nonce|replay|consumed|ledger|authority|github)")
        self.assertFalse((replay_out / "receipt.json").exists())
        self.assertEqual(Path(fixture["ssh_commands"]).read_text(encoding="utf-8"), ssh_after_first)


    def test_plan_requires_verified_provenance_and_emits_both_orders(self) -> None:
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["schema_version"], "oasis7.validator_pair_rebuild_plan.v1")
        self.assertRegex(receipt["plan_digest"], r"^[0-9a-f]{64}$")
        self.assertEqual(receipt["mutation_order"], ["storage-205", "sequencer-204"])
        self.assertEqual(receipt["startup_order"], ["sequencer-204", "storage-205"])
        self.assertEqual(receipt["phase"], "planned")

    def test_active_consumer_impact_approval_is_stop_authority_without_stopped_claim(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_active_impact_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")

        normalized = module.validate_impact(self.impact)

        self.assertEqual(normalized["decision"], "proceed")
        self.assertEqual(normalized["impact"], "active")
        self.assertFalse(normalized["validators_already_stopped"])

    def test_active_pair_plan_rejects_self_asserted_stopped_boolean_without_independent_proof(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": True,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        self._rewrite_request(
            self.human_direct_fixture,
            lambda request: request.update({"impact_record_sha256": sha256(self.impact)}),
        )
        proof = json.loads(self.quiescence_proof.read_text(encoding="utf-8"))
        proof["direct_request_sha256"] = sha256(Path(self.human_direct_fixture["request"]))
        proof["canonical_digest"] = canonical_digest(proof)
        self.quiescence_proof.write_text(json.dumps(proof, sort_keys=True) + "\n", encoding="utf-8")
        Path(self.human_direct_fixture["nonce_ledger"]).write_text("", encoding="utf-8")
        Path(self.human_direct_fixture["nonce_ledger"]).chmod(0o600)
        previous_state = os.environ.get("HUMAN_DIRECT_SSH_FAKE_STATE")
        os.environ["HUMAN_DIRECT_SSH_FAKE_STATE"] = "active"
        try:
            result = subprocess.run(self._base_args(), text=True, capture_output=True)
        finally:
            if previous_state is None:
                os.environ.pop("HUMAN_DIRECT_SSH_FAKE_STATE", None)
            else:
                os.environ["HUMAN_DIRECT_SSH_FAKE_STATE"] = previous_state

        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(
            result.stderr.lower(),
            r"independently verified.*(?:stopped|quiescence).*proof|stopped-state receipt|active|running|quiescence",
        )
        self.assertFalse(self.out.exists())

    def test_quiesce_emits_bounded_two_role_stopped_proof_without_mutation(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        fixture = self._write_human_direct_ssh_fixture(
            transaction_id="active-quiescence-3481",
            impact_path=self.impact,
        )
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(Path(fixture["receipt"]).read_text(encoding="utf-8"))
        self.assertEqual(receipt["phase"], "quiesce")
        self.assertEqual(set(receipt["nodes"]), {"storage-205", "sequencer-204"})
        self.assertTrue(all(item["active"] is False and item["running"] is False for item in receipt["nodes"].values()))
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())
        self.assertFalse((self.nodes["sequencer-204"] / "backups").exists())

    def test_quiesce_rejects_caller_attestable_fabricated_adapter_provenance(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        adapter = self._write_quiescence_adapter()
        source = adapter.read_text(encoding="utf-8")
        source = source.replace(
            "    'observer_mutation': False,\n",
            "    'observer_mutation': False,\n"
            "    'repository_executable': {\n"
            "        'schema_version': 'oasis7.validator_pair_rebuild_repository_executable.v1',\n"
            "        'path': 'caller-authored-adapter.py',\n"
            "        'sha256': '0' * 64,\n"
            "    },\n",
        )
        adapter.write_text(source, encoding="utf-8")
        out_dir = self.root / "fabricated-quiescence"
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "quiesce",
                "--consumer-impact-record",
                str(self.impact),
                "--quiescence-id",
                "fabricated-quiescence-3481",
                "--node",
                f"storage-205=local:{self.nodes['storage-205']}",
                "--node",
                f"sequencer-204=local:{self.nodes['sequencer-204']}",
                "--host-adapter",
                str(adapter),
                "--out-dir",
                str(out_dir),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"capability_blocked|trusted|attest|provenance|observe")
        self.assertFalse((out_dir / "quiescence-proof.json").exists())

    def test_quiesce_rejects_freshly_finalized_arbitrary_adapter_provenance(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        # This adapter is freshly written and chmod-finalized, so the current
        # mtime/ctime heuristic accepts it even though no governed producer
        # attestation or host observation exists.
        adapter = self._write_quiescence_adapter()
        out_dir = self.root / "fresh-arbitrary-quiescence"
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "quiesce",
                "--consumer-impact-record",
                str(self.impact),
                "--quiescence-id",
                "fresh-arbitrary-quiescence-3481",
                "--node",
                f"storage-205=local:{self.nodes['storage-205']}",
                "--node",
                f"sequencer-204=local:{self.nodes['sequencer-204']}",
                "--host-adapter",
                str(adapter),
                "--out-dir",
                str(out_dir),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"trusted|attest|provenance|producer|observe")
        self.assertFalse((out_dir / "quiescence-proof.json").exists())

    def test_plan_rejects_hand_authored_stopped_quiescence_proof_without_producer_attestation(self) -> None:
        generated_proof = json.loads(self.quiescence_proof.read_text(encoding="utf-8"))
        generated_source_path = Path(generated_proof["source_receipt_path"])
        generated_source = json.loads(generated_source_path.read_text(encoding="utf-8"))

        # Re-home the producer output as a caller-authored pair.  Every
        # currently checked digest, order, role, and stopped-state field is
        # internally consistent, but no governed producer attestation binds
        # this hand-authored source/proof pair to an adapter invocation.
        hand_source_path = self.root / "hand-authored-quiescence-source.json"
        hand_source_path.write_text(json.dumps(generated_source, indent=2) + "\n", encoding="utf-8")
        hand_proof = copy.deepcopy(generated_proof)
        hand_proof["source_receipt_path"] = str(hand_source_path)
        hand_proof["source_receipt_sha256"] = sha256(hand_source_path)
        hand_proof_path = self.root / "hand-authored-quiescence-proof.json"
        hand_proof_path.write_text(json.dumps(hand_proof, indent=2) + "\n", encoding="utf-8")

        args = self._base_args()
        args[args.index("--stopped-quiescence-proof") + 1] = str(hand_proof_path)
        result = subprocess.run(args, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"trusted|attest|provenance|producer")
        self.assertFalse(self.out.exists())

    def test_consumer_impact_requires_strict_boolean_and_rfc3339_timestamp(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_strict_impact_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        for invalid_value in ("false", 0, 1, None):
            with self.subTest(validators_already_stopped=invalid_value):
                impact = json.loads(self.impact.read_text(encoding="utf-8"))
                impact["validators_already_stopped"] = invalid_value
                self.impact.write_text(json.dumps(impact), encoding="utf-8")
                with self.assertRaisesRegex(SystemExit, "boolean"):
                    module.validate_impact(self.impact)

        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact["timestamp"] = "2026-08-28 00:00:00+00:00"
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        with self.assertRaisesRegex(SystemExit, "RFC3339"):
            module.validate_impact(self.impact)

    def test_quiesce_rejects_observer_mutation_receipt(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        adapter = self._write_quiescence_adapter()
        adapter_source = adapter.read_text(encoding="utf-8")
        adapter.write_text(
            adapter_source.replace(
                "    'observer_mutation': False,\n",
                "    'observer_mutation': True,\n",
            ),
            encoding="utf-8",
        )
        out_dir = self.root / "observer-mutation"
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "quiesce",
                "--consumer-impact-record",
                str(self.impact),
                "--quiescence-id",
                "observer-mutation-3481",
                "--node",
                f"storage-205=local:{self.nodes['storage-205']}",
                "--node",
                f"sequencer-204=local:{self.nodes['sequencer-204']}",
                "--host-adapter",
                str(adapter),
                "--out-dir",
                str(out_dir),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"capability_blocked|observer")
        self.assertFalse((out_dir / "quiescence-proof.json").exists())

    def test_plan_rejects_symlink_stopped_quiescence_proof(self) -> None:
        proof_link = self.root / "quiescence-proof-link.json"
        proof_link.symlink_to(self.quiescence_proof)
        args = self._base_args()
        args[args.index("--stopped-quiescence-proof") + 1] = str(proof_link)
        result = subprocess.run(args, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"regular file|symlink")
        self.assertFalse(self.out.exists())

    def test_plan_requires_explicit_transaction_id_in_stopped_quiescence_proof(self) -> None:
        proof = json.loads(self.quiescence_proof.read_text(encoding="utf-8"))
        proof.pop("transaction_id", None)
        self.quiescence_proof.write_text(json.dumps(proof), encoding="utf-8")
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(
            result.stderr.lower(),
            r"capability_blocked|transaction.*identity|transaction_id|persisted.*proof|audit.*record|producer.*attestation",
        )
        self.assertFalse(self.out.exists())

    def test_plan_requires_explicit_transaction_id_in_stopped_quiescence_source_receipt(self) -> None:
        proof = json.loads(self.quiescence_proof.read_text(encoding="utf-8"))
        source_path = Path(proof["source_receipt_path"])
        source = json.loads(source_path.read_text(encoding="utf-8"))
        source.pop("transaction_id", None)
        hand_source_path = self.root / "missing-source-transaction-id.json"
        hand_source_path.write_text(json.dumps(source, indent=2) + "\n", encoding="utf-8")
        proof["source_receipt_path"] = str(hand_source_path)
        proof["source_receipt_sha256"] = sha256(hand_source_path)
        hand_proof_path = self.root / "missing-source-transaction-id-proof.json"
        hand_proof_path.write_text(json.dumps(proof, indent=2) + "\n", encoding="utf-8")

        args = self._base_args()
        args[args.index("--stopped-quiescence-proof") + 1] = str(hand_proof_path)
        result = subprocess.run(args, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(
            result.stderr.lower(),
            r"capability_blocked|transaction.*identity|transaction_id|persisted.*proof|audit.*record|producer.*attestation",
        )
        self.assertFalse(self.out.exists())

    def test_quiesce_requires_explicit_transaction_id_in_adapter_receipt(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "active",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        adapter = self._write_quiescence_adapter()
        adapter_source = adapter.read_text(encoding="utf-8")
        adapter.write_text(
            adapter_source.replace("    'transaction_id': transaction['transaction_id'],\n", ""),
            encoding="utf-8",
        )
        out_dir = self.root / "missing-transaction-id"
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "quiesce",
                "--consumer-impact-record",
                str(self.impact),
                "--quiescence-id",
                "missing-transaction-id-3481",
                "--node",
                f"storage-205=local:{self.nodes['storage-205']}",
                "--node",
                f"sequencer-204=local:{self.nodes['sequencer-204']}",
                "--host-adapter",
                str(adapter),
                "--out-dir",
                str(out_dir),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertRegex(result.stderr.lower(), r"capability_blocked|transaction.*identity|transaction_id")
        self.assertFalse((out_dir / "quiescence-proof.json").exists())

    def test_quiesce_supports_none_impact_with_previously_stopped_validators(self) -> None:
        fixture = self._write_human_direct_ssh_fixture(
            transaction_id="none-impact-quiescence-3481",
            impact_path=self.impact,
        )
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(Path(fixture["receipt"]).read_text(encoding="utf-8"))
        self.assertEqual(receipt["phase"], "quiesce")
        self.assertEqual(set(receipt["nodes"]), {"storage-205", "sequencer-204"})
        self.assertTrue(all(item["active"] is False and item["running"] is False for item in receipt["nodes"].values()))
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())
        self.assertFalse((self.nodes["sequencer-204"] / "backups").exists())

    def test_wrapper_quiesce_envelope_forbids_mutation_phases(self) -> None:
        impact = json.loads(self.impact.read_text(encoding="utf-8"))
        impact.update(
            {
                "impact": "unknown",
                "evidence_source": "live public-testnet consumer impact record",
                "timestamp": "2026-08-28T00:00:00Z",
                "validators_already_stopped": False,
                "outage_update_channel": "issue-3481 outage update thread",
                "recovery_update_checkpoint": "issue-3481 Phase G recovery checkpoint",
                "producer_wording_approval": "issue-3481 producer approval",
                "decision": "proceed",
            }
        )
        self.impact.write_text(json.dumps(impact), encoding="utf-8")
        fixture = self._write_human_direct_ssh_fixture(
            transaction_id="wrapper-quiescence-3481",
            impact_path=self.impact,
        )
        result = self._run_human_direct_ssh(fixture)
        self.assertEqual(result.returncode, 0, result.stderr)
        envelope = json.loads(result.stdout)
        contract = envelope["receipt_contract"]
        self.assertEqual(contract["mode"], "human_direct_ssh")
        self.assertTrue(contract["quiescence_only"])
        self.assertFalse(contract["destructive_activity"])
        self.assertEqual(contract["reset"], "forbidden")
        self.assertEqual(contract["stage"], "forbidden")
        self.assertEqual(contract["authority"], "human_direct_ssh")
        self.assertEqual(contract["credential_seam"], "temporary-fd-or-environment; secret-free argv/log/receipt")
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_remote_identity_receipt_binds_metadata_without_local_key_access(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_remote_identity_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        receipt = self.root / "remote-identity-receipt.json"
        receipt.write_text(
            json.dumps(
                {
                    "schema_version": "oasis7.identity_receipt.v1",
                    "node_id": "triad-testnet-storage",
                    "peer_id": "peer-storage-205",
                    "key_path": "/remote/validator/config/node-keypair.toml",
                    "key_sha256": "a" * 64,
                    "key_size_bytes": 321,
                    "key_mode": 0o600,
                    "key_uid": 1001,
                    "key_gid": 1001,
                }
            ),
            encoding="utf-8",
        )
        expected_metadata = module._expected_identity_metadata(
            {
                "expected_key_mode": 0o600,
                "expected_key_uid": 1001,
                "expected_key_gid": 1001,
            },
            "identity manifest entry",
        )
        self.assertEqual(
            module._expected_identity_binding(
                {
                    "expected_node_id": "triad-testnet-storage",
                    "expected_peer_id": "peer-storage-205",
                },
                "identity manifest entry",
            ),
            {"node_id": "triad-testnet-storage", "peer_id": "peer-storage-205"},
        )
        with self.assertRaisesRegex(SystemExit, "governed expected key metadata"):
            module.verify_signed_attestation(
                receipt,
                {"root_digest": "remote-capture"},
                "remote identity receipt",
                "storage-205",
            )
        summary = module.verify_signed_attestation(
            receipt,
            {"root_digest": "remote-capture"},
            "remote identity receipt",
            "storage-205",
            expected_identity_metadata=expected_metadata,
        )
        self.assertEqual(summary["role"], "storage-205")
        self.assertEqual(summary["node_id"], "triad-testnet-storage")
        self.assertEqual(summary["peer_id"], "peer-storage-205")
        self.assertEqual(summary["key_path"], "/remote/validator/config/node-keypair.toml")
        self.assertEqual(summary["key_sha256"], "a" * 64)
        self.assertEqual(summary["key_size_bytes"], 321)
        self.assertEqual(summary["key_mode"], 0o600)
        self.assertEqual(summary["key_uid"], 1001)
        self.assertEqual(summary["key_gid"], 1001)

    def test_remote_identity_receipt_rejects_weak_metadata_contract(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_remote_identity_metadata_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        receipt = self.root / "weak-remote-identity-receipt.json"
        receipt.write_text(
            json.dumps(
                {
                    "schema_version": "oasis7.identity_receipt.v1",
                    "node_id": "triad-testnet-storage",
                    "peer_id": "peer-storage-205",
                    "key_path": "/remote/validator/config/node-keypair.toml",
                    "key_sha256": "b" * 64,
                    "key_size_bytes": 321,
                    "key_mode": 0o644,
                    "key_uid": 9999,
                    "key_gid": 9999,
                }
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(SystemExit, "metadata"):
            module.verify_signed_attestation(
                receipt,
                {"root_digest": "remote-capture"},
                "remote identity receipt",
                "storage-205",
                expected_identity_metadata={
                    "key_mode": 0o600,
                    "key_uid": 1001,
                    "key_gid": 1001,
                },
            )

    def test_identity_manifest_rejects_relative_governed_key_path(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_identity_manifest_path_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with self.assertRaisesRegex(SystemExit, "lexically absolute"):
            module._expected_identity_metadata(
                {
                    "expected_key_path": "config/node-keypair.toml",
                    "expected_key_sha256": "a" * 64,
                    "expected_key_size_bytes": 321,
                    "expected_key_mode": 0o600,
                    "expected_key_uid": 1001,
                    "expected_key_gid": 1001,
                },
                "identity manifest entry",
            )

    def test_plan_binds_complete_tree_inventory_to_capacity_and_inode_budget(self) -> None:
        for node in self.nodes.values():
            current = node / "current"
            release = node / "releases" / "known"
            release.mkdir(parents=True)
            (release / "bin").mkdir()
            (release / "bin" / "oasis7_chain_runtime").write_bytes(b"runtime-old\n")
            shutil.rmtree(current)
            current.symlink_to("releases/known", target_is_directory=True)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads(result.stdout)
        for role in ("storage-205", "sequencer-204"):
            inventory = plan["capacity"][role]["inventory"]
            self.assertEqual(
                set(inventory),
                {"entry_count", "link_count", "dir_count", "file_count", "total_bytes"},
            )
            self.assertEqual(
                inventory["entry_count"],
                inventory["link_count"] + inventory["dir_count"] + inventory["file_count"],
            )
            self.assertGreaterEqual(inventory["link_count"], 1)
            self.assertGreaterEqual(plan["capacity"][role]["required_inodes"], inventory["entry_count"])

    def test_plan_rejects_unsupported_fifo_before_any_quiesce(self) -> None:
        fifo = self.nodes["storage-205"] / "data" / "unsupported.fifo"
        os.mkfifo(fifo)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported", result.stderr.lower())
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_snapshot_failure_removes_partial_backup_for_unsupported_entry(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_snapshot_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        fifo = self.nodes["storage-205"] / "data" / "unsupported.fifo"
        os.mkfifo(fifo)
        with self.assertRaises(Exception):
            module.snapshot_node({"role": "storage-205", "root": str(self.nodes["storage-205"])}, "partial-backup")
        self.assertFalse((self.nodes["storage-205"] / "backups" / "partial-backup").exists())

    def test_plan_digest_is_stable_and_runtime_transaction_metadata_is_separate(self) -> None:
        first = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(first.returncode, 0, first.stderr)
        first_receipt = json.loads(first.stdout)
        second_out = self.root / "out-second"
        second_args = [arg for arg in self._base_args() if arg not in ("--out-dir", str(self.out))]
        second_args.extend(["--out-dir", str(second_out)])
        second = subprocess.run(second_args, text=True, capture_output=True)
        self.assertEqual(second.returncode, 0, second.stderr)
        second_receipt = json.loads(second.stdout)
        self.assertEqual(first_receipt, second_receipt)
        self.assertNotIn("transaction_id", first_receipt)
        self.assertNotIn("created_at", first_receipt)

    def test_plan_rejects_full_status_url_for_204(self) -> None:
        result = subprocess.run(
            self._base_args() + ["--sequencer-proof-url", "http://127.0.0.1:6631/v1/chain/status"],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("204", result.stderr)

    def test_active_runbook_has_no_sequencer_full_status_command(self) -> None:
        runbook = (ROOT / "doc/p2p/blockchain/public-testnet-governed-bootstrap.runbook.md").read_text(encoding="utf-8")
        self.assertNotIn("6631/v1/chain/status", runbook)
        self.assertNotIn("39.104.204.172:6631/v1/chain/status", runbook)

    def test_plan_rejects_untrusted_signer_even_when_detached_signature_is_valid(self) -> None:
        receipt = json.loads(self.provenance.read_text(encoding="utf-8"))
        receipt["signature"]["signer_id"] = "unlisted-attestor"
        body = {key: value for key, value in receipt.items() if key != "binding_digest"}
        receipt["binding_digest"] = hashlib.sha256(
            json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        payload_path = self.root / "untrusted-signing-payload.bin"
        payload_path.write_bytes(json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode())
        subprocess.run(
            ["openssl", "dgst", "-sha256", "-sign", str(self.signing_key), "-out", str(self.signature), str(payload_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self.provenance.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("trusted signer", result.stderr.lower())

    def test_plan_rejects_unexpected_governed_key(self) -> None:
        extra = self.governed / "unexpected.json"
        extra.write_text("{}\n", encoding="utf-8")
        self._write_provenance(extra_governed=extra)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("governed", result.stderr.lower())

    def test_canonical_current_symlink_is_counted_and_accepted(self) -> None:
        for role, node in self.nodes.items():
            current = node / "current"
            shutil_target = node / "releases" / "known"
            shutil_target.mkdir(parents=True)
            (shutil_target / "bin").mkdir()
            (shutil_target / "bin" / "oasis7_chain_runtime").write_bytes(b"runtime-old\n")
            shutil.rmtree(current)
            current.symlink_to("releases/known", target_is_directory=True)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_arbitrary_current_symlink_is_rejected(self) -> None:
        current = self.nodes["storage-205"] / "current"
        shutil.rmtree(current)
        current.symlink_to(self.root / "package", target_is_directory=True)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("current", result.stderr.lower())

    def test_apply_requires_phase_callbacks_and_signed_identity_proof(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        result = subprocess.run(
            self._apply_args(plan_path, adapter),
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "applied")
        self.assertIn("direct_quiescence_observation", receipt)
        self.assertIn("backup_receipt", receipt)
        self.assertIn("sequencer_rebuild_proof", receipt["host_receipt"])

    def test_host_receipt_rejects_trusted_but_unplanned_proof(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_host_binding_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        plan = json.loads(subprocess.run(self._base_args(), text=True, capture_output=True, check=True).stdout)
        plan["schema_version"] = "oasis7.validator_pair_rebuild_transaction.v1"
        plan["transaction_id"] = "pair-rebuild-test-binding"
        plan["phase"] = "staged"
        plan_path = self.out / "binding-transaction.json"
        plan_path.write_text(json.dumps(plan), encoding="utf-8")
        adapter = self._write_host_adapter()
        receipt = module.run_host_adapter(adapter, plan_path, plan, "apply")
        alternate_proof = self.root / "alternate-host-proof.json"
        alternate_signature = self.root / "alternate-host-proof.sig"
        self._write_attestation(
            alternate_proof,
            alternate_signature,
            "oasis7.validator_pair_rebuild_proof.v1",
            "sequencer-204",
            "peer-unplanned",
            "unplanned-sequencer",
        )
        receipt["sequencer_rebuild_proof"]["path"] = str(alternate_proof)
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(receipt, plan, "apply")

    def _bound_phase_receipt(self, phase: str):
        spec = importlib.util.spec_from_file_location("pair_rebuild_host_binding_fixture", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        plan = json.loads(subprocess.run(self._base_args(), text=True, capture_output=True, check=True).stdout)
        plan["schema_version"] = "oasis7.validator_pair_rebuild_transaction.v1"
        plan["transaction_id"] = f"pair-rebuild-test-{phase}"
        plan["phase"] = "staged" if phase == "apply" else "prepared"
        plan_path = self.out / f"{phase}-transaction.json"
        plan_path.write_text(json.dumps(plan), encoding="utf-8")
        receipt = module.run_host_adapter(self._write_host_adapter(), plan_path, plan, phase)
        if phase == "apply":
            empty_surface = self.root / "empty-post-delete-surface"
            empty_surface.mkdir(exist_ok=True)
            empty_inventory = module.inventory_tree(empty_surface)
            post_delete_absence = {
                "schema_version": "oasis7.validator_pair_rebuild_post_delete_absence.v1",
                "absent": True,
                "target_set": list(module.RESET_SURFACES),
                "target_set_sha256": module.reset_surface_digest(),
                "target_inventory": {
                    surface: empty_inventory
                    for surface in module.RESET_SURFACES
                },
            }
            for role in ("storage-205", "sequencer-204"):
                receipt["nodes"][role]["post_delete_absence"] = copy.deepcopy(post_delete_absence)
        return module, plan, receipt

    def _bound_host_receipt(self):
        return self._bound_phase_receipt("apply")

    def test_apply_requires_explicit_observer_mutation_false(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        receipt.pop("observer_mutation", None)
        with self.assertRaisesRegex(SystemExit, "observer"):
            module.validate_host_receipt(receipt, plan, "apply")

    def test_apply_requires_per_node_independent_observation(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        for role in ("storage-205", "sequencer-204"):
            receipt["nodes"][role].pop("independently_observed", None)
        with self.assertRaisesRegex(SystemExit, r"independent|observ"):
            module.validate_host_receipt(receipt, plan, "apply")

    def test_apply_requires_per_node_service_state(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        for role in ("storage-205", "sequencer-204"):
            receipt["nodes"][role].pop("service_state", None)
        with self.assertRaisesRegex(SystemExit, r"service.?state|service"):
            module.validate_host_receipt(receipt, plan, "apply")

    def test_apply_rejects_non_exact_per_node_role_and_root_bindings(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        cases = {
            "role substitution": ("role", "sequencer-204"),
            "root substitution": ("root", str(self.nodes["sequencer-204"])),
            "missing role": ("role", None),
            "missing root": ("root", None),
        }
        for label, (field, value) in cases.items():
            with self.subTest(binding=label):
                candidate = copy.deepcopy(receipt)
                node = candidate["nodes"]["storage-205"]
                if value is None:
                    node.pop(field, None)
                else:
                    node[field] = value
                with self.assertRaisesRegex(SystemExit, r"role|root|binding"):
                    module.validate_host_receipt(candidate, plan, "apply")

    def test_apply_rejects_missing_per_node_post_delete_absence_receipts(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        for role in ("storage-205", "sequencer-204"):
            receipt["nodes"][role].pop("post_delete_absence", None)
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(receipt, plan, "apply")

    def test_apply_rejects_target_surface_mismatch_including_advertised_bridge_root(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        advertised_targets = [*CANONICAL_RESET_SURFACES, "data/bridge-root"]
        target_digest = hashlib.sha256(
            json.dumps(advertised_targets, ensure_ascii=True, separators=(",", ":")).encode()
        ).hexdigest()
        for role in ("storage-205", "sequencer-204"):
            receipt["nodes"][role]["post_delete_absence"] = {
                "absent": True,
                "target_set": advertised_targets,
                "target_set_sha256": target_digest,
            }
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(receipt, plan, "apply")

    def test_backup_rejects_each_incomplete_hashed_manifest_binding(self) -> None:
        module, plan, receipt = self._bound_phase_receipt("backup")
        required_fields = (
            "backup_root",
            "backup_manifest_sha256",
            "backup_inventory",
            "backup_capacity",
            "backup_non_seed",
        )
        for missing_field in required_fields:
            incomplete = copy.deepcopy(receipt)
            for role in ("storage-205", "sequencer-204"):
                node = incomplete["nodes"][role]
                node.update(
                    {
                        "backup_root": str(Path(plan["nodes"][role]["root"]) / "backups" / "pair-rebuild-test-backup"),
                        "backup_manifest_sha256": "a" * 64,
                        "backup_inventory": plan["capacity"][role]["inventory"],
                        "backup_capacity": plan["capacity"][role],
                        "backup_non_seed": {
                            "forensic_only": True,
                            "seed_eligible": False,
                            "restore_deleted_chain_state": False,
                        },
                    }
                )
                node.pop(missing_field)
            with self.subTest(missing_field=missing_field):
                with self.assertRaises(SystemExit):
                    module.validate_host_receipt(incomplete, plan, "backup")

    def test_host_receipt_rejects_incomplete_swapped_or_stale_evidence(self) -> None:
        module, plan, receipt = self._bound_host_receipt()
        omitted = copy.deepcopy(receipt)
        omitted["identity_receipts"] = omitted["identity_receipts"][:1]
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(omitted, plan, "apply")
        swapped = copy.deepcopy(receipt)
        swapped["identity_receipts"][0]["role"], swapped["identity_receipts"][1]["role"] = (
            swapped["identity_receipts"][1]["role"],
            swapped["identity_receipts"][0]["role"],
        )
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(swapped, plan, "apply")
        stale = copy.deepcopy(receipt)
        stale["captured_at"] = "2000-01-01T00:00:00Z"
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(stale, plan, "apply")
        stale_window = copy.deepcopy(receipt)
        plan["adapter_binding"]["phase_window_started_at"] = "2000-01-01T00:00:00Z"
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(stale_window, plan, "apply")
        substituted = copy.deepcopy(receipt)
        substituted["evidence_bindings"]["identity_receipts"][0]["sha256"] = "0" * 64
        with self.assertRaises(SystemExit):
            module.validate_host_receipt(substituted, plan, "apply")

    def test_adapter_binding_separates_raw_and_verification_proof_files(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_binding_shape_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        identities = []
        for role in ("storage-205", "sequencer-204"):
            path = self.root / f"binding-{role}.json"
            path.write_text(role, encoding="utf-8")
            identities.append({
                "path": str(path),
                "sha256": sha256(path),
                "role": role,
                "node_id": f"node-{role}",
                "peer_id": f"peer-{role}",
                "key_path": f"/remote/{role}/config/node-keypair.toml",
                "key_sha256": "c" * 64,
                "key_size_bytes": 321,
                "key_mode": 0o600,
                "key_uid": 1001,
                "key_gid": 1001,
            })
        raw_proof = self.root / "binding-raw-proof.json"
        verification = self.root / "binding-verification.json"
        raw_proof.write_text("raw-proof", encoding="utf-8")
        verification.write_text("verification-receipt", encoding="utf-8")
        plan = {
            "plan_digest": "a" * 64,
            "transaction_id": "binding-shape",
            "proof": {
                "identity_receipts": identities,
                "sequencer_rebuild_proof_path": str(raw_proof),
                "sequencer_rebuild_proof_verification_path": str(verification),
                "sequencer_rebuild_proof": {
                    "path": str(verification),
                    "sha256": sha256(verification),
                    "proof_sha256": sha256(raw_proof),
                    "role": "sequencer-204",
                    "node_id": "node-sequencer-204",
                    "peer_id": "peer-sequencer-204",
                },
            },
        }
        evidence = module._expected_adapter_evidence(plan)
        self.assertEqual(evidence["identity_receipts"][0]["key_mode"], 0o600)
        self.assertEqual(evidence["identity_receipts"][0]["key_uid"], 1001)
        self.assertEqual(evidence["identity_receipts"][0]["key_gid"], 1001)
        self.assertEqual(evidence["sequencer_rebuild_proof"]["path"], str(raw_proof.resolve()))
        self.assertEqual(evidence["sequencer_rebuild_proof"]["sha256"], sha256(raw_proof))
        self.assertEqual(evidence["sequencer_rebuild_proof"]["verification_path"], str(verification.resolve()))
        self.assertEqual(evidence["sequencer_rebuild_proof"]["verification_sha256"], sha256(verification))

    def test_plan_rejects_missing_signature_before_any_node_access(self) -> None:
        self._write_provenance(signature=False)
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("signature", result.stderr.lower())

    def test_provenance_rejects_tampered_signed_body(self) -> None:
        receipt = json.loads(self.provenance.read_text(encoding="utf-8"))
        receipt["package"]["run_id"] = "tampered"
        body = {key: value for key, value in receipt.items() if key != "binding_digest"}
        receipt["binding_digest"] = hashlib.sha256(
            json.dumps(body, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        self.provenance.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "validate",
                "--provenance",
                str(self.provenance),
                "--package-dir",
                str(self.package),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("detached signature verification failed", result.stderr.lower())

    def test_staged_receipt_falls_back_to_copied_detached_files(self) -> None:
        staged = self.root / "staged-evidence"
        staged.mkdir()
        staged_receipt = staged / self.provenance.name
        staged_receipt.write_bytes(self.provenance.read_bytes())
        (staged / self.signature.name).write_bytes(self.signature.read_bytes())
        (staged / self.public_key.name).write_bytes(self.public_key.read_bytes())
        original_signature = self.signature.read_bytes()
        original_public_key = self.public_key.read_bytes()
        self.signature.unlink()
        self.public_key.unlink()
        result = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "validate",
                "--provenance",
                str(staged_receipt),
                "--package-dir",
                str(self.package),
            ],
            text=True,
            capture_output=True,
        )
        self.signature.write_bytes(original_signature)
        self.public_key.write_bytes(original_public_key)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_apply_writes_full_manifests_and_preserves_order(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        result = subprocess.run(
            self._apply_args(plan_path, adapter),
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "applied")
        self.assertEqual(receipt["mutation_order"], ["storage-205", "sequencer-204"])
        self.assertTrue(receipt["nodes"]["storage-205"]["backup"]["verified"])
        self.assertTrue(receipt["nodes"]["sequencer-204"]["backup"]["verified"])
        self.assertTrue((self.nodes["storage-205"] / "backups").exists())
        self.assertTrue((self.nodes["sequencer-204"] / "backups").exists())
        self.assertTrue(receipt["host_receipt"]["nodes"]["sequencer-204"]["healthz_ok"])
        self.assertTrue(receipt["capacity_apply"]["storage-205"]["verified"])
        for role in ("storage-205", "sequencer-204"):
            self.assertEqual(
                receipt["capacity_apply"][role]["inventory"],
                receipt["capacity"][role]["inventory"],
            )
        self.assertEqual(
            set(receipt["staged"]["storage-205"]["governed_inventory"]),
            {"manifest", "genesis", "registry", "bootstrap", "world"},
        )
        self.assertEqual(receipt["host_receipt"]["plan_digest"], receipt["plan_digest"])
        self.assertEqual(receipt["host_receipt"]["evidence_bindings"], receipt["adapter_binding"]["evidence_bindings"])
        self.assertTrue(receipt["host_receipt"]["captured_at"])

    def test_apply_transaction_only_invocation_recovers_direct_observation_routing(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(EXECUTOR),
                "apply",
                "--transaction",
                str(plan_path),
                "--host-adapter",
                str(self._write_host_adapter()),
            ],
            text=True,
            capture_output=True,
            env=os.environ.copy(),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["phase"], "applied")

    def test_apply_requires_host_gate_receipt(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "apply", "--transaction", str(plan_path)],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("host adapter", result.stderr.lower())
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_apply_rechecks_capacity_after_plan(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        transaction = json.loads(plan.stdout)
        transaction["capacity"]["storage-205"]["required_bytes"] = 10**30
        transaction["plan_digest"] = hashlib.sha256(
            json.dumps({key: value for key, value in transaction.items() if key != "plan_digest"}, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        plan_path = self.out / "transaction.json"
        plan_path.write_text(json.dumps(transaction), encoding="utf-8")
        result = subprocess.run(
            self._apply_args(plan_path, self._write_host_adapter()),
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("apply-time capacity", result.stderr.lower())
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_apply_rejects_inventory_drift_after_plan(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        (self.nodes["storage-205"] / "data" / "unexpected-entry").write_text("drift\n", encoding="utf-8")
        result = subprocess.run(
            self._apply_args(plan_path, self._write_host_adapter()),
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("inventory changed", result.stderr.lower())
        self.assertFalse((self.nodes["storage-205"] / "backups").exists())

    def test_rollback_restores_the_stopped_snapshot(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        applied = subprocess.run(
            self._apply_args(plan_path, adapter),
            text=True,
            capture_output=True,
            check=True,
        )
        applied_receipt = json.loads(applied.stdout)
        transaction_id = applied_receipt["transaction_id"]
        self.assertNotEqual(sha256(self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"), sha256(self.nodes["storage-205"] / "backups" / transaction_id / "snapshot" / "current" / "bin" / "oasis7_chain_runtime"))
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "rollback", "--transaction", str(plan_path), "--host-adapter", str(adapter)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["phase"], "rolled_back")
        self.assertEqual(sha256(self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"), sha256(self.nodes["storage-205"] / "backups" / transaction_id / "snapshot" / "current" / "bin" / "oasis7_chain_runtime"))

    def test_apply_failure_freshly_quiesces_before_automatic_rollback(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        phases = self.root / "automatic-rollback-phases.log"
        adapter = self._write_failure_host_adapter(phases, "apply")
        result = subprocess.run(
            self._apply_args(plan_path, adapter),
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(phases.read_text(encoding="utf-8").splitlines(), ["backup", "apply", "quiesce", "rollback"])
        transaction = json.loads(plan_path.read_text(encoding="utf-8"))
        self.assertIn("rollback_quiesce_receipt", transaction)
        self.assertEqual(transaction["rollback"]["status"], "verified")

    def test_apply_failure_does_not_restore_when_automatic_quiescence_fails(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        phases = self.root / "automatic-rollback-quiescence-failure-phases.log"
        adapter = self._write_failure_host_adapter(phases, "apply", "quiesce")
        result = subprocess.run(
            self._apply_args(plan_path, adapter),
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(phases.read_text(encoding="utf-8").splitlines(), ["backup", "apply", "quiesce"])
        transaction = json.loads(plan_path.read_text(encoding="utf-8"))
        self.assertEqual(transaction["phase"], "rollback_failed")
        self.assertEqual(transaction["rollback"]["status"], "failed")
        current_runtime = self.nodes["storage-205"] / "current" / "bin" / "oasis7_chain_runtime"
        backup_runtime = Path(transaction["backup"]["storage-205"]["snapshot"]) / "current" / "bin" / "oasis7_chain_runtime"
        self.assertNotEqual(sha256(current_runtime), sha256(backup_runtime))

    def test_rollback_rejects_tampered_backup_manifest(self) -> None:
        plan = subprocess.run(self._base_args(), text=True, capture_output=True, check=True)
        plan_path = self.out / "transaction.json"
        plan_path.write_text(plan.stdout, encoding="utf-8")
        adapter = self._write_host_adapter()
        applied = subprocess.run(
            self._apply_args(plan_path, adapter),
            text=True,
            capture_output=True,
            check=True,
        )
        transaction = json.loads(applied.stdout)
        manifest = Path(transaction["backup"]["storage-205"]["manifest"])
        manifest.write_text(manifest.read_text(encoding="utf-8") + "\n", encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(EXECUTOR), "rollback", "--transaction", str(plan_path), "--host-adapter", str(adapter)],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("manifest", result.stderr.lower())

    def test_plan_rejects_insufficient_capacity(self) -> None:
        self.capacity.write_text(
            json.dumps(
                {
                    role: {"free_bytes": 1, "free_inodes": 100_000, "same_filesystem": True}
                    for role in self.nodes
                }
            ),
            encoding="utf-8",
        )
        result = subprocess.run(self._base_args(), text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("capacity", result.stderr.lower())

    def test_provenance_create_and_validate_round_trip(self) -> None:
        generated = self.root / "generated-provenance.json"
        create = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "create",
                "--package-dir",
                str(self.package),
                "--manifest",
                str(self.governed / "manifest.json"),
                "--genesis",
                str(self.governed / "genesis.json"),
                "--registry",
                str(self.governed / "registry.json"),
                "--bootstrap",
                str(self.governed / "bootstrap.txt"),
                "--world",
                str(self.governed / "world.json"),
                "--network-id",
                "oasis7-public-testnet-governed-20260606",
                "--chain-id",
                "oasis7-public-testnet-governed-20260606",
                "--output",
                str(generated),
                "--signer-id",
                "testnet-package-attestor",
                "--signature-ref",
                str(self.signature),
                "--public-key-ref",
                str(self.public_key),
            ],
            text=True,
            capture_output=True,
        )
        self.assertEqual(create.returncode, 0, create.stderr)
        generated_payload = json.loads(generated.read_text(encoding="utf-8"))
        generated_payload["signature"]["status"] = "verified"
        body = {key: value for key, value in generated_payload.items() if key != "binding_digest"}
        generated_payload["binding_digest"] = hashlib.sha256(
            json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
        ).hexdigest()
        generated.write_text(json.dumps(generated_payload, indent=2) + "\n", encoding="utf-8")
        payload_path = self.root / "generated-provenance-signing-payload.bin"
        payload_path.write_bytes(json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode())
        subprocess.run(
            [
                "openssl",
                "dgst",
                "-sha256",
                "-sign",
                str(self.signing_key),
                "-out",
                str(self.signature),
                str(payload_path),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        validate = subprocess.run(
            [sys.executable, str(PROVENANCE), "validate", "--provenance", str(generated), "--package-dir", str(self.package)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr)
        self.assertEqual(json.loads(validate.stdout)["signature"]["signer_id"], "testnet-package-attestor")

    def test_provenance_rejects_nested_directory_symlink_before_file_filtering(self) -> None:
        world = self.root / "world-tree"
        world.mkdir()
        (world / "state.json").write_text('{"height":1}\n', encoding="utf-8")
        target = self.root / "outside-world"
        target.mkdir()
        (target / "hidden.json").write_text('{"hidden":true}\n', encoding="utf-8")
        (world / "nested-link").symlink_to(target, target_is_directory=True)
        generated = self.root / "symlink-provenance.json"
        create = subprocess.run(
            [
                sys.executable,
                str(PROVENANCE),
                "create",
                "--package-dir",
                str(self.package),
                "--manifest",
                str(self.governed / "manifest.json"),
                "--genesis",
                str(self.governed / "genesis.json"),
                "--registry",
                str(self.governed / "registry.json"),
                "--bootstrap",
                str(self.governed / "bootstrap.txt"),
                "--world",
                str(world),
                "--network-id",
                "oasis7-public-testnet-governed-20260606",
                "--chain-id",
                "oasis7-public-testnet-governed-20260606",
                "--output",
                str(generated),
                "--signer-id",
                "testnet-package-attestor",
                "--signature-ref",
                str(self.signature),
                "--public-key-ref",
                str(self.public_key),
            ],
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(create.returncode, 0)
        self.assertIn("symlink", create.stderr.lower())

    def test_release_bundle_validates_pair_provenance_binding(self) -> None:
        bundle = self.root / "release-bundle.json"
        create = subprocess.run(
            [
                "bash",
                str(RELEASE_BUNDLE),
                "create",
                "--bundle",
                str(bundle),
                "--candidate-id",
                "pair-rebuild-fixture",
                "--runtime-build-ref",
                str(self.package / "oasis7_chain_runtime"),
                "--world-snapshot-ref",
                str(self.governed / "world.json"),
                "--governance-manifest-ref",
                str(self.governed / "manifest.json"),
                "--validator-pair-provenance-ref",
                str(self.provenance),
                "--allow-dirty-worktree",
            ],
            text=True,
            capture_output=True,
        )
        self.assertEqual(create.returncode, 0, create.stderr)
        validate = subprocess.run(
            ["bash", str(RELEASE_BUNDLE), "validate", "--bundle", str(bundle)],
            text=True,
            capture_output=True,
        )
        self.assertEqual(validate.returncode, 0, validate.stderr)

    def test_runtime_verification_receipt_binds_peer_and_exact_raw_proof(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_runtime_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        public_key_hex = "9ea2a4ca4893c1d2fed7482510333464c28a0ddb9db497a1238edb71d4ba286b"
        signature_hex = "22258b4742876b6a997c410b268547b8a40280acbc5ef2ad5a21a4ef034bc463fb728d31ab1ca549e9441c50cf4be587413ece9f9c68067770ce97712096b704"
        claims = {
            "schema_version": "oasis7.rebuild_status.v1",
            "observed_at_unix_ms": 1,
            "node_id": "sequencer-node",
            "world_id": "",
            "ok": True,
            "liveness": {"running": True, "last_error": None},
            "readiness": {"status": "ready", "failed_gates": []},
            "heights": {"committed_height": 1, "network_committed_height": 1, "last_execution_height": 1},
            "network_head": {
                "source": "self",
                "decision": "ready",
                "height": 1,
                "block_hash": "block",
                "execution_block_hash": "execution-block",
                "execution_state_root": "state-root",
                "observed_peer_count": 0,
                "fresh_peer_count": 0,
            },
            "checkpoint": None,
            "local_peer_id": "peer-sequencer",
            "connected_peers": [],
            "connected_peer_count": 0,
        }
        claims_bytes = json.dumps(claims, ensure_ascii=True, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(claims_bytes).hexdigest(), "5acd019055674e080903296b8af0583fec0c86d0243809af187bd2aa11155c76")
        raw_proof = self.root / "raw-rebuild-proof.json"
        raw_value = {
            "schema_version": "oasis7.rebuild_status.v1",
            "observed_at_unix_ms": 1,
            "ok": True,
            "liveness": {"running": True, "last_error": None},
            "readiness": {"status": "ready", "failed_gates": []},
            "heights": {"committed_height": 1, "network_committed_height": 1, "last_execution_height": 1},
            "network_head": {
                "source": "self",
                "decision": "ready",
                "height": 1,
                "block_hash": "block",
                "execution_block_hash": "execution-block",
                "execution_state_root": "state-root",
                "observed_peer_count": 0,
                "fresh_peer_count": 0,
            },
            "checkpoint": None,
            "local_peer_id": "peer-sequencer",
            "connected_peers": [],
            "connected_peer_count": 0,
            "proof": {
                "schema_version": "oasis7.rebuild_proof.v1",
                "signer_id": "sequencer-node",
                "signer_public_key_hex": public_key_hex,
                "signed_payload_sha256": hashlib.sha256(claims_bytes).hexdigest(),
                "signature_hex": signature_hex,
            },
        }
        raw_proof.write_text(json.dumps(raw_value, ensure_ascii=True, separators=(",", ":")), encoding="utf-8")
        receipt = self.root / "runtime-verification-receipt.json"
        receipt_value = {
            "schema_version": "oasis7.rebuild_proof_verification.v1",
            "proof_schema_version": "oasis7.rebuild_proof.v1",
            "signer_id": "sequencer-node",
            "signer_public_key_hex": public_key_hex,
            "signed_payload_sha256": hashlib.sha256(claims_bytes).hexdigest(),
            "local_peer_id": "peer-sequencer",
            "proof_sha256": sha256(raw_proof),
            "verified": True,
        }
        receipt.write_text(json.dumps(receipt_value), encoding="utf-8")
        verifier = self.root / "governed-runtime-verifier.py"
        verifier.write_text(
            "#!/usr/bin/env python3\nimport json\nprint(" + repr(json.dumps(receipt_value)) + ")\n",
            encoding="utf-8",
        )
        verifier.chmod(0o755)
        trusted_root = {"allowlist": [{"signer_id": "sequencer-node", "public_key_hex": public_key_hex}]}
        summary = module.verify_signed_attestation(
            receipt,
            trusted_root,
            "runtime receipt",
            "sequencer-204",
            raw_proof,
            verifier,
        )
        self.assertEqual(summary["peer_id"], "peer-sequencer")
        self.assertEqual(summary["proof_sha256"], sha256(raw_proof))
        forged_verifier = self.root / "forged-runtime-verifier.py"
        forged_receipt = dict(receipt_value)
        forged_receipt["proof_sha256"] = "0" * 64
        forged_verifier.write_text(
            "#!/usr/bin/env python3\nimport json\nprint(" + repr(json.dumps(forged_receipt)) + ")\n",
            encoding="utf-8",
        )
        forged_verifier.chmod(0o755)
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(
                receipt,
                trusted_root,
                "runtime receipt",
                "sequencer-204",
                raw_proof,
                forged_verifier,
            )
        other_raw_proof = self.root / "other-raw-rebuild-proof.json"
        other_raw_proof.write_bytes(b"another canonical raw signed proof\n")
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trusted_root, "runtime receipt", "sequencer-204", other_raw_proof)
        raw_proof.write_bytes(b"different raw signed proof\n")
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trusted_root, "runtime receipt", "sequencer-204", raw_proof)

    def test_runtime_verification_receipt_rejects_unvalidated_raw_envelope(self) -> None:
        spec = importlib.util.spec_from_file_location("pair_rebuild_runtime_negative_test", EXECUTOR)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        raw_proof = self.root / "malformed-raw-rebuild-proof.json"
        raw_proof.write_text(
            json.dumps({
                "schema_version": "oasis7.rebuild_status.v1",
                "local_peer_id": "peer-sequencer",
                "proof": {
                    "schema_version": "oasis7.rebuild_proof.v1",
                    "signer_id": "sequencer-node",
                    "signer_public_key_hex": "a" * 64,
                    "signed_payload_sha256": "b" * 64,
                    "signature_hex": "c" * 128,
                },
            }),
            encoding="utf-8",
        )
        receipt = self.root / "malformed-runtime-verification-receipt.json"
        receipt.write_text(
            json.dumps({
                "schema_version": "oasis7.rebuild_proof_verification.v1",
                "proof_schema_version": "oasis7.rebuild_proof.v1",
                "signer_id": "sequencer-node",
                "signer_public_key_hex": "a" * 64,
                "signed_payload_sha256": "b" * 64,
                "local_peer_id": "peer-sequencer",
                "proof_sha256": sha256(raw_proof),
                "verified": True,
            }),
            encoding="utf-8",
        )
        trust_root = {"allowlist": [{"signer_id": "sequencer-node", "public_key_hex": "a" * 64}]}
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trust_root, "runtime receipt", "sequencer-204", raw_proof)
        with self.assertRaises(SystemExit):
            module.verify_signed_attestation(receipt, trust_root, "runtime receipt", "sequencer-204")


if __name__ == "__main__":
    unittest.main()
