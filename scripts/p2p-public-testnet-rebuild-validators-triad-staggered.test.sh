#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

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


root = Path(sys.argv[1]).resolve()
wrapper = root / "scripts" / "p2p-public-testnet-rebuild-validators.sh"
executor = root / "scripts" / "p2p-public-testnet-validator-pair-rebuild.py"


def load_executor_module():
    spec = importlib.util.spec_from_file_location("triad_staggered_contract_executor", executor)
    if spec is None or spec.loader is None:
        raise SystemExit("unable to load governed triad executor")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


executor_module = load_executor_module()

wrapper_source = wrapper.read_text(encoding="utf-8")
executor_source = executor.read_text(encoding="utf-8")
help_result = subprocess.run(
    [str(wrapper), "--help"], text=True, capture_output=True, check=False
)
if help_result.returncode != 0:
    raise SystemExit(f"validator rebuild --help failed: {help_result.stderr}")
for required in ("--execution-mode", "triad_staggered", "max", "stopped"):
    if required not in help_result.stdout:
        raise SystemExit(f"governed help is missing triad execution contract: {required}")
for required in (
    'PAIR_EXECUTION_MODE = "pair"',
    'TRIAD_STAGGERED_EXECUTION_MODE = "triad_staggered"',
    "--execution-mode",
    "apply_staggered_transaction",
    "rollback_staggered_transaction",
):
    if required not in executor_source:
        raise SystemExit(f"executor is missing explicit execution-mode contract: {required}")
main_source = executor_source[executor_source.index("def main()") :]
for required in ("TRIAD_STAGGERED_EXECUTION_MODE", "apply_staggered_transaction", "rollback_staggered_transaction"):
    if required not in main_source:
        raise SystemExit(f"executor main does not dispatch triad mode explicitly: {required}")
if "--sequencer-ssh-host" not in wrapper_source or "--storage-ssh-host" not in wrapper_source:
    raise SystemExit("historical SSH fixture route is not visible for the no-fallback guard")

reset_targets = [
    "data/execution-records",
    "data/execution-world",
    "data/execution-world-simulator-mirror",
    "data/storage",
    "data/runtime-root",
    "data/replication-root",
    "output/chain-runtime",
    "output/node-distfs",
]


def digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def run(command: list[str], env: dict[str, str], label: str) -> dict[str, object]:
    result = subprocess.run(command, text=True, capture_output=True, env=env, check=False)
    if result.returncode != 0:
        raise SystemExit(f"{label} failed ({result.returncode}): {result.stderr}")
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise SystemExit(f"{label} did not emit JSON: {error}: {result.stdout}") from error
    if not isinstance(value, dict):
        raise SystemExit(f"{label} did not emit a JSON object")
    return value


