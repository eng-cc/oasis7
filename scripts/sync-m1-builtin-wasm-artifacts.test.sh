#!/usr/bin/env bash
set -euo pipefail

# Run the real parser and directory setup in isolation; stop at the build boundary.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "$ROOT_DIR" <<'PY'
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class SyncCliTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="sync-wasm-cli-")
        self.root = Path(self.scratch.name)
        scripts = self.root / "scripts"
        scripts.mkdir()
        self.script = scripts / "sync-m1-builtin-wasm-artifacts.sh"
        shutil.copyfile(Path(sys.argv[1]) / "scripts" / self.script.name, self.script)
        build = scripts / "build-builtin-wasm-modules.sh"
        build.write_text("#!/usr/bin/env bash\necho fixture-build-boundary >&2\nexit 73\n")
        build.chmod(0o755)
        self.ids = self.root / "module_ids.txt"
        self.ids.write_text("fixture_module\n")
        self.distfs = self.root / "custom distfs root"
        self.out = self.root / "build output"

    def tearDown(self):
        self.scratch.cleanup()

    def run_sync(self, *args):
        env = os.environ.copy()
        env.update(OASIS7_WASM_CANONICAL_PLATFORMS="linux-x86_64",
                   OASIS7_WASM_CANONICAL_CONTAINER_PLATFORM="linux-x86_64")
        return subprocess.run(["bash", str(self.script), "--check",
                               "--module-ids-path", str(self.ids),
                               "--out-dir", str(self.out), *args],
                              env=env, text=True, capture_output=True)

    def test_distfs_root_creates_blobs_child_before_build(self):
        result = self.run_sync("--distfs-root", str(self.distfs))
        self.assertEqual(result.returncode, 73, result.stderr)
        self.assertIn("fixture-build-boundary", result.stderr)
        self.assertTrue((self.distfs / "blobs").is_dir())
        self.assertTrue(self.out.is_dir())
        self.assertFalse((self.distfs / "blobs" / "blobs").exists())

    def test_old_blobs_directory_flag_is_rejected_before_writes(self):
        result = self.run_sync("--artifact-dir", str(self.distfs / "blobs"))
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("unknown option: --artifact-dir", result.stderr)
        self.assertNotIn("fixture-build-boundary", result.stderr)
        self.assertFalse(self.distfs.exists())
        self.assertFalse(self.out.exists())

    def test_distfs_root_requires_value_before_writes(self):
        result = self.run_sync("--distfs-root")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("--distfs-root requires a value", result.stderr)
        self.assertFalse(self.out.exists())


unittest.main(argv=[sys.argv[0]], verbosity=2)
PY
