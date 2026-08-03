#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

python3 - "$ROOT_DIR/scripts/p2p-observer-checkpoint-closure-probe.py" <<'PY'
import importlib.util
import hashlib
import json
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
PY

echo "ok: checkpoint closure probe allocates a usable non-zero loopback status port"
