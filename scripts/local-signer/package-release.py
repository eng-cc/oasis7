#!/usr/bin/python3 -I
"""Package fixed reviewed release files; no host installation effects."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import sys


def parser():
    result = argparse.ArgumentParser()
    for name in ("binary-dir", "installer-dir", "output-dir", "release-id", "target", "source-revision"):
        result.add_argument("--" + name, required=True)
    return result


if __name__ == "__main__":
    args = vars(parser().parse_args())
    if os.geteuid() == 0:
        print(json.dumps({"status": "BLOCKED", "code": "ROOT_PACKAGING_FORBIDDEN", "host_mutated": False, "signing_enabled": False}))
        sys.exit(9)
    # Packaging is unprivileged; use the sibling source explicitly, never PYTHONPATH.
    spec = importlib.util.spec_from_file_location("installer", Path(__file__).absolute().with_name("installer.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    try:
        print(json.dumps(module.package_release(**args), sort_keys=True))
    except module.InstallError:
        print(json.dumps({"status": "BLOCKED", "code": "INVALID_RELEASE", "host_mutated": False, "signing_enabled": False}))
        sys.exit(9)
