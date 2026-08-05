#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

python3 - "$ROOT_DIR/scripts/p2p-observer-checkpoint-closure-probe.py" <<'PY'
import importlib.util
import hashlib
import json
import os
import socket
import sys
import tempfile
from pathlib import Path

module_path = Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("checkpoint_closure_probe", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)

port = module.allocate_loopback_tcp_port()
assert isinstance(port, int)
assert 1 <= port <= 65535

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
    listener.bind(("127.0.0.1", port))

with tempfile.TemporaryDirectory() as temp:
    app_root = Path(temp)
    config = app_root / "config"
    config.mkdir()
    runtime = app_root / "release/bin/oasis7_chain_runtime"
    runtime.parent.mkdir(parents=True)
    runtime.write_bytes(b"new-runtime")
    bundle = config / "public-testnet-governed-bootstrap-bundle-2026-06-06.json"
    bundle.write_text(
        json.dumps({"runtime_build": {"sha256": "0" * 64}}), encoding="utf-8"
    )
    buildinfo = {
        "commit": "a" * 40,
        "package_version": "0.0.0+testnet.1.aaaaaaaaaaaa",
        "run_id": "123",
    }
    module.bind_probe_runtime_metadata(app_root, runtime, buildinfo)
    updated = json.loads(bundle.read_text(encoding="utf-8"))
    assert updated["runtime_build"]["sha256"] == hashlib.sha256(b"new-runtime").hexdigest()
    assert updated["runtime_build"]["path"] == str(runtime)
    assert updated["runtime_build"]["package_version"] == buildinfo["package_version"]
    assert updated["runtime_build"]["run_id"] == buildinfo["run_id"]

gossip_port = module.allocate_udp_port()
assert 1 <= gossip_port <= 65535
with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as listener:
    listener.bind(("0.0.0.0", gossip_port))

overrides = module.clean_room_network_overrides(43123, gossip_port)
assert overrides["STATUS_BIND"] == "127.0.0.1:43123"
assert overrides["NODE_GOSSIP_BIND"] == f"0.0.0.0:{gossip_port}"
assert overrides["REPLICATION_NETWORK_LISTEN_ADDRS_CSV"] == "/ip4/127.0.0.1/tcp/0"
assert overrides["TRAFFIC_MONITOR_ENABLE"] == "0"

with tempfile.TemporaryDirectory() as temp:
    temp_root = Path(temp)
    source_root = temp_root / "live-observer"
    app_root = temp_root / "fresh-probe"
    source_config = source_root / "config"
    source_config.mkdir(parents=True)
    (source_config / "node-keypair.toml").write_text("[node]\n", encoding="utf-8")
    (source_config / "manifest.json").write_text("{}\n", encoding="utf-8")
    (source_config / "genesis-validator-registry.json").write_text("{}\n", encoding="utf-8")
    governed_env = {
        "STACK_ROOT": str(source_root),
        "CONFIG_PATH": "$STACK_ROOT/config/node-keypair.toml",
        "NETWORK_TIER_MANIFEST_PATH": "${STACK_ROOT}/config/manifest.json",
        "GENESIS_VALIDATOR_REGISTRY_PATH": "$STACK_ROOT/config/genesis-validator-registry.json",
        "EXECUTION_WORLD_DIR": str(source_root / "data/world"),
        "EXECUTION_RECORDS_DIR": str(source_root / "data/execution-records"),
        "STORAGE_ROOT": str(source_root / "data/storage"),
        "RUNTIME_ROOT": str(source_root / "data/runtime-root"),
        "REPLICATION_ROOT": str(source_root / "data/replication-root"),
    }
    normalized = module.normalize_clean_room_environment(
        governed_env, source_root, app_root
    )
    assert normalized["STACK_ROOT"] == str(app_root)
    assert normalized["CONFIG_PATH"] == str(app_root / "config/node-keypair.toml")
    assert normalized["NETWORK_TIER_MANIFEST_PATH"] == str(app_root / "config/manifest.json")
    assert normalized["GENESIS_VALIDATOR_REGISTRY_PATH"] == str(
        app_root / "config/genesis-validator-registry.json"
    )
    for key in (
        "EXECUTION_WORLD_DIR",
        "EXECUTION_RECORDS_DIR",
        "STORAGE_ROOT",
        "RUNTIME_ROOT",
        "REPLICATION_ROOT",
    ):
        assert Path(normalized[key]).is_relative_to(app_root)
        assert not Path(normalized[key]).is_relative_to(source_root)
    os.environ["OASIS7_TEST_LIVE_MUTABLE_ROOT"] = str(source_root / "host-state")
    launch_env = module.clean_room_launch_environment(normalized, "test-nonce")
    assert "OASIS7_TEST_LIVE_MUTABLE_ROOT" not in launch_env
    assert launch_env["OASIS7_CHECKPOINT_PROBE_NONCE"] == "test-nonce"

