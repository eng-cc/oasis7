#!/usr/bin/env python3
"""Exercise the native release version output before expensive bundle builds."""
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = (ROOT / ".github/workflows/release-packages.yml").read_text()
STEP = re.search(
    r"      - name: Resolve release version\n(.*?)(?=      - name:)",
    WORKFLOW, re.S,
).group(1)
RUN = STEP.split("        run: ", 1)[1]
if RUN.startswith("|\n"):
    RUN = "\n".join(line[10:] for line in RUN[2:].splitlines())


class ReleaseVersion(unittest.TestCase):
    def test_output_uses_bash_on_every_native_platform(self):
        self.assertIn("        shell: bash\n", STEP)
        self.assertLess(WORKFLOW.index("name: Resolve release version"),
                        WORKFLOW.index("name: Build launcher bundle"))

    def run_output(self, version):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            env = dict(os.environ, GITHUB_OUTPUT=str(output))
            env.pop("PACKAGE_VERSION", None)
            if version is not None:
                env["PACKAGE_VERSION"] = version
            result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", RUN],
                                    env=env, capture_output=True, text=True)
            return result, output.read_text() if output.exists() else ""

    def test_date_and_semantic_versions_reach_the_installer_output(self):
        for version in ("2026-10-11_v2", "0.1.0", "0.1.0-rc.1"):
            with self.subTest(version=version):
                result, output = self.run_output(version)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output, f"value={version}\n")

    def test_missing_or_empty_version_fails_before_emitting_output(self):
        for version in (None, ""):
            with self.subTest(version=version):
                result, output = self.run_output(version)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(output, "")


if __name__ == "__main__":
    unittest.main()
