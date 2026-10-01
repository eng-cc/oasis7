#!/usr/bin/env python3
"""Real CLI coverage for the canonical source cleanup policy."""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import ImageChops

from test_six_view_to_voxel_model import PIPELINE, SCRIPT, make_fixture


class SourceCleanupCliTests(unittest.TestCase):
    def test_removed_alias_is_rejected_before_output_creation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "output"
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--input", "missing.png",
                 "--out-dir", str(output), "--disable-source-cleanup"],
                text=True, capture_output=True,
            )
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("unrecognized arguments: --disable-source-cleanup", result.stderr)
            self.assertFalse(output.exists())

    def test_policy_off_preserves_source_masks_and_reports_off(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Path(temporary) / "fixture.png"
            make_fixture(fixture)
            original = fixture.read_bytes()
            cells = PIPELINE.split_six_grid(PIPELINE.Image.open(fixture))
            # A plan-view foreground patch outside the side-view support must survive off.
            for view in PIPELINE.PLAN_VIEW_ORDER:
                cells[view].paste((240, 240, 240, 255), (5, 5, 15, 15))
            masks, validation = PIPELINE.build_masks(
                cells, alpha_threshold=24, mask_threshold=28, cleanup_source=False,
            )
            cleaned_masks, _ = PIPELINE.build_masks(
                cells, alpha_threshold=24, mask_threshold=28, cleanup_source=True,
            )
            for view in PIPELINE.PLAN_VIEW_ORDER:
                raw = PIPELINE.foreground_mask(cells[view], alpha_threshold=24, mask_threshold=28)
                expected = PIPELINE.crop_mask(raw)
                self.assertEqual(masks[view].bbox, expected.bbox)
                self.assertIsNone(ImageChops.difference(masks[view].mask, expected.mask).getbbox())
                self.assertNotEqual(masks[view].bbox, cleaned_masks[view].bbox)
            self.assertFalse(validation["source_cleanup_enabled"])
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--input", str(fixture),
                 "--out-dir", str(Path(temporary) / "output"), "--height-cm", "8",
                 "--source-cleanup-policy", "off"],
                text=True, capture_output=True, check=True,
            )
            payload = json.loads(result.stdout)
            report = json.loads(Path(payload["source_validation"]).read_text())
            metadata = json.loads(Path(payload["metadata"]).read_text())
            self.assertEqual(report, metadata["source_validation"])
            self.assertEqual(report["source_cleanup_policy"], "off")
            self.assertFalse(report["source_cleanup_enabled"])
            self.assertTrue(all(view["cleanup"] == "none" for view in report["views"].values()))
            self.assertEqual(fixture.read_bytes(), original)
            self.assertGreater(payload["voxel_count"], 0)


if __name__ == "__main__":
    unittest.main()