def canonical_digest(value: dict[str, object]) -> str:
    return hashlib.sha256(
        json.dumps(
            {key: item for key, item in value.items() if key != "canonical_digest"},
            ensure_ascii=True,
            sort_keys=True,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()


TRIAD_ROLLBACK_CONTRACT = {
    "strategy": "staggered-target-only-cleanup",
    "required_on_gate_failure": True,
    "target_only_cleanup": True,
    "restore_deleted_chain_state": False,
    "restore_only_forensic_snapshot": True,
}


def write_transaction(path: Path, *, phase: str, execution_mode: str = "triad_staggered") -> None:
    value: dict[str, object] = {
        "schema_version": "oasis7.validator_pair_rebuild_transaction.v1",
        "execution_mode": execution_mode,
        "phase": phase,
        "rollback": dict(TRIAD_ROLLBACK_CONTRACT),
    }
    value["canonical_digest"] = canonical_digest(value)
    path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")


def assert_triad_receipt(value: dict[str, object], route: str) -> None:
    if value.get("execution_mode") != "triad_staggered":
        raise SystemExit(f"{route} receipt lost triad_staggered execution mode")
    contract = value.get("receipt_contract")
    if not isinstance(contract, dict):
        raise SystemExit(f"{route} receipt is missing receipt_contract")
    if contract.get("execution_mode") != "triad_staggered":
        raise SystemExit(f"{route} receipt contract is not mode-bound")
    if contract.get("mutation_order") != ["storage-205", "sequencer-204"]:
        raise SystemExit(f"{route} mutation order is not storage then sequencer")
    if contract.get("startup_order") != ["storage-205", "sequencer-204"]:
        raise SystemExit(f"{route} startup order is not storage then sequencer")
    if contract.get("max_simultaneously_stopped_validators") != 1:
        raise SystemExit(f"{route} receipt does not prove max-one-stopped")
    if contract.get("observer_mutation") is not False:
        raise SystemExit(f"{route} receipt does not prove observer hold")
    if contract.get("historical_ssh_fallback") is not False:
        raise SystemExit(f"{route} receipt does not prove no historical SSH fallback")
    phases = contract.get("phase_order")
    expected_phases = [
        "staggered-preflight",
        "staggered-storage",
        "staggered-sequencer",
    ]
    if route == "rollback":
        expected_phases.append("staggered-rollback")
    if phases != expected_phases:
        raise SystemExit(f"{route} phase ordering is not deterministic: {phases!r}")


with tempfile.TemporaryDirectory(prefix="oasis7-triad-staggered-contract-") as temp_dir:
    temp = Path(temp_dir)
    roots = {role: temp / role for role in ("storage-205", "sequencer-204")}
    for node_root in roots.values():
        (node_root / "backups").mkdir(parents=True)
    transaction = temp / "transaction.json"
    args_log = temp / "executor-args.jsonl"
    forbidden_log = temp / "forbidden.log"
    fake_bin = temp / "bin"
    fake_bin.mkdir()

    fake_python = fake_bin / "python3"
    roots_literal = repr({role: str(path) for role, path in roots.items()})
    reset_targets_literal = repr(reset_targets)
    temp_literal = repr(str(temp))
    fake_python.write_text(
        f"""#!{sys.executable}
import hashlib
import json
import os
import sys
from pathlib import Path

executor = {str(executor)!r}
args = sys.argv[1:]
if not args or Path(args[0]).resolve() != Path(executor).resolve():
    os.execv({sys.executable!r}, [{sys.executable!r}, *args])

with Path({str(args_log)!r}).open('a', encoding='utf-8') as handle:
    handle.write(json.dumps(args, separators=(',', ':')) + '\\n')

route = args[1]
try:
    execution_mode = args[args.index('--execution-mode') + 1]
except (ValueError, IndexError):
    execution_mode = 'pair'
bad_pair = os.environ.get('O7_BAD_PAIR_MODE') == '1'
bad_max = os.environ.get('O7_BAD_MAX_STOPPED') == '1'
bad_phase_order = os.environ.get('O7_BAD_PHASE_ORDER') == '1'
reported_mode = 'triad_staggered' if execution_mode == 'triad_staggered' else 'pair'
if bad_pair and execution_mode == 'pair':
    reported_mode = 'triad_staggered'
max_stopped = 2 if bad_max else 1
phase_order = [
    'staggered-preflight',
    'staggered-storage',
    'staggered-sequencer',
]
if route == 'rollback':
    phase_order.append('staggered-rollback')
if bad_phase_order:
    phase_order.reverse()

reset_targets = {reset_targets_literal}
target_digest = hashlib.sha256(json.dumps(reset_targets, separators=(',', ':')).encode()).hexdigest()
nodes = {{}}
for role, root in {roots_literal}.items():
    nodes[role] = {{'root': root, 'transport': 'local'}}
staged = {{}}
backup = {{}}
for role, node in nodes.items():
    root_path = Path(node['root'])
    manifest_root = root_path / 'backups' / 'fixture'
    manifest_root.mkdir(parents=True, exist_ok=True)
    manifest = manifest_root / 'manifest.json'
    manifest.write_text('{{}}\\n', encoding='utf-8')
    manifest_sha = hashlib.sha256(manifest.read_bytes()).hexdigest()
    staged[role] = {{'post_delete_absence': {{
        'absent': True,
        'target_set': reset_targets,
        'target_set_sha256': target_digest,
    }}}}
    backup[role] = {{
        'manifest': str(manifest),
        'backup_root': str(manifest_root),
        'manifest_sha256': manifest_sha,
    }}

value = {{
    'schema_version': 'oasis7.validator_pair_rebuild_transaction.v1',
    'execution_mode': reported_mode,
    'phase': 'rolled_back' if route in ('resume', 'rollback') else 'prepared',
    'mutation_order': ['storage-205', 'sequencer-204'],
    'startup_order': ['storage-205', 'sequencer-204'],
    'max_simultaneously_stopped_validators': max_stopped,
    'nodes': nodes,
    'staged': staged,
    'backup': backup,
    'capacity': {{'storage-205': {{}}, 'sequencer-204': {{}}}},
    'package': {{'directory': str(Path({temp_literal}) / 'package')}},
    'network': {{'governed': {{}}}},
    'provenance': {{'epoch': 'fixture'}},
    'host_receipt': {{'captured_at': '2026-09-19T00:00:00Z'}},
    'receipt_contract': {{
        'schema_version': 'oasis7.validator_pair_rebuild_receipt_contract.v1',
        'execution_mode': reported_mode,
        'max_simultaneously_stopped_validators': max_stopped,
        'phase_order': phase_order,
        'observer_mutation': False,
        'historical_ssh_fallback': False,
    }},
}}
print(json.dumps(value, separators=(',', ':')))
""",
        encoding="utf-8",
    )
    fake_python.chmod(0o755)
    for tool in ("ssh", "sshpass", "curl", "systemctl"):
        forbidden = fake_bin / tool
        forbidden.write_text(
            f"#!/usr/bin/env bash\nprintf '%s\\n' {tool!r} >>\"{forbidden_log}\"\nexit 91\n",
            encoding="utf-8",
        )
        forbidden.chmod(0o755)

    env = dict(os.environ)
    env["PATH"] = f"{fake_bin}:{env['PATH']}"
    env["OASIS7_VALIDATOR_PAIR_NONCE_LEDGER"] = str(temp / "nonce-ledger.jsonl")
    env["O7_TRIAD_FORBIDDEN_LOG"] = str(forbidden_log)

    common = [
        "--execution-mode",
        "triad_staggered",
        "--transaction",
        str(transaction),
        "--host-adapter",
        str(temp / "host-adapter.py"),
        "--request",
        str(temp / "live-request.json"),
        "--known-hosts",
        str(temp / "known-hosts"),
        "--credential-env",
        "O7_TRIAD_TEST_SECRET",
    ]
    plan = run(
        [str(wrapper), "plan", "--execution-mode", "triad_staggered", "--out-dir", str(temp / "plan")],
        env,
        "triad plan",
    )
    assert_triad_receipt(plan, "plan")
    transaction.write_text(json.dumps(plan) + "\n", encoding="utf-8")

    for route in ("apply", "resume", "rollback"):
        receipt = run([str(wrapper), route, *common], env, f"triad {route}")
        assert_triad_receipt(receipt, route)

    invocations = [json.loads(line) for line in args_log.read_text(encoding="utf-8").splitlines()]
    if len(invocations) != 4:
        raise SystemExit(f"expected four governed executor invocations, got {len(invocations)}")
    for invocation in invocations:
        if "--execution-mode" not in invocation or invocation[invocation.index("--execution-mode") + 1] != "triad_staggered":
            raise SystemExit(f"triad mode was not forwarded verbatim: {invocation!r}")
        if any(flag in invocation for flag in ("--sequencer-ssh-host", "--storage-ssh-host", "--sequencer-sshpass-env", "--storage-sshpass-env")):
            raise SystemExit(f"triad route fell back to historical SSH arguments: {invocation!r}")
    if forbidden_log.exists() and forbidden_log.read_text(encoding="utf-8").strip():
        raise SystemExit("triad governed route invoked legacy SSH/systemd/curl fallback")

    bad_max_env = dict(env)
    bad_max_env["O7_BAD_MAX_STOPPED"] = "1"
    rejected_max = subprocess.run(
        [str(wrapper), "plan", "--execution-mode", "triad_staggered", "--out-dir", str(temp / "bad-max")],
        text=True,
        capture_output=True,
        env=bad_max_env,
        check=False,
    )
    if rejected_max.returncode == 0 or "max" not in (rejected_max.stderr + rejected_max.stdout).lower():
        raise SystemExit("triad route accepted a receipt with max_simultaneously_stopped_validators > 1")

    bad_phase_env = dict(env)
    bad_phase_env["O7_BAD_PHASE_ORDER"] = "1"
    rejected_phase = subprocess.run(
        [str(wrapper), "plan", "--execution-mode", "triad_staggered", "--out-dir", str(temp / "bad-phase")],
        text=True,
        capture_output=True,
        env=bad_phase_env,
        check=False,
    )
    if rejected_phase.returncode == 0 or "phase" not in (rejected_phase.stderr + rejected_phase.stdout).lower():
        raise SystemExit("triad route accepted non-deterministic phase ordering")

    bad_pair_env = dict(env)
    bad_pair_env["O7_BAD_PAIR_MODE"] = "1"
    rejected_pair = subprocess.run(
        [str(wrapper), "plan", "--execution-mode", "pair", "--out-dir", str(temp / "bad-pair")],
        text=True,
        capture_output=True,
        env=bad_pair_env,
        check=False,
    )
    if rejected_pair.returncode == 0 or "mode" not in (rejected_pair.stderr + rejected_pair.stdout).lower():
        raise SystemExit("pair route accepted a receipt bound to triad_staggered mode")


def assert_mode_mismatch_fails_before_callback() -> None:
    """A caller-selected pair mode must fail before any adapter callback."""
    with tempfile.TemporaryDirectory(prefix="oasis7-triad-mode-mismatch-") as temp_dir:
        temp = Path(temp_dir)
        request = temp / "request.json"
        request.write_text("{}\n", encoding="utf-8")
        known_hosts = temp / "known-hosts"
        known_hosts.write_text("fixture\n", encoding="utf-8")
        env = dict(os.environ)
        env["OASIS7_VALIDATOR_PAIR_NONCE_LEDGER"] = str(temp / "nonce-ledger.jsonl")
        env["O7_MODE_TEST_SECRET"] = "fixture-secret"
        for requested_mode, persisted_mode in (
            ("pair", "triad_staggered"),
            ("triad_staggered", "pair"),
        ):
            transaction = temp / f"transaction-{requested_mode}-{persisted_mode}.json"
            write_transaction(transaction, phase="planned", execution_mode=persisted_mode)
            callback_log = temp / f"callback-{requested_mode}-{persisted_mode}.log"
            adapter = temp / f"host-adapter-{requested_mode}-{persisted_mode}.py"
            adapter.write_text(
                "#!/usr/bin/env python3\n"
                f"from pathlib import Path\nPath({str(callback_log)!r}).open('a', encoding='utf-8').write('callback\\n')\n"
                "print('{}')\n",
                encoding="utf-8",
            )
            adapter.chmod(0o755)
            for route in ("apply", "resume", "rollback"):
                command = [
                    str(wrapper),
                    route,
                    "--execution-mode",
                    requested_mode,
                    "--transaction",
                    str(transaction),
                    "--host-adapter",
                    str(adapter),
                ]
                if route == "resume":
                    command.extend(
                        [
                            "--request",
                            str(request),
                            "--known-hosts",
                            str(known_hosts),
                            "--credential-env",
                            "O7_MODE_TEST_SECRET",
                        ]
                    )
                result = subprocess.run(command, text=True, capture_output=True, env=env, check=False)
                combined = result.stderr + result.stdout
                if result.returncode == 0 or "mode" not in combined.lower():
                    raise SystemExit(
                        f"{route} accepted or misreported explicit mode mismatch "
                        f"requested={requested_mode} persisted={persisted_mode}: {combined}"
                    )
            if callback_log.exists() and callback_log.read_text(encoding="utf-8"):
                raise SystemExit(
                    f"mode mismatch invoked the host adapter before rejection "
                    f"requested={requested_mode} persisted={persisted_mode}"
                )


def assert_executor_records_member_observations_around_callbacks() -> None:
    """Each member callback must be bracketed by executor-owned observations."""
    with tempfile.TemporaryDirectory(prefix="oasis7-triad-member-observation-") as temp_dir:
        temp = Path(temp_dir)
        roots = {role: temp / role for role in ("storage-205", "sequencer-204")}
        for root_path in roots.values():
            root_path.mkdir()
        transaction = {
            "execution_mode": "triad_staggered",
            "transaction_id": "triad-observation-fixture",
            "nodes": {role: {"root": str(root_path)} for role, root_path in roots.items()},
            "capacity": {role: {} for role in roots},
            "capacity_apply": None,
            "backup": {},
            "staged": {},
            "staggered_completed_roles": [],
        }
        events: list[tuple[str, str]] = []
        originals = {
            "refresh_capacity": executor_module.refresh_capacity,
            "snapshot_node": executor_module.snapshot_node,
            "run_host_adapter": executor_module.run_host_adapter,
            "_record_staggered_stage_receipt": executor_module._record_staggered_stage_receipt,
            "_record_staggered_live_reobserve": executor_module._record_staggered_live_reobserve,
            "write_json": executor_module.write_json,
        }
        executor_module.refresh_capacity = lambda root, capacity, role: {"role": role, "verified": True}
        executor_module.snapshot_node = lambda node, transaction_id: {"role": node["root"], "snapshot": True}

        def fake_adapter(adapter, path, value, phase):
            events.append(("adapter", phase))
            return {"phase": phase}

        def fake_observe(value, direct_args, *, observation_key="initial"):
            events.append(("observe", observation_key))
            observations = value.setdefault("staggered_live_observations", {})
            observations[observation_key] = {"observation_state": "live"}
            return {"observation_state": "live"}

        executor_module.run_host_adapter = fake_adapter
        executor_module._record_staggered_stage_receipt = lambda value, role, receipt: None
        executor_module._record_staggered_live_reobserve = fake_observe
        executor_module.write_json = lambda path, value: None
        try:
            executor_module._continue_staggered_transaction(
                transaction,
                temp / "transaction.json",
                temp / "host-adapter.py",
                None,
                fresh_direct_observation=False,
            )
        finally:
            for name, value in originals.items():
                setattr(executor_module, name, value)
        expected = [
            ("observe", "before-staggered-preflight"),
            ("adapter", "staggered-preflight"),
            ("observe", "before-staggered-storage"),
            ("adapter", "staggered-storage"),
            ("observe", "after-staggered-storage"),
            ("observe", "before-staggered-sequencer"),
            ("adapter", "staggered-sequencer"),
            ("observe", "after-staggered-sequencer"),
        ]
        if events != expected:
            raise SystemExit(f"executor member observation/callback ordering drifted: {events!r}")
        if set(transaction.get("staggered_live_observations", {})) != {
            "before-staggered-preflight",
            "before-staggered-storage",
            "after-staggered-storage",
            "before-staggered-sequencer",
            "after-staggered-sequencer",
        }:
            raise SystemExit("executor did not persist every per-phase live observation")


def assert_executor_records_rollback_observations_before_and_after_callback() -> None:
    """Rollback must independently observe the mixed state on both sides."""
    with tempfile.TemporaryDirectory(prefix="oasis7-triad-rollback-observation-") as temp_dir:
        temp = Path(temp_dir)
        transaction = {
            "schema_version": executor_module.SCHEMA,
            "execution_mode": "triad_staggered",
            "transaction_id": "triad-rollback-fixture",
            "phase": "rollback_required",
            "rollback": dict(TRIAD_ROLLBACK_CONTRACT),
            "backup": {"storage-205": {"forensic": True}},
            "staggered_active_role": "storage-205",
        }
        path = temp / "transaction.json"
        transaction["canonical_digest"] = executor_module.canonical_digest(transaction)
        path.write_text(json.dumps(transaction, sort_keys=True) + "\n", encoding="utf-8")
        events: list[tuple[str, str]] = []
        originals = {
            "_validate_persisted_backup_refs": executor_module._validate_persisted_backup_refs,
            "_validate_adapter_callback_state": executor_module._validate_adapter_callback_state,
            "_record_staggered_rollback_reobserve": executor_module._record_staggered_rollback_reobserve,
            "run_host_adapter": executor_module.run_host_adapter,
            "write_json": executor_module.write_json,
        }
        executor_module._validate_persisted_backup_refs = lambda value: None
        executor_module._validate_adapter_callback_state = lambda value: None

        def fake_rollback_observe(value, direct_args, failed_role, *, observation_key="before"):
            events.append(("observe", observation_key))
            observations = value.setdefault("staggered_rollback_observations", {})
            observations[observation_key] = {"observation_state": "mixed", "stopped_role": failed_role}
            return {"observation_state": "mixed", "stopped_role": failed_role}

        def fake_rollback_adapter(adapter, transaction_path, value, phase):
            events.append(("adapter", phase))
            return {"phase": phase}

        executor_module._record_staggered_rollback_reobserve = fake_rollback_observe
        executor_module.run_host_adapter = fake_rollback_adapter
        executor_module.write_json = lambda path, value: None
        try:
            result = executor_module.rollback_staggered_transaction(
                path,
                temp / "host-adapter.py",
                None,
            )
        finally:
            for name, value in originals.items():
                setattr(executor_module, name, value)
        expected = [
            ("observe", "before"),
            ("adapter", "staggered-rollback"),
            ("observe", "after"),
        ]
        if events != expected:
            raise SystemExit(f"rollback observation/callback ordering drifted: {events!r}")
        if set(result.get("staggered_rollback_observations", {})) != {"before", "after"}:
            raise SystemExit("rollback did not persist both executor-owned observations")
        if result.get("rollback", {}).get("strategy") != TRIAD_ROLLBACK_CONTRACT["strategy"]:
            raise SystemExit("rollback result lost the triad target-only strategy")


def assert_resume_accepts_persisted_member_phase_names() -> None:
    """Persisted active-member phase names must be resumable past phase parsing."""
    for phase in ("staggered_storage-205_in_progress", "staggered_sequencer-204_in_progress"):
        with tempfile.TemporaryDirectory(prefix="oasis7-triad-resume-phase-") as temp_dir:
            path = Path(temp_dir) / "transaction.json"
            write_transaction(path, phase=phase)
            try:
                executor_module.resume_staggered_transaction(path, None, None)
            except SystemExit as error:
                message = str(error)
                if "not resumable" in message:
                    raise SystemExit(f"resume rejected persisted active-member phase {phase}: {message}") from error
            else:
                raise SystemExit(f"resume unexpectedly completed incomplete fixture phase {phase}")


def assert_rollback_contract_is_target_only_and_non_restoring() -> None:
    valid = {"rollback": dict(TRIAD_ROLLBACK_CONTRACT)}
    executor_module._validate_staggered_rollback_contract(valid)
    invalid = {"rollback": {**TRIAD_ROLLBACK_CONTRACT, "strategy": "same-filesystem-full-snapshot"}}
    try:
        executor_module._validate_staggered_rollback_contract(invalid)
    except SystemExit as error:
        if "target-only" not in str(error):
            raise SystemExit(f"rollback contract rejected with an unrelated error: {error}") from error
    else:
        raise SystemExit("triad rollback accepted the pair full-snapshot strategy")


def assert_legacy_destructive_entrypoint_fails_closed() -> None:
    """The historical positional SSH route must not reach destructive parsing."""
    with tempfile.TemporaryDirectory(prefix="oasis7-triad-legacy-route-") as temp_dir:
        result = subprocess.run(
            [str(wrapper), "--config-dir", str(Path(temp_dir) / "missing-config")],
            text=True,
            capture_output=True,
            check=False,
        )
        message = (result.stderr + result.stdout).lower()
        if result.returncode == 0 or not any(
            marker in message for marker in ("legacy", "historical", "positional")
        ) or not any(
            word in message for word in ("disabled", "rejected", "unsupported", "fail closed", "fail-closed")
        ):
            raise SystemExit(f"legacy destructive entrypoint did not fail closed: {result.stderr}{result.stdout}")


assert_mode_mismatch_fails_before_callback()
assert_executor_records_member_observations_around_callbacks()
assert_executor_records_rollback_observations_before_and_after_callback()
assert_resume_accepts_persisted_member_phase_names()
assert_rollback_contract_is_target_only_and_non_restoring()
assert_legacy_destructive_entrypoint_fails_closed()

print("ok: governed triad_staggered mode, observation, rollback, resume, and legacy-route contracts")
PY
