#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Reuse the signed local fixtures owned by the canonical executor contract
# test.  This test is deliberately a shell-entrypoint test: it proves that
# the governed shell dispatch is deterministic and cannot accidentally reach
# SSH/systemd or mutate either stopped node during plan mode.
python3 - "$ROOT_DIR" <<'PY'
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


root = Path(sys.argv[1])
test_path = root / "scripts" / "p2p-public-testnet-validator-pair-rebuild.test.py"
wrapper_path = root / "scripts" / "p2p-public-testnet-rebuild-validators.sh"
runbook_path = root / "doc" / "p2p" / "blockchain" / "public-testnet-governed-bootstrap.runbook.md"
wrapper_source = wrapper_path.read_text(encoding="utf-8")
runbook_source = runbook_path.read_text(encoding="utf-8")
help_output = subprocess.run(
    [str(wrapper_path), "--help"], text=True, capture_output=True, check=True
).stdout
if (
    "Plan is local-only" in help_output
    or "Plan is non-mutating but performs bounded live GitHub/SSH read-only" not in help_output
    or "re-observation" not in help_output
):
    raise SystemExit("wrapper help must disclose bounded live GitHub/SSH read-only plan observation")
for required in ("tempfile.mkstemp", "os.fsync", "os.replace"):
    if required not in wrapper_source:
        raise SystemExit(f"shell envelope is missing durable publication primitive: {required}")
if "destination.write_text(output + \"\\n\"" in wrapper_source:
    raise SystemExit("shell envelope must not publish receipts with plain write_text")
for required in (
    "/var/lib/oasis7/p2p-public-testnet/validator-pair-nonces.jsonl",
    "OASIS7_VALIDATOR_PAIR_NONCE_LEDGER",
    "mode `0600`",
    "nonce_ledger_path",
):
    if required not in runbook_source:
        raise SystemExit(f"runbook is missing nonce-ledger prerequisite: {required}")
spec = importlib.util.spec_from_file_location("pair_rebuild_contract_fixture", test_path)
if spec is None or spec.loader is None:
    raise SystemExit("cannot load canonical local fixture")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

fixture = module.ValidatorPairRebuildContractTests(methodName="runTest")
fixture.setUp()
try:
    base = fixture._base_args()
    # _base_args is [wrapper, plan, ...].  Replace only the owned dispatcher
    # while retaining the complete current plan argument set.
    shell = root / "scripts" / "p2p-public-testnet-rebuild-validators.sh"
    command = [str(shell), "plan", *base[2:]]
    env = dict(os.environ)
    with tempfile.TemporaryDirectory(prefix="oasis7-rebuild-receipt-contract-bin-") as fake_bin:
        fake = Path(fake_bin)
        # Plan now performs bounded live SSH/GitHub reads. Keep the fixture's
        # loopback-only fake SSH/GitHub tools and forbid only systemd activity.
        for command_name in ("systemctl",):
            path = fake / command_name
            path.write_text("#!/usr/bin/env bash\nprintf '%s\n' \"$0\" >>\"${O7_RECEIPT_FORBIDDEN_LOG:?}\"\nexit 91\n", encoding="utf-8")
            path.chmod(0o755)
        forbidden = fake / "forbidden.log"
        env["O7_RECEIPT_FORBIDDEN_LOG"] = str(forbidden)
        env["PATH"] = f"{fake}:{env['PATH']}"

        first = subprocess.run(command, text=True, capture_output=True, env=env, check=False)
        second = subprocess.run(command, text=True, capture_output=True, env=env, check=False)
    if first.returncode != 0:
        raise SystemExit(f"shell plan failed: {first.stderr}")
    if second.returncode != 0:
        raise SystemExit(f"second shell plan failed: {second.stderr}")
    if first.stdout != second.stdout:
        raise SystemExit("plan output is not byte-stable")
    if forbidden.exists() and forbidden.read_text(encoding="utf-8").strip():
        raise SystemExit("plan mode reached systemd")

    receipt = json.loads(first.stdout)
    contract = receipt.get("receipt_contract")
    if receipt.get("schema_version") != "oasis7.validator_pair_rebuild_plan.v1":
        raise SystemExit("plan schema is not validator_pair_rebuild_plan.v1")
    if not isinstance(contract, dict):
        raise SystemExit("receipt contract envelope is missing")
    if contract.get("remote_activity") != "executor-owned-direct-ssh-read-only" or contract.get("systemd_activity") is not False or contract.get("destructive_activity") is not False:
        raise SystemExit("plan activity gates do not describe bounded read-only observation")
    if contract.get("mutation_order") != ["storage-205", "sequencer-204"]:
        raise SystemExit("mutation order is not explicit")
    if contract.get("startup_order") != ["sequencer-204", "storage-205"]:
        raise SystemExit("startup order is not explicit")
    if contract.get("rollback_boundary", {}).get("restore_deleted_chain_state") is not False:
        raise SystemExit("rollback boundary permits deleted chain-state restore")
    for role in ("storage-205", "sequencer-204"):
        node = contract["nodes"][role]
        if not node["inventory"]["entry_class_equation"]:
            raise SystemExit(f"inventory class equation failed for {role}")
        if not node["forensic_backup"]["non_seed_proof"]["machine_checkable"]:
            raise SystemExit(f"backup non-seed proof missing for {role}")
        if not node["post_delete_absence_proof"]["required"]:
            raise SystemExit(f"post-delete proof missing for {role}")
        targets = node["destructive_target_resolution"]
        if not targets or not all(item["resolution_proof"]["target_under_root"] for item in targets):
            raise SystemExit(f"destructive path proof failed for {role}")

    expected_digest = hashlib.sha256(
        json.dumps(
            {key: item for key, item in receipt.items() if key != "plan_digest"},
            ensure_ascii=True,
            sort_keys=True,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    if receipt.get("plan_digest") != expected_digest:
        raise SystemExit("plan digest does not bind the receipt envelope")

    adapter = fixture._write_host_adapter()
    transaction = fixture.out / "transaction.json"
    applied = subprocess.run(
        [str(shell), "apply", "--transaction", str(transaction), "--host-adapter", str(adapter)],
        text=True,
        capture_output=True,
        env=env,
        check=False,
    )
    if applied.returncode != 0:
        raise SystemExit(f"shell apply failed: {applied.stderr}")
    applied_receipt = json.loads(applied.stdout)
    applied_contract = applied_receipt.get("receipt_contract", {})
    if applied_contract.get("mode") != "apply":
        raise SystemExit("apply receipt envelope mode is missing")
    if applied_contract.get("same_window_fleet_health", {}).get("status") != "verified":
        raise SystemExit("apply receipt is missing same-window fleet health")
    if not applied_contract.get("host_receipts"):
        raise SystemExit("apply receipt is missing phase receipts")

    rolled_back = subprocess.run(
        [str(shell), "rollback", "--transaction", str(transaction), "--host-adapter", str(adapter)],
        text=True,
        capture_output=True,
        env=env,
        check=False,
    )
    if rolled_back.returncode != 0:
        raise SystemExit(f"shell rollback failed: {rolled_back.stderr}")
    rollback_receipt = json.loads(rolled_back.stdout)
    if rollback_receipt.get("receipt_contract", {}).get("rollback_boundary", {}).get("restore_deleted_chain_state") is not False:
        raise SystemExit("rollback receipt weakens deleted-chain-state boundary")
finally:
    fixture.tearDown()

print("ok: governed shell plan receipt is deterministic and bounded live observation is disclosed")
PY
