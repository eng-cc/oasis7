#!/usr/bin/env python3
"""A selected malformed terminal receipt must fail before readiness writes."""
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    "prior_receipt_fixture", ROOT / "scripts/pm/readiness-repeat.test.py")
repeat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repeat)


def snapshot(fixture):
    return (fixture.state_path.read_bytes(), fixture.mapping_path.read_bytes(),
            {str(path.relative_to(fixture.receipt_root)): path.read_bytes()
             for path in fixture.receipt_root.rglob("*") if path.is_file()})


class PriorReceiptRed(unittest.TestCase):
    def test_selected_malformed_receipt_rejects_without_readiness_proof_write(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-prior-receipt-") as temp:
            fixture = repeat.PublisherPermissionFixture(Path(temp))
            fixture.prepare_native_v2_readiness()
            (fixture.receipt_root / "readiness-proof.json").unlink()
            raw = b'{"receipt_type":"oasis7_terminal_delivery","unknown":true}'
            (fixture.receipt_root / "terminal-delivery-receipt.json").write_bytes(raw)
            mapping = fixture.mapping()
            record = mapping["tasks"][repeat.protocol.UID]
            record["workflow_phase"] = "post_merge_done"
            record["phase_receipt_type"] = {"post_merge_done": "oasis7_terminal_delivery"}
            record["phase_receipt_sha256"] = {
                "post_merge_done": hashlib.sha256(raw).hexdigest()}
            fixture.mapping_path.write_text(json.dumps(mapping))
            before = snapshot(fixture)
            result = fixture.run_finalizer()
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("selected v2 terminal delivery mapping/receipt is not resumable", result.stderr,
                          "expected the real producer's prior-receipt rejection: " + result.stderr)
            after = snapshot(fixture)
            self.assertEqual(after, before,
                "rejected selected prior receipt changed evidence; added artifacts=" +
                repr(sorted(set(after[2]) - set(before[2]))) +
                "; producer exit=" + str(result.returncode) + "; stderr=" + result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