with tempfile.TemporaryDirectory() as temp:
    temp_root = Path(temp)
    source_root = temp_root / "live-observer"
    source_config = source_root / "config"
    source_config.mkdir(parents=True)
    manifest_path = source_config / "manifest.json"
    manifest_path.write_text('{"tier":"testnet"}\n', encoding="utf-8")
    (source_config / "node.env").write_text(
        "WORLD_ID=checkpoint-world\n"
        "STACK_ROOT=" + str(source_root) + "\n"
        "CONFIG_PATH=$STACK_ROOT/config/node-keypair.toml\n"
        "NETWORK_TIER_MANIFEST_PATH=${STACK_ROOT}/config/manifest.json\n",
        encoding="utf-8",
    )
    (source_config / "node-keypair.toml").write_text("[node]\n", encoding="utf-8")
    (source_config / "public-testnet-governed-bootstrap-bundle-2026-06-06.json").write_text(
        "{}\n", encoding="utf-8"
    )
    rollout_manifest = temp_root / "rollout-manifest.json"
    rollout_manifest.write_text(
        json.dumps({"nodes": [{"name": "observer", "node_root": str(source_root)}]}),
        encoding="utf-8",
    )
    package_dir = temp_root / "package"
    package_dir.mkdir()
    (package_dir / "linux-x64-BUILDINFO").write_text(
        "commit=" + "a" * 40 + "\n"
        "package_version=0.0.0+testnet.1.aaaaaaaaaaaa\n"
        "run_id=123\n",
        encoding="utf-8",
    )
    launch_observation = {}
    original_package_runtime = module.package_runtime
    original_popen = module.subprocess.Popen

    def fake_package_runtime(_package_dir, app_root):
        runtime = app_root / "release/bin/oasis7_chain_runtime"
        runtime.parent.mkdir(parents=True)
        runtime.write_bytes(b"runtime")
        return runtime

    class FakeProcess:
        def __init__(self, _command, env):
            manifest = Path(env["NETWORK_TIER_MANIFEST_PATH"])
            launch_observation["manifest_path"] = manifest
            launch_observation["manifest_exists"] = manifest.is_file()
            receipt_dir = Path(env["REPLICATION_ROOT"]) / "checkpoint-verification"
            receipt_dir.mkdir(parents=True)
            binding = {"expected_content_hash": "hash", "expected_size_bytes": 1,
                       "observed_content_hash": "hash", "observed_size_bytes": 1}
            receipt = {"schema_version": module.SCHEMA, "probe_nonce": env["OASIS7_CHECKPOINT_PROBE_NONCE"],
                       "world_id": "checkpoint-world", "height": 1,
                       "execution_block_hash": "block", "execution_state_root": "state", "manifest_hash": "manifest",
                       "objects": [binding], "fetch_observations": [{"source": "network_fetch", "content_hash": "hash",
                           "observed_content_hash": "hash", "observed_size_bytes": 1, "response_found": True,
                           "signed_request": True, "connected_candidate_ids": ["candidate"]}]}
            (receipt_dir / "runtime.json").write_text(json.dumps(receipt), encoding="utf-8")
        def poll(self): return None
        def send_signal(self, _signal): pass
        def wait(self, timeout): return 0

    module.package_runtime = fake_package_runtime
    module.subprocess.Popen = FakeProcess
    try:
        result = module.run_probe(type("Args", (), {"timeout_secs": 1, "manifest": rollout_manifest,
            "package_dir": package_dir})())
    finally:
        module.package_runtime = original_package_runtime
        module.subprocess.Popen = original_popen
    assert launch_observation["manifest_exists"] is True
    assert launch_observation["manifest_path"].name == "manifest.json"
    assert result["input_bindings"]["network_tier_manifest_sha256"] == module.sha256(manifest_path)
PY

echo "ok: checkpoint closure probe allocates a usable non-zero loopback status port"
