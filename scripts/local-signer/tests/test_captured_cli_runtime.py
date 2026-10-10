import importlib.util
from pathlib import Path
import sys
import unittest

SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE))
import installer
spec = importlib.util.spec_from_file_location("diagnostic_cli", SOURCE / "install-release.py")
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)

class CapturedCLIRuntime(unittest.TestCase):
    def test_runtime_guard_is_defined_in_captured_cli_namespace(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        pinned = "/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9"
        with patch.object(cli.sys, "flags", SimpleNamespace(isolated=1, no_site=1, dont_write_bytecode=1)), patch.object(cli.sys, "executable", pinned), patch.object(cli.sys, "version_info", (3, 9, 6)):
            with self.assertRaisesRegex(ValueError, "invalid immutable captured release"):
                cli.approved_modules("0" * 64, {})
