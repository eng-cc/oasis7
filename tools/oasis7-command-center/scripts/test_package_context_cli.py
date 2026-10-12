"""Packaging regression: Cargo artifacts need not live under checkout/target."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("package-context-cli.sh")


class PackageContextCli(unittest.TestCase):
    def run_fixture(self, mode):
        temporary = tempfile.TemporaryDirectory(prefix="context-package-")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name) / "checkout"
        package = root / "tools/oasis7-command-center/scripts/package-context-cli.sh"
        package.parent.mkdir(parents=True)
        shutil.copy2(SCRIPT, package)
        wrapper = root / "scripts/cargo-dev.sh"
        wrapper.parent.mkdir()
        wrapper.write_text("""#!/usr/bin/env python3
import json, os, sys
assert sys.argv[1:] == ['build','--locked','--release','-p','oasis7_context','--message-format=json-render-diagnostics']
assert os.environ['OASIS7_SOURCE_HEAD_OID'] in os.environ['OASIS7_RELEASE_ID']
def artifact(path):
    return dict(reason='compiler-artifact', target=dict(name='oasis7_context',kind=['bin']),executable=path)
mode=os.environ['FIXTURE_CARGO_MODE']
print(json.dumps(dict(reason='compiler-artifact',target=dict(name='oasis7_context',kind=['lib']),executable=None)))
if mode != 'missing':
    print(json.dumps(artifact(os.environ['FIXTURE_ARTIFACT_PATH'])))
if mode == 'ambiguous':
    print(json.dumps(artifact(os.environ['FIXTURE_ARTIFACT_PATH']+'-other')))
if mode == 'failed':
    sys.exit(1)
""")
        wrapper.chmod(0o755)
        artifact = Path(temporary.name) / "shared cache/configured target/release/oasis7_context"
        artifact.parent.mkdir(parents=True)
        data = b"fixture release binary"
        artifact.write_bytes(data)
        def git(*args):
            return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True)
        git("init")
        git("config", "user.name", "Packaging Fixture")
        git("config", "user.email", "fixture@example.invalid")
        git("add", ".")
        git("-c", "commit.gpgsign=false", "commit", "-m", "Fixture")
        head = git("rev-parse", "HEAD").stdout.strip()
        environment = dict(os.environ, CI="true", FIXTURE_ARTIFACT_PATH=str(artifact), FIXTURE_CARGO_MODE=mode)
        result = subprocess.run(["bash", str(package)], cwd=root, env=environment, capture_output=True, text=True)
        return root, data, head, result

    def test_configured_shared_artifact_is_packaged_without_checkout_target(self):
        root, data, head, result = self.run_fixture("success")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((root / "target").exists())
        output = root / "tools/oasis7-command-center/src-tauri/resources/context-cli"
        self.assertEqual((output / "oasis7_context").read_bytes(), data)
        catalog = json.loads((output / "catalog.json").read_text())
        self.assertEqual(catalog["source_head_oid"], head)
        self.assertEqual(catalog["release_id"], "context-" + head)
        self.assertEqual(catalog["sha256"], hashlib.sha256(data).hexdigest())
        self.assertEqual(catalog["size_bytes"], len(data))

    def test_missing_ambiguous_and_failed_build_never_publish(self):
        for mode in ("missing", "ambiguous", "failed"):
            with self.subTest(mode=mode):
                root, _, _, result = self.run_fixture(mode)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root / "tools/oasis7-command-center/src-tauri/resources/context-cli").exists())


if __name__ == "__main__":
    unittest.main()
