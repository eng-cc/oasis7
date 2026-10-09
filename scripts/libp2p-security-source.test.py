#!/usr/bin/env python3
"""Verify the fixed network source and byte-identical swarm compatibility snapshot."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
REVISION = "2d8497b2615086bd88018ef3a1fb32bc613a07ca"
URL = "https://github.com/libp2p/rust-libp2p"


class NetworkSourceContract(unittest.TestCase):
    def test_shared_facade_and_lock_sources(self):
        for crate in ("oasis7_net", "oasis7_node"):
            manifest = tomllib.loads((ROOT / "crates" / crate / "Cargo.toml").read_text())
            dep = manifest["dependencies"].get("libp2p") or manifest["target"]['cfg(not(target_arch = "wasm32"))']["dependencies"]["libp2p"]
            self.assertEqual((dep["git"], dep["rev"]), (URL, REVISION))
            self.assertFalse(dep["default-features"])
            self.assertTrue(dep["optional"])
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        for name in ("libp2p", "libp2p-core", "libp2p-rendezvous"):
            packages = [p for p in lock["package"] if p["name"] == name]
            self.assertEqual(len(packages), 1)
            self.assertEqual(packages[0]["source"], f"git+{URL}?rev={REVISION}#{REVISION}")
        timers = [p for p in lock["package"] if p["name"] == "futures-timer"]
        self.assertTrue(all(tuple(map(int, p["version"].split("."))) >= (3, 0, 4) for p in timers))

    def test_wasm_builder_includes_workspace_path_patch(self):
        # cargo install resolves root patches even for a non-network tool.
        root = tomllib.loads((ROOT / "Cargo.toml").read_text())
        patch = root["patch"][URL]["libp2p-swarm"]["path"]
        docker = (ROOT / "docker/wasm-builder/Dockerfile").read_text().splitlines()
        self.assertIn(f"COPY {patch} /opt/oasis7/{patch}", docker)
        self.assertTrue((ROOT / patch / "Cargo.toml").is_file())
        context_policy = (ROOT / ".dockerignore").read_text().splitlines()
        self.assertIn(f"!{patch}/", context_policy)
        self.assertIn(f"!{patch}/**", context_policy)

    def test_swarm_snapshot_has_no_source_changes(self):
        vendor = ROOT / "vendor-libp2p-swarm-0.48.0"
        hashes = json.loads((vendor / "upstream-source-sha256.json").read_text())
        actual = {str(p.relative_to(vendor)): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in (vendor / "src").rglob("*.rs")}
        self.assertEqual(actual, hashes)
        manifest = tomllib.loads((vendor / "Cargo.toml").read_text())
        self.assertEqual(manifest["package"]["metadata"]["oasis7-compat"]["upstream-revision"], REVISION)
        self.assertEqual(manifest["dependencies"]["futures-timer"]["version"], "3.0.4")
        self.assertEqual(manifest["target"]['cfg(target_family="wasm")']["dependencies"]["wasm-bindgen-futures"]["version"], "0.4")


if __name__ == "__main__":
    unittest.main()
